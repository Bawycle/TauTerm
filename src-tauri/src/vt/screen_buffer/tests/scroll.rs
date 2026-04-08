// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

use super::super::buffer::ScreenBuffer;

#[test]
fn scroll_up_full_screen_adds_to_scrollback() {
    let mut buf = ScreenBuffer::new(5, 3, 100);
    // Write something on row 0 so we can identify it in scrollback.
    if let Some(cell) = buf.get_mut(0, 0) {
        cell.grapheme = "A".into();
    }
    buf.scroll_up(0, 2, 1, true, false);
    assert_eq!(buf.scrollback_len(), 1);
}

#[test]
fn scroll_up_partial_region_does_not_add_to_scrollback() {
    let mut buf = ScreenBuffer::new(5, 5, 100);
    buf.scroll_up(1, 3, 1, false, false);
    assert_eq!(buf.scrollback_len(), 0);
}

#[test]
fn scroll_down_one_line_region_does_not_panic() {
    // Regression: scroll_down with top == bottom (1-row region) must not
    // panic. The old guard `top >= bottom` would early-return, preventing
    // the scroll; the corrected guard `top > bottom` allows scroll_down to
    // operate on a single row (clear it), which is the correct VT behaviour
    // for CSI T (SD) targeting a 1-row region.
    let mut buf = ScreenBuffer::new(5, 5, 100);
    // Write a marker on row 2.
    if let Some(cell) = buf.get_mut(2, 0) {
        cell.grapheme = "X".into();
    }
    // scroll_down with a 1-row region [2, 2] — must not panic.
    buf.scroll_down(2, 2, 1);
    // The single row in the region is cleared to Cell::default().
    assert_eq!(
        buf.get(2, 0).map(|c| c.grapheme.as_str()),
        Some(&*Cell::default().grapheme)
    );
}

#[test]
fn scrollback_respects_limit() {
    let limit = 3usize;
    let mut buf = ScreenBuffer::new(5, 1, limit);
    for _ in 0..10 {
        buf.scroll_up(0, 0, 1, true, false);
    }
    assert!(buf.scrollback_len() <= limit);
}

#[test]
fn scroll_eviction_content_preserved_in_scrollback() {
    let mut buf = ScreenBuffer::new(5, 3, 10);
    // Write 'H' into cell (0, 0)
    if let Some(cell) = buf.get_mut(0, 0) {
        cell.grapheme = "H".into();
        cell.width = 1;
    }
    // Scroll up by 1 full-screen
    buf.scroll_up(0, 2, 1, true, false);
    // The evicted row should be in scrollback
    assert_eq!(buf.scrollback_len(), 1);
    let line = buf
        .get_scrollback_line(0)
        .expect("scrollback must have 1 line");
    assert_eq!(
        line.cells[0].grapheme, "H",
        "scrollback must preserve evicted row content"
    );
}

#[test]
fn scroll_eviction_bottom_row_is_blank_after_scroll() {
    let mut buf = ScreenBuffer::new(5, 3, 10);
    // Write something on every row
    for row in 0..3u16 {
        if let Some(cell) = buf.get_mut(row, 0) {
            cell.grapheme = "X".into();
        }
    }
    buf.scroll_up(0, 2, 1, true, false);
    // Bottom row (row 2) must now be blank
    let bottom = buf.get_row(2).expect("row 2 must exist");
    for cell in bottom {
        assert_eq!(
            cell.grapheme, " ",
            "bottom row must be blank (space) after scroll"
        );
    }
}
