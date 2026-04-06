// SPDX-License-Identifier: MPL-2.0

//! Integration tests for DSR/CPR/DA response sequences.
//!
//! These tests verify that `VtProcessor` correctly enqueues responses in
//! `pending_responses` when it receives Device Status Report (`CSI 5n`,
//! `CSI 6n`) and Primary Device Attributes (`CSI c` / `CSI 0c`) requests.
//!
//! The `take_responses()` drain pattern mirrors what Task 1 of `pty_task`
//! does after each `process()` call.

use tau_term_lib::vt::VtProcessor;

// ---------------------------------------------------------------------------
// DSR — Device Status Report
// ---------------------------------------------------------------------------

/// `CSI 5n` must enqueue `\x1b[0n` (terminal ready).
#[test]
fn test_device_status_ready() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    vt.process(b"\x1b[5n");
    let responses = vt.take_responses();
    assert_eq!(
        responses.len(),
        1,
        "expected exactly one response for CSI 5n"
    );
    assert_eq!(
        responses[0], b"\x1b[0n",
        "CSI 5n must respond with \\x1b[0n (terminal ready)"
    );
}

/// `CSI 6n` must enqueue `\x1b[row;colR` with 1-based cursor position.
#[test]
fn test_dsr_cursor_position_at_home() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    // Cursor starts at (0, 0) — expect `\x1b[1;1R`.
    vt.process(b"\x1b[6n");
    let responses = vt.take_responses();
    assert_eq!(
        responses.len(),
        1,
        "expected exactly one response for CSI 6n"
    );
    assert_eq!(
        responses[0], b"\x1b[1;1R",
        "CSI 6n at home must respond with \\x1b[1;1R"
    );
}

/// `CSI 6n` after a `CUP` move must report the updated 1-based position.
#[test]
fn test_dsr_cursor_position_after_cup() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    // Move cursor to row 6, col 11 (1-based) via CUP — 0-based: (5, 10).
    vt.process(b"\x1b[6;11H");
    vt.process(b"\x1b[6n");
    let responses = vt.take_responses();
    assert_eq!(
        responses.len(),
        1,
        "expected exactly one response for CSI 6n"
    );
    assert_eq!(
        responses[0], b"\x1b[6;11R",
        "CSI 6n after CUP 6;11H must respond with \\x1b[6;11R"
    );
}

// ---------------------------------------------------------------------------
// DA — Primary Device Attributes
// ---------------------------------------------------------------------------

/// `CSI c` (parameter omitted, defaults to 0) must enqueue `\x1b[?1;2c`.
#[test]
fn test_da_primary_attributes_no_param() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    vt.process(b"\x1b[c");
    let responses = vt.take_responses();
    assert_eq!(
        responses.len(),
        1,
        "expected exactly one response for CSI c"
    );
    assert_eq!(
        responses[0], b"\x1b[?1;2c",
        "CSI c must respond with \\x1b[?1;2c (VT100 + AVO)"
    );
}

/// `CSI 0c` (explicit parameter 0) must also enqueue `\x1b[?1;2c`.
#[test]
fn test_da_primary_attributes_explicit_zero() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    vt.process(b"\x1b[0c");
    let responses = vt.take_responses();
    assert_eq!(
        responses.len(),
        1,
        "expected exactly one response for CSI 0c"
    );
    assert_eq!(
        responses[0], b"\x1b[?1;2c",
        "CSI 0c must respond with \\x1b[?1;2c (VT100 + AVO)"
    );
}

/// Non-zero `CSI Nc` (Secondary DA request) must be silently ignored.
#[test]
fn test_da_nonzero_param_ignored() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    vt.process(b"\x1b[1c");
    let responses = vt.take_responses();
    assert!(
        responses.is_empty(),
        "CSI 1c (non-zero) must not enqueue any response"
    );
}

// ---------------------------------------------------------------------------
// take_responses() drain semantics
// ---------------------------------------------------------------------------

/// Calling `take_responses()` a second time on the same processor must
/// return an empty Vec (responses are consumed, not cloned).
#[test]
fn test_responses_cleared_after_take() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    vt.process(b"\x1b[5n");
    let first = vt.take_responses();
    assert_eq!(first.len(), 1, "first take must return the response");
    let second = vt.take_responses();
    assert!(
        second.is_empty(),
        "second take must return empty (responses are drained)"
    );
}

/// Multiple requests in one `process()` call must each enqueue a response.
#[test]
fn test_multiple_responses_in_one_process_call() {
    let mut vt = VtProcessor::new(80, 24, 1_000);
    // Three requests back-to-back in a single byte slice.
    vt.process(b"\x1b[5n\x1b[c\x1b[6n");
    let responses = vt.take_responses();
    assert_eq!(
        responses.len(),
        3,
        "three requests must produce three responses (got {})",
        responses.len()
    );
    assert_eq!(responses[0], b"\x1b[0n", "first response must be DSR ready");
    assert_eq!(responses[1], b"\x1b[?1;2c", "second response must be DA");
    // Third is CPR at home position.
    assert_eq!(
        responses[2], b"\x1b[1;1R",
        "third response must be CPR at home"
    );
}
