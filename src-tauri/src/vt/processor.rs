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
    cell::CellAttrs,
    modes::{CharsetSlot, ModeState},
    screen_buffer::{DirtyRegion, ScreenBuffer, ScreenSnapshot, ScrollbackLineRef},
    search::{SearchMatch, SearchQuery},
};

mod dispatch;
mod emoji;
mod regional_indicator;
mod screen;
#[cfg(test)]
mod tests;
mod write;

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
pub(super) struct CursorPos {
    row: u16,
    col: u16,
    attrs: CellAttrs,
    charset_slot: CharsetSlot,
    /// DECAWM (auto-wrap mode) state at save time.
    decawm: bool,
    /// DECOM (origin mode) state at save time.
    decom: bool,
}

/// Lightweight screen metadata snapshot — no cell data cloned.
///
/// Returned by `VtProcessor::get_screen_meta()` and used by the partial-update
/// path in `build_screen_update_event` to access cursor state and dimensions
/// without the cost of a full `get_snapshot()` call.
pub struct ScreenMeta {
    pub cols: u16,
    pub rows: u16,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_visible: bool,
    pub cursor_shape: u8,
    pub cursor_blink: bool,
    pub scrollback_lines: usize,
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

    /// Returns `true` when the alternate screen is active.
    pub fn is_alt_screen_active(&self) -> bool {
        self.alt_active
    }

    /// Returns the number of lines currently stored in the scrollback buffer.
    pub fn scrollback_len(&self) -> usize {
        self.normal.scrollback_len()
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

    /// Get lightweight screen metadata without cloning any cell data.
    ///
    /// Used by the partial-update path in `build_screen_update_event` to obtain
    /// cursor and dimension information without paying the cost of a full snapshot.
    pub fn get_screen_meta(&self) -> ScreenMeta {
        let buf = self.active_buf_ref();
        let cursor = self.active_cursor();
        ScreenMeta {
            cols: buf.cols,
            rows: buf.rows,
            cursor_row: cursor.row,
            cursor_col: cursor.col,
            cursor_visible: self.modes.cursor_visible,
            cursor_shape: self.cursor_shape,
            cursor_blink: self.cursor_blink,
            scrollback_lines: buf.scrollback_len(),
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn flush_dirty(&mut self) -> DirtyRegion {
        let buf_dirty = self.active_buf_mut().take_dirty();
        self.pending_dirty.merge(&buf_dirty);
        std::mem::take(&mut self.pending_dirty)
    }

    pub fn active_buf_ref(&self) -> &ScreenBuffer {
        if self.alt_active {
            &self.alternate
        } else {
            &self.normal
        }
    }

    pub(super) fn active_buf_mut(&mut self) -> &mut ScreenBuffer {
        if self.alt_active {
            &mut self.alternate
        } else {
            &mut self.normal
        }
    }

    pub(super) fn active_cursor(&self) -> &CursorPos {
        if self.alt_active {
            &self.alt_cursor
        } else {
            &self.normal_cursor
        }
    }

    pub(super) fn active_cursor_mut(&mut self) -> &mut CursorPos {
        if self.alt_active {
            &mut self.alt_cursor
        } else {
            &mut self.normal_cursor
        }
    }

    pub(super) fn cursor_row(&self) -> u16 {
        self.active_cursor().row
    }

    pub(super) fn cursor_col(&self) -> u16 {
        self.active_cursor().col
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
