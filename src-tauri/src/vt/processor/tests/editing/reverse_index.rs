// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::{grapheme_at, make_vt};

// ---------------------------------------------------------------------------
// RI — ESC M — Reverse Index
// ---------------------------------------------------------------------------

/// RI when cursor is NOT at the top of scroll region: cursor moves up one row.
#[test]
fn ri_moves_cursor_up_when_not_at_top() {
    let mut vt = make_vt(10, 5);
    // Move cursor to row 2.
    vt.process(b"\x1b[3;1H");
    vt.process(b"\x1bM"); // RI
    assert_eq!(
        vt.active_cursor().row,
        1,
        "RI should move cursor up one row"
    );
}

/// RI when cursor is AT the top of scroll region: scrolls down (inserts blank at top).
#[test]
fn ri_scrolls_down_when_at_scroll_top() {
    let mut vt = make_vt(10, 5);
    vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
    // Cursor at row 0 (top of default scroll region).
    vt.process(b"\x1b[1;1H");
    vt.process(b"\x1bM"); // RI
    // Row 0 should now be blank (new blank line inserted at top).
    let row0_text: String = (0..10).map(|c| grapheme_at(&vt, 0, c)).collect();
    assert!(
        row0_text.trim().is_empty(),
        "row 0 should be blank after RI at top of scroll region, got: {row0_text:?}"
    );
    // Original row 0 should now be at row 1.
    assert_eq!(grapheme_at(&vt, 1, 0), "R");
    assert_eq!(grapheme_at(&vt, 1, 3), "0");
    // Cursor should remain at row 0.
    assert_eq!(vt.active_cursor().row, 0);
}

/// RI at top of a non-default scroll region scrolls within that region.
#[test]
fn ri_at_top_of_partial_scroll_region() {
    let mut vt = make_vt(10, 5);
    vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
    // Scroll region rows 2-4 (1-based) → 0-based (1,3).
    vt.process(b"\x1b[2;4r");
    // Move cursor to row 1 (0-based) = top of scroll region.
    vt.process(b"\x1b[2;1H");
    vt.process(b"\x1bM"); // RI
    // Row 1 should be blank (inserted).
    let row1_text: String = (0..10).map(|c| grapheme_at(&vt, 1, c)).collect();
    assert!(
        row1_text.trim().is_empty(),
        "row 1 (top of partial scroll region) should be blank after RI, got: {row1_text:?}"
    );
    // Row 0 must be unchanged (outside scroll region).
    assert_eq!(grapheme_at(&vt, 0, 0), "R");
    assert_eq!(grapheme_at(&vt, 0, 3), "0");
}
