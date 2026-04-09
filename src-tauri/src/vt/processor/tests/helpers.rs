// SPDX-License-Identifier: MPL-2.0

use crate::vt::VtProcessor;

pub fn make_vt(cols: u16, rows: u16) -> VtProcessor {
    VtProcessor::new(cols, rows, 10_000, 0, false)
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
