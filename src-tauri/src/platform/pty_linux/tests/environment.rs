// SPDX-License-Identifier: MPL-2.0

use super::{open_linux_session_with_env, read_until_timeout};

// -----------------------------------------------------------------------
// FPL-S-004 to FPL-S-009 — Environment variable injection (FS-PTY-011)
// -----------------------------------------------------------------------

/// FPL-S-004: TERM must be set to "xterm-256color" in the child process.
#[test]
fn fpl_s_004_env_term_is_xterm_256color() {
    let rows: u16 = 24;
    let cols: u16 = 80;
    let session = open_linux_session_with_env(
        cols,
        rows,
        "/bin/sh",
        &["-c", "printenv TERM; exit"],
        &[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")],
    );
    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");
    let output = read_until_timeout(reader, "xterm-256color", std::time::Duration::from_secs(5));
    assert!(
        output.is_some(),
        "FPL-S-004: TERM=xterm-256color must appear in child process output"
    );
}

/// FPL-S-005: COLORTERM must be set to "truecolor" in the child process.
#[test]
fn fpl_s_005_env_colorterm_is_truecolor() {
    let session = open_linux_session_with_env(
        80,
        24,
        "/bin/sh",
        &["-c", "printenv COLORTERM; exit"],
        &[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")],
    );
    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");
    let output = read_until_timeout(reader, "truecolor", std::time::Duration::from_secs(5));
    assert!(
        output.is_some(),
        "FPL-S-005: COLORTERM=truecolor must appear in child process output"
    );
}

/// FPL-S-006: LINES must match the rows passed to open_session.
#[test]
fn fpl_s_006_env_lines_matches_rows() {
    let rows: u16 = 30;
    let cols: u16 = 80;
    let session = open_linux_session_with_env(
        cols,
        rows,
        "/bin/sh",
        &["-c", "printenv LINES; exit"],
        &[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LINES", &rows.to_string()),
            ("COLUMNS", &cols.to_string()),
        ],
    );
    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");
    let output = read_until_timeout(reader, "30", std::time::Duration::from_secs(5));
    assert!(
        output.is_some(),
        "FPL-S-006: LINES=30 must appear in child process output"
    );
}

/// FPL-S-007: COLUMNS must match the cols passed to open_session.
#[test]
fn fpl_s_007_env_columns_matches_cols() {
    let rows: u16 = 24;
    let cols: u16 = 132;
    let session = open_linux_session_with_env(
        cols,
        rows,
        "/bin/sh",
        &["-c", "printenv COLUMNS; exit"],
        &[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LINES", &rows.to_string()),
            ("COLUMNS", &cols.to_string()),
        ],
    );
    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");
    let output = read_until_timeout(reader, "132", std::time::Duration::from_secs(5));
    assert!(
        output.is_some(),
        "FPL-S-007: COLUMNS=132 must appear in child process output"
    );
}

/// FPL-S-008: TERM_PROGRAM must be set to "TauTerm" in the child process.
#[test]
fn fpl_s_008_env_term_program_is_tauterm() {
    let session = open_linux_session_with_env(
        80,
        24,
        "/bin/sh",
        &["-c", "printenv TERM_PROGRAM; exit"],
        &[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("TERM_PROGRAM", "TauTerm"),
        ],
    );
    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");
    let output = read_until_timeout(reader, "TauTerm", std::time::Duration::from_secs(5));
    assert!(
        output.is_some(),
        "FPL-S-008: TERM_PROGRAM=TauTerm must appear in child process output"
    );
}

/// FPL-S-009: TERM_PROGRAM_VERSION must be set in the child process.
#[test]
fn fpl_s_009_env_term_program_version_is_set() {
    let version = env!("CARGO_PKG_VERSION");
    let session = open_linux_session_with_env(
        80,
        24,
        "/bin/sh",
        &["-c", "printenv TERM_PROGRAM_VERSION; exit"],
        &[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("TERM_PROGRAM", "TauTerm"),
            ("TERM_PROGRAM_VERSION", version),
        ],
    );
    let reader = session
        .reader_handle()
        .expect("LinuxPtySession must have a reader");
    let output = read_until_timeout(reader, version, std::time::Duration::from_secs(5));
    assert!(
        output.is_some(),
        "FPL-S-009: TERM_PROGRAM_VERSION={version} must appear in child process output"
    );
}
