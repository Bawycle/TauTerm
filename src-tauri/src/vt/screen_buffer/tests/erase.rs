// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

use super::super::buffer::ScreenBuffer;

#[test]
fn erase_cells_replaces_with_default() {
    let mut buf = ScreenBuffer::new(10, 5, 100);
    if let Some(cell) = buf.get_mut(0, 3) {
        cell.grapheme = "X".into();
    }
    buf.erase_cells(0, 0, 10);
    assert_eq!(buf.get(0, 3), Some(&Cell::default()));
}

#[test]
fn erase_lines_replaces_with_default_rows() {
    let mut buf = ScreenBuffer::new(5, 5, 100);
    if let Some(cell) = buf.get_mut(1, 0) {
        cell.grapheme = "Y".into();
    }
    buf.erase_lines(1, 2);
    assert_eq!(buf.get(1, 0), Some(&Cell::default()));
}
