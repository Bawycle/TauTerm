// SPDX-License-Identifier: MPL-2.0

use super::helpers::*;

// ---------------------------------------------------------------------------
// TEST-SB-002 — scrollback_lines preference is honoured (FS-SB-002)
// ---------------------------------------------------------------------------

#[test]
fn scrollback_limit_from_constructor_is_respected() {
    let mut vt = crate::vt::VtProcessor::new(5, 1, 3, 0, false);
    // Scroll 5 lines into scrollback — only 3 should be retained.
    for _ in 0..5 {
        vt.process(b"A\r\n");
    }
    let sb_len = vt.normal.scrollback_len();
    assert!(
        sb_len <= 3,
        "scrollback must be capped at the constructor limit (3), got {sb_len}"
    );
}

// ---------------------------------------------------------------------------
// R5 — get_scrollback_line exposes soft_wrapped (FS-SB-011)
// ---------------------------------------------------------------------------

/// R5-hard: a line terminated by a hard newline (LF) must have soft_wrapped=false.
#[test]
fn r5_get_scrollback_line_hard_newline_soft_wrapped_false() {
    // 5-column terminal, 1 visible row → first LF pushes row 0 to scrollback.
    let mut vt = crate::vt::VtProcessor::new(5, 1, 100, 0, false);
    // Write text then a hard LF — the current row is pushed to scrollback.
    vt.process(b"ABC\r\n");
    let sb = vt
        .get_scrollback_line(0)
        .expect("scrollback line 0 must exist");
    assert!(
        !sb.soft_wrapped,
        "hard newline: soft_wrapped must be false, got true"
    );
}

/// R5-soft: a line pushed to scrollback by auto-wrap must have soft_wrapped=true.
#[test]
fn r5_get_scrollback_line_soft_wrap_soft_wrapped_true() {
    // 3-column terminal, 1 visible row.
    // Writing 4 chars forces auto-wrap + scroll → scrollback entry is soft-wrapped.
    let mut vt = crate::vt::VtProcessor::new(3, 1, 100, 0, false);
    // Writing 4 printable chars on a 3-wide terminal:
    //   - chars 1-3 fill row 0, set wrap_pending on char 3.
    //   - char 4 triggers delayed wrap → row 0 scrolls into scrollback (soft_wrapped=true).
    vt.process(b"ABCD");
    let sb = vt
        .get_scrollback_line(0)
        .expect("scrollback line 0 must exist");
    assert!(
        sb.soft_wrapped,
        "auto-wrap push: soft_wrapped must be true, got false"
    );
}

/// R5-cells: the cells returned by get_scrollback_line match the written content.
#[test]
fn r5_get_scrollback_line_cells_content() {
    let mut vt = crate::vt::VtProcessor::new(5, 1, 100, 0, false);
    vt.process(b"Hi\r\n");
    let sb = vt
        .get_scrollback_line(0)
        .expect("scrollback line 0 must exist");
    assert_eq!(sb.cells[0].grapheme, "H", "cell 0 must be 'H'");
    assert_eq!(sb.cells[1].grapheme, "i", "cell 1 must be 'i'");
}

/// R5-oob: get_scrollback_line past the end returns None.
#[test]
fn r5_get_scrollback_line_out_of_bounds_returns_none() {
    let vt = crate::vt::VtProcessor::new(80, 24, 1000, 0, false);
    assert!(
        vt.get_scrollback_line(0).is_none(),
        "empty scrollback: get_scrollback_line(0) must return None"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-OSC8-001 — OSC 8 hyperlinks stored on cells (FS-VT-070–073)
// ---------------------------------------------------------------------------

/// Cells written while an OSC 8 hyperlink is active receive the URI.
#[test]
fn osc8_cell_inside_hyperlink_receives_uri() {
    let mut vt = make_vt(80, 24);
    // ESC ] 8 ; ; https://example.com BEL  followed by text 'A'
    vt.process(b"\x1b]8;;https://example.com\x07A");
    let cell = vt.normal.get(0, 0).expect("cell (0,0) must exist");
    assert_eq!(
        cell.hyperlink.as_deref(),
        Some("https://example.com"),
        "cell inside OSC 8 hyperlink must carry the URI"
    );
}

/// Cells written after OSC 8 ;; (end-of-hyperlink) have no hyperlink.
#[test]
fn osc8_cell_after_end_sequence_has_no_hyperlink() {
    let mut vt = make_vt(80, 24);
    // Open hyperlink, write 'A', close hyperlink, write 'B'.
    vt.process(b"\x1b]8;;https://example.com\x07A\x1b]8;;\x07B");
    let cell_a = vt.normal.get(0, 0).expect("cell (0,0)");
    let cell_b = vt.normal.get(0, 1).expect("cell (0,1)");
    assert_eq!(
        cell_a.hyperlink.as_deref(),
        Some("https://example.com"),
        "cell 'A' must carry the URI"
    );
    assert!(
        cell_b.hyperlink.is_none(),
        "cell 'B' after OSC 8 ;; must have no hyperlink, got {:?}",
        cell_b.hyperlink
    );
}

/// OSC 8 with the same ID on two successive opens reuses the same URI (FS-VT-072).
#[test]
fn osc8_same_id_on_two_lines_carries_same_uri() {
    let mut vt = make_vt(80, 24);
    // First open: id=link1, write 'A'.
    vt.process(b"\x1b]8;id=link1;https://example.com\x07A");
    // Close hyperlink.
    vt.process(b"\x1b]8;;\x07");
    // Re-open with the same ID — URI must still be present on written cell.
    vt.process(b"\x1b]8;id=link1;https://example.com\x07B");

    let cell_a = vt.normal.get(0, 0).expect("cell (0,0)");
    let cell_b = vt.normal.get(0, 1).expect("cell (0,1)");
    assert_eq!(cell_a.hyperlink.as_deref(), Some("https://example.com"));
    assert_eq!(cell_b.hyperlink.as_deref(), Some("https://example.com"));
}

/// Cells written before any OSC 8 sequence have no hyperlink.
#[test]
fn osc8_no_hyperlink_by_default() {
    let mut vt = make_vt(80, 24);
    vt.process(b"Hello");
    let cell = vt.normal.get(0, 0).expect("cell (0,0)");
    assert!(
        cell.hyperlink.is_none(),
        "cells written without an active hyperlink must have hyperlink=None"
    );
}

// ---------------------------------------------------------------------------
// TEST-VT-OSC52-001 — OSC 52 clipboard write forwarding (FS-VT-075)
// ---------------------------------------------------------------------------

/// With `allow_osc52_write = false` (default), no clipboard event is queued.
#[test]
fn osc52_write_blocked_by_default_policy() {
    let mut vt = make_vt(80, 24);
    // Base64("hello") = "aGVsbG8="
    vt.process(b"\x1b]52;c;aGVsbG8=\x07");
    assert!(
        vt.take_osc52_write().is_none(),
        "OSC 52 write must be blocked when allow_osc52_write = false (default)"
    );
}

/// With `allow_osc52_write = true`, the decoded payload is returned by `take_osc52_write`.
#[test]
fn osc52_write_forwarded_when_policy_allows() {
    let mut vt = make_vt(80, 24);
    vt.allow_osc52_write = true;
    // Base64("hello") = "aGVsbG8="
    vt.process(b"\x1b]52;c;aGVsbG8=\x07");
    let payload = vt.take_osc52_write();
    assert_eq!(
        payload.as_deref(),
        Some("hello"),
        "OSC 52 decoded payload must be forwarded when allow_osc52_write = true"
    );
}

/// `take_osc52_write` drains the pending payload — second call returns None.
#[test]
fn osc52_take_drains_pending_payload() {
    let mut vt = make_vt(80, 24);
    vt.allow_osc52_write = true;
    vt.process(b"\x1b]52;c;aGVsbG8=\x07");
    let _ = vt.take_osc52_write(); // first call drains
    assert!(
        vt.take_osc52_write().is_none(),
        "second call to take_osc52_write must return None (payload already drained)"
    );
}

// ---------------------------------------------------------------------------
// TEST-SEC-OSC52-ISOLATION-001 — OSC 52 policy is per-VtProcessor
// ---------------------------------------------------------------------------

/// OSC 52 allow/deny policy is stored per-VtProcessor instance, not globally.
///
/// Two processors with opposite policies receiving the same sequence must produce
/// independent outcomes. This test guards against accidental global state
/// (e.g. a static flag or a shared Arc<AtomicBool>).
#[test]
fn test_sec_osc52_policy_is_per_processor() {
    // TEST-SEC-OSC52-ISOLATION-001
    // proc_allow: OSC 52 writes permitted.
    let mut proc_allow = crate::vt::VtProcessor::new(80, 24, 10_000, 0, true);
    // proc_deny: OSC 52 writes blocked.
    let mut proc_deny = crate::vt::VtProcessor::new(80, 24, 10_000, 0, false);

    // Base64("hello") = "aGVsbG8="
    let osc52_seq = b"\x1b]52;c;aGVsbG8=\x07";
    proc_allow.process(osc52_seq);
    proc_deny.process(osc52_seq);

    // proc_allow must have a pending clipboard write.
    assert_eq!(
        proc_allow.take_osc52_write().as_deref(),
        Some("hello"),
        "TEST-SEC-OSC52-ISOLATION-001: proc_allow must forward OSC 52 write"
    );

    // proc_deny must NOT have a pending clipboard write.
    assert!(
        proc_deny.take_osc52_write().is_none(),
        "TEST-SEC-OSC52-ISOLATION-001: proc_deny must block OSC 52 write; \
         policy must be per-VtProcessor, not global"
    );
}
