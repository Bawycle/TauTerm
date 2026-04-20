// SPDX-License-Identifier: MPL-2.0

//! Unit tests for `extract_process_output` — the SSH-side helper that owns
//! the VT write-lock window and drains every `take_*()` side-effect in one
//! pass (ADR-0028 Commit 2).
//!
//! Test ID group:
//!
//! - SSH-EXTRACT-001 — full extraction across all 6 fields + responses
//! - SSH-EXTRACT-002 — empty input yields an "is_empty" `ProcessOutput`
//! - SSH-EXTRACT-003 — DSR query populates the responses tuple
//! - SSH-EXTRACT-004 — `mode_changed` toggled then reset on the VT
//! - SSH-EXTRACT-005 — VT write-lock is released after the helper returns
//!
//! Note: `extract_process_output` is intentionally `pub(super)`-equivalent
//! (private module item) — we cannot test it without `super::`. These tests
//! therefore live as a child module so they share visibility with the
//! production helper.

use std::sync::Arc;

use parking_lot::RwLock;

use super::extract_process_output;
use crate::vt::VtProcessor;

fn make_vt() -> Arc<RwLock<VtProcessor>> {
    // 80×24, 1000 lines scrollback, default cursor shape, OSC 52 allowed.
    Arc::new(RwLock::new(VtProcessor::new(80, 24, 1_000, 0, true)))
}

// -----------------------------------------------------------------------
// SSH-EXTRACT-001
// -----------------------------------------------------------------------

/// SSH-EXTRACT-001: a single chunk that triggers title (OSC 0), bell (BEL),
/// OSC 52 clipboard, cursor shape (DECSCUSR), CWD (OSC 7), and printable
/// content yields a fully populated `ProcessOutput` with all 6 fields set
/// and a non-empty `dirty` region. Verifies the helper drains every
/// `take_*()` extraction in a single VT write-lock window.
#[test]
fn ssh_extract_001_extracts_all_fields_in_single_pass() {
    let vt = make_vt();

    // Sequence layout (each segment ends with BEL `\x07` for OSC string
    // termination where applicable, then starts the next):
    //   - OSC 0 ; title              → title
    //   - DECSCUSR (CSI 3 SP q)      → cursor shape = 3 (blinking underline)
    //   - OSC 52 ; c ; <base64>      → clipboard write
    //   - OSC 7 ; file://h/tmp       → CWD
    //   - "hello"                    → printable content (dirty)
    //   - BEL (`\x07`)               → bell pending
    //
    // "aGk=" is base64 for "hi".
    let bytes: &[u8] = b"\x1b]0;mytitle\x07\
                          \x1b[3 q\
                          \x1b]52;c;aGk=\x07\
                          \x1b]7;file://localhost/tmp\x07\
                          hello\
                          \x07";

    let (output, responses) = extract_process_output(&vt, bytes);

    assert_eq!(
        output.new_title.as_deref(),
        Some("mytitle"),
        "title must be extracted from OSC 0"
    );
    assert_eq!(
        output.new_cursor_shape,
        Some(3),
        "cursor shape must be extracted from DECSCUSR"
    );
    assert_eq!(
        output.osc52.as_deref(),
        Some("hi"),
        "OSC 52 payload must be decoded and extracted"
    );
    assert_eq!(
        output.new_cwd.as_deref(),
        Some("/tmp"),
        "CWD must be extracted from OSC 7 (file:// stripped)"
    );
    assert!(output.bell, "BEL must set the bell flag");
    assert!(
        !output.dirty.is_empty(),
        "printable 'hello' must produce a non-empty dirty region"
    );
    assert!(
        responses.is_empty(),
        "no DSR/CPR/DA query → no responses expected"
    );
    assert!(
        !output.needs_immediate_flush,
        "no responses → no immediate-flush hint"
    );
}

// -----------------------------------------------------------------------
// SSH-EXTRACT-002
// -----------------------------------------------------------------------

/// SSH-EXTRACT-002: empty input bytes yield an empty `ProcessOutput`
/// (every field cleared, `is_empty()` true) and an empty responses vector.
#[test]
fn ssh_extract_002_empty_input_yields_empty_output() {
    let vt = make_vt();

    let (output, responses) = extract_process_output(&vt, &[]);

    assert!(
        output.is_empty(),
        "empty input must produce an empty ProcessOutput"
    );
    assert!(
        !output.needs_immediate_flush,
        "empty input → no immediate-flush hint"
    );
    assert!(responses.is_empty(), "empty input → no responses");
}

// -----------------------------------------------------------------------
// SSH-EXTRACT-003
// -----------------------------------------------------------------------

/// SSH-EXTRACT-003: a DSR ready query (`CSI 5 n`) populates the responses
/// vector with at least one VT reply (`CSI 0 n`) and sets the
/// `needs_immediate_flush` hint on the `ProcessOutput`.
#[test]
fn ssh_extract_003_dsr_query_populates_responses() {
    let vt = make_vt();

    // CSI 5 n — Device Status Report (request "are you ready?")
    let (output, responses) = extract_process_output(&vt, b"\x1b[5n");

    assert!(
        !responses.is_empty(),
        "DSR query must produce at least one VT response"
    );
    assert!(
        output.needs_immediate_flush,
        "any response → needs_immediate_flush must be set"
    );
    // Sanity-check the response content: DSR ready replies with `\x1b[0n`.
    let merged: Vec<u8> = responses.into_iter().flatten().collect();
    assert!(
        merged.windows(4).any(|w| w == b"\x1b[0n"),
        "DSR response must contain `\\x1b[0n`, got {merged:?}"
    );
}

// -----------------------------------------------------------------------
// SSH-EXTRACT-004
// -----------------------------------------------------------------------

/// SSH-EXTRACT-004: `mode_changed` is correctly toggled by a mode-changing
/// sequence (DECCKM `CSI ? 1 h` — application cursor keys) and reset on
/// the VT processor itself after extraction (subsequent extraction with
/// no mode change reports `mode_changed = false`).
#[test]
fn ssh_extract_004_mode_changed_toggled_and_reset() {
    let vt = make_vt();

    // First extraction: DECCKM (application cursor keys) flips a mode flag.
    let (output1, _) = extract_process_output(&vt, b"\x1b[?1h");
    assert!(
        output1.mode_changed,
        "DECCKM must set mode_changed on the extraction"
    );
    assert!(
        !vt.read().mode_changed,
        "mode_changed must be reset on the VT processor after extraction"
    );

    // Second extraction: idle bytes (just printable text) → mode_changed
    // must remain false on both the output and the processor.
    let (output2, _) = extract_process_output(&vt, b"x");
    assert!(
        !output2.mode_changed,
        "subsequent extraction with no mode change must report mode_changed = false"
    );
    assert!(
        !vt.read().mode_changed,
        "VT processor mode_changed flag must remain false"
    );
}

// -----------------------------------------------------------------------
// SSH-EXTRACT-005
// -----------------------------------------------------------------------

/// SSH-EXTRACT-005: the VT write-lock is released by the time
/// `extract_process_output` returns. Verified by re-acquiring the
/// write-lock on the same VT immediately after the helper call —
/// `try_write()` returns `Some(_)` iff no writer guard is alive elsewhere.
///
/// This is the structural enforcement of the lock-ordering invariant
/// (ADR-0028 §Security Risk #1): callers can rely on this guarantee to
/// safely take `channel.lock().await` afterwards without risk of
/// deadlock.
#[test]
fn ssh_extract_005_vt_lock_released_after_return() {
    let vt = make_vt();

    let (_output, _responses) = extract_process_output(&vt, b"hello");

    // If any write-guard from the helper had escaped, `try_write` would
    // return `None`. A successful re-acquisition proves the guard was
    // dropped at the helper's closing brace.
    let guard = vt.try_write();
    assert!(
        guard.is_some(),
        "VT write-lock must be released after extract_process_output returns"
    );
}
