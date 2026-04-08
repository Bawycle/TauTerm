// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::{grapheme_at, make_vt};

// ---------------------------------------------------------------------------
// TEST-VT-052 — CSI S / T (scroll up/down) within scroll region (FS-VT-052)
// ---------------------------------------------------------------------------

#[test]
fn csi_scroll_up_moves_content() {
    let mut vt = make_vt(10, 5);
    // Write distinct content on rows 0–2.
    vt.process(b"AAA\r\nBBB\r\nCCC");
    // Cursor is now on row 2. Set full-screen scroll region (rows 0–4, default).
    // CSI 1 S — scroll up 1 line within region.
    vt.process(b"\x1b[1S");
    // After scroll up: row 0 should contain what was on row 1 ("BBB").
    let g = grapheme_at(&vt, 0, 0);
    assert_eq!(
        g, "B",
        "after CSI S: row 0 must contain former row 1 content"
    );
}

#[test]
fn csi_scroll_down_moves_content() {
    let mut vt = make_vt(10, 5);
    // Write content on row 0.
    vt.process(b"AAA");
    // CSI 1 T — scroll down 1 line.
    vt.process(b"\x1b[1T");
    // Row 0 should now be blank; former row 0 content is on row 1.
    let g0 = grapheme_at(&vt, 0, 0);
    let g1 = grapheme_at(&vt, 1, 0);
    assert!(
        g0.trim().is_empty() || g0 == " ",
        "after CSI T: row 0 must be blank, got: {g0:?}"
    );
    assert_eq!(
        g1, "A",
        "after CSI T: former row 0 content must be on row 1"
    );
}

// ---------------------------------------------------------------------------
// R2 — LF outside scroll region (FS-VT-055)
// ---------------------------------------------------------------------------

/// R2-above: cursor above scroll region top — LF moves cursor down, no scroll.
#[test]
fn r2_lf_above_scroll_region_moves_cursor_no_scroll() {
    // 24-row terminal, scroll region rows 5–10 (0-based: top=4, bottom=9).
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;10r"); // DECSTBM: region rows 5–10 (1-based)
    // DECSTBM resets cursor to home (0,0). Cursor is at row 0 — above region top=4.
    assert_eq!(vt.normal_cursor.row, 0);
    // Position cursor at row 1 (0-based) — still above top=4.
    vt.process(b"\x1b[2;1H"); // CUP row=2 (1-based) → 0-based=1
    assert_eq!(vt.normal_cursor.row, 1);
    let scrollback_before = vt.normal.scrollback_len();
    // LF — must move down without scrolling region.
    vt.process(b"\n");
    assert_eq!(
        vt.normal_cursor.row, 2,
        "LF above region top must move cursor to row 2 (no scroll)"
    );
    assert_eq!(
        vt.normal.scrollback_len(),
        scrollback_before,
        "LF above region must not add to scrollback"
    );
}

/// R2-below: cursor below scroll region bottom — LF moves cursor down, no scroll.
#[test]
fn r2_lf_below_scroll_region_moves_cursor_no_scroll() {
    // 24-row terminal, region rows 5–10 (0-based: top=4, bottom=9).
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;10r"); // DECSTBM
    // Position cursor below region: row 15 (0-based), 1-based = 16.
    vt.process(b"\x1b[16;1H");
    assert_eq!(vt.normal_cursor.row, 15);
    let scrollback_before = vt.normal.scrollback_len();
    vt.process(b"\n");
    assert_eq!(
        vt.normal_cursor.row, 16,
        "LF below region bottom must move cursor to row 16 (no scroll)"
    );
    assert_eq!(
        vt.normal.scrollback_len(),
        scrollback_before,
        "LF below region must not add to scrollback"
    );
}

/// R2-last-row: cursor on last screen row and outside region — LF is ignored.
#[test]
fn r2_lf_at_last_row_outside_region_is_noop() {
    // 24-row terminal (rows 0–23), region rows 5–10 (top=4, bottom=9).
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;10r");
    // Position cursor at last row (row 23, 0-based), 1-based = 24.
    vt.process(b"\x1b[24;1H");
    assert_eq!(vt.normal_cursor.row, 23);
    let scrollback_before = vt.normal.scrollback_len();
    vt.process(b"\n");
    assert_eq!(
        vt.normal_cursor.row, 23,
        "LF at last screen row outside region must be ignored (no cursor move)"
    );
    assert_eq!(
        vt.normal.scrollback_len(),
        scrollback_before,
        "LF at last screen row outside region must not scroll"
    );
}

/// R2-in-region: cursor inside region at bottom — existing scroll behaviour preserved.
#[test]
fn r2_lf_at_region_bottom_scrolls_normally() {
    // 24-row terminal, region rows 5–10 (top=4, bottom=9).
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;10r");
    // Position cursor at region bottom (row 9, 0-based), 1-based = 10.
    vt.process(b"\x1b[10;1H");
    assert_eq!(vt.normal_cursor.row, 9);
    // LF at bottom of a PARTIAL region: scroll within region, no scrollback.
    let scrollback_before = vt.normal.scrollback_len();
    vt.process(b"\n");
    // Cursor stays at bottom of region after scroll.
    assert_eq!(
        vt.normal_cursor.row, 9,
        "LF at region bottom must keep cursor at region bottom after scroll"
    );
    // No scrollback for partial region.
    assert_eq!(
        vt.normal.scrollback_len(),
        scrollback_before,
        "partial region scroll must not add to scrollback"
    );
}

// ---------------------------------------------------------------------------
// Bug C — DECSTBM single-row region (top == bottom) must be accepted
// ---------------------------------------------------------------------------

#[test]
fn decstbm_single_row_region_accepted() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;5r");
    assert_eq!(
        vt.modes.scroll_region,
        (4, 4),
        "single-row DECSTBM must set scroll_region to (4, 4)"
    );
}

#[test]
fn decstbm_single_row_region_cursor_home() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[10;5H");
    vt.process(b"\x1b[5;5r");
    assert_eq!(vt.normal_cursor.row, 0, "DECSTBM must home cursor row to 0");
    assert_eq!(vt.normal_cursor.col, 0, "DECSTBM must home cursor col to 0");
}
