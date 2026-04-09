// SPDX-License-Identifier: MPL-2.0

use crate::vt::VtProcessor;

fn make_vt(cols: u16, rows: u16) -> VtProcessor {
    VtProcessor::new(cols, rows, 10_000, 0, false)
}

#[test]
fn cr_marks_cursor_moved() {
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\r");
    assert!(dirty.cursor_moved, "CR must set cursor_moved");
    assert!(!dirty.is_empty(), "CR must yield non-empty DirtyRegion");
    assert!(
        dirty.rows.is_empty(),
        "CR must not mark any cell rows dirty"
    );
}

#[test]
fn cup_marks_cursor_moved() {
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[5;10H");
    assert!(dirty.cursor_moved, "CUP must set cursor_moved");
    assert!(!dirty.is_empty());
}

#[test]
fn cuu_marks_cursor_moved() {
    // Move cursor to row 5 first so there is room to move up.
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[6;1H");
    let dirty = vt.process(b"\x1b[2A");
    assert!(dirty.cursor_moved, "CUU must set cursor_moved");
}

#[test]
fn cud_marks_cursor_moved() {
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[2B");
    assert!(dirty.cursor_moved, "CUD must set cursor_moved");
}

#[test]
fn cuf_marks_cursor_moved() {
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[2C");
    assert!(dirty.cursor_moved, "CUF must set cursor_moved");
}

#[test]
fn cub_marks_cursor_moved() {
    // Move cursor right first so there is room to move left.
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[1;10H");
    let dirty = vt.process(b"\x1b[2D");
    assert!(dirty.cursor_moved, "CUB must set cursor_moved");
}

#[test]
fn cha_marks_cursor_moved() {
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[10G");
    assert!(dirty.cursor_moved, "CHA must set cursor_moved");
}

#[test]
fn vpa_marks_cursor_moved() {
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[5d");
    assert!(dirty.cursor_moved, "VPA must set cursor_moved");
}

#[test]
fn cnl_marks_cursor_moved() {
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[1E");
    assert!(dirty.cursor_moved, "CNL must set cursor_moved");
}

#[test]
fn cpl_marks_cursor_moved() {
    // Move cursor down first.
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;1H");
    let dirty = vt.process(b"\x1b[1F");
    assert!(dirty.cursor_moved, "CPL must set cursor_moved");
}

#[test]
fn lf_no_scroll_marks_cursor_moved() {
    // LF when cursor is not on the bottom row — moves cursor down without scrolling.
    let mut vt = make_vt(80, 24);
    // Cursor starts at row 0 — LF moves it to row 1.
    let dirty = vt.process(b"\x0A");
    assert!(dirty.cursor_moved, "LF (no scroll) must set cursor_moved");
}

#[test]
fn bs_marks_cursor_moved() {
    // Move right first, then BS.
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[1;5H");
    let dirty = vt.process(b"\x08");
    assert!(dirty.cursor_moved, "BS must set cursor_moved");
}

#[test]
fn decset_25l_marks_cursor_moved() {
    // Hide cursor — cursor_visible changes from true to false.
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[?25l");
    assert!(
        dirty.cursor_moved,
        "?25l (hide cursor) must mark cursor_moved"
    );
}

#[test]
fn decset_25h_no_change_does_not_mark() {
    // Show cursor when it is already visible — no change, no cursor_moved.
    let mut vt = make_vt(80, 24);
    // Default state: cursor_visible = true.
    let dirty = vt.process(b"\x1b[?25h");
    assert!(
        !dirty.cursor_moved,
        "?25h when cursor already visible must NOT mark cursor_moved"
    );
}

#[test]
fn cursor_only_move_no_dirty_rows() {
    // Pure cursor move: cursor_moved set, no dirty rows, not empty.
    let mut vt = make_vt(80, 24);
    let dirty = vt.process(b"\x1b[10;5H");
    assert!(!dirty.is_full_redraw, "CUP must not trigger full redraw");
    assert!(
        dirty.rows.is_empty(),
        "CUP must not mark any cell rows dirty"
    );
    assert!(dirty.cursor_moved, "CUP must set cursor_moved");
    assert!(!dirty.is_empty(), "CUP must yield non-empty DirtyRegion");
}
