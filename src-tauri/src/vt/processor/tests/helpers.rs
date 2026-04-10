// SPDX-License-Identifier: MPL-2.0

use crate::vt::VtProcessor;

pub fn make_vt(cols: u16, rows: u16) -> VtProcessor {
    VtProcessor::new(cols, rows, 10_000, 0, false)
}

/// Construct a `VtProcessor` with a non-zero `initial_cursor_shape`.
///
/// Used by `preferred_cursor_shape` tests: the preferred shape is the shape
/// passed as `initial_cursor_shape` at construction time (and when
/// `propagate_cursor_shape` is called). Tests that need to verify DECSCUSR 0
/// restoration must use this helper instead of `make_vt`.
pub fn make_vt_with_shape(cols: u16, rows: u16, initial_shape: u8) -> VtProcessor {
    VtProcessor::new(cols, rows, 10_000, initial_shape, false)
}

pub fn grapheme_at(vt: &VtProcessor, row: u16, col: u16) -> String {
    vt.active_buf_ref()
        .get(row, col)
        .map(|c| c.grapheme.to_string())
        .unwrap_or_default()
}

pub fn attrs_at(vt: &VtProcessor, row: u16, col: u16) -> crate::vt::cell::CellAttrs {
    vt.active_buf_ref()
        .get(row, col)
        .map(|c| c.attrs)
        .unwrap_or_default()
}
