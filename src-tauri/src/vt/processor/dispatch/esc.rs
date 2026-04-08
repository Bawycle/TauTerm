// SPDX-License-Identifier: MPL-2.0

use crate::vt::modes::Charset;
use crate::vt::processor::VtProcessor;

pub(super) fn handle_esc(p: &mut VtProcessor, intermediates: &[u8], byte: u8) {
    match (intermediates, byte) {
        // DECSC — save cursor (ESC 7)
        ([], b'7') => {
            let mut saved = p.active_cursor().clone();
            // Capture current SGR attributes, charset slot, DECAWM, and DECOM.
            saved.attrs = p.current_attrs;
            saved.charset_slot = p.modes.charset_slot;
            saved.decawm = p.modes.decawm;
            saved.decom = p.modes.decom;
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
                p.modes.decawm = pos.decawm;
                p.modes.decom = pos.decom;
                *p.active_cursor_mut() = pos;
                p.wrap_pending = false;
                p.pending_dirty.mark_cursor_moved();
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
            p.pending_dirty.mark_cursor_moved();
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
