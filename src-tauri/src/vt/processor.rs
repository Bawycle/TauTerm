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
    screen_buffer::{DirtyRegion, ScreenBuffer, ScreenSnapshot},
    search::{SearchMatch, SearchQuery},
};

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
}

/// Cursor position and attributes.
/// `attrs` and `charset_slot` are saved/restored with DECSC/DECRC — not yet wired
/// in the full implementation, but present for correctness of the saved state.
#[derive(Debug, Clone, Default)]
struct CursorPos {
    row: u16,
    col: u16,
    #[allow(dead_code)]
    attrs: CellAttrs,
    #[allow(dead_code)]
    charset_slot: CharsetSlot,
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
            bell_pending: false,
            last_bell_instant: None,
            current_hyperlink: None,
            current_hyperlink_id: None,
            allow_osc52_write: false,
            pending_osc52_write: None,
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
    pub fn get_scrollback_line(&self, index: usize) -> Option<Vec<Cell>> {
        self.normal
            .get_scrollback_line(index)
            .map(|sl| sl.cells.clone())
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
    fn write_char(&mut self, c: char) {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) as u8;
        let row = self.cursor_row();
        let col = self.cursor_col();
        // Extract attrs before the mutable borrow on the buffer.
        let attrs = self.current_attrs;
        let cols = self.cols;
        // Snapshot hyperlink — Arc clone is cheap.
        let hyperlink = self.current_hyperlink.clone();

        // Combining / zero-width characters (width == 0):
        // Attach the combining mark to the previous cell (or the current cell when
        // at the start of a line) without advancing the cursor (FS-VT-012/013).
        if char_width == 0 {
            // Target: the cell immediately before the cursor, or (0,0) when at start.
            let (target_row, target_col) = if col > 0 { (row, col - 1) } else { (row, 0) };
            if let Some(cell) = self.active_buf_mut().get_mut(target_row, target_col) {
                // Append the combining character to the base grapheme of that cell.
                cell.grapheme.push(c);
            }
            // Cursor does not advance for zero-width characters.
            return;
        }

        // DEC "delayed wrap": if the previous character set wrap_pending, apply
        // the implicit LF+CR now before writing the new character.
        // DECAWM (mode ?7) controls whether delayed-wrap is active at all.
        let (mut row, mut col) = (row, col);
        if self.wrap_pending && self.modes.decawm {
            self.wrap_pending = false;
            let (top, bottom) = self.modes.scroll_region;
            let is_full = top == 0 && bottom == self.rows - 1;
            if row == bottom {
                // The line being scrolled out is a soft-wrapped line.
                self.active_buf_mut()
                    .scroll_up(top, bottom, 1, is_full, true);
            } else {
                self.active_cursor_mut().row = (row + 1).min(self.rows - 1);
            }
            self.active_cursor_mut().col = 0;
            row = self.cursor_row();
            col = self.cursor_col();
        }

        if let Some(cell) = self.active_buf_mut().get_mut(row, col) {
            cell.grapheme = c.to_string();
            cell.attrs = attrs;
            cell.width = char_width;
            cell.hyperlink = hyperlink.clone();
        }

        // Place phantom cell for wide characters (FS-VT-011).
        if char_width == 2
            && col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(row, col + 1)
        {
            *cell = Cell::phantom();
        }

        // Advance cursor.
        let advance = char_width as u16;
        let new_col = col + advance;
        if new_col >= cols {
            if self.modes.decawm {
                // DECAWM on: set the delayed-wrap flag.
                // The actual line wrap occurs when the next printable character arrives.
                self.wrap_pending = true;
            }
            // In both cases (DECAWM on or off), cursor stays at the last column.
            self.active_cursor_mut().col = cols - 1;
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
