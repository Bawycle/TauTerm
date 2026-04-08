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
