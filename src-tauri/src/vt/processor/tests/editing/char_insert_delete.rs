// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::{grapheme_at, make_vt};

// ---------------------------------------------------------------------------
// ICH — CSI Ps @ — Insert Character (ECMA-48 §8.3.64)
// ---------------------------------------------------------------------------

/// ICH basic: insert 1 blank at col 2, existing chars shift right.
/// "ABCDE" at cols 0-4 → after CSI 1 @ at col 2 → "AB CDE" (E pushed off if line is 5 cols)
#[test]
fn ich_inserts_blank_and_shifts_right() {
    let mut vt = make_vt(10, 5);
    // Write "ABCDE" then move cursor back to col 2.
    vt.process(b"ABCDE\x1b[1;3H"); // CUP row=1,col=3 → (row=0, col=2)
    vt.process(b"\x1b[1@"); // ICH 1
    assert_eq!(grapheme_at(&vt, 0, 0), "A");
    assert_eq!(grapheme_at(&vt, 0, 1), "B");
    assert_eq!(grapheme_at(&vt, 0, 2), " "); // blank inserted
    assert_eq!(grapheme_at(&vt, 0, 3), "C");
    assert_eq!(grapheme_at(&vt, 0, 4), "D");
    // E is at col 5 — shifted but not lost because line is 10 wide.
    assert_eq!(grapheme_at(&vt, 0, 5), "E");
}

/// ICH with N > remaining cols: remaining cells are blanked, nothing wraps.
#[test]
fn ich_clamps_to_line_end() {
    let mut vt = make_vt(5, 5);
    // "ABCDE", cursor at col 1.
    vt.process(b"ABCDE\x1b[1;2H");
    vt.process(b"\x1b[10@"); // ICH 10 — more than remaining cols
    assert_eq!(grapheme_at(&vt, 0, 0), "A");
    // cols 1-4 should all be blank.
    for col in 1..5 {
        assert_eq!(
            grapheme_at(&vt, 0, col),
            " ",
            "col {col} should be blank after ICH overcount"
        );
    }
}

/// ICH with N=0 is treated as N=1 (ECMA-48: default is 1).
#[test]
fn ich_n0_treated_as_1() {
    let mut vt = make_vt(10, 5);
    vt.process(b"ABCDE\x1b[1;1H"); // cursor at col 0
    vt.process(b"\x1b[@"); // ICH with no param → default 1
    assert_eq!(grapheme_at(&vt, 0, 0), " "); // blank at col 0
    assert_eq!(grapheme_at(&vt, 0, 1), "A"); // A shifted right
}

// ---------------------------------------------------------------------------
// DCH — CSI Ps P — Delete Character (ECMA-48 §8.3.26)
// ---------------------------------------------------------------------------

/// DCH basic: delete 1 char at col 2, chars to the right shift left.
#[test]
fn dch_deletes_and_shifts_left() {
    let mut vt = make_vt(10, 5);
    vt.process(b"ABCDE\x1b[1;3H"); // cursor at col 2
    vt.process(b"\x1b[1P"); // DCH 1
    assert_eq!(grapheme_at(&vt, 0, 0), "A");
    assert_eq!(grapheme_at(&vt, 0, 1), "B");
    assert_eq!(grapheme_at(&vt, 0, 2), "D"); // C deleted, D shifted left
    assert_eq!(grapheme_at(&vt, 0, 3), "E");
    assert_eq!(grapheme_at(&vt, 0, 4), " "); // trailing blank
}

/// DCH with N > remaining: all remaining cols become blank.
#[test]
fn dch_clamps_to_line_end() {
    let mut vt = make_vt(5, 5);
    vt.process(b"ABCDE\x1b[1;3H"); // cursor at col 2
    vt.process(b"\x1b[10P"); // DCH 10 — more than remaining
    assert_eq!(grapheme_at(&vt, 0, 0), "A");
    assert_eq!(grapheme_at(&vt, 0, 1), "B");
    for col in 2..5 {
        assert_eq!(
            grapheme_at(&vt, 0, col),
            " ",
            "col {col} should be blank after DCH overcount"
        );
    }
}

// ---------------------------------------------------------------------------
// ECH — Erase Character (CSI X)
// ---------------------------------------------------------------------------

/// ECH erases N chars at cursor position without moving the cursor.
#[test]
fn test_ech_erases_without_moving_cursor() {
    let mut vt = make_vt(80, 24);
    // Write "ABCDE" at row=0, then position cursor at col=1.
    vt.process(b"ABCDE");
    // Move cursor to col=1 (CHA 2).
    vt.process(b"\x1b[2G");
    // ECH 2 — erase 2 chars starting at col=1 (B and C).
    vt.process(b"\x1b[2X");
    // Cursor must remain at col=1.
    assert_eq!(vt.normal_cursor.col, 1, "ECH must not move the cursor");
    // Cell at col=0 must still be 'A'.
    assert_eq!(
        grapheme_at(&vt, 0, 0),
        "A",
        "ECH must not erase cells before cursor"
    );
    // Cells at col=1 and col=2 must be erased (default ' ').
    let g1 = grapheme_at(&vt, 0, 1);
    let g2 = grapheme_at(&vt, 0, 2);
    assert!(
        g1 == " " || g1.is_empty(),
        "ECH must erase cell at col=1, got {:?}",
        g1
    );
    assert!(
        g2 == " " || g2.is_empty(),
        "ECH must erase cell at col=2, got {:?}",
        g2
    );
    // Cell at col=3 must still be 'D'.
    assert_eq!(
        grapheme_at(&vt, 0, 3),
        "D",
        "ECH must not erase cells past N"
    );
}

/// ECH with N larger than remaining columns is clamped to end of line.
#[test]
fn test_ech_clamps_to_eol() {
    let mut vt = make_vt(10, 5);
    // Fill row=0 with 'X' chars.
    vt.process(b"XXXXXXXXXX");
    // Move to col=8 (CHA 9).
    vt.process(b"\x1b[9G");
    // ECH 999 — far beyond EOL; must clamp to remaining 2 cells (col=8, col=9).
    vt.process(b"\x1b[999X");
    // Cursor stays at col=8.
    assert_eq!(vt.normal_cursor.col, 8, "ECH must not move cursor");
    // Cells 8 and 9 must be erased.
    let g8 = grapheme_at(&vt, 0, 8);
    let g9 = grapheme_at(&vt, 0, 9);
    assert!(
        g8 == " " || g8.is_empty(),
        "ECH must erase col=8 when N > remaining"
    );
    assert!(
        g9 == " " || g9.is_empty(),
        "ECH must erase col=9 when N > remaining"
    );
    // Cells before cursor must be intact.
    assert_eq!(
        grapheme_at(&vt, 0, 0),
        "X",
        "ECH must not erase before cursor"
    );
}
