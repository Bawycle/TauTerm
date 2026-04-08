// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::{grapheme_at, make_vt};

// ---------------------------------------------------------------------------
// DECAWM — DEC Auto Wrap Mode (mode ?7)
// ---------------------------------------------------------------------------

/// DECAWM enabled (default): character at last column triggers auto-wrap.
#[test]
fn decawm_on_wraps_at_right_margin() {
    let mut vt = make_vt(5, 5);
    // Write 6 chars — the 6th should wrap to row 1 col 0.
    vt.process(b"ABCDEF");
    // Row 0: ABCDE, row 1: F
    assert_eq!(grapheme_at(&vt, 0, 0), "A");
    assert_eq!(grapheme_at(&vt, 0, 4), "E");
    assert_eq!(grapheme_at(&vt, 1, 0), "F");
}

/// DECAWM disabled (?7l): characters at or beyond the last column overwrite
/// the last column, cursor stays at last column.
#[test]
fn decawm_off_does_not_wrap() {
    let mut vt = make_vt(5, 5);
    vt.process(b"\x1b[?7l"); // DECAWM off
    // Write 7 chars — cols 0-4 fill normally, then chars 6 and 7 overwrite col 4.
    vt.process(b"ABCDEFG");
    // All chars should be on row 0; no wrap to row 1.
    assert_eq!(grapheme_at(&vt, 0, 0), "A");
    assert_eq!(grapheme_at(&vt, 0, 1), "B");
    assert_eq!(grapheme_at(&vt, 0, 2), "C");
    assert_eq!(grapheme_at(&vt, 0, 3), "D");
    // The last 3 chars (E, F, G) all land on col 4 — G is the final value.
    assert_eq!(grapheme_at(&vt, 0, 4), "G");
    // Row 1 must remain blank.
    let row1_text: String = (0..5).map(|c| grapheme_at(&vt, 1, c)).collect();
    assert!(
        row1_text.trim().is_empty(),
        "row 1 should be empty when DECAWM is off, got: {row1_text:?}"
    );
}

/// DECAWM can be re-enabled with ?7h after being disabled.
#[test]
fn decawm_can_be_reenabled() {
    let mut vt = make_vt(5, 5);
    vt.process(b"\x1b[?7l"); // disable
    vt.process(b"\x1b[?7h"); // re-enable
    vt.process(b"ABCDEF");
    // F should wrap to row 1.
    assert_eq!(grapheme_at(&vt, 1, 0), "F");
}

/// DECAWM off: cursor is clamped to last column, not one past it.
#[test]
fn decawm_off_cursor_stays_at_last_col() {
    let mut vt = make_vt(5, 5);
    vt.process(b"\x1b[?7l");
    vt.process(b"ABCDE"); // fill the line exactly
    // Cursor should be at col 4 (last column), not col 5 or wrap_pending.
    assert_eq!(
        vt.active_cursor().col,
        4,
        "cursor should be at last col after filling line with DECAWM off"
    );
    assert!(
        !vt.wrap_pending,
        "wrap_pending must be false when DECAWM is off"
    );
}

// -----------------------------------------------------------------------
// R3 — DECAWM saved/restored by DECSC (ESC 7 / CSI s) and DECRC (ESC 8 / CSI u)
//
// Spec: DECSC saves cursor position + SGR + charset + DECAWM.
//       DECRC restores all of the above atomically.
//
// Scenario tested:
//   1. Disable DECAWM (DECRST ?7)        → decawm = false
//   2. DECSC (ESC 7)                      → snapshot decawm = false
//   3. Re-enable DECAWM (DECSET ?7)       → decawm = true
//   4. DECRC (ESC 8)                      → decawm restored to false
// -----------------------------------------------------------------------

/// R3 (ESC 7 / ESC 8): DECAWM state is included in the DECSC snapshot and
/// fully restored by DECRC.
#[test]
fn r3_decawm_saved_and_restored_by_decsc_decrc_esc() {
    let mut vt = make_vt(80, 24);
    // 1. Disable DECAWM via DECRST ?7.
    vt.process(b"\x1b[?7l");
    assert!(
        !vt.mode_state().decawm,
        "DECAWM must be off after DECRST ?7"
    );
    // 2. Save cursor with ESC 7.
    vt.process(b"\x1b7");
    // 3. Re-enable DECAWM.
    vt.process(b"\x1b[?7h");
    assert!(vt.mode_state().decawm, "DECAWM must be on after DECSET ?7");
    // 4. Restore cursor with ESC 8 — DECAWM must revert to false.
    vt.process(b"\x1b8");
    assert!(
        !vt.mode_state().decawm,
        "DECRC (ESC 8) must restore DECAWM to false (R3)"
    );
}

/// R3 (CSI s / CSI u): DECAWM state is included in the DECSC snapshot and
/// fully restored by DECRC via the CSI variants.
#[test]
fn r3_decawm_saved_and_restored_by_decsc_decrc_csi() {
    let mut vt = make_vt(80, 24);
    // 1. Disable DECAWM.
    vt.process(b"\x1b[?7l");
    assert!(!vt.mode_state().decawm);
    // 2. Save cursor (CSI s).
    vt.process(b"\x1b[s");
    // 3. Re-enable DECAWM.
    vt.process(b"\x1b[?7h");
    assert!(vt.mode_state().decawm);
    // 4. Restore cursor (CSI u) — DECAWM must be false again.
    vt.process(b"\x1b[u");
    assert!(
        !vt.mode_state().decawm,
        "DECRC (CSI u) must restore DECAWM to false (R3)"
    );
}

/// R3: when DECAWM is enabled at save time, DECRC must restore it to true
/// even if it was disabled in between.
#[test]
fn r3_decawm_enabled_at_save_is_restored_after_disable() {
    let mut vt = make_vt(80, 24);
    // DECAWM starts true (default). Save it.
    assert!(vt.mode_state().decawm);
    vt.process(b"\x1b7"); // DECSC — saves decawm = true
    // Disable DECAWM.
    vt.process(b"\x1b[?7l");
    assert!(!vt.mode_state().decawm);
    // Restore — DECAWM must be true again.
    vt.process(b"\x1b8"); // DECRC
    assert!(
        vt.mode_state().decawm,
        "DECRC must restore DECAWM to true when it was true at save time (R3)"
    );
}

/// R3: DECRC with no prior DECSC must not change DECAWM (no saved state → no-op).
#[test]
fn r3_decrc_without_decsc_is_noop() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[?7l");
    assert!(!vt.mode_state().decawm);
    // No DECSC was issued. DECRC must be a no-op.
    vt.process(b"\x1b8");
    assert!(
        !vt.mode_state().decawm,
        "DECRC with no prior DECSC must not modify DECAWM (R3)"
    );
}

// ---------------------------------------------------------------------------
// Bug A — wrap_pending must be cleared by explicit cursor movements
// ---------------------------------------------------------------------------

#[test]
fn wrap_pending_cleared_by_cup() {
    let mut vt = make_vt(80, 24);
    vt.process(&[b'A'; 80]);
    assert!(
        vt.wrap_pending,
        "wrap_pending must be true after filling line"
    );
    vt.process(b"\x1b[1;1H");
    assert!(!vt.wrap_pending, "CUP must clear wrap_pending");
    vt.process(b"X");
    assert_eq!(grapheme_at(&vt, 0, 0), "X", "X must land at row=0, col=0");
    assert_eq!(vt.normal_cursor.row, 0, "cursor must remain on row 0");
}

#[test]
fn wrap_pending_cleared_by_cuu() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[3;1H");
    vt.process(&[b'A'; 80]);
    assert!(vt.wrap_pending);
    vt.process(b"\x1b[2A");
    assert!(!vt.wrap_pending, "CUU must clear wrap_pending");
}

#[test]
fn wrap_pending_cleared_by_cud() {
    let mut vt = make_vt(80, 24);
    vt.process(&[b'A'; 80]);
    assert!(vt.wrap_pending);
    vt.process(b"\x1b[1B");
    assert!(!vt.wrap_pending, "CUD must clear wrap_pending");
}

#[test]
fn wrap_pending_cleared_by_cha() {
    let mut vt = make_vt(80, 24);
    vt.process(&[b'A'; 80]);
    assert!(vt.wrap_pending);
    vt.process(b"\x1b[1G");
    assert!(!vt.wrap_pending, "CHA must clear wrap_pending");
}

#[test]
fn wrap_pending_cleared_by_vpa() {
    let mut vt = make_vt(80, 24);
    vt.process(&[b'A'; 80]);
    assert!(vt.wrap_pending);
    vt.process(b"\x1b[5d");
    assert!(!vt.wrap_pending, "VPA must clear wrap_pending");
}

#[test]
fn wrap_pending_cleared_by_decrc() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b7");
    vt.process(&[b'A'; 80]);
    assert!(vt.wrap_pending, "wrap_pending must be true before restore");
    vt.process(b"\x1b8");
    assert!(!vt.wrap_pending, "DECRC (ESC 8) must clear wrap_pending");
}

#[test]
fn wrap_pending_cleared_by_decstbm() {
    let mut vt = make_vt(80, 24);
    vt.process(&[b'A'; 80]);
    assert!(vt.wrap_pending);
    vt.process(b"\x1b[1;20r");
    assert!(!vt.wrap_pending, "DECSTBM must clear wrap_pending");
    assert_eq!(vt.normal_cursor.row, 0);
    assert_eq!(vt.normal_cursor.col, 0);
}

#[test]
fn wrap_pending_cleared_by_ed2() {
    let mut vt = make_vt(80, 24);
    vt.process(&[b'A'; 80]);
    assert!(vt.wrap_pending);
    vt.process(b"\x1b[2J");
    assert!(!vt.wrap_pending, "ED 2 must clear wrap_pending");
}

#[test]
fn htop_footer_scenario() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[23;1H");
    vt.process(&[b'X'; 79]);
    assert!(!vt.wrap_pending);
    vt.process(b"\x1b[1;23r");
    assert!(!vt.wrap_pending, "DECSTBM must clear wrap_pending");
    assert_eq!(vt.normal_cursor.row, 0);
    vt.process(b"\x1b[24;1H");
    assert_eq!(vt.normal_cursor.row, 23);
    vt.process(b"FOOTER");
    assert_eq!(grapheme_at(&vt, 23, 0), "F");
    assert_eq!(grapheme_at(&vt, 23, 1), "O");
    assert_eq!(grapheme_at(&vt, 22, 0), "X");
}

#[test]
fn htop_footer_scenario_with_wrap_pending() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[23;1H");
    vt.process(&[b'X'; 80]);
    assert!(vt.wrap_pending);
    vt.process(b"\x1b[1;23r");
    assert!(!vt.wrap_pending, "DECSTBM must clear wrap_pending");
    assert_eq!(vt.normal_cursor.row, 0);
    vt.process(b"\x1b[24;1H");
    assert_eq!(vt.normal_cursor.row, 23);
    vt.process(b"FOOTER");
    assert_eq!(grapheme_at(&vt, 23, 0), "F", "footer must land on row 23");
    assert_eq!(grapheme_at(&vt, 22, 0), "X");
}
