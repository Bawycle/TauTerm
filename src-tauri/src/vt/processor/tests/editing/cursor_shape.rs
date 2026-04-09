// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::make_vt;

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
// TEST-VT-031 — DECSCUSR parameterised sweep: codes 0–6
// ---------------------------------------------------------------------------
//
// Each DECSCUSR code must be stored verbatim in `cursor_shape`.
// The backend does NOT interpret blink semantics from DECSCUSR — that
// distinction (even codes = steady, odd codes = blinking) is resolved
// exclusively by the frontend (`cursorBlinks()` composable).  DECSET ?12 is
// the only mechanism that modifies `cursor_blink` at the Rust level.
// This architectural split is intentional: the backend is a dumb store for
// the raw DECSCUSR code; all semantic derivation lives in the frontend.

#[test]
fn decscusr_all_codes_0_to_6_set_cursor_shape() {
    // Codes 1–6: each must update cursor_shape and raise cursor_shape_changed.
    // We iterate from 1 to 6 because make_vt() initialises cursor_shape = 0,
    // so code 0 would be the "no-change" case (tested separately below).
    for code in 1u8..=6 {
        let mut vt = make_vt(80, 24);
        // Sanity: default shape is 0.
        assert_eq!(
            vt.cursor_shape, 0,
            "code {code}: default cursor_shape must be 0"
        );
        assert!(
            !vt.cursor_shape_changed,
            "code {code}: cursor_shape_changed must start false"
        );

        // Build "CSI <code> SP q" and process it.
        let seq = format!("\x1b[{code} q");
        vt.process(seq.as_bytes());

        assert_eq!(
            vt.cursor_shape, code,
            "code {code}: cursor_shape must equal the DECSCUSR parameter"
        );
        assert!(
            vt.cursor_shape_changed,
            "code {code}: cursor_shape_changed must be set after a value change"
        );

        // take_cursor_shape_changed must return the new value and reset the flag.
        let taken = vt.take_cursor_shape_changed();
        assert_eq!(
            taken,
            Some(code),
            "code {code}: take_cursor_shape_changed must return Some({code})"
        );
        assert!(
            !vt.cursor_shape_changed,
            "code {code}: flag must be reset after take"
        );
    }
}

#[test]
fn decscusr_code_0_no_change_flag_when_already_default() {
    // cursor_shape starts at 0; sending DECSCUSR 0 must not raise the flag.
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[0 q");
    assert!(
        !vt.cursor_shape_changed,
        "DECSCUSR 0 when cursor_shape is already 0: flag must remain false"
    );
}

#[test]
fn decscusr_repeated_same_code_does_not_raise_flag_on_second_send() {
    // First send (1→3): flag is raised.  Second send of the same code: flag
    // must NOT be raised again because the value has not changed.
    let mut vt = make_vt(80, 24);

    vt.process(b"\x1b[3 q");
    assert!(
        vt.cursor_shape_changed,
        "first DECSCUSR 3: flag must be set"
    );
    // Consume the flag.
    vt.take_cursor_shape_changed();
    assert!(!vt.cursor_shape_changed, "flag must be clear after take");

    // Send the same code a second time.
    vt.process(b"\x1b[3 q");
    assert!(
        !vt.cursor_shape_changed,
        "second DECSCUSR 3 (same value): flag must NOT be raised"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-032 — DECSCUSR must NOT affect `cursor_blink`
//
// Architectural contract: DECSCUSR (CSI Ps SP q) controls *shape only*.
// The `cursor_blink` field is exclusively controlled by DECSET/DECRST ?12.
// Sending DECSCUSR 1 (xterm "blinking block") must leave cursor_blink
// untouched — the backend stores the raw code; the frontend derives blink
// semantics from that code independently.
// ---------------------------------------------------------------------------

#[test]
fn decscusr_does_not_modify_cursor_blink() {
    let mut vt = make_vt(80, 24);
    // Default: blink disabled.
    assert!(!vt.cursor_blink, "cursor_blink must default to false");

    // DECSCUSR 1 — "blinking block" in xterm semantics, but the Rust backend
    // must NOT interpret this and must leave cursor_blink unchanged.
    vt.process(b"\x1b[1 q");
    assert_eq!(vt.cursor_shape, 1, "cursor_shape must be updated to 1");
    assert!(
        !vt.cursor_blink,
        "DECSCUSR must not modify cursor_blink (only DECSET ?12 does)"
    );

    // Enable blink via DECSET ?12, then send a "steady" DECSCUSR code (2).
    // cursor_blink must stay true — DECSCUSR 2 must not disable it.
    vt.process(b"\x1b[?12h");
    assert!(vt.cursor_blink, "cursor_blink must be true after ?12h");

    vt.process(b"\x1b[2 q");
    assert_eq!(vt.cursor_shape, 2, "cursor_shape must be updated to 2");
    assert!(
        vt.cursor_blink,
        "DECSCUSR 2 (steady) must not reset cursor_blink — only DECRST ?12 does"
    );
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
