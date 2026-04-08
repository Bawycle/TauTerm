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
