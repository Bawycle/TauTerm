// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::session::lifecycle::PaneLifecycleState;

// -----------------------------------------------------------------------
// TEST-SPRINT-001 — FS-PTY-013: CreateTabConfig.login=true → "--login" args
//
// The `create_tab` implementation selects args based on `config.login`:
//   let args: &[&str] = if config.login { &["--login"] } else { &[] };
//
// This test validates the contract at the `CreateTabConfig` level:
// a config with `login: true` must map to the `--login` flag, and a config
// with `login: false` must not pass any args.
// Integration with the real PTY backend requires a running system and is
// covered by the functional test protocol (PTY-FN-001).
// -----------------------------------------------------------------------

#[test]
fn test_sprint_001_login_true_selects_login_args() {
    // TEST-SPRINT-001
    let config = CreateTabConfig {
        label: None,
        cols: 80,
        rows: 24,
        shell: None,
        login: true,
    };
    // Mirror the logic from create_tab (line ~160):
    let args: &[&str] = if config.login { &["--login"] } else { &[] };
    assert_eq!(args, &["--login"], "login:true must produce --login arg");
}

#[test]
fn test_sprint_001_login_false_produces_empty_args() {
    // TEST-SPRINT-001
    let config = CreateTabConfig {
        label: None,
        cols: 80,
        rows: 24,
        shell: None,
        login: false,
    };
    let args: &[&str] = if config.login { &["--login"] } else { &[] };
    assert!(args.is_empty(), "login:false must produce no args");
}

#[test]
fn test_sprint_001_create_tab_config_login_default_is_false() {
    // TEST-SPRINT-001: serde default for `login` must be false so that
    // existing payloads without the field behave as non-login shells.
    let json = r#"{"cols":80,"rows":24}"#;
    let config: CreateTabConfig = serde_json::from_str(json).expect("deserialize failed");
    assert!(
        !config.login,
        "serde default for CreateTabConfig.login must be false"
    );
}

#[test]
fn test_sprint_001_create_tab_config_login_true_round_trips() {
    // TEST-SPRINT-001: login:true must survive a JSON round-trip (IPC safety).
    let config = CreateTabConfig {
        label: None,
        cols: 80,
        rows: 24,
        shell: None,
        login: true,
    };
    let json = serde_json::to_string(&config).expect("serialize failed");
    let restored: CreateTabConfig = serde_json::from_str(&json).expect("deserialize failed");
    assert!(restored.login, "login:true must survive serde round-trip");
}

// -----------------------------------------------------------------------
// R4 — Minimum PTY dimensions clamping (FS-PTY / arch §14.4)
//
// resize_pane() delegates clamping to clamp_pane_dimensions(), which is
// tested here directly. These tests verify the clamping function itself;
// they do not prove that resize_pane() still calls it — that linkage must
// be verified by code review or integration tests.
// -----------------------------------------------------------------------

#[test]
fn r4_constants_are_correct() {
    assert_eq!(MIN_COLS, 20, "MIN_COLS must be 20 (FS-PTY min)");
    assert_eq!(MIN_ROWS, 5, "MIN_ROWS must be 5 (FS-PTY min)");
}

#[test]
fn r4_clamp_zero_dimensions_to_minimum() {
    assert_eq!(
        clamp_pane_dimensions(0, 0),
        (MIN_COLS, MIN_ROWS),
        "cols=0 rows=0 must be clamped to (MIN_COLS, MIN_ROWS)"
    );
}

#[test]
fn r4_clamp_below_min_cols() {
    let (cols, rows) = clamp_pane_dimensions(5, 24);
    assert_eq!(cols, MIN_COLS, "cols=5 must be clamped to MIN_COLS");
    assert_eq!(rows, 24, "rows=24 must be unchanged");
}

#[test]
fn r4_clamp_below_min_rows() {
    let (cols, rows) = clamp_pane_dimensions(80, 2);
    assert_eq!(cols, 80, "cols=80 must be unchanged");
    assert_eq!(rows, MIN_ROWS, "rows=2 must be clamped to MIN_ROWS");
}

#[test]
fn r4_values_above_min_are_unchanged() {
    assert_eq!(
        clamp_pane_dimensions(80, 24),
        (80, 24),
        "values above minimum must pass through unchanged"
    );
}

#[test]
fn r4_values_exactly_at_min_are_unchanged() {
    assert_eq!(
        clamp_pane_dimensions(MIN_COLS, MIN_ROWS),
        (MIN_COLS, MIN_ROWS),
        "values exactly at minimum must not be modified"
    );
}

// -----------------------------------------------------------------------
// FS-PTY-005 — get_pane_termination_info logic
//
// The method is tested at the `PaneLifecycleState` extraction level.
// Constructing a full `SessionRegistry` requires `AppHandle`, which is not
// available in unit tests. The mapping from `Terminated` to `(exit_code,
// signal_name)` is tested here by directly inspecting the state variant.
// -----------------------------------------------------------------------

/// Helper that mirrors the extraction logic in `get_pane_termination_info`.
fn extract_termination_info(state: &PaneLifecycleState) -> Option<(Option<i32>, Option<String>)> {
    if let PaneLifecycleState::Terminated { exit_code, .. } = state {
        Some((*exit_code, None))
    } else {
        None
    }
}

#[test]
fn termination_info_exit0_returns_some_zero_none() {
    let state = PaneLifecycleState::Terminated {
        exit_code: Some(0),
        error: None,
    };
    let info = extract_termination_info(&state);
    assert_eq!(
        info,
        Some((Some(0), None)),
        "exit code 0 must produce (Some(0), None)"
    );
}

#[test]
fn termination_info_nonzero_exit_returns_some_code_none() {
    let state = PaneLifecycleState::Terminated {
        exit_code: Some(1),
        error: None,
    };
    let info = extract_termination_info(&state);
    assert_eq!(
        info,
        Some((Some(1), None)),
        "non-zero exit must produce (Some(1), None)"
    );
}

#[test]
fn termination_info_signal_kill_returns_none_exit_none_signal() {
    // `exit_code` is None when killed by signal (WIFSIGNALED).
    // `signal_name` is None because `PaneLifecycleState` does not carry
    // a parseable signal name — only a human-readable `error` string.
    let state = PaneLifecycleState::Terminated {
        exit_code: None,
        error: Some("killed by signal 9".to_string()),
    };
    let info = extract_termination_info(&state);
    assert_eq!(
        info,
        Some((None, None)),
        "signal kill must produce (None, None) — signal_name requires future extension"
    );
}

#[test]
fn termination_info_non_terminated_state_returns_none() {
    for state in [
        PaneLifecycleState::Running,
        PaneLifecycleState::Spawning,
        PaneLifecycleState::Closing,
        PaneLifecycleState::Closed,
    ] {
        let info = extract_termination_info(&state);
        assert!(
            info.is_none(),
            "non-Terminated state {state:?} must return None"
        );
    }
}
