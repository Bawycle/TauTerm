// SPDX-License-Identifier: MPL-2.0

//! `VtProcessor` — the central VT/ANSI state machine.
//!
//! Implements `vte::Perform` to receive callbacks from the `vte` parser.
//! Owns `ScreenBuffer` (normal + alternate), `ModeState`, cursor position,
//! and dispatches to sub-handlers (SGR, OSC, charset, mouse mode tracking).
//!
//! Public API (§3.3 of ARCHITECTURE.md):
//! - `new(cols, rows)` — construct
//! - `process(bytes)` — feed raw PTY output, returns `DirtyRegion`
//! - `resize(cols, rows)` — resize terminal grid
//! - `get_snapshot()` — full screen snapshot
//! - `get_scrollback_line(index)` — single scrollback row
//! - `search(query)` — search scrollback

use std::sync::Arc;
use std::time::{Duration, Instant};

use vte::Parser;

use crate::vt::{
    cell::{Cell, CellAttrs},
    modes::{CharsetSlot, ModeState},
    screen_buffer::{DirtyRegion, ScreenBuffer, ScreenSnapshot, ScrollbackLineRef},
    search::{SearchMatch, SearchQuery},
};

/// Buffered base codepoint waiting for a potential variation selector (R6 / FS-VT-017).
///
/// We buffer width-1, non-ASCII codepoints so that FE0F (emoji presentation,
/// forces width=2) or FE0E (text presentation, keeps width=1) can adjust the
/// cell width *before* the cursor advances to its final position.
struct PendingEmoji {
    /// The base codepoint.
    ch: char,
    /// SGR attributes at the time the base was received.
    attrs: CellAttrs,
    /// Hyperlink at the time the base was received.
    hyperlink: Option<Arc<str>>,
    /// Column at which the base should be written (pre-wrap-adjusted).
    col: u16,
    /// Row at which the base should be written (pre-wrap-adjusted).
    row: u16,
}

/// Minimum interval between BEL events per pane (FS-VT-090).
const BELL_RATE_LIMIT: Duration = Duration::from_millis(100);

/// The central VT processing unit for one pane.
pub struct VtProcessor {
    // The vte parser (incremental, handles split sequences across reads).
    parser: Parser,
    // Normal screen buffer.
    normal: ScreenBuffer,
    // Alternate screen buffer (no scrollback).
    alternate: ScreenBuffer,
    // Whether the alternate screen is active.
    alt_active: bool,
    // Cursor position on the normal screen.
    normal_cursor: CursorPos,
    // Saved cursor (DECSC) on normal screen.
    saved_normal_cursor: Option<CursorPos>,
    // Cursor position on the alternate screen.
    alt_cursor: CursorPos,
    // Saved cursor (DECSC) on alternate screen.
    saved_alt_cursor: Option<CursorPos>,
    // Terminal mode state.
    modes: ModeState,
    // Saved mode state for the normal screen (restored on alt-screen exit).
    saved_normal_modes: Option<ModeState>,
    // Current SGR attributes.
    current_attrs: CellAttrs,
    // Terminal dimensions.
    cols: u16,
    rows: u16,
    // Title stack (OSC 22/23).
    title_stack: Vec<String>,
    // Current title.
    pub title: String,
    // Accumulated dirty region since last flush.
    pending_dirty: DirtyRegion,
    // Whether DECCKM or DECKPAM changed since last flush.
    pub mode_changed: bool,
    // Whether the OSC title changed since last flush.
    pub title_changed: bool,
    // DEC "delayed wrap" flag: set when the cursor reaches the last column after
    // a printable character. The next printed character will trigger an implicit
    // LF+CR before writing. Used to mark scrollback lines as soft-wrapped.
    pub wrap_pending: bool,
    // Current cursor shape as set by DECSCUSR (0–6). Default 0 = block.
    pub cursor_shape: u8,
    // Whether the cursor shape changed since last flush.
    pub cursor_shape_changed: bool,
    // Whether cursor blinking is enabled (DECSET 12 / DECRST 12).
    pub cursor_blink: bool,
    // Whether a rate-limited BEL event is pending since last flush.
    pub bell_pending: bool,
    // Timestamp of the last BEL that was allowed through (for rate-limiting).
    last_bell_instant: Option<Instant>,
    // OSC 8 hyperlink state: URI currently active (None = no active hyperlink).
    pub(super) current_hyperlink: Option<Arc<str>>,
    // OSC 8 hyperlink ID parameter: used to match multi-line runs with the same ID.
    pub(super) current_hyperlink_id: Option<Arc<str>>,
    // OSC 52 clipboard write policy flag.
    // When `true`, `ClipboardWrite` events are forwarded to the frontend.
    // Defaults to `false` (restrictive policy, FS-VT-075, SEC-OSC-002).
    pub allow_osc52_write: bool,
    // Pending OSC 52 clipboard write payload (set during VT processing, drained by caller).
    pub(super) pending_osc52_write: Option<String>,
    // Pending Regional Indicator for RI-pair detection (FS-VT-019, R8).
    //
    // When the first RI of a potential pair is received, it is written
    // provisionally as a 2-cell wide char and stored here.  On the second RI
    // (same row), the two are confirmed as a 2-cell flag unit; the first RI slot
    // is reused and the second RI is appended as a combining codepoint.  On any
    // non-RI input the provisional RI is downgraded to 1 cell (FS-VT-019: an
    // unpaired RI is narrow).
    //
    // Tuple: (codepoint, col, row) at which the provisional cell was written.
    pending_ri: Option<(char, u16, u16)>,
    // Responses to be written back to the PTY master (DSR, DA, CPR).
    // Drained by `take_responses()` in Task 1 of the PTY read task.
    // The write-lock MUST be released before writing these to the PTY master
    // to avoid deadlocking when the shell echoes back the response.
    pending_responses: Vec<Vec<u8>>,
    // Pending base codepoint for variation selector look-ahead (FS-VT-017, R6).
    //
    // FE0F/FE0E arrive *after* the base codepoint.  We buffer the base here so
    // that we can decide its final cell width once the selector is (or is not)
    // present.  When the *next* codepoint is *not* a variation selector the
    // buffered base is flushed at its natural unicode_width.
    //
    // A variation selector only affects codepoints that *could* be emoji —
    // concretely those that `unicode_width` returns 1 for and that are outside
    // the ASCII range.  ASCII and inherently-wide (width=2) codepoints are
    // written immediately and never buffered.
    //
    // Tuple: (codepoint, resolved_attrs, resolved_hyperlink, col, row) at the
    // moment the base was received.  col/row are the cursor position *before*
    // writing the base, stored so that a rétroactive width=2 upgrade can insert
    // the phantom cell correctly.
    pending_emoji: Option<PendingEmoji>,
}

/// Cursor position and attributes saved/restored by DECSC/DECRC (ESC 7 / ESC 8).
///
/// `attrs` holds the SGR attribute state at save time; `charset_slot` holds the
/// active G0/G1 slot; `decawm` holds the DECAWM (mode ?7) flag; `decom` holds
/// the DECOM (origin mode, ?6) flag. All are fully restored on DECRC.
#[derive(Debug, Clone, Default)]
struct CursorPos {
    row: u16,
    col: u16,
    attrs: CellAttrs,
    charset_slot: CharsetSlot,
    /// DECAWM (auto-wrap mode) state at save time.
    decawm: bool,
    /// DECOM (origin mode) state at save time.
    decom: bool,
}

/// Returns `true` if `c` is a codepoint that may appear in Unicode
/// `emoji-variation-sequences.txt` — i.e. one that can be followed by U+FE0F
/// (emoji presentation) or U+FE0E (text presentation) to alter its render width.
///
/// Only these codepoints are buffered for variation-selector look-ahead (R6 /
/// FS-VT-017).  All other codepoints — including box-drawing (U+2500–U+257F),
/// Latin extended, and currency symbols — are written immediately.
///
/// Source: Unicode 15.1 `emoji-variation-sequences.txt` (grouped by block).
fn is_emoji_vs_eligible(c: char) -> bool {
    // © and ® (Latin-1 Supplement)
    matches!(c, '\u{00A9}' | '\u{00AE}')
    // General Punctuation and Letterlike Symbols
    || matches!(c,
        '\u{203C}' | '\u{2049}' | '\u{2122}' | '\u{2139}'
    )
    // Arrows block
    || matches!(c,
        '\u{2194}'..='\u{2199}'
        | '\u{21A9}'..='\u{21AA}'
    )
    // Miscellaneous Technical
    || matches!(c,
        '\u{231A}'..='\u{231B}'
        | '\u{2328}'
        | '\u{23CF}'
        | '\u{23E9}'..='\u{23F3}'
        | '\u{23F8}'..='\u{23FA}'
    )
    // Enclosed Alphanumerics
    || matches!(c, '\u{24C2}')
    // Geometric Shapes
    || matches!(c,
        '\u{25AA}'..='\u{25AB}'
        | '\u{25B6}'
        | '\u{25C0}'
        | '\u{25FB}'..='\u{25FE}'
    )
    // Miscellaneous Symbols (U+2600–U+26FF)
    || matches!(c,
        '\u{2600}'..='\u{2604}'
        | '\u{260E}'
        | '\u{2611}'
        | '\u{2614}'..='\u{2615}'
        | '\u{2618}'
        | '\u{261D}'
        | '\u{2620}'
        | '\u{2622}'..='\u{2623}'
        | '\u{2626}'
        | '\u{262A}'
        | '\u{262E}'..='\u{262F}'
        | '\u{2638}'..='\u{263A}'
        | '\u{2640}'
        | '\u{2642}'
        | '\u{2648}'..='\u{2653}'
        | '\u{265F}'..='\u{2660}'
        | '\u{2663}'
        | '\u{2665}'..='\u{2666}'
        | '\u{2668}'
        | '\u{267B}'
        | '\u{267E}'..='\u{267F}'
        | '\u{2692}'..='\u{2697}'
        | '\u{2699}'
        | '\u{269B}'..='\u{269C}'
        | '\u{26A0}'..='\u{26A1}'
        | '\u{26A7}'
        | '\u{26AA}'..='\u{26AB}'
        | '\u{26B0}'..='\u{26B1}'
        | '\u{26BD}'..='\u{26BE}'
        | '\u{26C4}'..='\u{26C5}'
        | '\u{26CE}'..='\u{26CF}'
        | '\u{26D1}'
        | '\u{26D3}'..='\u{26D4}'
        | '\u{26E9}'..='\u{26EA}'
        | '\u{26F0}'..='\u{26F5}'
        | '\u{26F7}'..='\u{26FA}'
        | '\u{26FD}'
        // ☆ (U+2606) and ★ (U+2605) appear in emoji-variation-sequences.txt
        | '\u{2605}'..='\u{2606}'
    )
    // Dingbats (U+2700–U+27BF)
    || matches!(c,
        '\u{2702}'
        | '\u{2705}'
        | '\u{2708}'..='\u{270D}'
        | '\u{270F}'
        | '\u{2712}'
        | '\u{2714}'
        | '\u{2716}'
        | '\u{271D}'
        | '\u{2721}'
        | '\u{2728}'
        | '\u{2733}'..='\u{2734}'
        | '\u{2744}'
        | '\u{2747}'
        | '\u{274C}'
        | '\u{274E}'
        | '\u{2753}'..='\u{2755}'
        | '\u{2757}'
        | '\u{2763}'..='\u{2764}'
        | '\u{2795}'..='\u{2797}'
        | '\u{27A1}'
        | '\u{27B0}'
        | '\u{27BF}'
    )
    // Supplemental Arrows-B and other blocks
    || matches!(c,
        '\u{2934}'..='\u{2935}'
        | '\u{2B05}'..='\u{2B07}'
        | '\u{2B1B}'..='\u{2B1C}'
        | '\u{2B50}'
        | '\u{2B55}'
    )
    // CJK Symbols and Punctuation / Enclosed CJK
    || matches!(c,
        '\u{3030}'
        | '\u{303D}'
        | '\u{3297}'
        | '\u{3299}'
    )
}

impl VtProcessor {
    /// Create a new `VtProcessor` with explicit scrollback capacity (FS-SB-002).
    ///
    /// `scrollback_lines` is clamped to `MAX_SCROLLBACK_LINES` inside `ScreenBuffer::new`.
    pub fn new(cols: u16, rows: u16, scrollback_lines: usize) -> Self {
        Self {
            parser: Parser::new(),
            normal: ScreenBuffer::new(cols, rows, scrollback_lines),
            alternate: ScreenBuffer::new(cols, rows, 0),
            alt_active: false,
            normal_cursor: CursorPos::default(),
            saved_normal_cursor: None,
            alt_cursor: CursorPos::default(),
            saved_alt_cursor: None,
            modes: ModeState::new(rows),
            saved_normal_modes: None,
            current_attrs: CellAttrs::default(),
            cols,
            rows,
            title_stack: Vec::new(),
            title: String::new(),
            pending_dirty: DirtyRegion::default(),
            mode_changed: false,
            title_changed: false,
            wrap_pending: false,
            cursor_shape: 0,
            cursor_shape_changed: false,
            cursor_blink: false,
            bell_pending: false,
            last_bell_instant: None,
            current_hyperlink: None,
            current_hyperlink_id: None,
            allow_osc52_write: false,
            pending_osc52_write: None,
            pending_ri: None,
            pending_emoji: None,
            pending_responses: Vec::new(),
        }
    }

    /// Feed raw bytes from the PTY into the parser. Returns the dirty region.
    pub fn process(&mut self, bytes: &[u8]) -> DirtyRegion {
        // `vte::Parser::advance` takes `&mut dyn Perform`, so we cannot simultaneously
        // hold `&mut self.parser` and pass `self` as the Perform impl.
        // Solution: temporarily extract the parser from self, process, then restore it.
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        let mut bridge = VtPerformBridge { inner: self };
        parser.advance(&mut bridge, bytes);
        self.parser = parser;
        self.flush_dirty()
    }

    /// Resize the terminal. Updates both screen buffers and resets scroll region.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        self.normal.resize(cols, rows);
        self.alternate.resize(cols, rows);
        self.modes.reset_scroll_region(rows);
        // Clamp cursors.
        self.normal_cursor.row = self.normal_cursor.row.min(rows.saturating_sub(1));
        self.normal_cursor.col = self.normal_cursor.col.min(cols.saturating_sub(1));
        self.alt_cursor.row = self.alt_cursor.row.min(rows.saturating_sub(1));
        self.alt_cursor.col = self.alt_cursor.col.min(cols.saturating_sub(1));
    }

    /// Get a full screen snapshot for `get_pane_screen_snapshot`.
    pub fn get_snapshot(&self) -> ScreenSnapshot {
        let buf = self.active_buf_ref();
        let cursor = self.active_cursor();
        buf.snapshot(
            cursor.row,
            cursor.col,
            self.modes.cursor_visible,
            self.cursor_shape,
            0, // scroll_offset
        )
    }

    /// Get a scrollback line by 0-based index (oldest first).
    ///
    /// Returns a `ScrollbackLineRef` that includes both the cell content and the
    /// `soft_wrapped` flag (FS-SB-011). Returns `None` when `index` is out of range.
    pub fn get_scrollback_line(&self, index: usize) -> Option<ScrollbackLineRef> {
        self.normal
            .get_scrollback_line(index)
            .map(|sl| ScrollbackLineRef {
                cells: sl.cells.clone(),
                soft_wrapped: sl.soft_wrapped,
            })
    }

    /// Search the scrollback buffer.
    ///
    /// Returns an empty `Vec` when the alternate screen is active: the alternate
    /// buffer has no scrollback (FS-SB-004), so no matches are possible (TEST-VT-022).
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchMatch> {
        if self.alt_active {
            return Vec::new();
        }
        use crate::vt::search::search_scrollback;
        search_scrollback(self.normal.scrollback_iter(), query)
    }

    /// If the OSC title changed since last call, returns the new title and resets the flag.
    pub fn take_title_changed(&mut self) -> Option<String> {
        if self.title_changed {
            self.title_changed = false;
            Some(self.title.clone())
        } else {
            None
        }
    }

    /// If the cursor shape changed since last call, returns the new shape value
    /// (DECSCUSR 0–6) and resets the flag (FS-VT-030).
    pub fn take_cursor_shape_changed(&mut self) -> Option<u8> {
        if self.cursor_shape_changed {
            self.cursor_shape_changed = false;
            Some(self.cursor_shape)
        } else {
            None
        }
    }

    /// If a rate-limited BEL is pending since last call, returns `true` and
    /// resets the flag (FS-VT-090).
    pub fn take_bell_pending(&mut self) -> bool {
        if self.bell_pending {
            self.bell_pending = false;
            true
        } else {
            false
        }
    }

    /// Register a BEL event, respecting the 100 ms rate limit.
    /// Sets `bell_pending` only if enough time has elapsed since the last allowed BEL.
    pub(super) fn register_bell(&mut self) {
        let now = Instant::now();
        let allowed = match self.last_bell_instant {
            None => true,
            Some(last) => now.duration_since(last) >= BELL_RATE_LIMIT,
        };
        if allowed {
            self.last_bell_instant = Some(now);
            self.bell_pending = true;
        }
    }

    /// All frontend-relevant mode flags — used to emit `mode-state-changed`.
    pub fn mode_state(&self) -> &ModeState {
        &self.modes
    }

    /// If an OSC 52 clipboard write is pending (and `allow_osc52_write` is true),
    /// drains and returns the payload. Returns `None` otherwise.
    pub fn take_osc52_write(&mut self) -> Option<String> {
        self.pending_osc52_write.take()
    }

    /// Drain all pending PTY responses (sequences to write back to the PTY master).
    ///
    /// Called by Task 1 of the PTY read task immediately after `process()`,
    /// while the write-lock is still held. The caller MUST release the
    /// write-lock before writing the returned bytes to the PTY master to
    /// avoid a deadlock when the shell echoes back the response.
    ///
    /// Covers: DSR ready (`CSI 5n` → `\x1b[0n`), CPR (`CSI 6n` → `\x1b[row;colR`),
    /// and Primary DA (`CSI c` / `CSI 0c` → `\x1b[?1;2c`).
    pub fn take_responses(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.pending_responses)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn flush_dirty(&mut self) -> DirtyRegion {
        let buf_dirty = self.active_buf_mut().take_dirty();
        self.pending_dirty.merge(&buf_dirty);
        std::mem::take(&mut self.pending_dirty)
    }

    fn active_buf_ref(&self) -> &ScreenBuffer {
        if self.alt_active {
            &self.alternate
        } else {
            &self.normal
        }
    }

    fn active_buf_mut(&mut self) -> &mut ScreenBuffer {
        if self.alt_active {
            &mut self.alternate
        } else {
            &mut self.normal
        }
    }

    fn active_cursor(&self) -> &CursorPos {
        if self.alt_active {
            &self.alt_cursor
        } else {
            &self.normal_cursor
        }
    }

    fn active_cursor_mut(&mut self) -> &mut CursorPos {
        if self.alt_active {
            &mut self.alt_cursor
        } else {
            &mut self.normal_cursor
        }
    }

    fn cursor_row(&self) -> u16 {
        self.active_cursor().row
    }

    fn cursor_col(&self) -> u16 {
        self.active_cursor().col
    }

    /// Write the current character to the active buffer at cursor position, then advance.
    ///
    /// Handles the following special cases before the main write path:
    ///
    /// - **R7 (FS-VT-018)**: Skin-tone modifiers U+1F3FB–U+1F3FF are treated as
    ///   combining marks (width=0): they attach to the preceding cell without advancing
    ///   the cursor.  `unicode_width` would return 2 for these codepoints, so the check
    ///   must occur *before* the width lookup.
    ///
    /// - **R8 (FS-VT-019)**: Regional Indicator (RI) codepoints U+1F1E6–U+1F1FF are
    ///   written provisionally as 2-cell chars.  A *second consecutive* RI on the same
    ///   row confirms a flag pair in those same 2 cells.  A non-RI codepoint confirms
    ///   the previous RI as a 1-cell narrow char (spec: unpaired RI = narrow).
    ///
    /// - **R6 (FS-VT-017)**: Width-1 non-ASCII codepoints that could be emoji are
    ///   buffered in `pending_emoji` until the next codepoint is known.  U+FE0F forces
    ///   width=2 (emoji presentation); U+FE0E keeps width=1 (text presentation).  Any
    ///   other codepoint flushes the buffer at the natural width and is then processed
    ///   normally.  ASCII and inherently-wide (width=2) codepoints are never buffered.
    fn write_char(&mut self, c: char) {
        // --- R7: skin-tone modifiers are combining marks (width=0) ------------------
        if matches!(c, '\u{1F3FB}'..='\u{1F3FF}') {
            // A skin-tone modifier is not a second Regional Indicator, so a pending
            // lone RI must be committed as narrow (width=1), not as a confirmed flag.
            self.flush_pending_ri_narrow();
            self.flush_pending_emoji(None);
            let row = self.cursor_row();
            let col = self.cursor_col();
            // Locate the base cell: step back from the cursor, skipping phantom cells
            // (which are the trailing slot of wide characters).  A skin-tone modifier
            // must attach to the *base* cell of the preceding grapheme, not its phantom.
            let target_col = if col > 0 {
                let mut tc = col - 1;
                // If the cell immediately before the cursor is a phantom, walk back
                // one more position to reach the base cell.
                if self
                    .active_buf_ref()
                    .get(row, tc)
                    .is_some_and(|cell| cell.is_phantom())
                    && tc > 0
                {
                    tc -= 1;
                }
                tc
            } else {
                0
            };
            if let Some(cell) = self.active_buf_mut().get_mut(row, target_col) {
                cell.grapheme.push(c);
            }
            return;
        }

        // --- R8: Regional Indicator pair detection ----------------------------------
        if matches!(c, '\u{1F1E6}'..='\u{1F1FF}') {
            self.flush_pending_emoji(None);
            self.handle_regional_indicator(c);
            return;
        }

        // Any non-RI codepoint confirms a pending RI as narrow (unpaired).
        if self.pending_ri.is_some() {
            self.flush_pending_ri_narrow();
        }

        // --- R6: variation selectors ------------------------------------------------
        if c == '\u{FE0F}' {
            // Emoji presentation: upgrade pending base to width=2 (if eligible).
            self.flush_pending_emoji(Some(2));
            return;
        }
        if c == '\u{FE0E}' {
            // Text presentation: flush pending base at width=1.
            self.flush_pending_emoji(Some(1));
            return;
        }

        // Any other codepoint: flush any pending emoji base at its natural width,
        // then proceed to write `c`.
        self.flush_pending_emoji(None);

        // --- Compute width ----------------------------------------------------------
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) as u8;

        // --- Combining / zero-width characters (width == 0) ------------------------
        // Attach the combining mark to the previous cell (or the current cell when
        // at the start of a line) without advancing the cursor (FS-VT-012/013).
        if char_width == 0 {
            let row = self.cursor_row();
            let col = self.cursor_col();
            let (target_row, target_col) = if col > 0 { (row, col - 1) } else { (row, 0) };
            if let Some(cell) = self.active_buf_mut().get_mut(target_row, target_col) {
                cell.grapheme.push(c);
            }
            return;
        }

        // --- R6 buffering: width-1, potentially-emoji codepoints -------------------
        // Buffer codepoints that are width=1 by unicode_width but *could* have an
        // emoji variation sequence (FE0F / FE0E).  Only codepoints listed in the
        // Unicode `emoji-variation-sequences.txt` data file need to be buffered.
        // ASCII and non-emoji blocks (e.g. box-drawing U+2500–U+257F, Latin extended,
        // currency symbols) are written immediately without buffering.
        if char_width == 1 && is_emoji_vs_eligible(c) {
            // Snapshot the cursor position *after* any pending wrap is applied so
            // that the stored col/row are the actual write position.
            let (write_row, write_col) = self.apply_wrap_pending();
            let attrs = self.current_attrs;
            let hyperlink = self.current_hyperlink.clone();
            self.pending_emoji = Some(PendingEmoji {
                ch: c,
                attrs,
                hyperlink,
                col: write_col,
                row: write_row,
            });
            return;
        }

        // --- Normal write path -----------------------------------------------------
        self.write_char_at_width(c, char_width);
    }

    /// Apply the DEC delayed-wrap (if set) and return the resulting (row, col).
    ///
    /// If `wrap_pending` is true *and* DECAWM is enabled, the cursor is moved to
    /// the first column of the next row (or scrolled).  In all cases the final
    /// (row, col) after the potential wrap is returned.  This helper does *not*
    /// write anything to the grid.
    fn apply_wrap_pending(&mut self) -> (u16, u16) {
        let row = self.cursor_row();
        let _col = self.cursor_col();
        if self.wrap_pending && self.modes.decawm {
            self.wrap_pending = false;
            let (top, bottom) = self.modes.scroll_region;
            let is_full = top == 0 && bottom == self.rows.saturating_sub(1);
            if row == bottom {
                self.active_buf_mut()
                    .scroll_up(top, bottom, 1, is_full, true);
            } else {
                self.active_cursor_mut().row = (row + 1).min(self.rows.saturating_sub(1));
            }
            self.active_cursor_mut().col = 0;
        }
        (self.cursor_row(), self.cursor_col())
    }

    /// Core write: place `c` at the current cursor position with explicit `width`,
    /// write a phantom cell if `width == 2`, then advance the cursor.
    ///
    /// Applies the DEC delayed-wrap before writing.
    fn write_char_at_width(&mut self, c: char, width: u8) {
        let (row, col) = self.apply_wrap_pending();
        let attrs = self.current_attrs;
        let cols = self.cols;
        let hyperlink = self.current_hyperlink.clone();

        if let Some(cell) = self.active_buf_mut().get_mut(row, col) {
            cell.grapheme = c.to_string();
            cell.attrs = attrs;
            cell.width = width;
            cell.hyperlink = hyperlink;
        }

        // Place phantom cell for wide characters (FS-VT-011).
        if width == 2
            && col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(row, col + 1)
        {
            *cell = Cell::phantom();
        }

        // Advance cursor.
        let new_col = col + width as u16;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols - 1;
        } else {
            self.active_cursor_mut().col = new_col;
        }
    }

    // --- R6 helpers ----------------------------------------------------------------

    /// Flush the pending emoji base at the given `forced_width`, or at its natural
    /// `unicode_width` when `forced_width` is `None`.
    ///
    /// A `forced_width` of `Some(2)` comes from FE0F and must only take effect when
    /// the base is a codepoint that *could* be an emoji (non-ASCII, not already
    /// wide).  `Some(1)` from FE0E and `None` always use the natural / forced width
    /// as-is.
    fn flush_pending_emoji(&mut self, forced_width: Option<u8>) {
        let Some(pe) = self.pending_emoji.take() else {
            return;
        };
        let width = match forced_width {
            Some(2) => {
                // Only widen if the base is a codepoint that could be emoji.
                // We define "could be emoji" as non-ASCII (already guaranteed by the
                // buffering condition) and outside the 0x0000–0x00FF Latin range.
                // A simple heuristic: anything >= U+00A0 and non-ASCII is eligible.
                if pe.ch as u32 >= 0x00A0 { 2 } else { 1 }
            }
            Some(w) => w,
            None => unicode_width::UnicodeWidthChar::width(pe.ch).unwrap_or(1) as u8,
        };

        // Position the cursor at the stored write position before delegating to the
        // width-aware write.  The wrap was already applied when the base was buffered,
        // so we can set the cursor directly.
        self.active_cursor_mut().col = pe.col;
        self.active_cursor_mut().row = pe.row;

        let cols = self.cols;
        if let Some(cell) = self.active_buf_mut().get_mut(pe.row, pe.col) {
            cell.grapheme = pe.ch.to_string();
            cell.attrs = pe.attrs;
            cell.width = width;
            cell.hyperlink = pe.hyperlink.clone();
        }
        if width == 2
            && pe.col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(pe.row, pe.col + 1)
        {
            *cell = Cell::phantom();
        }
        let new_col = pe.col + width as u16;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols.saturating_sub(1);
        } else {
            self.active_cursor_mut().col = new_col;
        }
    }

    // --- R8 helpers ----------------------------------------------------------------

    /// Process a Regional Indicator codepoint.
    ///
    /// - If no RI is pending: write provisionally as 2-cell wide char and store in
    ///   `pending_ri`.
    /// - If an RI is pending on the *same row*: confirm as a flag pair — rewrite
    ///   both RIs into the same 2 cells (grapheme = base + second RI), clear pending.
    /// - If the pending RI is on a *different row*: confirm the previous as narrow
    ///   (1-cell) and start a fresh provisional for the new RI.
    fn handle_regional_indicator(&mut self, c: char) {
        let current_row = self.cursor_row();

        if let Some((prev_ch, prev_col, prev_row)) = self.pending_ri.take() {
            if prev_row == current_row {
                // Second RI on the same row → confirmed flag pair.
                // The first RI was written provisionally at (prev_row, prev_col) as width=2.
                // We now update the grapheme of that cell to include both codepoints and
                // leave the cursor at prev_col + 2 (the phantom cell stays in place).
                let prev_attrs = self
                    .active_buf_ref()
                    .get(prev_row, prev_col)
                    .map(|cell| cell.attrs)
                    .unwrap_or_default();
                let prev_hyperlink = self
                    .active_buf_ref()
                    .get(prev_row, prev_col)
                    .and_then(|cell| cell.hyperlink.clone());
                let mut flag_grapheme = String::new();
                flag_grapheme.push(prev_ch);
                flag_grapheme.push(c);
                // Rewrite the base cell with the full flag grapheme; phantom stays.
                if let Some(cell) = self.active_buf_mut().get_mut(prev_row, prev_col) {
                    cell.grapheme = flag_grapheme;
                    cell.attrs = prev_attrs;
                    cell.width = 2;
                    cell.hyperlink = prev_hyperlink;
                }
                // Cursor is already at prev_col + 2 from the provisional write.
                return;
            } else {
                // Different row: confirm the previous RI as narrow (1-cell).
                self.confirm_ri_narrow(prev_ch, prev_col, prev_row);
            }
        }

        // Write the new RI provisionally as a 2-cell wide char and store pending.
        let (write_row, write_col) = self.apply_wrap_pending();
        let attrs = self.current_attrs;
        let hyperlink = self.current_hyperlink.clone();
        let cols = self.cols;
        if let Some(cell) = self.active_buf_mut().get_mut(write_row, write_col) {
            cell.grapheme = c.to_string();
            cell.attrs = attrs;
            cell.width = 2;
            cell.hyperlink = hyperlink;
        }
        if write_col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(write_row, write_col + 1)
        {
            *cell = Cell::phantom();
        }
        let new_col = write_col + 2;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols.saturating_sub(1);
        } else {
            self.active_cursor_mut().col = new_col;
        }
        self.pending_ri = Some((c, write_col, write_row));
    }

    /// Confirm a pending RI as narrow (width=1) — called when followed by non-RI.
    fn flush_pending_ri_narrow(&mut self) {
        if let Some((ch, col, row)) = self.pending_ri.take() {
            self.confirm_ri_narrow(ch, col, row);
        }
    }

    /// Rewrite the RI cell at `(row, col)` as width=1 and clear the phantom.
    fn confirm_ri_narrow(&mut self, ch: char, col: u16, row: u16) {
        let attrs = self
            .active_buf_ref()
            .get(row, col)
            .map(|c| c.attrs)
            .unwrap_or_default();
        let hyperlink = self
            .active_buf_ref()
            .get(row, col)
            .and_then(|c| c.hyperlink.clone());
        let cols = self.cols;
        // Rewrite the RI cell as narrow (width=1).
        if let Some(cell) = self.active_buf_mut().get_mut(row, col) {
            cell.grapheme = ch.to_string();
            cell.attrs = attrs;
            cell.width = 1;
            cell.hyperlink = hyperlink;
        }
        // Clear the former phantom cell.
        if col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(row, col + 1)
            && cell.is_phantom()
        {
            *cell = Cell::default();
        }
        // Move the cursor to just after the narrow RI cell.
        self.active_cursor_mut().row = row;
        let new_col = col + 1;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols.saturating_sub(1);
        } else {
            self.active_cursor_mut().col = new_col;
        }
    }

    /// Switch to alternate screen (mode 1049 / 47 / 1047).
    fn enter_alternate(&mut self, save_cursor: bool) {
        if self.alt_active {
            return;
        }
        if save_cursor {
            self.saved_normal_cursor = Some(self.normal_cursor.clone());
        }
        self.saved_normal_modes = Some(self.modes.clone());
        self.modes = ModeState::new(self.rows);
        self.alt_active = true;
        self.alternate.erase_lines(0, self.rows);
        self.alt_cursor = CursorPos::default();
        self.pending_dirty.mark_full_redraw();
    }

    /// Return to normal screen.
    fn leave_alternate(&mut self, restore_cursor: bool) {
        if !self.alt_active {
            return;
        }
        self.alt_active = false;
        if let Some(saved) = self.saved_normal_modes.take() {
            self.modes = saved;
        }
        // FS-VT-086: force mouse reporting off on alt-screen exit.
        // Guards against apps that activate mouse tracking but crash without
        // sending ?1000l (or equivalent reset). This makes mouse capture opt-in
        // per screen, not sticky across screen switches.
        self.modes.mouse_reporting = crate::vt::modes::MouseReportingMode::None;
        if restore_cursor && let Some(saved) = self.saved_normal_cursor.take() {
            self.normal_cursor = saved;
        }
        self.pending_dirty.mark_full_redraw();
    }
}

// ---------------------------------------------------------------------------
// vte::Perform bridge
// ---------------------------------------------------------------------------

/// Wrapper that implements `vte::Perform` and delegates to `VtProcessor` methods.
/// The parser is extracted from `VtProcessor` before `advance()` is called to
/// avoid the simultaneous mutable borrow on `self.parser` and `self`.
struct VtPerformBridge<'a> {
    inner: &'a mut VtProcessor,
}

mod dispatch;
#[cfg(test)]
mod tests;
