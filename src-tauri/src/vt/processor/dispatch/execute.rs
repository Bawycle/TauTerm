// SPDX-License-Identifier: MPL-2.0

use crate::vt::modes::CharsetSlot;
use crate::vt::processor::VtProcessor;

pub(super) fn handle_execute(p: &mut VtProcessor, byte: u8) {
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
                p.pending_dirty.mark_cursor_moved();
            }
        }
        // HT (0x09) — advance to next tab stop (every 8 columns).
        0x09 => {
            let next_tab = ((col / 8) + 1) * 8;
            p.active_cursor_mut().col = next_tab.min(p.cols.saturating_sub(1));
            p.pending_dirty.mark_cursor_moved();
        }
        // LF / VT / FF (0x0A / 0x0B / 0x0C)
        0x0A..=0x0C => {
            // A hard newline always clears the pending wrap flag — the line
            // is terminated by an explicit LF, not by auto-wrap.
            // Flush any pending emoji base (R6) and confirm any pending RI as
            // narrow (R8): a newline changes the row, so an unpaired RI is narrow.
            p.flush_pending_emoji(None);
            p.flush_pending_ri_narrow();
            p.wrap_pending = false;
            let (top, bottom) = p.modes.scroll_region;
            // FS-VT-055: when the cursor is outside the active scroll region,
            // move down one line without scrolling. If already at the last
            // screen row, the LF is ignored entirely.
            if row < top || row > bottom {
                if row < p.rows.saturating_sub(1) {
                    p.active_cursor_mut().row = row + 1;
                }
                // else: last screen row outside region → no-op.
            } else {
                let is_full = top == 0 && bottom == p.rows.saturating_sub(1);
                if row == bottom {
                    // `soft_wrapped: false` — this is an explicit newline.
                    p.active_buf_mut().scroll_up(top, bottom, 1, is_full, false);
                } else {
                    p.active_cursor_mut().row = (row + 1).min(p.rows.saturating_sub(1));
                }
            }
            p.pending_dirty.mark_cursor_moved();
        }
        // CR (0x0D)
        0x0D => {
            // Carriage return cancels any pending wrap.
            // Flush pending emoji (R6) and confirm pending RI as narrow (R8):
            // CR moves the cursor to column 0, which is equivalent to a row
            // boundary for RI-pair detection purposes.
            p.flush_pending_emoji(None);
            p.flush_pending_ri_narrow();
            p.wrap_pending = false;
            p.active_cursor_mut().col = 0;
            p.pending_dirty.mark_cursor_moved();
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
