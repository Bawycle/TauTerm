// SPDX-License-Identifier: MPL-2.0

use crate::vt::processor::VtProcessor;

/// DECSTBM — Set Top and Bottom Margins / scroll region (CSI Pt ; Pb r)
pub(super) fn decstbm(p: &mut VtProcessor, param0: u16, param1: u16) {
    let top = param0.max(1).saturating_sub(1);
    let bottom = if param1 == 0 {
        p.rows.saturating_sub(1)
    } else {
        param1.saturating_sub(1)
    };
    if top <= bottom && bottom < p.rows {
        p.modes.scroll_region = (top, bottom);
        p.wrap_pending = false;
    }
    // Move cursor to home position.
    p.active_cursor_mut().row = 0;
    p.active_cursor_mut().col = 0;
    p.pending_dirty.mark_cursor_moved();
}

/// CSI Ps S — scroll up Ps lines (default 1) within the scroll region (FS-VT-052).
pub(super) fn su(p: &mut VtProcessor, n: u16) {
    let (top, bottom) = p.modes.scroll_region;
    let is_full = top == 0 && bottom == p.rows.saturating_sub(1);
    p.active_buf_mut().scroll_up(top, bottom, n, is_full, false);
}

/// CSI Ps T — scroll down Ps lines (default 1) within the scroll region (FS-VT-052).
pub(super) fn sd(p: &mut VtProcessor, n: u16) {
    let (top, bottom) = p.modes.scroll_region;
    p.active_buf_mut().scroll_down(top, bottom, n);
}

/// IL — Insert Line (CSI Ps L)
pub(super) fn il(p: &mut VtProcessor, n: u16) {
    let row = p.cursor_row();
    let (top, bottom) = p.modes.scroll_region;
    // Cursor must be within the scroll region; otherwise ignore.
    if row >= top && row <= bottom {
        p.active_buf_mut().scroll_down(row, bottom, n);
    }
}

/// DL — Delete Line (CSI Ps M)
pub(super) fn dl(p: &mut VtProcessor, n: u16) {
    let row = p.cursor_row();
    let (top, bottom) = p.modes.scroll_region;
    // Cursor must be within the scroll region; otherwise ignore.
    if row >= top && row <= bottom {
        let is_full = top == 0 && bottom == p.rows.saturating_sub(1);
        p.active_buf_mut().scroll_up(row, bottom, n, is_full, false);
    }
}
