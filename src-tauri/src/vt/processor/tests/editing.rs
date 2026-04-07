// SPDX-License-Identifier: MPL-2.0

use super::helpers::*;

// ---------------------------------------------------------------------------
// TEST-VT-012 — OSC title sanitization (via parse_osc, exercised end-to-end)
// FS-VT-060, FS-VT-062
// ---------------------------------------------------------------------------

#[test]
fn osc_title_control_chars_are_stripped() {
    // TEST-VT-012 step 3-4
    let mut vt = make_vt(80, 24);
    // OSC title containing a C0 control char (0x01).
    vt.process(b"\x1b]0;Title\x01WithControl\x07");
    assert!(
        !vt.title.contains('\x01'),
        "C0 control chars must be stripped from OSC title"
    );
}

#[test]
fn osc_title_truncated_to_256_chars() {
    // TEST-VT-012 step 5-6
    let mut vt = make_vt(80, 24);
    let long_title: Vec<u8> = std::iter::once(b'\x1b')
        .chain(b"]0;".iter().copied())
        .chain(std::iter::repeat_n(b'A', 300))
        .chain(std::iter::once(b'\x07'))
        .collect();
    vt.process(&long_title);
    assert!(
        vt.title.len() <= 256,
        "OSC title must be truncated to max 256 chars, got {}",
        vt.title.len()
    );
}

#[test]
fn osc_title_plain_title_is_stored() {
    // TEST-VT-012 step 1-2
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b]0;My Title\x07");
    assert_eq!(vt.title, "My Title");
}

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
// TEST-VT-012 — Combining / zero-width characters (FS-VT-012/013)
// ---------------------------------------------------------------------------

/// A combining character (width=0) must attach to the previous cell and must
/// not advance the cursor.
#[test]
fn combining_char_attaches_to_previous_cell_no_cursor_advance() {
    let mut vt = make_vt(80, 24);
    // Write 'e' followed by combining acute accent U+0301 (width=0).
    vt.process("e\u{0301}".as_bytes());
    // The grapheme at (0,0) must contain both codepoints.
    let g = grapheme_at(&vt, 0, 0);
    assert!(
        g.contains('e') && g.contains('\u{0301}'),
        "combining acute accent must merge into the base char cell, got: {g:?}"
    );
    // The cursor must be at col=1, not col=2 (no extra advance for the combining char).
    assert_eq!(
        vt.normal_cursor.col, 1,
        "cursor must be at col=1 after e + combining"
    );
}

/// Combining character at column 0 must attach to cell (0,0) without panicking.
#[test]
fn combining_char_at_column_zero_does_not_panic() {
    let mut vt = make_vt(80, 24);
    // Feed a combining mark at the very start — should attach to cell (0,0).
    vt.process("\u{0301}".as_bytes()); // combining acute at col=0
    // No panic is the primary assertion.
    let snap = vt.get_snapshot();
    assert_eq!(snap.cursor_row, 0);
    assert_eq!(
        snap.cursor_col, 0,
        "cursor must not move for a combining-only input"
    );
}

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
// TEST-VT-030 — DECSCUSR cursor shape (FS-VT-030)
// ---------------------------------------------------------------------------

#[test]
fn decscusr_sets_cursor_shape_and_flags_change() {
    let mut vt = make_vt(80, 24);
    assert_eq!(vt.cursor_shape, 0, "default cursor shape must be 0");
    assert!(!vt.cursor_shape_changed);

    // CSI 2 SP q — steady block.
    vt.process(b"\x1b[2 q");
    assert_eq!(
        vt.cursor_shape, 2,
        "cursor shape must be 2 after DECSCUSR 2"
    );
    assert!(
        vt.cursor_shape_changed,
        "cursor_shape_changed flag must be set"
    );

    // take_cursor_shape_changed must return Some and reset the flag.
    let shape = vt.take_cursor_shape_changed();
    assert_eq!(shape, Some(2));
    assert!(!vt.cursor_shape_changed, "flag must be reset after take");
}

#[test]
fn decscusr_same_value_does_not_set_changed_flag() {
    let mut vt = make_vt(80, 24);
    // Already at shape=0; sending DECSCUSR 0 must not set the flag.
    vt.process(b"\x1b[0 q");
    assert!(
        !vt.cursor_shape_changed,
        "no change: flag must remain false"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-090 — BEL rate limiting (FS-VT-090)
// ---------------------------------------------------------------------------

#[test]
fn bel_sets_bell_pending() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x07");
    assert!(vt.bell_pending, "BEL must set bell_pending");
    let fired = vt.take_bell_pending();
    assert!(fired, "take_bell_pending must return true");
    assert!(!vt.bell_pending, "flag must be reset after take");
}

#[test]
fn bel_rate_limited_second_immediate_bell_ignored() {
    let mut vt = make_vt(80, 24);
    // First BEL — allowed.
    vt.process(b"\x07");
    let _ = vt.take_bell_pending(); // consume + reset

    // Second BEL immediately after — must be suppressed (< 100 ms).
    vt.process(b"\x07");
    assert!(
        !vt.bell_pending,
        "second immediate BEL must be suppressed by rate limit"
    );
}

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
// R7 — Skin-tone modifiers U+1F3FB–U+1F3FF treated as combining (FS-VT-018)
// ---------------------------------------------------------------------------

/// "👍\u{1F3FB}": thumbs-up + light skin tone modifier must occupy 2 cells,
/// with the modifier attached to the base emoji grapheme.
#[test]
fn r7_skin_tone_modifier_attaches_to_base_emoji_two_cells() {
    let mut vt = make_vt(80, 24);
    // U+1F44D (thumbs up, width=2) followed by U+1F3FB (light skin tone).
    vt.process("👍\u{1F3FB}".as_bytes());
    // The whole sequence must occupy exactly 2 cells (not 4).
    // Cell at (0,0) must contain both codepoints as a single grapheme.
    let g = vt
        .active_buf_ref()
        .get(0, 0)
        .map(|c| c.grapheme.clone())
        .unwrap_or_default();
    assert!(
        g.contains('👍') && g.contains('\u{1F3FB}'),
        "cell (0,0) must contain base emoji + skin-tone modifier, got: {g:?}"
    );
    let w = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(w, 2, "base emoji cell must still have width=2");
    // Phantom cell at (0,1).
    let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
    assert_eq!(w1, 0, "cell (0,1) must be phantom (width=0)");
    // Cursor must be at col=2 (only the base emoji's advance).
    assert_eq!(
        vt.normal_cursor.col, 2,
        "cursor must be at col=2, not col=4"
    );
}

/// Skin-tone modifier at column 0 (no preceding cell) must not panic.
#[test]
fn r7_skin_tone_at_column_zero_does_not_panic() {
    let mut vt = make_vt(80, 24);
    // No preceding character — modifier must attach to (0,0) without panic.
    vt.process("\u{1F3FC}".as_bytes());
    // No crash is the primary assertion. Cursor must not advance.
    assert_eq!(
        vt.normal_cursor.col, 0,
        "skin-tone modifier at col=0 must not advance the cursor"
    );
}

// ---------------------------------------------------------------------------
// R8 — Regional Indicator pairs form a 2-cell flag emoji (FS-VT-019)
// ---------------------------------------------------------------------------

/// "🇫🇷" (U+1F1EB U+1F1F7) must occupy exactly 2 cells as a confirmed flag.
#[test]
fn r8_regional_indicator_pair_forms_two_cell_flag() {
    let mut vt = make_vt(80, 24);
    // 🇫🇷 = U+1F1EB (F) + U+1F1F7 (R).
    vt.process("🇫🇷".as_bytes());
    // Must occupy 2 cells total.
    let g0 = vt
        .active_buf_ref()
        .get(0, 0)
        .map(|c| c.grapheme.clone())
        .unwrap_or_default();
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
    assert!(
        g0.contains('\u{1F1EB}') && g0.contains('\u{1F1F7}'),
        "cell (0,0) must contain both RI codepoints, got: {g0:?}"
    );
    assert_eq!(w0, 2, "confirmed flag cell must have width=2");
    assert_eq!(w1, 0, "cell (0,1) must be phantom");
    // Cursor must be at col=2.
    assert_eq!(
        vt.normal_cursor.col, 2,
        "cursor must be at col=2 after flag"
    );
}

/// Unpaired RI (followed by a non-RI char) must occupy 1 cell (FS-VT-019).
#[test]
fn r8_unpaired_regional_indicator_occupies_one_cell() {
    let mut vt = make_vt(80, 24);
    // U+1F1EB alone, followed by ASCII 'A'.
    vt.process("\u{1F1EB}A".as_bytes());
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(
        w0, 1,
        "unpaired RI must occupy 1 cell after confirmation by non-RI char"
    );
    // 'A' must be at col=1.
    let g1 = vt
        .active_buf_ref()
        .get(0, 1)
        .map(|c| c.grapheme.clone())
        .unwrap_or_default();
    assert_eq!(g1, "A", "char after unpaired RI must be at col=1");
    assert_eq!(vt.normal_cursor.col, 2, "cursor must be at col=2");
}

/// Two flags "🇫🇷🇩🇪" must occupy exactly 4 cells total.
#[test]
fn r8_two_flags_occupy_four_cells() {
    let mut vt = make_vt(80, 24);
    // 🇫🇷 = U+1F1EB U+1F1F7, 🇩🇪 = U+1F1E9 U+1F1EA.
    vt.process("🇫🇷🇩🇪".as_bytes());
    // First flag at cols 0-1, second flag at cols 2-3.
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
    let w2 = vt.active_buf_ref().get(0, 2).map(|c| c.width).unwrap_or(0);
    let w3 = vt.active_buf_ref().get(0, 3).map(|c| c.width).unwrap_or(99);
    assert_eq!(w0, 2, "first flag: width=2 at col=0");
    assert_eq!(w1, 0, "first flag: phantom at col=1");
    assert_eq!(w2, 2, "second flag: width=2 at col=2");
    assert_eq!(w3, 0, "second flag: phantom at col=3");
    assert_eq!(vt.normal_cursor.col, 4, "cursor must be at col=4");
}

// ---------------------------------------------------------------------------
// R6 — Variation selectors U+FE0F / U+FE0E (FS-VT-017)
// ---------------------------------------------------------------------------

/// "☆\u{FE0F}": text star + emoji presentation selector must yield 2 cells.
#[test]
fn r6_fe0f_emoji_presentation_forces_two_cells() {
    let mut vt = make_vt(80, 24);
    // U+2606 (white star) is ambiguous/width=1 by unicode_width.
    // FE0F must upgrade it to width=2.
    vt.process("☆\u{FE0F}".as_bytes());
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(w0, 2, "☆+FE0F must occupy 2 cells");
    let w1 = vt.active_buf_ref().get(0, 1).map(|c| c.width).unwrap_or(99);
    assert_eq!(w1, 0, "cell (0,1) must be phantom after FE0F");
    assert_eq!(vt.normal_cursor.col, 2, "cursor must be at col=2");
}

/// "☆\u{FE0E}": text presentation selector must keep the star at 1 cell.
#[test]
fn r6_fe0e_text_presentation_keeps_one_cell() {
    let mut vt = make_vt(80, 24);
    vt.process("☆\u{FE0E}".as_bytes());
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(w0, 1, "☆+FE0E must remain 1 cell");
    assert_eq!(vt.normal_cursor.col, 1, "cursor must be at col=1");
}

/// "A\u{FE0F}": FE0F after a plain ASCII letter must not widen it.
#[test]
fn r6_fe0f_does_not_widen_non_emoji() {
    let mut vt = make_vt(80, 24);
    vt.process("A\u{FE0F}".as_bytes());
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(w0, 1, "FE0F must not widen a plain ASCII character");
    assert_eq!(vt.normal_cursor.col, 1, "cursor must be at col=1");
}

/// FE0E after a non-eligible character (e.g. U+00E0 'à') must not buffer
/// it and must not crash — the VS is silently dropped.
#[test]
fn r6_fe0e_on_non_eligible_char_is_dropped() {
    let mut vt = make_vt(80, 24);
    // U+00E0 'à' is not in the emoji VS-eligible set.
    vt.process("à\u{FE0E}".as_bytes());
    // 'à' is width=1 (Latin extended), must remain unchanged.
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(w0, 1, "non-eligible char must stay width=1 after FE0E");
    // Cursor at col=1 — the VS consumed no visual column.
    assert_eq!(vt.normal_cursor.col, 1, "cursor must be at col=1");
}

/// A lone RI at the end of input (no following char) stays pending as a
/// 2-cell provisional slot. The next process() call will confirm or narrow it.
/// This test verifies the provisional state is visible after the first call.
#[test]
fn r8_lone_ri_at_end_of_input_is_provisional_two_cells() {
    let mut vt = make_vt(80, 24);
    // Single Regional Indicator (U+1F1EB), no following character.
    vt.process("\u{1F1EB}".as_bytes());
    // The provisional cell must be written as width=2.
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(w0, 2, "provisional RI must occupy 2 cells");
    // A subsequent non-RI character must narrow it to 1 cell.
    vt.process("A".as_bytes());
    let w0_after = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(
        w0_after, 1,
        "RI must be narrowed to 1 cell when followed by non-RI"
    );
    let g1 = vt
        .active_buf_ref()
        .get(0, 1)
        .map(|c| c.grapheme.clone())
        .unwrap_or_default();
    assert_eq!(g1, "A", "'A' must land at col=1");
}

/// RI followed immediately by a skin-tone modifier: the RI must be committed
/// as narrow (1 cell) before the skin-tone attaches to whatever precedes it.
#[test]
fn r7_skin_tone_after_lone_ri_narrows_ri() {
    let mut vt = make_vt(80, 24);
    // U+1F1EB alone, then skin-tone U+1F3FB.
    vt.process("\u{1F1EB}\u{1F3FB}".as_bytes());
    // The RI must be committed as narrow (width=1) because a skin-tone is
    // not a second RI — the RI is unpaired.
    let w0 = vt.active_buf_ref().get(0, 0).map(|c| c.width).unwrap_or(0);
    assert_eq!(w0, 1, "lone RI before skin-tone must be narrowed to 1 cell");
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
// Cursor blink — DECSET 12 / DECRST 12
// ---------------------------------------------------------------------------

/// `?12h` enables cursor blinking; `?12l` disables it.
#[test]
fn test_cursor_blink_decset12() {
    let mut vt = make_vt(80, 24);
    // Default state: blink disabled.
    assert!(!vt.cursor_blink, "cursor_blink must default to false");
    // DECSET 12 — enable blink.
    vt.process(b"\x1b[?12h");
    assert!(vt.cursor_blink, "cursor_blink must be true after ?12h");
    // DECRST 12 — disable blink.
    vt.process(b"\x1b[?12l");
    assert!(!vt.cursor_blink, "cursor_blink must be false after ?12l");
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
