// SPDX-License-Identifier: MPL-2.0

//! `vte::Perform` implementation for `VtPerformBridge`.
//!
//! Dispatches parsed VT/ANSI sequences to the `VtProcessor` state machine:
//! C0 controls (`execute`), printable characters (`print`), CSI, OSC, and ESC sequences.

use std::sync::Arc;

use vte::Perform;

use crate::vt::{
    charset::translate_dec_special,
    modes::{Charset, CharsetSlot, MouseEncoding, MouseReportingMode},
    osc::{OscAction, parse_osc},
    sgr::apply_sgr,
};

use super::VtPerformBridge;

impl Perform for VtPerformBridge<'_> {
    fn print(&mut self, c: char) {
        let p = &mut self.inner;
        // Apply DEC Special Graphics mapping if active.
        let mapped_c = if c.is_ascii() && (c as u8) >= 0x60 {
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
            // BEL (0x07) — rate-limited; PTY read task emits the event (FS-VT-090).
            0x07 => {
                p.register_bell();
            }
            // BS (0x08)
            0x08 => {
                if col > 0 {
                    p.active_cursor_mut().col = col - 1;
                }
            }
            // HT (0x09) — advance to next tab stop (every 8 columns).
            0x09 => {
                let next_tab = ((col / 8) + 1) * 8;
                p.active_cursor_mut().col = next_tab.min(p.cols.saturating_sub(1));
            }
            // LF / VT / FF (0x0A / 0x0B / 0x0C)
            0x0A..=0x0C => {
                // A hard newline always clears the pending wrap flag — the line
                // is terminated by an explicit LF, not by auto-wrap.
                p.wrap_pending = false;
                let (top, bottom) = p.modes.scroll_region;
                let is_full = top == 0 && bottom == p.rows.saturating_sub(1);
                if row == bottom {
                    // `soft_wrapped: false` — this is an explicit newline.
                    p.active_buf_mut().scroll_up(top, bottom, 1, is_full, false);
                } else {
                    p.active_cursor_mut().row = (row + 1).min(p.rows.saturating_sub(1));
                }
            }
            // CR (0x0D)
            0x0D => {
                // Carriage return cancels any pending wrap.
                p.wrap_pending = false;
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
                if self.inner.title != title {
                    self.inner.title = title;
                    self.inner.title_changed = true;
                }
            }
            OscAction::PushTitle => {
                self.inner.title_stack.push(self.inner.title.clone());
            }
            OscAction::PopTitle => {
                if let Some(t) = self.inner.title_stack.pop()
                    && self.inner.title != t
                {
                    self.inner.title = t;
                    self.inner.title_changed = true;
                }
            }
            OscAction::SetHyperlink { uri, id } => {
                // FS-VT-070–073: store the active hyperlink URI/ID in the processor.
                // Subsequent printable characters will inherit this URI until it is cleared.
                match uri {
                    None => {
                        // OSC 8 ;; — end hyperlink.
                        self.inner.current_hyperlink = None;
                        self.inner.current_hyperlink_id = None;
                    }
                    Some(uri_str) => {
                        let new_id: Option<Arc<str>> = id.map(|s| Arc::from(s.as_str()));
                        // FS-VT-072: if same ID as current hyperlink, reuse the existing
                        // Arc to keep identity stable across multi-line continuations.
                        let reuse = matches!(
                            (&self.inner.current_hyperlink_id, &new_id),
                            (Some(existing), Some(new)) if existing == new
                        );
                        if reuse {
                            // Same ID → URI should be the same; keep existing Arc.
                        } else {
                            self.inner.current_hyperlink = Some(Arc::from(uri_str.as_str()));
                            self.inner.current_hyperlink_id = new_id;
                        }
                    }
                }
            }
            OscAction::ClipboardWrite(text) => {
                // FS-VT-075 / SEC-OSC-002: forward only when the policy allows it.
                if self.inner.allow_osc52_write {
                    self.inner.pending_osc52_write = Some(text);
                }
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
                let n = param0.max(1);
                let (top, _) = p.modes.scroll_region;
                p.active_cursor_mut().row = p.cursor_row().saturating_sub(n).max(top);
            }
            // CUD — cursor down
            ([], 'B') => {
                let n = param0.max(1);
                let (_, bottom) = p.modes.scroll_region;
                p.active_cursor_mut().row = (p.cursor_row() + n).min(bottom);
            }
            // CUF — cursor forward
            ([], 'C') => {
                let n = param0.max(1);
                p.active_cursor_mut().col = (p.cursor_col() + n).min(p.cols.saturating_sub(1));
            }
            // CUB — cursor back
            ([], 'D') => {
                let n = param0.max(1);
                p.active_cursor_mut().col = p.cursor_col().saturating_sub(n);
            }
            // CUP / HVP — cursor position
            ([], 'H') | ([], 'f') => {
                let row = param0
                    .max(1)
                    .saturating_sub(1)
                    .min(p.rows.saturating_sub(1));
                let col = param1
                    .max(1)
                    .saturating_sub(1)
                    .min(p.cols.saturating_sub(1));
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
                let top = param0.max(1).saturating_sub(1);
                let bottom = if param1 == 0 {
                    p.rows.saturating_sub(1)
                } else {
                    param1.saturating_sub(1)
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
                    let prev_mouse_reporting = p.modes.mouse_reporting;
                    let prev_mouse_encoding = p.modes.mouse_encoding;
                    let prev_focus_events = p.modes.focus_events;
                    let prev_bracketed_paste = p.modes.bracketed_paste;
                    match mode {
                        1 => p.modes.decckm = true,
                        7 => p.modes.decawm = true,
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
                    if p.modes.decckm != prev_decckm
                        || p.modes.mouse_reporting != prev_mouse_reporting
                        || p.modes.mouse_encoding != prev_mouse_encoding
                        || p.modes.focus_events != prev_focus_events
                        || p.modes.bracketed_paste != prev_bracketed_paste
                    {
                        p.mode_changed = true;
                    }
                }
            }
            // DECRST — DEC private mode reset
            ([b'?'], 'l') => {
                for param in params.iter() {
                    let mode = param.first().copied().unwrap_or(0);
                    let prev_decckm = p.modes.decckm;
                    let prev_mouse_reporting = p.modes.mouse_reporting;
                    let prev_mouse_encoding = p.modes.mouse_encoding;
                    let prev_focus_events = p.modes.focus_events;
                    let prev_bracketed_paste = p.modes.bracketed_paste;
                    match mode {
                        1 => p.modes.decckm = false,
                        7 => {
                            p.modes.decawm = false;
                            // Disabling DECAWM cancels any pending wrap immediately.
                            p.wrap_pending = false;
                        }
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
                    if p.modes.decckm != prev_decckm
                        || p.modes.mouse_reporting != prev_mouse_reporting
                        || p.modes.mouse_encoding != prev_mouse_encoding
                        || p.modes.focus_events != prev_focus_events
                        || p.modes.bracketed_paste != prev_bracketed_paste
                    {
                        p.mode_changed = true;
                    }
                }
            }
            // DECSC (7) — save cursor (CSI s)
            ([], 's') => {
                let mut saved = p.active_cursor().clone();
                // Capture current SGR attributes and charset slot into the saved state.
                saved.attrs = p.current_attrs;
                saved.charset_slot = p.modes.charset_slot;
                if p.alt_active {
                    p.saved_alt_cursor = Some(saved);
                } else {
                    p.saved_normal_cursor = Some(saved);
                }
            }
            // DECRC (8) — restore cursor (CSI u)
            ([], 'u') => {
                let restored = if p.alt_active {
                    p.saved_alt_cursor.clone()
                } else {
                    p.saved_normal_cursor.clone()
                };
                if let Some(pos) = restored {
                    p.current_attrs = pos.attrs;
                    p.modes.charset_slot = pos.charset_slot;
                    *p.active_cursor_mut() = pos;
                }
            }
            // ICH — CSI Ps @ — Insert Character (ECMA-48 §8.3.64).
            // Inserts Ps blank cells at the cursor position; cells to the right shift right.
            // Cells shifted beyond the right margin are discarded. Default Ps = 1.
            ([], '@') => {
                let n = param0.max(1);
                let row = p.cursor_row();
                let col = p.cursor_col();
                p.active_buf_mut().insert_cells(row, col, n);
            }
            // DCH — CSI Ps P — Delete Character (ECMA-48 §8.3.26).
            // Deletes Ps cells starting at the cursor; remaining cells shift left.
            // The right end of the line is filled with blanks. Default Ps = 1.
            ([], 'P') => {
                let n = param0.max(1);
                let row = p.cursor_row();
                let col = p.cursor_col();
                p.active_buf_mut().delete_cells(row, col, n);
            }
            // IL — CSI Ps L — Insert Line.
            // Inserts Ps blank lines at the cursor row; lines within the scroll
            // region below shift down. Lines pushed below the scroll region bottom
            // are discarded. Default Ps = 1.
            ([], 'L') => {
                let n = param0.max(1);
                let row = p.cursor_row();
                let (top, bottom) = p.modes.scroll_region;
                // Cursor must be within the scroll region; otherwise ignore.
                if row >= top && row <= bottom {
                    p.active_buf_mut().scroll_down(row, bottom, n);
                }
            }
            // DL — CSI Ps M — Delete Line.
            // Deletes Ps lines at the cursor row; lines within the scroll region
            // below shift up. Blank lines are inserted at the bottom. Default Ps = 1.
            ([], 'M') => {
                let n = param0.max(1);
                let row = p.cursor_row();
                let (top, bottom) = p.modes.scroll_region;
                // Cursor must be within the scroll region; otherwise ignore.
                if row >= top && row <= bottom {
                    let is_full = top == 0 && bottom == p.rows.saturating_sub(1);
                    p.active_buf_mut().scroll_up(row, bottom, n, is_full, false);
                }
            }
            // CSI Ps S — scroll up Ps lines (default 1) within the scroll region (FS-VT-052).
            ([], 'S') => {
                let n = param0.max(1);
                let (top, bottom) = p.modes.scroll_region;
                let is_full = top == 0 && bottom == p.rows.saturating_sub(1);
                p.active_buf_mut().scroll_up(top, bottom, n, is_full, false);
            }
            // CSI Ps T — scroll down Ps lines (default 1) within the scroll region (FS-VT-052).
            ([], 'T') => {
                let n = param0.max(1);
                let (top, bottom) = p.modes.scroll_region;
                p.active_buf_mut().scroll_down(top, bottom, n);
            }
            // DECSCUSR — set cursor shape (FS-VT-030).
            // Values: 0/1 = blinking block, 2 = steady block, 3 = blinking underline,
            // 4 = steady underline, 5 = blinking bar, 6 = steady bar.
            ([b' '], 'q') => {
                // SAFETY: param0 is clamped to [0, 6] — fits in u8 without truncation.
                let shape = param0.min(6) as u8;
                if p.cursor_shape != shape {
                    p.cursor_shape = shape;
                    p.cursor_shape_changed = true;
                }
            }
            _ => {} // Unknown CSI sequence — ignore.
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        let p = &mut self.inner;
        match (intermediates, byte) {
            // DECSC — save cursor (ESC 7)
            ([], b'7') => {
                let mut saved = p.active_cursor().clone();
                // Capture current SGR attributes and charset slot into the saved state.
                saved.attrs = p.current_attrs;
                saved.charset_slot = p.modes.charset_slot;
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
                    p.current_attrs = pos.attrs;
                    p.modes.charset_slot = pos.charset_slot;
                    *p.active_cursor_mut() = pos;
                }
            }
            // RI — Reverse Index (ESC M).
            // If the cursor is at the top of the scroll region, scroll the region down
            // by one line (insert a blank line at the top). Otherwise move cursor up one row.
            ([], b'M') => {
                let row = p.cursor_row();
                let (top, bottom) = p.modes.scroll_region;
                if row == top {
                    p.active_buf_mut().scroll_down(top, bottom, 1);
                } else {
                    p.active_cursor_mut().row = row.saturating_sub(1);
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
