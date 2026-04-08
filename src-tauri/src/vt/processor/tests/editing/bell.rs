// SPDX-License-Identifier: MPL-2.0

use super::super::helpers::make_vt;

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
