// SPDX-License-Identifier: MPL-2.0

use super::helpers::*;

// ---------------------------------------------------------------------------
// TEST-VT-008 — cursor visibility and DECTCEM
// FS-VT-030, FS-VT-031
// ---------------------------------------------------------------------------

#[test]
fn dectcem_hide_and_show_cursor() {
    // TEST-VT-008 (partial — cursor shape stub)
    let mut vt = make_vt(80, 24);
    assert!(vt.modes.cursor_visible, "cursor must be visible by default");

    // Hide cursor.
    vt.process(b"\x1b[?25l");
    assert!(
        !vt.modes.cursor_visible,
        "cursor must be hidden after DECTCEM hide"
    );

    // Show cursor.
    vt.process(b"\x1b[?25h");
    assert!(
        vt.modes.cursor_visible,
        "cursor must be visible after DECTCEM show"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-009 — alternate screen cursor save/restore (DECSC + mode 1049)
// FS-VT-033
// ---------------------------------------------------------------------------

#[test]
fn alternate_screen_cursor_save_restore() {
    // TEST-VT-009
    let mut vt = make_vt(80, 24);
    // Position cursor at (5, 10) on normal screen via CUP.
    vt.process(b"\x1b[6;11H"); // row=6 col=11 (1-based) → row=5 col=10 (0-based)
    assert_eq!(vt.normal_cursor.row, 5);
    assert_eq!(vt.normal_cursor.col, 10);

    // Switch to alternate screen (saves cursor via mode 1049).
    // DECSET uses CSI ? Pm h (with '?' intermediate byte).
    vt.process(b"\x1b[?1049h");
    assert!(vt.alt_active, "alternate screen must be active");

    // Move cursor to (0, 0) on alternate screen.
    vt.process(b"\x1b[1;1H");
    assert_eq!(vt.alt_cursor.row, 0);
    assert_eq!(vt.alt_cursor.col, 0);

    // Return to normal screen (restores cursor).
    // DECRST uses CSI ? Pm l.
    vt.process(b"\x1b[?1049l");
    assert!(!vt.alt_active, "normal screen must be active");
    assert_eq!(vt.normal_cursor.row, 5, "cursor row must be restored");
    assert_eq!(vt.normal_cursor.col, 10, "cursor col must be restored");
}

// ---------------------------------------------------------------------------
// TEST-VT-010 — alternate screen isolation and no scrollback
// FS-VT-040, FS-VT-041, FS-VT-042, FS-VT-044
// ---------------------------------------------------------------------------

#[test]
fn alternate_screen_is_isolated_from_normal_screen() {
    // TEST-VT-010
    let mut vt = make_vt(10, 5);
    // Write content on normal screen.
    vt.process(b"HELLO");
    assert_eq!(grapheme_at(&vt, 0, 0), "H");

    // Switch to alternate screen — must be blank.
    // DECSET uses CSI ? Pm h.
    vt.process(b"\x1b[?1049h");
    assert!(vt.alt_active);
    assert_eq!(
        grapheme_at(&vt, 0, 0),
        " ",
        "alternate screen must be blank on entry"
    );

    // Write on alternate screen.
    vt.process(b"WORLD");

    // Return to normal screen.
    // DECRST uses CSI ? Pm l.
    vt.process(b"\x1b[?1049l");
    assert!(!vt.alt_active);
    assert_eq!(
        grapheme_at(&vt, 0, 0),
        "H",
        "normal screen content must survive alt-screen usage"
    );

    // Alternate screen must not have added scrollback.
    assert_eq!(
        vt.normal.scrollback_len(),
        0,
        "alternate screen must not contribute to scrollback"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-011 — DECSTBM scroll region
// FS-VT-050, FS-VT-051, FS-VT-053
// ---------------------------------------------------------------------------

#[test]
fn decstbm_partial_scroll_region_no_scrollback() {
    // TEST-VT-011
    let mut vt = make_vt(80, 10);
    // Set scroll region rows 2–8 (1-based) = indices 1–7 (0-based).
    vt.process(b"\x1b[2;8r");
    assert_eq!(vt.modes.scroll_region, (1, 7));
    // Cursor must be moved to home position after DECSTBM.
    assert_eq!(vt.normal_cursor.row, 0);
    assert_eq!(vt.normal_cursor.col, 0);
    // Scrolling within the partial region must not add to scrollback.
    // Position cursor at bottom of region (row 7, 0-based).
    vt.process(b"\x1b[8;1H"); // row=8 col=1 (1-based)
    // Feed 3 LF to scroll within region.
    vt.process(b"\n\n\n");
    assert_eq!(
        vt.normal.scrollback_len(),
        0,
        "partial scroll region must not add to scrollback (FS-VT-053)"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-018 — OSC buffer overflow protection
// FS-SEC-005
// ---------------------------------------------------------------------------

#[test]
fn osc_overflow_does_not_crash_and_subsequent_sequences_parse() {
    // TEST-VT-018
    let mut vt = make_vt(80, 24);
    // Feed OSC 0 ; followed by 5000 bytes without a terminator.
    let mut overflow_seq: Vec<u8> = b"\x1b]0;".to_vec();
    overflow_seq.extend(std::iter::repeat_n(b'X', 5000));
    // No BEL or ST — simulate abandonment. Then a valid sequence.
    vt.process(&overflow_seq);
    // Feed a valid sequence that follows — must not be corrupted.
    vt.process(b"\x1b[31mA");
    // No panic is the primary assertion; but also verify A is written.
    let attrs = attrs_at(&vt, 0, 0);
    // The VTE parser's behavior on overlong OSC is to discard and continue —
    // verify subsequent input parses (red foreground set).
    assert_eq!(
        attrs.fg,
        Some(crate::vt::cell::Color::Ansi { index: 1 }),
        "SGR 31 after overlong OSC must be applied"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-023 — DEC Special Graphics charset
// FS-VT-015
// ---------------------------------------------------------------------------

#[test]
fn dec_special_graphics_so_maps_j_to_box_drawing() {
    // TEST-VT-023
    let mut vt = make_vt(80, 24);
    // Designate G1 as DEC Special Graphics.
    vt.process(b"\x1b)0");
    // SO (0x0E) — shift to G1.
    vt.process(b"\x0e");
    // Feed 0x6A ('j' in ASCII; maps to '┘' in DEC Special Graphics).
    vt.process(b"\x6a");
    let g = grapheme_at(&vt, 0, 0);
    assert_eq!(
        g, "┘",
        "0x6A with DEC Special Graphics active must map to '┘'"
    );
    // SI (0x0F) — return to G0 (ASCII).
    vt.process(b"\x0f");
    vt.process(b"j");
    let g2 = grapheme_at(&vt, 0, 1);
    assert_eq!(g2, "j", "0x6A with ASCII active must remain 'j'");
}

// ---------------------------------------------------------------------------
// DECCKM mode tracking
// FS-VT-030
// ---------------------------------------------------------------------------

#[test]
fn decckm_mode_set_and_reset() {
    let mut vt = make_vt(80, 24);
    assert!(!vt.modes.decckm, "DECCKM must be false by default");
    vt.process(b"\x1b[?1h"); // DECSET 1 = DECCKM
    assert!(vt.modes.decckm, "DECCKM must be true after ESC[?1h");
    assert!(vt.mode_changed, "mode_changed flag must be set");
    vt.mode_changed = false;
    vt.process(b"\x1b[?1l"); // DECRST 1
    assert!(!vt.modes.decckm, "DECCKM must be false after ESC[?1l");
    assert!(vt.mode_changed, "mode_changed flag must be set again");
}

// ---------------------------------------------------------------------------
// Bracketed paste mode tracking
// FS-KBD related
// ---------------------------------------------------------------------------

#[test]
fn bracketed_paste_mode_tracking() {
    let mut vt = make_vt(80, 24);
    assert!(!vt.modes.bracketed_paste);
    vt.process(b"\x1b[?2004h");
    assert!(vt.modes.bracketed_paste);
    vt.process(b"\x1b[?2004l");
    assert!(!vt.modes.bracketed_paste);
}

// ---------------------------------------------------------------------------
// Mouse mode reset on alt-screen exit (fix #2, FS-VT-086)
// ---------------------------------------------------------------------------

/// Mouse reporting mode must be None after leaving the alternate screen,
/// even when the app never sent the reset sequence.
#[test]
fn mouse_mode_reset_on_leave_alternate_mode_1049() {
    use crate::vt::modes::MouseReportingMode;
    let mut vt = make_vt(80, 24);
    // Enter alt screen and activate normal mouse tracking.
    vt.process(b"\x1b[?1049h"); // enter alt screen
    vt.process(b"\x1b[?1000h"); // activate mouse normal tracking
    assert_eq!(vt.modes.mouse_reporting, MouseReportingMode::Normal);
    // Leave alt screen without sending reset — simulates app crash.
    vt.process(b"\x1b[?1049l");
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::None,
        "mouse reporting must be None after leaving alt screen (FS-VT-086)"
    );
}

/// Mouse mode reset also applies to mode 47 exit.
#[test]
fn mouse_mode_reset_on_leave_alternate_mode_47() {
    use crate::vt::modes::MouseReportingMode;
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[?47h");
    vt.process(b"\x1b[?1000h");
    vt.process(b"\x1b[?47l");
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::None,
        "mouse reporting must be None after leaving alt screen via mode 47"
    );
}

// ---------------------------------------------------------------------------
// R1 — DECOM: origin mode (DECSET/DECRST ?6) (VT220 / xterm)
// ---------------------------------------------------------------------------

/// R1-cup: DECSET 6 + CUP with scroll region top=5 (0-based=4) — row is offset.
#[test]
fn r1_decom_cup_offsets_row_by_scroll_region_top() {
    // 24-row terminal, region rows 5–20 (1-based) → 0-based top=4, bottom=19.
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;20r"); // DECSTBM
    // Activate DECOM.
    vt.process(b"\x1b[?6h");
    // CUP row=3, col=1 (1-based). With DECOM, row → top + (3-1) = 4 + 2 = 6.
    vt.process(b"\x1b[3;1H");
    assert_eq!(
        vt.normal_cursor.row, 6,
        "DECOM CUP row=3 with top=4 must place cursor at row 6"
    );
    assert_eq!(vt.normal_cursor.col, 0, "column must be 0 (1-based col=1)");
}

/// R1-clamp-top: DECOM + CUP row=0 (before region) — clamp to top.
#[test]
fn r1_decom_cup_clamps_above_region_top() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;20r"); // top=4, bottom=19
    vt.process(b"\x1b[?6h");
    // CUP with row=0 (would become top + (0-1) = 3 < top): clamp to top=4.
    // vte delivers param0=0, which gets .max(1) → 1 → offset = top + 0 = 4.
    vt.process(b"\x1b[1;1H"); // row=1 (minimum), col=1 → row 0-based = top+0 = 4
    assert_eq!(
        vt.normal_cursor.row, 4,
        "DECOM CUP row=1 (minimum) must clamp to top=4"
    );
}

/// R1-clamp-bottom: DECOM + CUP row past bottom — clamp to bottom.
#[test]
fn r1_decom_cup_clamps_past_region_bottom() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;10r"); // top=4, bottom=9
    vt.process(b"\x1b[?6h");
    // CUP row=99 (1-based) → offset = 4 + 98 = 102, clamped to bottom=9.
    vt.process(b"\x1b[99;1H");
    assert_eq!(
        vt.normal_cursor.row, 9,
        "DECOM CUP row past bottom must clamp to region bottom=9"
    );
}

/// R1-off: DECRST 6 — CUP uses absolute coordinates, no offset.
#[test]
fn r1_decom_off_cup_uses_absolute_coordinates() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[5;20r"); // top=4
    // Enable then disable DECOM.
    vt.process(b"\x1b[?6h");
    vt.process(b"\x1b[?6l");
    // CUP row=3 col=1 (1-based) → 0-based row=2 (absolute, no offset).
    vt.process(b"\x1b[3;1H");
    assert_eq!(
        vt.normal_cursor.row, 2,
        "After DECRST 6, CUP must use absolute row (row 3 → 0-based 2)"
    );
}

/// R1-decsc-decrc-decom: DECSC saves DECOM state; DECRC restores it.
#[test]
fn r1_decsc_saves_and_decrc_restores_decom_true() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[?6h"); // DECOM on
    assert!(vt.modes.decom, "DECOM must be true after DECSET 6");
    vt.process(b"\x1b7"); // DECSC — save (includes decom=true)
    vt.process(b"\x1b[?6l"); // DECOM off
    assert!(!vt.modes.decom, "DECOM must be false after DECRST 6");
    vt.process(b"\x1b8"); // DECRC — restore
    assert!(
        vt.modes.decom,
        "DECRC must restore DECOM=true from saved state"
    );
}

/// R1-decsc-decrc-decom-false: DECSC saves DECOM=false; DECRC restores it.
#[test]
fn r1_decsc_saves_and_decrc_restores_decom_false() {
    let mut vt = make_vt(80, 24);
    // DECOM starts false (default).
    vt.process(b"\x1b7"); // DECSC — save with decom=false
    vt.process(b"\x1b[?6h"); // DECOM on
    assert!(vt.modes.decom);
    vt.process(b"\x1b8"); // DECRC — restore decom=false
    assert!(
        !vt.modes.decom,
        "DECRC must restore DECOM=false when it was false at save time"
    );
}

/// R1-alt-screen: entering alt screen resets DECOM to false.
#[test]
fn r1_decom_reset_on_alt_screen_entry() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[?6h"); // DECOM on
    assert!(vt.modes.decom);
    // Enter alt screen (mode 1049) — must reset DECOM.
    vt.process(b"\x1b[?1049h");
    assert!(
        !vt.modes.decom,
        "entering alt screen must reset DECOM to false"
    );
}

// ---------------------------------------------------------------------------
// VT-3 / VT-4 — Mouse mode activation/deactivation and round-trip encoding
// FS-VT-080, FS-VT-081, FS-VT-082, FS-VT-083
// ---------------------------------------------------------------------------

/// TEST-VT-024 (partial) — mode 1002 (ButtonEvent) activates and deactivates.
#[test]
fn mouse_mode_1002_activate_and_deactivate() {
    use crate::vt::modes::MouseReportingMode;
    let mut vt = make_vt(80, 24);
    assert_eq!(vt.modes.mouse_reporting, MouseReportingMode::None);

    vt.process(b"\x1b[?1002h");
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::ButtonEvent,
        "mode 1002h must activate ButtonEvent"
    );

    vt.process(b"\x1b[?1002l");
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::None,
        "mode 1002l must reset to None"
    );
}

/// TEST-VT-024 (partial) — mode 1003 (AnyEvent) activates and deactivates.
#[test]
fn mouse_mode_1003_activate_and_deactivate() {
    use crate::vt::modes::MouseReportingMode;
    let mut vt = make_vt(80, 24);

    vt.process(b"\x1b[?1003h");
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::AnyEvent,
        "mode 1003h must activate AnyEvent"
    );

    vt.process(b"\x1b[?1003l");
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::None,
        "mode 1003l must reset to None"
    );
}

/// TEST-VT-024 (partial) — mode 1006 (SGR encoding) activates and deactivates.
#[test]
fn mouse_encoding_1006_sgr_activate_and_deactivate() {
    use crate::vt::modes::MouseEncoding;
    let mut vt = make_vt(80, 24);
    assert_eq!(vt.modes.mouse_encoding, MouseEncoding::X10);

    vt.process(b"\x1b[?1006h");
    assert_eq!(
        vt.modes.mouse_encoding,
        MouseEncoding::Sgr,
        "mode 1006h must activate SGR encoding"
    );

    vt.process(b"\x1b[?1006l");
    assert_eq!(
        vt.modes.mouse_encoding,
        MouseEncoding::X10,
        "mode 1006l must reset encoding to X10"
    );
}

/// TEST-VT-024 (partial) — mode 1015 (URXVT encoding) activates and deactivates.
#[test]
fn mouse_encoding_1015_urxvt_activate_and_deactivate() {
    use crate::vt::modes::MouseEncoding;
    let mut vt = make_vt(80, 24);

    vt.process(b"\x1b[?1015h");
    assert_eq!(
        vt.modes.mouse_encoding,
        MouseEncoding::Urxvt,
        "mode 1015h must activate URXVT encoding"
    );

    vt.process(b"\x1b[?1015l");
    assert_eq!(
        vt.modes.mouse_encoding,
        MouseEncoding::X10,
        "mode 1015l must reset encoding to X10"
    );
}

/// TEST-VT-025 (partial) — mode 1000 + 1006: reporting mode and encoding are independent.
#[test]
fn mouse_mode_interaction_1000_then_1006() {
    use crate::vt::modes::{MouseEncoding, MouseReportingMode};
    let mut vt = make_vt(80, 24);

    vt.process(b"\x1b[?1000h");
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::Normal,
        "mode 1000h must activate Normal reporting"
    );

    vt.process(b"\x1b[?1006h");
    assert_eq!(
        vt.modes.mouse_encoding,
        MouseEncoding::Sgr,
        "mode 1006h must activate SGR encoding"
    );
    // Reporting mode must remain unchanged after switching encoding.
    assert_eq!(
        vt.modes.mouse_reporting,
        MouseReportingMode::Normal,
        "mouse_reporting must stay Normal after activating SGR encoding"
    );
}

/// TEST-VT-024 round-trip — ButtonEvent + SGR: left press and release.
#[test]
fn mouse_round_trip_button_event_sgr() {
    use crate::vt::modes::MouseEncoding;
    let mut vt = make_vt(80, 24);

    // Activate ButtonEvent (1002) and SGR encoding (1006).
    vt.process(b"\x1b[?1002h");
    vt.process(b"\x1b[?1006h");

    // Left button press at col=10, row=5, no modifiers.
    let press = crate::vt::mouse::MouseEvent {
        col: 10,
        row: 5,
        button: 0,
        is_press: true,
        shift: false,
        alt: false,
        ctrl: false,
        is_motion: false,
    };
    // SAFETY (UTF-8): encode_sgr produces ASCII-only bytes via format! on integer
    // fields and ASCII byte literals; ASCII is always valid UTF-8.
    let encoded_press = String::from_utf8(press.encode(vt.modes.mouse_encoding))
        .expect("SGR encoding produces valid UTF-8");
    assert_eq!(
        encoded_press, "\x1b[<0;10;5M",
        "left press at (10,5) must encode as ESC[<0;10;5M"
    );

    // Left button release at col=10, row=5.
    let release = crate::vt::mouse::MouseEvent {
        is_press: false,
        ..press
    };
    // SAFETY (UTF-8): encode_sgr produces ASCII-only bytes via format! on integer
    // fields and ASCII byte literals; ASCII is always valid UTF-8.
    let encoded_release = String::from_utf8(release.encode(vt.modes.mouse_encoding))
        .expect("SGR encoding produces valid UTF-8");
    assert_eq!(
        encoded_release, "\x1b[<0;10;5m",
        "left release at (10,5) must encode as ESC[<0;10;5m (lowercase m)"
    );

    // Verify mode is active.
    assert_eq!(
        vt.modes.mouse_encoding,
        MouseEncoding::Sgr,
        "SGR encoding must remain active throughout"
    );
}

/// TEST-VT-024 round-trip — AnyEvent + SGR: motion event carries bit 32.
#[test]
fn mouse_round_trip_any_event_sgr() {
    let mut vt = make_vt(80, 24);

    // Activate AnyEvent (1003) and SGR encoding (1006).
    vt.process(b"\x1b[?1003h");
    vt.process(b"\x1b[?1006h");

    // Motion event at col=20, row=10: no button pressed, is_motion=true.
    // button=0 (left button bits) with is_motion=true: cb = 0 | 32 = 32.
    let motion = crate::vt::mouse::MouseEvent {
        col: 20,
        row: 10,
        button: 0,
        is_press: true, // motion events use 'M' trailer in SGR
        shift: false,
        alt: false,
        ctrl: false,
        is_motion: true, // sets bit 32 in the control byte
    };
    // SAFETY (UTF-8): encode_sgr produces ASCII-only bytes via format! on integer
    // fields and ASCII byte literals; ASCII is always valid UTF-8.
    let encoded = String::from_utf8(motion.encode(vt.modes.mouse_encoding))
        .expect("SGR encoding produces valid UTF-8");
    // button=0 → button_bits=0, motion bit=32 → cb = 0|32 = 32
    assert_eq!(
        encoded, "\x1b[<32;20;10M",
        "motion event at (20,10) must encode cb=32 (button_bits=0 | motion=32)"
    );
}
