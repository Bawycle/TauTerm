// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::{grapheme_at, make_vt};

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
