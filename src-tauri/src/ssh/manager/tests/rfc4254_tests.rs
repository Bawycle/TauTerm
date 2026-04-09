// SPDX-License-Identifier: MPL-2.0

//! RFC 4254 §8 terminal mode opcode regression tests.
//!
//! These tests verify that `russh::Pty` discriminant values match the opcode
//! table in RFC 4254 §8. If russh ever changes its internal numbering, these
//! tests will catch the mismatch before it silently corrupts SSH PTY requests.
//!
//! Current status (russh 0.60.0): values match RFC 4254 exactly.

// Verify individual opcodes that TauTerm's TERMINAL_MODES table depends on.

#[test]
fn rfc4254_pty_opcode_vintr_is_1() {
    assert_eq!(
        russh::Pty::VINTR as u8,
        1,
        "RFC 4254 §8: VINTR opcode must be 1"
    );
}

#[test]
fn rfc4254_pty_opcode_vquit_is_2() {
    assert_eq!(
        russh::Pty::VQUIT as u8,
        2,
        "RFC 4254 §8: VQUIT opcode must be 2"
    );
}

#[test]
fn rfc4254_pty_opcode_verase_is_3() {
    assert_eq!(
        russh::Pty::VERASE as u8,
        3,
        "RFC 4254 §8: VERASE opcode must be 3"
    );
}

#[test]
fn rfc4254_pty_opcode_vkill_is_4() {
    assert_eq!(
        russh::Pty::VKILL as u8,
        4,
        "RFC 4254 §8: VKILL opcode must be 4"
    );
}

#[test]
fn rfc4254_pty_opcode_veof_is_5() {
    assert_eq!(
        russh::Pty::VEOF as u8,
        5,
        "RFC 4254 §8: VEOF opcode must be 5"
    );
}

#[test]
fn rfc4254_pty_opcode_vsusp_is_10() {
    assert_eq!(
        russh::Pty::VSUSP as u8,
        10,
        "RFC 4254 §8: VSUSP opcode must be 10"
    );
}

#[test]
fn rfc4254_pty_opcode_isig_is_50() {
    assert_eq!(
        russh::Pty::ISIG as u8,
        50,
        "RFC 4254 §8: ISIG opcode must be 50"
    );
}

#[test]
fn rfc4254_pty_opcode_icanon_is_51() {
    assert_eq!(
        russh::Pty::ICANON as u8,
        51,
        "RFC 4254 §8: ICANON opcode must be 51"
    );
}

#[test]
fn rfc4254_pty_opcode_echo_is_53() {
    assert_eq!(
        russh::Pty::ECHO as u8,
        53,
        "RFC 4254 §8: ECHO opcode must be 53"
    );
}

/// Comprehensive table check: all opcodes used in TERMINAL_MODES must match
/// their RFC 4254 §8 values simultaneously.
///
/// This test documents the expected mapping in a single place and catches
/// regressions where one opcode changes without others being noticed.
#[test]
fn rfc4254_terminal_modes_opcodes_match_rfc_table() {
    // (russh::Pty variant, expected RFC 4254 opcode)
    let expected: &[(russh::Pty, u8)] = &[
        (russh::Pty::VINTR, 1),
        (russh::Pty::VQUIT, 2),
        (russh::Pty::VERASE, 3),
        (russh::Pty::VKILL, 4),
        (russh::Pty::VEOF, 5),
        (russh::Pty::VSUSP, 10),
        (russh::Pty::ISIG, 50),
        (russh::Pty::ICANON, 51),
        (russh::Pty::ECHO, 53),
    ];

    for (pty, expected_opcode) in expected {
        assert_eq!(
            *pty as u8, *expected_opcode,
            "RFC 4254 §8 opcode mismatch for {pty:?}: expected {expected_opcode}, got {}",
            *pty as u8,
        );
    }
}
