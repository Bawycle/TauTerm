// SPDX-License-Identifier: MPL-2.0

//! `vte::Perform` implementation for `VtPerformBridge`.
//!
//! Dispatches parsed VT/ANSI sequences to the `VtProcessor` state machine:
//! C0 controls (`execute`), printable characters (`print`), CSI, OSC, and ESC sequences.

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
            0x0A..=0x0C => {
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
                p.active_cursor_mut().col = (p.cursor_col() + n).min(p.cols - 1);
            }
            // CUB — cursor back
            ([], 'D') => {
                let n = param0.max(1);
                p.active_cursor_mut().col = p.cursor_col().saturating_sub(n);
            }
            // CUP / HVP — cursor position
            ([], 'H') | ([], 'f') => {
                let row = param0.max(1).saturating_sub(1).min(p.rows - 1);
                let col = param1.max(1).saturating_sub(1).min(p.cols - 1);
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
                    p.rows - 1
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
