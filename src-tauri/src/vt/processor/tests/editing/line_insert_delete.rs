// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::{make_vt, grapheme_at};

// ---------------------------------------------------------------------------
// IL — CSI Ps L — Insert Line (ECMA-48 / xterm)
// ---------------------------------------------------------------------------

/// IL basic: insert 1 blank line at cursor row; lines below shift down.
/// Last line in scroll region is lost.
#[test]
fn il_inserts_blank_line_and_shifts_down() {
    let mut vt = make_vt(10, 5);
    // Fill rows 0-2 with distinguishable content.
    vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
    // Move cursor to row 1.
    vt.process(b"\x1b[2;1H"); // CUP row=2 (1-based) → row=1 (0-based)
    vt.process(b"\x1b[1L"); // IL 1
    // Row 1 is now blank.
    let row1_text: String = (0..10).map(|c| grapheme_at(&vt, 1, c)).collect();
    assert!(
        row1_text.trim().is_empty(),
        "row 1 should be blank after IL, got: {row1_text:?}"
    );
    // Row 0 is unchanged.
    assert_eq!(grapheme_at(&vt, 0, 0), "R");
    assert_eq!(grapheme_at(&vt, 0, 1), "O");
    // Original row 1 content is now at row 2.
    assert_eq!(grapheme_at(&vt, 2, 0), "R");
    assert_eq!(grapheme_at(&vt, 2, 3), "1");
}

/// IL within a scroll region: lines below bottom of region are unaffected.
#[test]
fn il_respects_scroll_region() {
    let mut vt = make_vt(10, 5);
    vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
    // Set scroll region rows 1-3 (1-based).
    vt.process(b"\x1b[2;4r"); // DECSTBM top=2,bottom=4 → 0-based (1,3)
    // Move cursor to row 1 (0-based), which is inside the region.
    vt.process(b"\x1b[2;1H");
    vt.process(b"\x1b[1L"); // IL 1
    // Row 4 (0-based) is outside the region — must be unchanged.
    assert_eq!(grapheme_at(&vt, 4, 0), "R");
    assert_eq!(grapheme_at(&vt, 4, 3), "4");
}

// ---------------------------------------------------------------------------
// DL — CSI Ps M — Delete Line (ECMA-48 / xterm)
// ---------------------------------------------------------------------------

/// DL basic: delete 1 line at cursor row; lines below shift up.
#[test]
fn dl_deletes_line_and_shifts_up() {
    let mut vt = make_vt(10, 5);
    vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
    // Move cursor to row 1.
    vt.process(b"\x1b[2;1H");
    vt.process(b"\x1b[1M"); // DL 1
    // Row 1 should now contain what was row 2.
    assert_eq!(grapheme_at(&vt, 1, 0), "R");
    assert_eq!(grapheme_at(&vt, 1, 3), "2");
    // Row 4 (last) should now be blank.
    let row4_text: String = (0..10).map(|c| grapheme_at(&vt, 4, c)).collect();
    assert!(
        row4_text.trim().is_empty(),
        "last row should be blank after DL, got: {row4_text:?}"
    );
}

/// DL respects the bottom of scroll region.
#[test]
fn dl_respects_scroll_region() {
    let mut vt = make_vt(10, 5);
    vt.process(b"ROW0\r\nROW1\r\nROW2\r\nROW3\r\nROW4");
    // Scroll region rows 1-3 (1-based).
    vt.process(b"\x1b[2;4r");
    vt.process(b"\x1b[2;1H");
    vt.process(b"\x1b[1M");
    // Row 4 (outside region) must be unchanged.
    assert_eq!(grapheme_at(&vt, 4, 0), "R");
    assert_eq!(grapheme_at(&vt, 4, 3), "4");
}
