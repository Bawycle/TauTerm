// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

use super::super::buffer::ScreenBuffer;

#[test]
fn new_buffer_has_correct_dimensions() {
    let buf = ScreenBuffer::new(80, 24, 1000);
    assert_eq!(buf.cols, 80);
    assert_eq!(buf.rows, 24);
    assert_eq!(buf.scrollback_limit, 1000);
}

#[test]
fn new_buffer_cells_are_default() {
    let buf = ScreenBuffer::new(10, 5, 100);
    for row in 0..5u16 {
        for col in 0..10u16 {
            assert_eq!(buf.get(row, col), Some(&Cell::default()));
        }
    }
}

#[test]
fn new_buffer_has_empty_scrollback() {
    let buf = ScreenBuffer::new(80, 24, 1000);
    assert_eq!(buf.scrollback_len(), 0);
}

#[test]
fn get_out_of_bounds_returns_none() {
    let buf = ScreenBuffer::new(5, 5, 100);
    assert!(buf.get(5, 0).is_none());
    assert!(buf.get(0, 5).is_none());
    assert!(buf.get(10, 10).is_none());
}

#[test]
fn get_mut_marks_row_dirty() {
    let mut buf = ScreenBuffer::new(5, 5, 100);
    // Take initial clean dirty region.
    let _ = buf.take_dirty();
    let _ = buf.get_mut(2, 3);
    let dirty = buf.take_dirty();
    assert!(dirty.rows.contains(2));
}

#[test]
fn take_dirty_clears_region() {
    let mut buf = ScreenBuffer::new(5, 5, 100);
    let _ = buf.get_mut(0, 0);
    let dirty = buf.take_dirty();
    assert!(!dirty.is_empty());
    let after = buf.take_dirty();
    assert!(after.is_empty());
}

#[test]
fn resize_updates_dimensions() {
    let mut buf = ScreenBuffer::new(80, 24, 1000);
    buf.resize(120, 40);
    assert_eq!(buf.cols, 120);
    assert_eq!(buf.rows, 40);
}

#[test]
fn resize_triggers_full_redraw() {
    let mut buf = ScreenBuffer::new(80, 24, 1000);
    let _ = buf.take_dirty();
    buf.resize(100, 30);
    let dirty = buf.take_dirty();
    assert!(dirty.is_full_redraw);
}
