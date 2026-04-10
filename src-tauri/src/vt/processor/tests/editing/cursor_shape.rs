// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::{make_vt, make_vt_with_shape};

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
    // Default: blink enabled (FS-VT-031a).
    assert!(vt.cursor_blink, "cursor_blink must default to true");

    // DECSCUSR 2 — "steady block" in xterm semantics, but the Rust backend
    // must NOT interpret this and must leave cursor_blink unchanged.
    vt.process(b"\x1b[2 q");
    assert_eq!(vt.cursor_shape, 2, "cursor_shape must be updated to 2");
    assert!(
        vt.cursor_blink,
        "DECSCUSR must not modify cursor_blink (only DECRST ?12 does)"
    );

    // Disable blink via DECRST ?12, then send a "blinking" DECSCUSR code (1).
    // cursor_blink must stay false — DECSCUSR 1 must not enable it.
    vt.process(b"\x1b[?12l");
    assert!(!vt.cursor_blink, "cursor_blink must be false after ?12l");

    vt.process(b"\x1b[1 q");
    assert_eq!(vt.cursor_shape, 1, "cursor_shape must be updated to 1");
    assert!(
        !vt.cursor_blink,
        "DECSCUSR 1 (blinking block xterm) must not re-enable cursor_blink — only DECSET ?12 does"
    );
}

// ---------------------------------------------------------------------------
// Cursor blink — DECSET 12 / DECRST 12
// ---------------------------------------------------------------------------

/// Default `cursor_blink` is `true` (FS-VT-031a: blink on by default).
/// `?12l` disables it; `?12h` re-enables it.
#[test]
fn test_cursor_blink_decset12() {
    let mut vt = make_vt(80, 24);
    // Default state: blink enabled (FS-VT-031a).
    assert!(vt.cursor_blink, "cursor_blink must default to true");
    // DECRST 12 — disable blink.
    vt.process(b"\x1b[?12l");
    assert!(!vt.cursor_blink, "cursor_blink must be false after ?12l");
    // DECSET 12 — re-enable blink.
    vt.process(b"\x1b[?12h");
    assert!(vt.cursor_blink, "cursor_blink must be true after ?12h");
}

// ---------------------------------------------------------------------------
// TEST-VT-030b — DECSCUSR 0 restores preferred_cursor_shape
//
// DECSCUSR 0 means "restore default cursor shape". After this fix, the Rust
// backend tracks `preferred_cursor_shape` (the user-preference-derived initial
// shape). DECSCUSR 0 must restore that preferred value, not hardcode 0.
//
// The preferred shape is the `initial_cursor_shape` passed to `VtProcessor::new`.
// `propagate_cursor_shape()` also updates `preferred_cursor_shape`.
// ---------------------------------------------------------------------------

/// DECSCUSR 0 with preferred=1 (blinking block) restores cursor_shape to 1.
#[test]
fn decscusr_code_0_restores_preferred_block_blinking() {
    // Preferred shape: 1 (blinking block = to_decscusr for Block with blink default)
    let mut vt = make_vt_with_shape(80, 24, 1);
    assert_eq!(
        vt.preferred_cursor_shape, 1,
        "preferred must be 1 (blinking block)"
    );

    // Application overrides to shape 4 (steady underline).
    vt.process(b"\x1b[4 q");
    assert_eq!(vt.cursor_shape, 4, "override to 4 must be applied");

    // DECSCUSR 0 — restore to preferred.
    vt.process(b"\x1b[0 q");
    assert_eq!(
        vt.cursor_shape, 1,
        "DECSCUSR 0 must restore preferred cursor shape 1 (blinking block)"
    );
    assert!(
        vt.cursor_shape_changed,
        "cursor_shape_changed must be set when restoring from 4 to 1"
    );
}

/// DECSCUSR 0 with preferred=3 (blinking underline) restores cursor_shape to 3.
#[test]
fn decscusr_code_0_restores_preferred_underline_blinking() {
    let mut vt = make_vt_with_shape(80, 24, 3);
    assert_eq!(
        vt.preferred_cursor_shape, 3,
        "preferred must be 3 (blinking underline)"
    );

    // Application overrides to shape 2 (steady block).
    vt.process(b"\x1b[2 q");
    assert_eq!(vt.cursor_shape, 2, "override to 2 must be applied");

    // DECSCUSR 0 — restore to preferred.
    vt.process(b"\x1b[0 q");
    assert_eq!(
        vt.cursor_shape, 3,
        "DECSCUSR 0 must restore preferred cursor shape 3 (blinking underline)"
    );
    assert!(
        vt.cursor_shape_changed,
        "cursor_shape_changed must be set when restoring from 2 to 3"
    );
}

/// DECSCUSR 0 with preferred=5 (blinking bar) restores cursor_shape to 5.
#[test]
fn decscusr_code_0_restores_preferred_bar_blinking() {
    let mut vt = make_vt_with_shape(80, 24, 5);
    assert_eq!(
        vt.preferred_cursor_shape, 5,
        "preferred must be 5 (blinking bar)"
    );

    // Application overrides to shape 6 (steady bar).
    vt.process(b"\x1b[6 q");
    assert_eq!(vt.cursor_shape, 6, "override to 6 must be applied");

    // DECSCUSR 0 — restore to preferred.
    vt.process(b"\x1b[0 q");
    assert_eq!(
        vt.cursor_shape, 5,
        "DECSCUSR 0 must restore preferred cursor shape 5 (blinking bar)"
    );
    assert!(
        vt.cursor_shape_changed,
        "cursor_shape_changed must be set when restoring from 6 to 5"
    );
}

/// DECSCUSR 0 when cursor_shape already equals preferred_cursor_shape: no-change flag.
#[test]
fn decscusr_code_0_no_change_flag_when_already_preferred() {
    // preferred=1, cursor_shape=1 (already at preferred, no app override).
    let mut vt = make_vt_with_shape(80, 24, 1);
    assert_eq!(vt.cursor_shape, 1, "cursor_shape starts at preferred");
    assert!(!vt.cursor_shape_changed, "flag must start false");

    // DECSCUSR 0 — restore to preferred=1, but cursor_shape is already 1.
    vt.process(b"\x1b[0 q");
    assert!(
        !vt.cursor_shape_changed,
        "DECSCUSR 0 when cursor_shape == preferred_cursor_shape must not set the changed flag"
    );
    assert_eq!(vt.cursor_shape, 1, "cursor_shape must remain 1");
}

// ---------------------------------------------------------------------------
// TEST-VT-030c — propagate_cursor_shape updates preferred_cursor_shape
//
// When the user changes their cursor style preference at runtime,
// `propagate_cursor_shape()` must update both `cursor_shape` and
// `preferred_cursor_shape` so that subsequent DECSCUSR 0 restores the new
// preference, not the original initial_shape.
//
// This test exercises the VtProcessor fields directly (the AppHandle-dependent
// emit side of propagate_cursor_shape is covered by the functional test
// protocol). It documents the contract that the field must be public and mutable.
// ---------------------------------------------------------------------------

/// After simulating what propagate_cursor_shape does (updating both cursor_shape
/// and preferred_cursor_shape), DECSCUSR 0 must restore the new preferred value.
#[test]
fn preferred_cursor_shape_update_then_decscusr_0_restores_new_preferred() {
    // Start with preferred=1 (blinking block, from user prefs at pane creation).
    let mut vt = make_vt_with_shape(80, 24, 1);
    assert_eq!(vt.preferred_cursor_shape, 1, "initial preferred must be 1");
    assert_eq!(vt.cursor_shape, 1, "initial cursor_shape must be 1");

    // Simulate propagate_cursor_shape(shape=5) — user changed prefs to blinking bar.
    // propagate_cursor_shape sets both fields and raises cursor_shape_changed.
    vt.cursor_shape = 5;
    vt.preferred_cursor_shape = 5;
    vt.cursor_shape_changed = true;

    // Application then overrides to shape 2 (steady block).
    vt.take_cursor_shape_changed(); // consume the propagate flag
    vt.process(b"\x1b[2 q");
    assert_eq!(vt.cursor_shape, 2, "app override to 2 must be applied");

    // DECSCUSR 0 — must restore the new preferred (5), not the old (1).
    vt.process(b"\x1b[0 q");
    assert_eq!(
        vt.cursor_shape, 5,
        "DECSCUSR 0 must restore new preferred 5 after propagate_cursor_shape"
    );
    assert!(
        vt.cursor_shape_changed,
        "cursor_shape_changed must be set when restoring from 2 to 5"
    );
    assert_eq!(
        vt.preferred_cursor_shape, 5,
        "preferred_cursor_shape must remain 5 after DECSCUSR 0"
    );
}

/// preferred_cursor_shape must be a public field so propagate_cursor_shape
/// can write to it without an accessor method.
#[test]
fn preferred_cursor_shape_field_is_accessible() {
    let mut vt = make_vt_with_shape(80, 24, 3);
    // Direct read
    let initial = vt.preferred_cursor_shape;
    assert_eq!(
        initial, 3,
        "preferred_cursor_shape must equal initial_shape"
    );
    // Direct write (mimics propagate_cursor_shape)
    vt.preferred_cursor_shape = 5;
    assert_eq!(
        vt.preferred_cursor_shape, 5,
        "preferred_cursor_shape must be writable"
    );
}
