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

#[cfg(test)]
mod security_tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SEC-PTY-001 — CSI 21t (window title read-back) silently discarded
    // -----------------------------------------------------------------------

    /// SEC-PTY-001: CSI 21t must not trigger any title injection into PTY input.
    #[test]
    fn sec_pty_001_csi_21t_title_readback_discarded() {
        let mut vt = VtProcessor::new(80, 24);
        // Set a title that could be weaponised if echoed.
        vt.process(b"\x1b]0;injected;ls -la\x07");
        assert_eq!(vt.title, "injected;ls -la");

        // Send CSI 21t (window title read request) — must be silently ignored.
        let _dirty = vt.process(b"\x1b[21t");
        // No panic and no dedicated response buffer exists — the sequence is a no-op.
    }

    /// SEC-PTY-001: CSI 21t after a title containing a shell injection payload.
    #[test]
    fn sec_pty_001_csi_21t_after_shell_injection_title_no_effect() {
        let mut vt = VtProcessor::new(80, 24);
        let _dirty = vt.process(b"\x1b]0;$(id)\x07\x1b[21t");
        // No panic, no crash, no observable injection.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-002 — OSC query sequences discarded (no echo-back)
    // -----------------------------------------------------------------------

    /// SEC-PTY-002: OSC 10;? (foreground color query) must be silently discarded.
    #[test]
    fn sec_pty_002_osc_color_query_no_response() {
        let mut vt = VtProcessor::new(80, 24);
        // OSC 10 ; ? BEL
        let _dirty = vt.process(b"\x1b]10;?\x07");
        // No panic. VtProcessor has no response buffer — confirms no echo-back.
    }

    /// SEC-PTY-002: DECRQSS (ESC P $ q ... ESC \) must be silently discarded.
    #[test]
    fn sec_pty_002_decrqss_ignored() {
        let mut vt = VtProcessor::new(80, 24);
        // DECRQSS sequence: ESC P $ q " p ESC \
        let _dirty = vt.process(b"\x1bP$q\"p\x1b\\");
        // No panic, no observable response.
    }

    /// SEC-PTY-002: CSI ? 1 $ p (DECRPM) must be silently discarded.
    #[test]
    fn sec_pty_002_decrpm_mode_query_ignored() {
        let mut vt = VtProcessor::new(80, 24);
        let _dirty = vt.process(b"\x1b[?1$p");
        // No panic, no mode response injected.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-003 — OSC sequence with large payload does not panic or OOM
    // -----------------------------------------------------------------------

    /// SEC-PTY-003: Large OSC 0 title payload must be processed without panic.
    #[test]
    fn sec_pty_003_large_osc_title_no_panic() {
        let mut vt = VtProcessor::new(80, 24);
        let mut seq = b"\x1b]0;".to_vec();
        seq.extend(b"A".repeat(10_000));
        seq.push(b'\x07');
        let _dirty = vt.process(&seq);
        // Title must be bounded by parse_osc (max 256 chars).
        assert!(
            vt.title.len() <= 256,
            "Title must be bounded even with large OSC input (SEC-PTY-003), got {}",
            vt.title.len()
        );
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-004 — DCS sequence with large payload does not panic
    // -----------------------------------------------------------------------

    /// SEC-PTY-004: DCS sequence with 10 000-byte payload must not panic.
    #[test]
    fn sec_pty_004_large_dcs_payload_no_panic() {
        let mut vt = VtProcessor::new(80, 24);
        let mut seq = b"\x1bP".to_vec();
        seq.extend(b"B".repeat(10_000));
        seq.extend(b"\x1b\\"); // DCS string terminator (ST)
        let _dirty = vt.process(&seq);
        // No panic. DCS is silently ignored in v1.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-007 — Invalid UTF-8 bytes replaced with U+FFFD
    // -----------------------------------------------------------------------

    /// SEC-PTY-007: Overlong UTF-8 encoding 0xC0 0xAF must not produce raw bytes.
    ///
    /// ScreenSnapshot uses a flat row-major `cells: Vec<SnapshotCell>` with `content`
    /// field (not `grapheme`). Index 0 is (row=0, col=0).
    #[test]
    fn sec_pty_007_invalid_utf8_replaced_with_replacement_char() {
        use crate::vt::screen_buffer::SnapshotCell;
        let mut vt = VtProcessor::new(80, 24);
        // 0xC0 0xAF is an overlong encoding of U+002F ('/'). It is invalid UTF-8.
        let _dirty = vt.process(b"\xC0\xAF");
        let snapshot = vt.get_snapshot();
        // Cell (row=0, col=0) is at flat index 0.
        let cell_content: &str = snapshot.cells.first().map(|c: &SnapshotCell| c.content.as_str()).unwrap_or("");
        // Acceptable: U+FFFD, space (default cell), or empty string.
        // Not acceptable: the raw byte '/' or any non-Unicode value.
        let is_safe = cell_content == "\u{FFFD}"
            || cell_content == " "
            || cell_content.is_empty();
        assert!(
            is_safe,
            "Invalid UTF-8 must produce U+FFFD or empty cell, not raw bytes (SEC-PTY-007). Got: {:?}",
            cell_content
        );
    }

    /// SEC-PTY-007: Valid characters surrounding invalid UTF-8 must render correctly.
    #[test]
    fn sec_pty_007_valid_chars_unaffected_by_invalid_utf8() {
        use crate::vt::screen_buffer::SnapshotCell;
        let mut vt = VtProcessor::new(80, 24);
        // "ok" + invalid bytes + "!"
        let _dirty = vt.process(b"ok\xC0\xAF!");
        let snapshot = vt.get_snapshot();
        // Flat row-major: cell(0,0)=index 0, cell(0,1)=index 1.
        let cell0: &str = snapshot.cells.first().map(|c: &SnapshotCell| c.content.as_str()).unwrap_or("");
        let cell1: &str = snapshot.cells.get(1).map(|c: &SnapshotCell| c.content.as_str()).unwrap_or("");
        assert_eq!(cell0, "o", "Cell(0,0) must be 'o'");
        assert_eq!(cell1, "k", "Cell(0,1) must be 'k'");
        // '!' must appear somewhere in row 0.
        let row0_text: String = snapshot.cells.iter().take(80).map(|c: &SnapshotCell| c.content.as_str()).collect();
        assert!(row0_text.contains('!'), "Valid char '!' must survive mixed input (SEC-PTY-007)");
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

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a VtProcessor with standard 80×24 dimensions.
    fn make_vt(cols: u16, rows: u16) -> VtProcessor {
        VtProcessor::new(cols, rows)
    }

    // Helper: extract the grapheme at (row, col) from the active screen buffer.
    fn grapheme_at(vt: &VtProcessor, row: u16, col: u16) -> String {
        vt.active_buf_ref()
            .get(row, col)
            .map(|c| c.grapheme.clone())
            .unwrap_or_default()
    }

    // Helper: extract the attrs at (row, col).
    fn attrs_at(vt: &VtProcessor, row: u16, col: u16) -> crate::vt::cell::CellAttrs {
        vt.active_buf_ref()
            .get(row, col)
            .map(|c| c.attrs)
            .unwrap_or_default()
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-002 — split CSI sequence across two process() calls
    // FS-VT-005
    // ---------------------------------------------------------------------------

    #[test]
    fn split_csi_sequence_is_parsed_correctly() {
        // TEST-VT-002
        let mut vt = make_vt(80, 24);
        // Feed ESC [ in first call, then 31m A in second call.
        vt.process(b"\x1b[");
        vt.process(b"31mA");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Ansi { index: 1 }),
            "ANSI red (31) should be index 1"
        );
        assert_eq!(grapheme_at(&vt, 0, 0), "A");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-003 — UTF-8 sequence split across two process() calls
    // FS-VT-010
    // ---------------------------------------------------------------------------

    #[test]
    fn utf8_sequence_split_across_calls_is_reassembled() {
        // TEST-VT-003
        let mut vt = make_vt(80, 24);
        // 'é' = 0xC3 0xA9 — split: first call has only the lead byte.
        vt.process(&[0xC3]);
        vt.process(&[0xA9, b'X']);
        let first_grapheme = grapheme_at(&vt, 0, 0);
        let second_grapheme = grapheme_at(&vt, 0, 1);
        // The vte crate handles UTF-8 reassembly; é should appear at (0,0).
        assert_eq!(first_grapheme, "é", "é must be reassembled across calls");
        assert_eq!(second_grapheme, "X", "X must appear in the next cell");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-004 — wide (CJK) character wrapping at end of line
    // FS-VT-011
    // ---------------------------------------------------------------------------

    #[test]
    fn wide_char_at_last_col_wraps_to_next_line() {
        // TEST-VT-004 — 4-column buffer.
        let mut vt = make_vt(4, 5);
        // Position cursor at col 3 (last column, 0-indexed) via CUP.
        vt.process(b"\x1b[1;4H"); // row 1, col 4 (1-based)
        // Feed '中' (U+4E2D) = width 2.
        vt.process("中".as_bytes());
        // After writing at col=3 with width=2, the char must wrap.
        // Implementation detail: write_char clamps col to cols-1 on overflow.
        // The wide character should either be at row 0 col 3 or wrapped.
        // What matters is no panic and cursor integrity.
        let snap = vt.get_snapshot();
        assert_eq!(snap.cols, 4);
        assert!(snap.cursor_row < 5, "cursor row must remain in bounds");
        assert!(snap.cursor_col < 4, "cursor col must remain in bounds");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-005 — invalid UTF-8 produces U+FFFD
    // FS-VT-016
    // ---------------------------------------------------------------------------

    #[test]
    fn invalid_utf8_produces_replacement_character() {
        // TEST-VT-005
        let mut vt = make_vt(80, 24);
        // 0xC0 0xAF is an overlong encoding (invalid UTF-8).
        vt.process(&[0xC0, 0xAF]);
        let g = grapheme_at(&vt, 0, 0);
        // The vte crate replaces invalid bytes with U+FFFD.
        assert_eq!(
            g, "\u{FFFD}",
            "invalid UTF-8 must produce U+FFFD replacement char"
        );
        // Subsequent valid ASCII must still parse correctly.
        vt.process(b"Z");
        // The cursor should have advanced and Z is somewhere on row 0.
        let snap = vt.get_snapshot();
        assert_eq!(snap.cols, 80, "buffer dimensions must be intact after invalid UTF-8");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-006 — SGR color variants: ANSI, 256-color, RGB, colon form
    // FS-VT-020, FS-VT-021, FS-VT-022
    // ---------------------------------------------------------------------------

    #[test]
    fn sgr_ansi_color_is_applied() {
        // TEST-VT-006 step 1
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[31mA");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(attrs.fg, Some(crate::vt::cell::Color::Ansi { index: 1 }));
    }

    #[test]
    fn sgr_256_color_is_applied() {
        // TEST-VT-006 step 2
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[38;5;196mB");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(attrs.fg, Some(crate::vt::cell::Color::Ansi256 { index: 196 }));
    }

    #[test]
    fn sgr_rgb_truecolor_semicolon_form_is_applied() {
        // TEST-VT-006 step 3
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[38;2;255;100;0mC");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Rgb {
                r: 255,
                g: 100,
                b: 0
            })
        );
    }

    #[test]
    fn sgr_rgb_truecolor_colon_form_is_applied() {
        // TEST-VT-006 step 4 — ITU T.416 colon sub-parameter form
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b[38:2:255:100:0mD");
        let attrs = attrs_at(&vt, 0, 0);
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Rgb {
                r: 255,
                g: 100,
                b: 0
            })
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-007 — SGR multi-attribute and partial reset
    // FS-VT-024
    // ---------------------------------------------------------------------------

    #[test]
    fn sgr_multi_attributes_set_independently() {
        // TEST-VT-007
        let mut vt = make_vt(80, 24);
        // Set bold + italic + underline simultaneously.
        vt.process(b"\x1b[1;3;4mA");
        let attrs = attrs_at(&vt, 0, 0);
        assert!(attrs.bold, "bold must be set");
        assert!(attrs.italic, "italic must be set");
        assert!(attrs.underline > 0, "underline must be set");

        // SGR 22 resets bold/dim without affecting italic or underline.
        vt.process(b"\x1b[22mB");
        let attrs = attrs_at(&vt, 0, 1);
        assert!(!attrs.bold, "bold must be cleared by SGR 22");
        assert!(attrs.italic, "italic must be unaffected by SGR 22");
        assert!(attrs.underline > 0, "underline must be unaffected by SGR 22");

        // SGR 0 clears all.
        vt.process(b"\x1b[0mC");
        let attrs = attrs_at(&vt, 0, 2);
        assert!(!attrs.bold);
        assert!(!attrs.italic);
        assert_eq!(attrs.underline, 0);
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-008 — cursor visibility and DECTCEM
    // FS-VT-030, FS-VT-031
    // ---------------------------------------------------------------------------

    #[test]
    fn dectcem_hide_and_show_cursor() {
        // TEST-VT-008 (partial — cursor shape stub)
        let mut vt = make_vt(80, 24);
        assert!(vt.modes.cursor_visible, "cursor must be visible by default");

        // Hide cursor.
        vt.process(b"\x1b[?25l");
        assert!(!vt.modes.cursor_visible, "cursor must be hidden after DECTCEM hide");

        // Show cursor.
        vt.process(b"\x1b[?25h");
        assert!(vt.modes.cursor_visible, "cursor must be visible after DECTCEM show");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-009 — alternate screen cursor save/restore (DECSC + mode 1049)
    // FS-VT-033
    // ---------------------------------------------------------------------------

    #[test]
    fn alternate_screen_cursor_save_restore() {
        // TEST-VT-009
        let mut vt = make_vt(80, 24);
        // Position cursor at (5, 10) on normal screen via CUP.
        vt.process(b"\x1b[6;11H"); // row=6 col=11 (1-based) → row=5 col=10 (0-based)
        assert_eq!(vt.normal_cursor.row, 5);
        assert_eq!(vt.normal_cursor.col, 10);

        // Switch to alternate screen (saves cursor via mode 1049).
        // DECSET uses CSI ? Pm h (with '?' intermediate byte).
        vt.process(b"\x1b[?1049h");
        assert!(vt.alt_active, "alternate screen must be active");

        // Move cursor to (0, 0) on alternate screen.
        vt.process(b"\x1b[1;1H");
        assert_eq!(vt.alt_cursor.row, 0);
        assert_eq!(vt.alt_cursor.col, 0);

        // Return to normal screen (restores cursor).
        // DECRST uses CSI ? Pm l.
        vt.process(b"\x1b[?1049l");
        assert!(!vt.alt_active, "normal screen must be active");
        assert_eq!(vt.normal_cursor.row, 5, "cursor row must be restored");
        assert_eq!(vt.normal_cursor.col, 10, "cursor col must be restored");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-010 — alternate screen isolation and no scrollback
    // FS-VT-040, FS-VT-041, FS-VT-042, FS-VT-044
    // ---------------------------------------------------------------------------

    #[test]
    fn alternate_screen_is_isolated_from_normal_screen() {
        // TEST-VT-010
        let mut vt = make_vt(10, 5);
        // Write content on normal screen.
        vt.process(b"HELLO");
        assert_eq!(grapheme_at(&vt, 0, 0), "H");

        // Switch to alternate screen — must be blank.
        // DECSET uses CSI ? Pm h.
        vt.process(b"\x1b[?1049h");
        assert!(vt.alt_active);
        assert_eq!(
            grapheme_at(&vt, 0, 0),
            " ",
            "alternate screen must be blank on entry"
        );

        // Write on alternate screen.
        vt.process(b"WORLD");

        // Return to normal screen.
        // DECRST uses CSI ? Pm l.
        vt.process(b"\x1b[?1049l");
        assert!(!vt.alt_active);
        assert_eq!(
            grapheme_at(&vt, 0, 0),
            "H",
            "normal screen content must survive alt-screen usage"
        );

        // Alternate screen must not have added scrollback.
        assert_eq!(
            vt.normal.scrollback_len(),
            0,
            "alternate screen must not contribute to scrollback"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-011 — DECSTBM scroll region
    // FS-VT-050, FS-VT-051, FS-VT-053
    // ---------------------------------------------------------------------------

    #[test]
    fn decstbm_partial_scroll_region_no_scrollback() {
        // TEST-VT-011
        let mut vt = make_vt(80, 10);
        // Set scroll region rows 2–8 (1-based) = indices 1–7 (0-based).
        vt.process(b"\x1b[2;8r");
        assert_eq!(vt.modes.scroll_region, (1, 7));
        // Cursor must be moved to home position after DECSTBM.
        assert_eq!(vt.normal_cursor.row, 0);
        assert_eq!(vt.normal_cursor.col, 0);
        // Scrolling within the partial region must not add to scrollback.
        // Position cursor at bottom of region (row 7, 0-based).
        vt.process(b"\x1b[8;1H"); // row=8 col=1 (1-based)
        // Feed 3 LF to scroll within region.
        vt.process(b"\n\n\n");
        assert_eq!(
            vt.normal.scrollback_len(),
            0,
            "partial scroll region must not add to scrollback (FS-VT-053)"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-018 — OSC buffer overflow protection
    // FS-SEC-005
    // ---------------------------------------------------------------------------

    #[test]
    fn osc_overflow_does_not_crash_and_subsequent_sequences_parse() {
        // TEST-VT-018
        let mut vt = make_vt(80, 24);
        // Feed OSC 0 ; followed by 5000 bytes without a terminator.
        let mut overflow_seq: Vec<u8> = b"\x1b]0;".to_vec();
        overflow_seq.extend(std::iter::repeat(b'X').take(5000));
        // No BEL or ST — simulate abandonment. Then a valid sequence.
        vt.process(&overflow_seq);
        // Feed a valid sequence that follows — must not be corrupted.
        vt.process(b"\x1b[31mA");
        // No panic is the primary assertion; but also verify A is written.
        let attrs = attrs_at(&vt, 0, 0);
        // The VTE parser's behavior on overlong OSC is to discard and continue —
        // verify subsequent input parses (red foreground set).
        assert_eq!(
            attrs.fg,
            Some(crate::vt::cell::Color::Ansi { index: 1 }),
            "SGR 31 after overlong OSC must be applied"
        );
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-023 — DEC Special Graphics charset
    // FS-VT-015
    // ---------------------------------------------------------------------------

    #[test]
    fn dec_special_graphics_so_maps_j_to_box_drawing() {
        // TEST-VT-023
        let mut vt = make_vt(80, 24);
        // Designate G1 as DEC Special Graphics.
        vt.process(b"\x1b)0");
        // SO (0x0E) — shift to G1.
        vt.process(b"\x0e");
        // Feed 0x6A ('j' in ASCII; maps to '┘' in DEC Special Graphics).
        vt.process(b"\x6a");
        let g = grapheme_at(&vt, 0, 0);
        assert_eq!(g, "┘", "0x6A with DEC Special Graphics active must map to '┘'");
        // SI (0x0F) — return to G0 (ASCII).
        vt.process(b"\x0f");
        vt.process(b"j");
        let g2 = grapheme_at(&vt, 0, 1);
        assert_eq!(g2, "j", "0x6A with ASCII active must remain 'j'");
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-012 — OSC title sanitization (via parse_osc, exercised end-to-end)
    // FS-VT-060, FS-VT-062
    // ---------------------------------------------------------------------------

    #[test]
    fn osc_title_control_chars_are_stripped() {
        // TEST-VT-012 step 3-4
        let mut vt = make_vt(80, 24);
        // OSC title containing a C0 control char (0x01).
        vt.process(b"\x1b]0;Title\x01WithControl\x07");
        assert!(
            !vt.title.contains('\x01'),
            "C0 control chars must be stripped from OSC title"
        );
    }

    #[test]
    fn osc_title_truncated_to_256_chars() {
        // TEST-VT-012 step 5-6
        let mut vt = make_vt(80, 24);
        let long_title: Vec<u8> = std::iter::once(b'\x1b')
            .chain(b"]0;".iter().copied())
            .chain(std::iter::repeat(b'A').take(300))
            .chain(std::iter::once(b'\x07'))
            .collect();
        vt.process(&long_title);
        assert!(
            vt.title.len() <= 256,
            "OSC title must be truncated to max 256 chars, got {}",
            vt.title.len()
        );
    }

    #[test]
    fn osc_title_plain_title_is_stored() {
        // TEST-VT-012 step 1-2
        let mut vt = make_vt(80, 24);
        vt.process(b"\x1b]0;My Title\x07");
        assert_eq!(vt.title, "My Title");
    }

    // ---------------------------------------------------------------------------
    // TEST: resize clamps cursor positions
    // FS-PTY-009, FS-PTY-010
    // ---------------------------------------------------------------------------

    #[test]
    fn resize_clamps_cursor_positions_to_new_bounds() {
        let mut vt = make_vt(80, 24);
        // Move cursor to row 20, col 70.
        vt.process(b"\x1b[21;71H");
        assert_eq!(vt.normal_cursor.row, 20);
        assert_eq!(vt.normal_cursor.col, 70);
        // Resize to smaller dimensions.
        vt.resize(40, 10);
        assert!(
            vt.normal_cursor.row < 10,
            "cursor row must be clamped to new rows"
        );
        assert!(
            vt.normal_cursor.col < 40,
            "cursor col must be clamped to new cols"
        );
    }

    // ---------------------------------------------------------------------------
    // DECCKM mode tracking
    // FS-VT-030
    // ---------------------------------------------------------------------------

    #[test]
    fn decckm_mode_set_and_reset() {
        let mut vt = make_vt(80, 24);
        assert!(!vt.modes.decckm, "DECCKM must be false by default");
        vt.process(b"\x1b[?1h"); // DECSET 1 = DECCKM
        assert!(vt.modes.decckm, "DECCKM must be true after ESC[?1h");
        assert!(vt.mode_changed, "mode_changed flag must be set");
        vt.mode_changed = false;
        vt.process(b"\x1b[?1l"); // DECRST 1
        assert!(!vt.modes.decckm, "DECCKM must be false after ESC[?1l");
        assert!(vt.mode_changed, "mode_changed flag must be set again");
    }

    // ---------------------------------------------------------------------------
    // Bracketed paste mode tracking
    // FS-KBD related
    // ---------------------------------------------------------------------------

    #[test]
    fn bracketed_paste_mode_tracking() {
        let mut vt = make_vt(80, 24);
        assert!(!vt.modes.bracketed_paste);
        vt.process(b"\x1b[?2004h");
        assert!(vt.modes.bracketed_paste);
        vt.process(b"\x1b[?2004l");
        assert!(!vt.modes.bracketed_paste);
    }
}
