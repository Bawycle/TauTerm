// SPDX-License-Identifier: MPL-2.0

//! Tests for VT response sequences: DA1, DA2, DSR/CPR.
//!
//! These tests verify that `VtProcessor` populates `pending_responses` with the
//! correct bytes when the terminal receives device attribute or status queries.

use super::helpers::make_vt;

// ---------------------------------------------------------------------------
// DA1 — Primary Device Attributes (CSI c / CSI 0 c → \x1b[?1;2c)
// ---------------------------------------------------------------------------

#[test]
fn da1_responds_with_primary_device_attributes() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[c");
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    assert_eq!(
        response, b"\x1b[?1;2c",
        "Primary DA (CSI c) must respond \\x1b[?1;2c"
    );
}

#[test]
fn da1_explicit_zero_responds_with_primary_device_attributes() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[0c");
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    assert_eq!(
        response, b"\x1b[?1;2c",
        "Primary DA (CSI 0c) must respond \\x1b[?1;2c"
    );
}

#[test]
fn da1_nonzero_param_produces_no_response() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[1c");
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    assert!(
        response.is_empty(),
        "Primary DA with non-zero param must produce no response"
    );
}

// ---------------------------------------------------------------------------
// DA2 — Secondary Device Attributes (CSI > c / CSI > 0 c → \x1b[>0;10;0c)
// ---------------------------------------------------------------------------

#[test]
fn da2_responds_with_secondary_device_attributes() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[>c");
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    assert_eq!(
        response, b"\x1b[>0;10;0c",
        "Secondary DA (CSI > c) must respond \\x1b[>0;10;0c"
    );
}

#[test]
fn da2_explicit_zero_responds_with_secondary_device_attributes() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[>0c");
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    assert_eq!(
        response, b"\x1b[>0;10;0c",
        "Secondary DA (CSI > 0 c) must respond \\x1b[>0;10;0c"
    );
}

#[test]
fn da2_nonzero_param_produces_no_response() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[>1c");
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    assert!(
        response.is_empty(),
        "Secondary DA with non-zero param must produce no response"
    );
}

// ---------------------------------------------------------------------------
// DA1 + DA2 in sequence — both responses produced in order
// ---------------------------------------------------------------------------

#[test]
fn da1_and_da2_in_sequence_both_respond() {
    let mut vt = make_vt(80, 24);
    vt.process(b"\x1b[c\x1b[>c");
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    assert_eq!(
        response, b"\x1b[?1;2c\x1b[>0;10;0c",
        "DA1 followed by DA2 must produce both responses in order"
    );
}

// ---------------------------------------------------------------------------
// DSR CPR — Cursor Position Report (CSI 6n → \x1b[row;colR)
// Moved from editing/cursor.rs — tests VT response mechanics, not cursor editing.
// ---------------------------------------------------------------------------

#[test]
fn dsr_cpr_reports_normalized_position() {
    // Wide char at col 0; CUP to phantom col 1; CPR must report col 1 (1-based) = base cell.
    let mut vt = make_vt(10, 5);
    vt.process(b"\xe4\xb8\xad"); // 中 at col 0; phantom at col 1
    vt.process(b"\x1b[1;2H"); // CUP row 1, col 2 (phantom at col 1 0-indexed)
    // Trigger DSR CPR:
    vt.process(b"\x1b[6n");
    // Collect CPR response:
    let response: Vec<u8> = vt.take_responses().into_iter().flatten().collect();
    // Expected: ESC[1;1R (row 1, col 1, both 1-based)
    assert_eq!(
        response, b"\x1b[1;1R",
        "CPR must report normalized (base cell) position"
    );
}
