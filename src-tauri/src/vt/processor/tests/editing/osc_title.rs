// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::make_vt;

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
// title_changed flag and take_title_changed() — end-to-end via VtProcessor
// ---------------------------------------------------------------------------

#[test]
fn title_changed_flag_is_set_by_osc0() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b]0;Title\x07");
    assert!(
        vt.title_changed,
        "title_changed must be true after OSC 0 sets a new title"
    );
}

#[test]
fn take_title_changed_returns_title_and_clears_flag() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b]0;Title\x07");

    // First call: must return the title and clear the flag.
    let first = vt.take_title_changed();
    assert_eq!(
        first,
        Some("Title".to_string()),
        "take_title_changed must return Some(title) after OSC 0"
    );

    // Second call: flag was cleared — must return None.
    let second = vt.take_title_changed();
    assert_eq!(
        second, None,
        "take_title_changed must return None when called a second time (flag already cleared)"
    );
}

#[test]
fn osc1_and_osc2_also_set_title_changed_flag() {
    for (seq, label) in [
        (b"\x1b]1;title1\x07".as_slice(), "OSC 1"),
        (b"\x1b]2;title2\x07".as_slice(), "OSC 2"),
    ] {
        let mut vt = make_vt(80, 24);
        vt.process(seq);
        assert!(
            vt.title_changed,
            "{label} must set title_changed flag in VtProcessor"
        );
        assert!(
            vt.take_title_changed().is_some(),
            "{label} take_title_changed must return Some after processing"
        );
    }
}
