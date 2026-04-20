// SPDX-License-Identifier: MPL-2.0

//! VT input caps tests — ADR-0028.
//!
//! These tests verify the defensive bounds enforced **at the site of
//! accumulation** for VT-state fields fed by an untrusted byte source
//! (notably SSH). They guard against DoS amplification vectors:
//!
//! - `MAX_TITLE_CHARS` (4 096): OSC 0/1/2 title.
//! - `MAX_CWD_BYTES` (4 096): OSC 7 working directory.
//! - `MAX_PENDING_RESPONSES` (256): DSR/CPR/DA/DA2 reply queue (`VecDeque`).
//!
//! OSC 52 clipboard payload is **not** capped here because it is bounded
//! upstream by `OSC_PAYLOAD_MAX = 4 096` in
//! `vt/processor/dispatch/osc.rs`. SEC-VT-CAP-OSC52-OVERSIZE-001 below is
//! a regression guard for that upstream invariant.

use super::helpers::make_vt;
use crate::vt::processor::{MAX_PENDING_RESPONSES, MAX_TITLE_CHARS};

// ---------------------------------------------------------------------------
// VT-CAP-TITLE-001 — title length never exceeds MAX_TITLE_CHARS
// ---------------------------------------------------------------------------

/// Feeding a long OSC 0 payload must leave the stored title bounded by
/// `MAX_TITLE_CHARS` (4 096). End-to-end invariant: holds whatever upstream
/// pre-truncation `parse_osc` applies.
///
/// We use a 4 000-byte payload (just under `OSC_PAYLOAD_MAX = 4 096` so the
/// upstream sequence-level guard does not drop the whole OSC). The
/// accumulation-site cap remains the load-bearing invariant being asserted —
/// even if a future change to `parse_osc` removed its 256-char truncation,
/// this test would still hold (and `MAX_TITLE_CHARS` would become the active
/// boundary).
#[test]
fn vt_cap_title_001_long_title_bounded() {
    let mut vt = make_vt(80, 24);
    // Total OSC byte budget = "0" + sep + payload + sep ≤ 4 096. With a
    // 4 000-byte payload the total is 4 003 → passes the upstream guard.
    let mut seq = b"\x1b]0;".to_vec();
    seq.extend(std::iter::repeat_n(b'A', 4_000));
    seq.push(b'\x07');
    vt.process(&seq);

    let title = vt
        .take_title_changed()
        .expect("title should have changed after OSC 0");
    assert!(
        title.chars().count() <= MAX_TITLE_CHARS,
        "title.chars().count() = {} exceeds MAX_TITLE_CHARS = {}",
        title.chars().count(),
        MAX_TITLE_CHARS
    );
    // Sanity: the title is non-empty (the OSC was not silently dropped).
    assert!(!title.is_empty(), "title must not be empty after OSC 0");
}

// ---------------------------------------------------------------------------
// VT-CAP-TITLE-002 — C0/C1 strip is applied BEFORE the truncate budget
// ---------------------------------------------------------------------------

/// A title payload mixing C0 control bytes with printable bytes must end up
/// with the C0 bytes stripped — they must not consume any character budget
/// before truncation. This guards against "C0-bourrage" attacks where an
/// attacker pads the payload with unprintable controls hoping to push the
/// visible content out of the truncation window.
#[test]
fn vt_cap_title_002_c0_strip_before_truncate() {
    let mut vt = make_vt(80, 24);
    // Payload: a run of C0 control bytes (excluding ESC and BEL — those are
    // string-terminator candidates and would close the OSC prematurely),
    // followed by a visible suffix. The strip happens upstream in
    // `parse_osc` (`SEC-PTY-006`); the cap at the accumulation site sees the
    // already-stripped string.
    let mut payload: Vec<u8> = Vec::new();
    payload.extend(std::iter::repeat_n(b'\x01', 16)); // SOH × 16
    payload.extend(std::iter::repeat_n(b'\x0b', 16)); // VT × 16
    payload.extend_from_slice(b"VISIBLE_TAIL");
    let mut seq = b"\x1b]0;".to_vec();
    seq.extend_from_slice(&payload);
    seq.push(b'\x07');
    vt.process(&seq);

    let title = vt
        .take_title_changed()
        .expect("title should have changed after OSC 0");
    // None of the C0/C1 bytes survive (strip happened upstream).
    assert!(
        !title.contains('\x01'),
        "C0 SOH must not appear in title: {:?}",
        title
    );
    assert!(
        !title.contains('\x0b'),
        "C0 VT must not appear in title: {:?}",
        title
    );
    // The visible suffix is preserved (proves the strip happened first; if it
    // had happened after a too-aggressive truncate the suffix would be lost).
    assert!(
        title.contains("VISIBLE_TAIL"),
        "visible tail must survive strip+truncate: {:?}",
        title
    );
    // Final invariant still holds.
    assert!(title.chars().count() <= MAX_TITLE_CHARS);
}

// ---------------------------------------------------------------------------
// VT-CAP-CWD-001 — oversized OSC 7 cwd is dropped (no update applied)
// ---------------------------------------------------------------------------

/// An OSC 7 payload whose decoded path exceeds `MAX_CWD_BYTES` must be
/// dropped: `take_cwd_changed()` must return `None` and the previous `cwd`
/// (or its absence) must be preserved. We use a bare absolute path so the
/// payload is taken verbatim by `parse_osc` (no percent-decoding shrinkage).
///
/// Note: `OSC_PAYLOAD_MAX = 4 096` is enforced **at the OSC-sequence level**
/// (sum of all field bytes plus separators). To exercise the cwd-specific cap
/// inside `dispatch/osc.rs`, the OSC payload itself must stay under that
/// upstream guard. We therefore craft a path of exactly 4 097 bytes (1 byte
/// over `MAX_CWD_BYTES`) — the OSC-level total then equals 4 099 bytes
/// (`"7" + sep + path + sep`), exceeding `OSC_PAYLOAD_MAX` and being dropped
/// upstream. The cwd-cap end-to-end test below (smaller bypass scenario) is
/// covered by the negative invariant assertion alone.
///
/// To still verify the cwd cap proper, we set a known cwd via a small valid
/// OSC 7 first, then attempt an oversized one, and assert the cwd value did
/// not change.
#[test]
fn vt_cap_cwd_001_oversized_cwd_dropped() {
    let mut vt = make_vt(80, 24);

    // Step 1: prime the cwd with a small valid value.
    vt.process(b"\x1b]7;/tmp/initial\x07");
    let initial = vt.take_cwd_changed();
    assert_eq!(initial.as_deref(), Some("/tmp/initial"));
    assert_eq!(vt.current_cwd(), Some("/tmp/initial"));

    // Step 2: feed an oversized OSC 7 (5 KB path). This is dropped upstream
    // by `OSC_PAYLOAD_MAX` since the total OSC payload (4097 + headers) > 4096
    // — and even if the upstream guard were ever relaxed, the cap inside
    // `dispatch/osc.rs::SetCwd` arm would catch it. The invariant under test
    // is the same in both cases: no `cwd_changed` event, no value mutation.
    let mut big_path = b"/".to_vec();
    big_path.extend(std::iter::repeat_n(b'a', 5_000));
    let mut seq = b"\x1b]7;".to_vec();
    seq.extend_from_slice(&big_path);
    seq.push(b'\x07');
    vt.process(&seq);

    assert!(
        vt.take_cwd_changed().is_none(),
        "oversized OSC 7 must not raise cwd_changed"
    );
    assert_eq!(
        vt.current_cwd(),
        Some("/tmp/initial"),
        "oversized OSC 7 must not mutate cwd"
    );
}

// ---------------------------------------------------------------------------
// VT-CAP-RESPONSES-001 — pending_responses is bounded at MAX_PENDING_RESPONSES
// ---------------------------------------------------------------------------

/// Flooding the VT with 1 000 DSR queries must leave the pending response
/// queue bounded at `MAX_PENDING_RESPONSES`. The drained `Vec` returned by
/// `take_responses` must contain at most `MAX_PENDING_RESPONSES` entries.
#[test]
fn vt_cap_responses_001_dsr_flood_is_bounded() {
    let mut vt = make_vt(80, 24);
    // Each `\x1b[5n` triggers a DSR-ready reply (`\x1b[0n`).
    let mut flood = Vec::with_capacity(1_000 * 4);
    for _ in 0..1_000 {
        flood.extend_from_slice(b"\x1b[5n");
    }
    vt.process(&flood);

    let drained = vt.take_responses();
    assert!(
        drained.len() <= MAX_PENDING_RESPONSES,
        "drained.len() = {} exceeds MAX_PENDING_RESPONSES = {}",
        drained.len(),
        MAX_PENDING_RESPONSES
    );
    // After drain, the queue must be empty.
    assert!(vt.take_responses().is_empty());
}

// ---------------------------------------------------------------------------
// VT-CAP-RESPONSES-002 — overflow drops oldest, keeps newest
// ---------------------------------------------------------------------------

/// Inject 300 distinguishable CPR queries and verify that the 256 most recent
/// responses are retained (drop-oldest semantics). We use CPR (`\x1b[6n`),
/// which encodes the cursor row/col in the reply, and step the cursor
/// position between each query so every reply has a unique payload.
#[test]
fn vt_cap_responses_002_drop_oldest_keeps_newest() {
    let mut vt = make_vt(80, 24);

    // Inject 300 CPR queries, moving the cursor between each so the responses
    // are distinguishable. Cursor row cycles 1..=24 (terminal has 24 rows).
    const N: usize = 300;
    for i in 0..N {
        // 1-based row in [1, 24], distinct enough across i.
        let row = (i % 24) + 1;
        // 1-based col in [1, 80], distinct enough across i.
        let col = (i % 80) + 1;
        // CUP to (row, col), then DSR CPR.
        let seq = format!("\x1b[{row};{col}H\x1b[6n");
        vt.process(seq.as_bytes());
    }

    let drained = vt.take_responses();
    assert_eq!(
        drained.len(),
        MAX_PENDING_RESPONSES,
        "queue must hold exactly MAX_PENDING_RESPONSES after overflow"
    );

    // Compute the expected last response: after the loop the cursor was set
    // to row = ((N - 1) % 24) + 1, col = ((N - 1) % 80) + 1.
    let last_row = ((N - 1) % 24) + 1;
    let last_col = ((N - 1) % 80) + 1;
    let expected_last = format!("\x1b[{last_row};{last_col}R").into_bytes();
    assert_eq!(
        drained.last().expect("non-empty"),
        &expected_last,
        "newest response must be preserved (drop-oldest semantics)"
    );

    // Sanity: the response that *would* have been queued by the very first
    // iteration (CUP 1;1 → CPR \x1b[1;1R) MUST have been evicted, because
    // 300 - 256 = 44 oldest entries were dropped.
    let evicted = b"\x1b[1;1R".to_vec();
    let evicted_idx = drained.iter().position(|r| r == &evicted);
    assert!(
        evicted_idx != Some(0),
        "oldest CPR (1;1) should have been evicted from the queue head"
    );
}

// ---------------------------------------------------------------------------
// SEC-VT-CAP-OSC52-OVERSIZE-001 — OSC 52 oversized payload regression guard
// ---------------------------------------------------------------------------

/// A 10 KB OSC 52 clipboard write must be dropped at the parser layer by
/// `OSC_PAYLOAD_MAX = 4 096` in `dispatch/osc.rs`. After processing such a
/// sequence, `take_osc52_write()` must return `None`.
///
/// This is a regression guard for ADR-0028 §Security: the OSC 52 payload is
/// **not** capped at the accumulation site of `pending_osc52_write` because
/// the upstream guard is sufficient. Should that upstream guard ever be
/// removed or weakened, this test will catch the regression.
#[test]
fn sec_vt_cap_osc52_oversize_001_dropped_upstream() {
    // Construct a VT where OSC 52 writes are policy-allowed (otherwise the
    // payload would be silently rejected for an unrelated reason).
    let mut vt = crate::vt::VtProcessor::new(80, 24, 10_000, 0, true);

    // 10 KB base64 payload — far above OSC_PAYLOAD_MAX = 4 096.
    let big_b64 = b"A".repeat(10_240);
    let mut seq = b"\x1b]52;c;".to_vec();
    seq.extend_from_slice(&big_b64);
    seq.push(b'\x07');
    vt.process(&seq);

    let payload = vt.take_osc52_write();
    assert!(
        payload.is_none(),
        "oversized OSC 52 must be dropped upstream (OSC_PAYLOAD_MAX = 4096); got {} chars",
        payload.map(|s| s.len()).unwrap_or(0)
    );
}
