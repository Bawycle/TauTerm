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

use vte::{Parser, Perform};

use crate::vt::{
    cell::{Cell, CellAttrs},
    charset::translate_dec_special,
    modes::{Charset, CharsetSlot, ModeState, MouseEncoding, MouseReportingMode},
    osc::{OscAction, parse_osc},
    screen_buffer::{DirtyRegion, ScreenBuffer, ScreenSnapshot},
    search::{SearchMatch, SearchQuery},
    sgr::apply_sgr,
};

/// Default scrollback limit — overridden by preferences in the full implementation.
const DEFAULT_SCROLLBACK_LIMIT: usize = 10_000;

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
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: Parser::new(),
            normal: ScreenBuffer::new(cols, rows, DEFAULT_SCROLLBACK_LIMIT),
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
            0, // shape — stub for now
            0, // scroll_offset
        )
    }

    /// Get a scrollback line by 0-based index (oldest first).
    pub fn get_scrollback_line(&self, index: usize) -> Option<Vec<Cell>> {
        self.normal.get_scrollback_line(index).cloned()
    }

    /// Search the scrollback buffer.
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchMatch> {
        let _ = query; // TODO: implement in full pass
        Vec::new()
    }

    /// DECCKM and DECKPAM state — used to emit `mode-state-changed`.
    pub fn mode_flags(&self) -> (bool, bool) {
        (self.modes.decckm, self.modes.deckpam)
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
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) as u8;
        let row = self.cursor_row();
        let col = self.cursor_col();
        // Extract attrs before the mutable borrow on the buffer.
        let attrs = self.current_attrs;
        let cols = self.cols;

        if let Some(cell) = self.active_buf_mut().get_mut(row, col) {
            cell.grapheme = c.to_string();
            cell.attrs = attrs;
            cell.width = width;
        }

        // Place phantom cell for wide characters.
        if width == 2 && col + 1 < cols {
            if let Some(cell) = self.active_buf_mut().get_mut(row, col + 1) {
                *cell = Cell::phantom();
            }
        }

        // Advance cursor.
        let advance = width.max(1) as u16;
        let new_col = col + advance;
        if new_col >= cols {
            // Wrap — handled by next LF/print sequence.
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
        if restore_cursor {
            if let Some(saved) = self.saved_normal_cursor.take() {
                self.normal_cursor = saved;
            }
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

impl Perform for VtPerformBridge<'_> {
    fn print(&mut self, c: char) {
        let p = &mut self.inner;
        // Apply DEC Special Graphics mapping if active.
        let mapped_c = if (c as u8) >= 0x60 {
            let active_charset = match p.modes.charset_slot {
                CharsetSlot::G0 => p.modes.g0,
                CharsetSlot::G1 => p.modes.g1,
            };
            if active_charset == Charset::DecSpecialGraphics {
                translate_dec_special(c as u8)
            } else {
                c
            }
        } else {
            c
        };
        p.write_char(mapped_c);
    }

    fn execute(&mut self, byte: u8) {
        let p = &mut self.inner;
        let row = p.cursor_row();
        let col = p.cursor_col();
        match byte {
            // BEL (0x07) — handled in PtyReadTask via dirty region + notification.
            0x07 => {}
            // BS (0x08)
            0x08 => {
                if col > 0 {
                    p.active_cursor_mut().col = col - 1;
                }
            }
            // HT (0x09) — advance to next tab stop (every 8 columns).
            0x09 => {
                let next_tab = ((col / 8) + 1) * 8;
                p.active_cursor_mut().col = next_tab.min(p.cols - 1);
            }
            // LF / VT / FF (0x0A / 0x0B / 0x0C)
            0x0A | 0x0B | 0x0C => {
                let (top, bottom) = p.modes.scroll_region;
                let is_full = top == 0 && bottom == p.rows - 1;
                if row == bottom {
                    p.active_buf_mut().scroll_up(top, bottom, 1, is_full);
                } else {
                    p.active_cursor_mut().row = (row + 1).min(p.rows - 1);
                }
            }
            // CR (0x0D)
            0x0D => {
                p.active_cursor_mut().col = 0;
            }
            // SI (0x0F) — switch to G0
            0x0F => {
                p.modes.charset_slot = CharsetSlot::G0;
            }
            // SO (0x0E) — switch to G1
            0x0E => {
                p.modes.charset_slot = CharsetSlot::G1;
            }
            _ => {} // Ignore other C0 controls.
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        // DCS hook — no-op for v1 (all DCS sequences ignored except DECRQSS handled in unhook).
    }

    fn put(&mut self, _byte: u8) {
        // DCS data byte — no-op for v1.
    }

    fn unhook(&mut self) {
        // DCS unhook — no-op for v1.
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let raw: Vec<u8> = params.join(&b';');
        match parse_osc(&raw) {
            OscAction::SetTitle(title) => {
                self.inner.title = title;
            }
            OscAction::PushTitle => {
                self.inner.title_stack.push(self.inner.title.clone());
            }
            OscAction::PopTitle => {
                if let Some(t) = self.inner.title_stack.pop() {
                    self.inner.title = t;
                }
            }
            OscAction::SetHyperlink { .. } => {
                // TODO: store hyperlink state on current cell position.
            }
            OscAction::ClipboardWrite(_text) => {
                // TODO: forward to clipboard backend respecting allow_osc52_write policy.
            }
            OscAction::Ignore => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let p = &mut self.inner;
        // Extract first param (0 if absent).
        let param0 = params
            .iter()
            .next()
            .and_then(|p| p.first().copied())
            .unwrap_or(0);
        let param1 = params
            .iter()
            .nth(1)
            .and_then(|p| p.first().copied())
            .unwrap_or(0);

        match (intermediates, action) {
            // SGR — CSI Pm m
            ([], 'm') => {
                apply_sgr(params, &mut p.current_attrs);
            }
            // CUU — cursor up
            ([], 'A') => {
                let n = param0.max(1) as u16;
                let (top, _) = p.modes.scroll_region;
                p.active_cursor_mut().row = p.cursor_row().saturating_sub(n).max(top);
            }
            // CUD — cursor down
            ([], 'B') => {
                let n = param0.max(1) as u16;
                let (_, bottom) = p.modes.scroll_region;
                p.active_cursor_mut().row = (p.cursor_row() + n).min(bottom);
            }
            // CUF — cursor forward
            ([], 'C') => {
                let n = param0.max(1) as u16;
                p.active_cursor_mut().col = (p.cursor_col() + n).min(p.cols - 1);
            }
            // CUB — cursor back
            ([], 'D') => {
                let n = param0.max(1) as u16;
                p.active_cursor_mut().col = p.cursor_col().saturating_sub(n);
            }
            // CUP / HVP — cursor position
            ([], 'H') | ([], 'f') => {
                let row = (param0.max(1) as u16).saturating_sub(1).min(p.rows - 1);
                let col = (param1.max(1) as u16).saturating_sub(1).min(p.cols - 1);
                p.active_cursor_mut().row = row;
                p.active_cursor_mut().col = col;
            }
            // ED — erase in display
            ([], 'J') => {
                let row = p.cursor_row();
                let col = p.cursor_col();
                let cols = p.cols;
                let rows = p.rows;
                match param0 {
                    0 => {
                        // From cursor to end of screen.
                        p.active_buf_mut().erase_cells(row, col, cols);
                        p.active_buf_mut().erase_lines(row + 1, rows);
                    }
                    1 => {
                        // From start to cursor.
                        p.active_buf_mut().erase_lines(0, row);
                        p.active_buf_mut().erase_cells(row, 0, col + 1);
                    }
                    2 | 3 => {
                        // Entire screen.
                        p.active_buf_mut().erase_lines(0, rows);
                    }
                    _ => {}
                }
            }
            // EL — erase in line
            ([], 'K') => {
                let row = p.cursor_row();
                let col = p.cursor_col();
                let cols = p.cols;
                match param0 {
                    0 => p.active_buf_mut().erase_cells(row, col, cols),
                    1 => p.active_buf_mut().erase_cells(row, 0, col + 1),
                    2 => p.active_buf_mut().erase_cells(row, 0, cols),
                    _ => {}
                }
            }
            // DECSTBM — set scroll region
            ([], 'r') => {
                let top = (param0.max(1) as u16).saturating_sub(1);
                let bottom = if param1 == 0 {
                    p.rows - 1
                } else {
                    (param1 as u16).saturating_sub(1)
                };
                if top < bottom && bottom < p.rows {
                    p.modes.scroll_region = (top, bottom);
                }
                // Move cursor to home position.
                p.active_cursor_mut().row = 0;
                p.active_cursor_mut().col = 0;
            }
            // DECSET — DEC private mode set
            ([b'?'], 'h') => {
                for param in params.iter() {
                    let mode = param.first().copied().unwrap_or(0);
                    let prev_decckm = p.modes.decckm;
                    match mode {
                        1 => p.modes.decckm = true,
                        9 => p.modes.mouse_reporting = MouseReportingMode::X10,
                        25 => p.modes.cursor_visible = true,
                        47 => p.enter_alternate(false),
                        1000 => p.modes.mouse_reporting = MouseReportingMode::Normal,
                        1002 => p.modes.mouse_reporting = MouseReportingMode::ButtonEvent,
                        1003 => p.modes.mouse_reporting = MouseReportingMode::AnyEvent,
                        1004 => p.modes.focus_events = true,
                        1006 => p.modes.mouse_encoding = MouseEncoding::Sgr,
                        1015 => p.modes.mouse_encoding = MouseEncoding::Urxvt,
                        1047 => p.enter_alternate(false),
                        1049 => p.enter_alternate(true),
                        2004 => p.modes.bracketed_paste = true,
                        _ => {}
                    }
                    if p.modes.decckm != prev_decckm {
                        p.mode_changed = true;
                    }
                }
            }
            // DECRST — DEC private mode reset
            ([b'?'], 'l') => {
                for param in params.iter() {
                    let mode = param.first().copied().unwrap_or(0);
                    let prev_decckm = p.modes.decckm;
                    match mode {
                        1 => p.modes.decckm = false,
                        9 | 1000 | 1002 | 1003 => {
                            p.modes.mouse_reporting = MouseReportingMode::None
                        }
                        25 => p.modes.cursor_visible = false,
                        47 => p.leave_alternate(false),
                        1004 => p.modes.focus_events = false,
                        1006 => p.modes.mouse_encoding = MouseEncoding::X10,
                        1015 => p.modes.mouse_encoding = MouseEncoding::X10,
                        1047 => p.leave_alternate(false),
                        1049 => p.leave_alternate(true),
                        2004 => p.modes.bracketed_paste = false,
                        _ => {}
                    }
                    if p.modes.decckm != prev_decckm {
                        p.mode_changed = true;
                    }
                }
            }
            // DECSC (7) — save cursor
            ([], 's') => {
                let saved = p.active_cursor().clone();
                if p.alt_active {
                    p.saved_alt_cursor = Some(saved);
                } else {
                    p.saved_normal_cursor = Some(saved);
                }
            }
            // DECRC (8) — restore cursor
            ([], 'u') => {
                let restored = if p.alt_active {
                    p.saved_alt_cursor.clone()
                } else {
                    p.saved_normal_cursor.clone()
                };
                if let Some(pos) = restored {
                    *p.active_cursor_mut() = pos;
                }
            }
            // DECSCUSR — cursor shape
            ([b' '], 'q') => {
                // Shape stored; frontend reads it via snapshot cursor_shape field.
                // TODO: propagate shape to cursor state in snapshot.
                let _ = param0;
            }
            _ => {} // Unknown CSI sequence — ignore.
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        let p = &mut self.inner;
        match (intermediates, byte) {
            // DECSC — save cursor (ESC 7)
            ([], b'7') => {
                let saved = p.active_cursor().clone();
                if p.alt_active {
                    p.saved_alt_cursor = Some(saved);
                } else {
                    p.saved_normal_cursor = Some(saved);
                }
            }
            // DECRC — restore cursor (ESC 8)
            ([], b'8') => {
                let restored = if p.alt_active {
                    p.saved_alt_cursor.clone()
                } else {
                    p.saved_normal_cursor.clone()
                };
                if let Some(pos) = restored {
                    *p.active_cursor_mut() = pos;
                }
            }
            // DECKPAM — application keypad mode (ESC =)
            ([], b'=') => {
                p.modes.deckpam = true;
                p.mode_changed = true;
            }
            // DECKPNM — normal keypad mode (ESC >)
            ([], b'>') => {
                p.modes.deckpam = false;
                p.mode_changed = true;
            }
            // G0 charset designation: ESC ( <final>
            ([b'('], b'0') => {
                p.modes.g0 = Charset::DecSpecialGraphics;
            }
            ([b'('], b'B') => {
                p.modes.g0 = Charset::Ascii;
            }
            // G1 charset designation: ESC ) <final>
            ([b')'], b'0') => {
                p.modes.g1 = Charset::DecSpecialGraphics;
            }
            ([b')'], b'B') => {
                p.modes.g1 = Charset::Ascii;
            }
            _ => {}
        }
    }
}
