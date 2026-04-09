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

/// FPL-S-010: Variables absent from the explicit allowlist MUST NOT appear
/// in the child process environment after spawn (FS-PTY-011, L3).
///
/// Strategy: plant a unique canary variable in TauTerm's process environment,
/// spawn a shell that dumps all env vars, assert the canary is absent.
/// nextest runs each test in its own process, so set_var is safe here.
#[test]
fn fpl_s_010_unlisted_env_var_not_present_in_child() {
    let canary_key = format!("TAUTERM_SECRET_CANARY_{}", std::process::id());
    let canary_value = format!("TAUTERM_CANARY_VALUE_{}", std::process::id());
    // SAFETY: nextest runs each test in its own process — no concurrent threads
    // read or write the process environment at this point.
    unsafe { std::env::set_var(&canary_key, &canary_value) };

    let session = open_linux_session_with_env(
        80,
        24,
        "/bin/sh",
        &["-c", "printenv; echo ENV_DUMP_DONE; exit"],
        &[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LINES", "24"),
            ("COLUMNS", "80"),
            ("TERM_PROGRAM", "TauTerm"),
        ],
    );
    let reader = session.reader_handle().expect("must have reader");
    let output = read_until_timeout(reader, "ENV_DUMP_DONE", std::time::Duration::from_secs(5));
    // SAFETY: same process-isolation guarantee as above.
    unsafe { std::env::remove_var(&canary_key) };

    let output =
        output.expect("FPL-S-010: shell must complete printenv and print ENV_DUMP_DONE within 5s");
    assert!(
        !output.contains(&canary_value),
        "FPL-S-010: canary '{canary_key}' MUST NOT appear in child env — env_clear() policy broken"
    );
}
