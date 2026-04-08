// SPDX-License-Identifier: MPL-2.0

use crate::vt::processor::VtProcessor;

/// CUU — cursor up (CSI Ps A)
pub(super) fn cuu(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    let (top, _) = p.modes.scroll_region;
    let top_clamp = if p.modes.decom { top } else { 0 };
    p.active_cursor_mut().row = p.cursor_row().saturating_sub(n).max(top_clamp);
    p.pending_dirty.mark_cursor_moved();
}

/// CUD — cursor down (CSI Ps B)
pub(super) fn cud(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    let (_, bottom) = p.modes.scroll_region;
    let bottom_clamp = if p.modes.decom {
        bottom
    } else {
        p.rows.saturating_sub(1)
    };
    p.active_cursor_mut().row = (p.cursor_row() + n).min(bottom_clamp);
    p.pending_dirty.mark_cursor_moved();
}

/// CUF — cursor forward (CSI Ps C)
pub(super) fn cuf(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    p.active_cursor_mut().col = (p.cursor_col() + n).min(p.cols.saturating_sub(1));
    p.pending_dirty.mark_cursor_moved();
}

/// CUB — cursor back (CSI Ps D)
pub(super) fn cub(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    p.active_cursor_mut().col = p.cursor_col().saturating_sub(n);
    p.pending_dirty.mark_cursor_moved();
}

/// CUP / HVP — cursor position (CSI Pr ; Pc H or f)
pub(super) fn cup(p: &mut VtProcessor, row_param: u16, col_param: u16) {
    p.wrap_pending = false;
    // 1-based param, converted to 0-based.
    let row_0 = row_param.max(1).saturating_sub(1);
    let col = col_param
        .max(1)
        .saturating_sub(1)
        .min(p.cols.saturating_sub(1));
    // DECOM (origin mode): row is relative to the top of the scroll region,
    // and constrained within [top, bottom].
    let row = if p.modes.decom {
        let (top, bottom) = p.modes.scroll_region;
        (top + row_0).clamp(top, bottom)
    } else {
        row_0.min(p.rows.saturating_sub(1))
    };
    p.active_cursor_mut().row = row;
    p.active_cursor_mut().col = col;
    p.pending_dirty.mark_cursor_moved();
}

/// CHA — Cursor Horizontal Absolute (CSI Ps G)
pub(super) fn cha(p: &mut VtProcessor, col_param: u16) {
    p.wrap_pending = false;
    let col = col_param
        .max(1)
        .saturating_sub(1)
        .min(p.cols.saturating_sub(1));
    p.active_cursor_mut().col = col;
    p.pending_dirty.mark_cursor_moved();
}

/// HPA — Horizontal Position Absolute (CSI Ps `)
/// Identical to CHA; uses backtick final byte.
pub(super) fn hpa(p: &mut VtProcessor, col_param: u16) {
    p.wrap_pending = false;
    let col = col_param
        .max(1)
        .saturating_sub(1)
        .min(p.cols.saturating_sub(1));
    p.active_cursor_mut().col = col;
    p.pending_dirty.mark_cursor_moved();
}

/// VPA — Vertical Position Absolute (CSI Ps d)
pub(super) fn vpa(p: &mut VtProcessor, row_param: u16) {
    p.wrap_pending = false;
    let row = row_param
        .max(1)
        .saturating_sub(1)
        .min(p.rows.saturating_sub(1));
    p.active_cursor_mut().row = row;
    p.pending_dirty.mark_cursor_moved();
}

/// CNL — Cursor Next Line (CSI Ps E)
pub(super) fn cnl(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    let (_, bottom) = p.modes.scroll_region;
    let bottom_clamp = if p.modes.decom {
        bottom
    } else {
        p.rows.saturating_sub(1)
    };
    p.active_cursor_mut().row = (p.cursor_row() + n).min(bottom_clamp);
    p.active_cursor_mut().col = 0;
    p.pending_dirty.mark_cursor_moved();
}

/// CPL — Cursor Previous Line (CSI Ps F)
pub(super) fn cpl(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    let (top, _) = p.modes.scroll_region;
    let top_clamp = if p.modes.decom { top } else { 0 };
    p.active_cursor_mut().row = p.cursor_row().saturating_sub(n).max(top_clamp);
    p.active_cursor_mut().col = 0;
    p.pending_dirty.mark_cursor_moved();
}

/// HPR — Horizontal Position Relative (CSI Ps a)
pub(super) fn hpr(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    p.active_cursor_mut().col = (p.cursor_col() + n).min(p.cols.saturating_sub(1));
    p.pending_dirty.mark_cursor_moved();
}

/// VPR — Vertical Position Relative (CSI Ps e)
pub(super) fn vpr(p: &mut VtProcessor, n: u16) {
    p.wrap_pending = false;
    let (_, bottom) = p.modes.scroll_region;
    let bottom_clamp = if p.modes.decom {
        bottom
    } else {
        p.rows.saturating_sub(1)
    };
    p.active_cursor_mut().row = (p.cursor_row() + n).min(bottom_clamp);
    p.pending_dirty.mark_cursor_moved();
}
