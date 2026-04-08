// SPDX-License-Identifier: MPL-2.0

use super::super::buffer::ScreenBuffer;

#[test]
fn snapshot_cell_count_equals_cols_times_rows() {
    let buf = ScreenBuffer::new(80, 24, 1000);
    let snap = buf.snapshot(0, 0, true, 0, 0);
    assert_eq!(snap.cells.len(), 80 * 24);
    assert_eq!(snap.cols, 80);
    assert_eq!(snap.rows, 24);
}
