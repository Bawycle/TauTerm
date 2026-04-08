// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::make_vt;

// ---------------------------------------------------------------------------
// TEST: resize clamps cursor positions
// FS-PTY-009, FS-PTY-010
// ---------------------------------------------------------------------------

#[test]
fn resize_clamps_cursor_positions_to_new_bounds() {
    let mut vt = make_vt(80, 24);
    // Move cursor to row 20, col 70.
    vt.process(b"\x1b[21;71H");
    assert_eq!(vt.normal_cursor.row, 20);
    assert_eq!(vt.normal_cursor.col, 70);
    // Resize to smaller dimensions.
    vt.resize(40, 10);
    assert!(
        vt.normal_cursor.row < 10,
        "cursor row must be clamped to new rows"
    );
    assert!(
        vt.normal_cursor.col < 40,
        "cursor col must be clamped to new cols"
    );
}

// ---------------------------------------------------------------------------
// DECSC / DECRC — save and restore attrs + charset_slot (fix #1)
// ---------------------------------------------------------------------------

/// DECSC (ESC 7) saves current SGR attributes; DECRC (ESC 8) restores them.
#[test]
fn decsc_decrc_esc_saves_and_restores_attrs() {
    let mut vt = make_vt(80, 24);
    // Set bold + red foreground.
    vt.process(b"\x1b[1;31m"); // bold + red
    // Save cursor (ESC 7).
    vt.process(b"\x1b7");
    // Reset SGR.
    vt.process(b"\x1b[0m");
    // Confirm attrs are now default.
    assert!(
        !vt.current_attrs.bold,
        "attrs should be reset after ESC [0m"
    );
    // Restore cursor (ESC 8).
    vt.process(b"\x1b8");
    // Attrs should be restored to bold + red.
    assert!(vt.current_attrs.bold, "DECRC must restore bold attribute");
    assert_eq!(
        vt.current_attrs.fg,
        Some(crate::vt::cell::Color::Ansi { index: 1 }),
        "DECRC must restore fg color"
    );
}

/// DECSC (ESC 7) saves charset_slot; DECRC (ESC 8) restores it.
#[test]
fn decsc_decrc_esc_saves_and_restores_charset_slot() {
    use crate::vt::modes::CharsetSlot;
    let mut vt = make_vt(80, 24);
    // Switch to G1 (SO = 0x0E).
    vt.process(b"\x0E"); // SO → G1
    assert_eq!(vt.modes.charset_slot, CharsetSlot::G1);
    // Save cursor.
    vt.process(b"\x1b7");
    // Switch back to G0 (SI = 0x0F).
    vt.process(b"\x0F"); // SI → G0
    assert_eq!(vt.modes.charset_slot, CharsetSlot::G0);
    // Restore cursor — charset_slot should return to G1.
    vt.process(b"\x1b8");
    assert_eq!(
        vt.modes.charset_slot,
        CharsetSlot::G1,
        "DECRC must restore charset_slot"
    );
}

/// DECSC (CSI s) / DECRC (CSI u) also save/restore attrs.
#[test]
fn decsc_decrc_csi_saves_and_restores_attrs() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[1m"); // bold
    vt.process(b"\x1b[s"); // CSI s = DECSC
    vt.process(b"\x1b[0m"); // reset
    assert!(!vt.current_attrs.bold);
    vt.process(b"\x1b[u"); // CSI u = DECRC
    assert!(vt.current_attrs.bold, "CSI u must restore bold");
}

// ---------------------------------------------------------------------------
// u16 underflow guards — no panic on 1-row / 1-col terminal (fix #3)
// ---------------------------------------------------------------------------

/// Resize to 1 row must not panic on LF (scroll_up path uses rows - 1).
#[test]
fn no_panic_on_lf_with_one_row() {
    let mut vt = make_vt(80, 1);
    // LF on a 1-row terminal would have triggered u16 underflow before the fix.
    vt.process(b"A\n");
    // If we reach here without panic, the guard works.
}

/// Resize to 1 col must not panic on HT (tab stop uses cols - 1).
#[test]
fn no_panic_on_ht_with_one_col() {
    let mut vt = make_vt(1, 24);
    vt.process(b"\x09"); // HT
}

/// CUF (cursor forward) on a 1-col terminal must not panic.
#[test]
fn no_panic_on_cuf_with_one_col() {
    let mut vt = make_vt(1, 24);
    vt.process(b"\x1b[C"); // CUF 1
}

/// CUP on a 1×1 terminal must not panic.
#[test]
fn no_panic_on_cup_with_one_by_one() {
    let mut vt = make_vt(1, 1);
    vt.process(b"\x1b[1;1H"); // CUP 1,1
}

/// DECSTBM default (param1=0) on a 1-row terminal must not panic.
#[test]
fn no_panic_on_decstbm_with_one_row() {
    let mut vt = make_vt(80, 1);
    vt.process(b"\x1b[r"); // DECSTBM with defaults
}

// ---------------------------------------------------------------------------
// CHA — Cursor Horizontal Absolute (CSI G)
// ---------------------------------------------------------------------------

/// CHA positions the cursor at the column indicated (1-based → 0-indexed).
#[test]
fn test_cha_basic() {
    let mut vt = make_vt(80, 24);
    // CSI 5 G — move to column 5 (1-based), i.e. col=4 (0-based).
    vt.process(b"\x1b[5G");
    assert_eq!(vt.normal_cursor.col, 4, "CHA 5 must place cursor at col=4");
}

/// CHA with value exceeding column count is clamped to cols-1.
#[test]
fn test_cha_clamps_to_cols() {
    let mut vt = make_vt(80, 24);
    // CSI 999 G — far beyond 80 cols; must clamp to col=79.
    vt.process(b"\x1b[999G");
    assert_eq!(
        vt.normal_cursor.col, 79,
        "CHA beyond cols must clamp to cols-1"
    );
}

// ---------------------------------------------------------------------------
// VPA — Vertical Position Absolute (CSI d)
// ---------------------------------------------------------------------------

/// VPA positions the cursor at the row indicated (1-based → 0-indexed).
#[test]
fn test_vpa_basic() {
    let mut vt = make_vt(80, 24);
    // CSI 3 d — move to row 3 (1-based), i.e. row=2 (0-based).
    vt.process(b"\x1b[3d");
    assert_eq!(vt.normal_cursor.row, 2, "VPA 3 must place cursor at row=2");
}

/// VPA with value exceeding row count is clamped to rows-1.
#[test]
fn test_vpa_clamps_to_rows() {
    let mut vt = make_vt(80, 24);
    // CSI 999 d — far beyond 24 rows; must clamp to row=23.
    vt.process(b"\x1b[999d");
    assert_eq!(
        vt.normal_cursor.row, 23,
        "VPA beyond rows must clamp to rows-1"
    );
}

// ---------------------------------------------------------------------------
// HPA — Horizontal Position Absolute (CSI `)
// ---------------------------------------------------------------------------

/// HPA (backtick) behaves identically to CHA.
#[test]
fn test_hpa_equivalent_to_cha() {
    let mut vt = make_vt(80, 24);
    // CSI 10 ` — move to column 10 (1-based), i.e. col=9 (0-based).
    vt.process(b"\x1b[10`");
    assert_eq!(
        vt.normal_cursor.col, 9,
        "HPA must behave identically to CHA"
    );
}

// ---------------------------------------------------------------------------
// CNL — Cursor Next Line (CSI E)
// ---------------------------------------------------------------------------

/// CNL moves cursor down N lines and sets col=0.
#[test]
fn test_cnl_moves_cursor() {
    let mut vt = make_vt(80, 24);
    // Start at col=5 via CHA.
    vt.process(b"\x1b[6G");
    // CNL 3 — move down 3 lines.
    vt.process(b"\x1b[3E");
    assert_eq!(vt.normal_cursor.row, 3, "CNL 3 must move to row=3");
    assert_eq!(vt.normal_cursor.col, 0, "CNL must set col=0");
}

/// CNL with DECOM off clamps to screen bottom, not scroll region bottom.
#[test]
fn test_cnl_respects_scroll_bottom() {
    let mut vt = make_vt(80, 24);
    // Set scroll region rows 2–5 (1-based): DECSTBM CSI 2 ; 5 r.
    vt.process(b"\x1b[2;5r");
    // Position cursor at row=4 (CUP row=5, col=1 in 1-based → row=4).
    vt.process(b"\x1b[5;1H");
    // CNL 999 — with DECOM off, must clamp to screen bottom (row=23), not scroll region bottom.
    vt.process(b"\x1b[999E");
    assert_eq!(
        vt.normal_cursor.row, 23,
        "CNL with DECOM off must clamp to screen bottom (row 23), not scroll region bottom"
    );
    assert_eq!(vt.normal_cursor.col, 0, "CNL must set col=0");
}

// ---------------------------------------------------------------------------
// CPL — Cursor Previous Line (CSI F)
// ---------------------------------------------------------------------------

/// CPL moves cursor up N lines and sets col=0.
#[test]
fn test_cpl_moves_cursor() {
    let mut vt = make_vt(80, 24);
    // Position at row=5 via CUP.
    vt.process(b"\x1b[6;5H");
    // CPL 2 — move up 2 lines.
    vt.process(b"\x1b[2F");
    assert_eq!(
        vt.normal_cursor.row, 3,
        "CPL 2 from row=5 must land at row=3"
    );
    assert_eq!(vt.normal_cursor.col, 0, "CPL must set col=0");
}

/// CPL with DECOM off clamps to screen top, not scroll region top.
#[test]
fn test_cpl_respects_scroll_top() {
    let mut vt = make_vt(80, 24);
    // Set scroll region rows 3–10 (1-based): DECSTBM CSI 3 ; 10 r → top=2, bottom=9 (0-based).
    vt.process(b"\x1b[3;10r");
    // Position cursor at row=3 (CUP row=4, col=1 → row=3 0-based).
    vt.process(b"\x1b[4;1H");
    // CPL 999 — with DECOM off, must clamp to screen top (row=0), not scroll region top.
    vt.process(b"\x1b[999F");
    assert_eq!(
        vt.normal_cursor.row, 0,
        "CPL with DECOM off must clamp to screen top (row 0), not scroll region top"
    );
    assert_eq!(vt.normal_cursor.col, 0, "CPL must set col=0");
}

// ---------------------------------------------------------------------------
// Bug B — CUU/CUD/CNL/CPL/VPR clamp to screen edges when DECOM is off
// ---------------------------------------------------------------------------

#[test]
fn cuu_clamps_to_screen_top_when_decom_off() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;20r");
    vt.process(b"\x1b[7;1H");
    assert_eq!(vt.normal_cursor.row, 6);
    vt.process(b"\x1b[10A");
    assert_eq!(
        vt.normal_cursor.row, 0,
        "CUU with DECOM off must clamp to screen top (row 0), not scroll region top (row 4)"
    );
}

#[test]
fn cud_clamps_to_screen_bottom_when_decom_off() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[1;15r");
    vt.process(b"\x1b[14;1H");
    assert_eq!(vt.normal_cursor.row, 13);
    vt.process(b"\x1b[10B");
    assert_eq!(
        vt.normal_cursor.row, 23,
        "CUD with DECOM off must clamp to screen bottom (row 23), not scroll region bottom (row 14)"
    );
}

#[test]
fn cuu_clamps_to_region_top_when_decom_on() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;20r");
    vt.process(b"\x1b[?6h");
    vt.process(b"\x1b[3;1H");
    assert_eq!(vt.normal_cursor.row, 6);
    vt.process(b"\x1b[10A");
    assert_eq!(
        vt.normal_cursor.row, 4,
        "CUU with DECOM on must clamp to scroll region top (row 4)"
    );
}
