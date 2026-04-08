// SPDX-License-Identifier: MPL-2.0

use crate::vt::processor::VtProcessor;

/// ED — Erase in Display (CSI Ps J)
pub(super) fn ed(p: &mut VtProcessor, param: u16) {
    let row = p.cursor_row();
    let col = p.cursor_col();
    let cols = p.cols;
    let rows = p.rows;
    match param {
        0 => {
            p.wrap_pending = false;
            // From cursor to end of screen.
            p.active_buf_mut().erase_cells(row, col, cols);
            p.active_buf_mut().erase_lines(row + 1, rows);
        }
        1 => {
            p.wrap_pending = false;
            // From start to cursor.
            p.active_buf_mut().erase_lines(0, row);
            p.active_buf_mut().erase_cells(row, 0, col + 1);
        }
        2 | 3 => {
            p.wrap_pending = false;
            // Entire screen.
            p.active_buf_mut().erase_lines(0, rows);
        }
        _ => {}
    }
}

/// EL — Erase in Line (CSI Ps K)
pub(super) fn el(p: &mut VtProcessor, param: u16) {
    let row = p.cursor_row();
    let col = p.cursor_col();
    let cols = p.cols;
    match param {
        0 => p.active_buf_mut().erase_cells(row, col, cols),
        1 => p.active_buf_mut().erase_cells(row, 0, col + 1),
        2 => p.active_buf_mut().erase_cells(row, 0, cols),
        _ => {}
    }
}

/// ECH — Erase Character (CSI Ps X)
pub(super) fn ech(p: &mut VtProcessor, n: u16) {
    let row = p.cursor_row();
    let col = p.cursor_col();
    let cols = p.cols;
    // Clamp to end of line.
    let end = (col + n).min(cols);
    p.active_buf_mut().erase_cells(row, col, end);
}
