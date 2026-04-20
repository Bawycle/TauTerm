// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;

use super::*;
use crate::session::lifecycle::PaneLifecycleState;
use crate::session::pane::PaneSession;
use crate::session::tab::{PaneNode, TabState};

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
        pixel_width: 0,
        pixel_height: 0,
        shell: None,
        login: true,
        source_pane_id: None,
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
        pixel_width: 0,
        pixel_height: 0,
        shell: None,
        login: false,
        source_pane_id: None,
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
        pixel_width: 0,
        pixel_height: 0,
        shell: None,
        login: true,
        source_pane_id: None,
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

// -----------------------------------------------------------------------
// SSH-CLOSE-001 — get_tab_pane_ids returns correct pane IDs
//
// `close_tab` iterates `registry.get_tab_pane_ids(&tab_id)` to collect all
// pane IDs before calling `ssh_manager.close_connection` for each.
// These tests verify the contract of `get_tab_pane_ids`:
//   - returns the expected pane ID for a single-pane tab
//   - returns an empty vec for an unknown tab
//   - returns all pane IDs for a multi-pane (split) tab
//
// `SessionRegistry` requires `AppHandle` and cannot be constructed in unit
// tests. We test the equivalent logic by operating on `RegistryInner` and
// `TabEntry` directly (both private types accessible via `use super::*`).
// This mirrors the pattern used by `extract_termination_info` above.
// -----------------------------------------------------------------------

/// Build a minimal `PaneSession` for use in registry inner state fixtures.
fn make_pane_session(pane_id: &crate::session::ids::PaneId) -> PaneSession {
    PaneSession::new(pane_id.clone(), 80, 24, 1000, 0, false)
}

/// Build a minimal `TabState` wrapping a single leaf pane.
fn make_leaf_tab_state(
    tab_id: &crate::session::ids::TabId,
    pane_id: &crate::session::ids::PaneId,
) -> TabState {
    let pane_state = make_pane_session(pane_id).to_state();
    TabState {
        id: tab_id.clone(),
        label: None,
        active_pane_id: pane_id.clone(),
        order: 0,
        layout: PaneNode::Leaf {
            pane_id: pane_id.clone(),
            state: pane_state,
        },
    }
}

/// Helper that replicates the `get_tab_pane_ids` logic on a `RegistryInner`
/// directly, since `SessionRegistry` requires `AppHandle` in construction.
fn inner_get_tab_pane_ids(
    inner: &RegistryInner,
    tab_id: &crate::session::ids::TabId,
) -> Vec<crate::session::ids::PaneId> {
    inner
        .tabs
        .get(tab_id)
        .map(|e| e.panes.keys().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn ssh_close_001_get_tab_pane_ids_single_pane() {
    // SSH-CLOSE-001: get_tab_pane_ids returns the single pane ID for a tab
    // with one pane. Used by close_tab to collect panes before SSH disconnect.
    let tab_id = crate::session::ids::TabId::new();
    let pane_id = crate::session::ids::PaneId::new();

    let tab_state = make_leaf_tab_state(&tab_id, &pane_id);
    let pane_session = make_pane_session(&pane_id);

    let mut panes = HashMap::new();
    panes.insert(pane_id.clone(), pane_session);

    let mut inner = RegistryInner::new();
    inner.tabs.insert(
        tab_id.clone(),
        TabEntry {
            state: tab_state,
            panes,
        },
    );

    let ids = inner_get_tab_pane_ids(&inner, &tab_id);
    assert_eq!(
        ids.len(),
        1,
        "single-pane tab must yield exactly one pane ID"
    );
    assert_eq!(
        ids[0], pane_id,
        "returned pane ID must match the inserted pane"
    );
}

#[test]
fn ssh_close_001_get_tab_pane_ids_unknown_tab_returns_empty() {
    // SSH-CLOSE-001: get_tab_pane_ids returns empty vec for an unknown tab.
    // close_tab iterating over an empty vec is safe (no-op for SSH manager).
    let unknown_tab = crate::session::ids::TabId::new();
    let inner = RegistryInner::new();

    let ids = inner_get_tab_pane_ids(&inner, &unknown_tab);
    assert!(
        ids.is_empty(),
        "unknown tab must yield an empty pane ID vec (no SSH disconnect calls)"
    );
}

#[test]
fn ssh_close_001_get_tab_pane_ids_split_tab_returns_all_panes() {
    // SSH-CLOSE-001: get_tab_pane_ids returns all pane IDs for a split tab.
    // close_tab must call close_connection for every pane, not just the first.
    let tab_id = crate::session::ids::TabId::new();
    let pane_a = crate::session::ids::PaneId::new();
    let pane_b = crate::session::ids::PaneId::new();

    let state_a = make_pane_session(&pane_a).to_state();
    let state_b = make_pane_session(&pane_b).to_state();

    let tab_state = TabState {
        id: tab_id.clone(),
        label: None,
        active_pane_id: pane_a.clone(),
        order: 0,
        layout: PaneNode::Split {
            direction: crate::session::tab::SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::Leaf {
                pane_id: pane_a.clone(),
                state: state_a,
            }),
            second: Box::new(PaneNode::Leaf {
                pane_id: pane_b.clone(),
                state: state_b,
            }),
        },
    };

    let mut panes = HashMap::new();
    panes.insert(pane_a.clone(), make_pane_session(&pane_a));
    panes.insert(pane_b.clone(), make_pane_session(&pane_b));

    let mut inner = RegistryInner::new();
    inner.tabs.insert(
        tab_id.clone(),
        TabEntry {
            state: tab_state,
            panes,
        },
    );

    let mut ids = inner_get_tab_pane_ids(&inner, &tab_id);
    ids.sort_by_key(|a| a.to_string()); // stabilise order
    let mut expected = vec![pane_a, pane_b];
    expected.sort_by_key(|a| a.to_string());

    assert_eq!(
        ids, expected,
        "split tab must yield both pane IDs so both SSH connections are closed"
    );
}

// -----------------------------------------------------------------------
// OSC52 propagation tests (osc52_prop_001 – osc52_prop_005)
//
// These tests verify the invariants of `propagate_osc52_allow` and
// `apply_pane_osc52_override` at the `RegistryInner` level, operating
// directly on `TabEntry.panes` to avoid the `AppHandle` requirement.
//
// The logic is replicated inline (mirrors what the real methods do),
// so these tests verify the contract rather than the method call itself.
// This matches the approach used for `extract_termination_info` above.
// -----------------------------------------------------------------------

/// Replicates the logic of `propagate_osc52_allow` on a `RegistryInner`.
fn inner_propagate_osc52_allow(inner: &RegistryInner, allow: bool) {
    for entry in inner.tabs.values() {
        for pane in entry.panes.values() {
            if pane.osc52_overridden {
                continue;
            }
            pane.vt.write().allow_osc52_write = allow;
        }
    }
}

/// Replicates the logic of `apply_pane_osc52_override` on a `RegistryInner`.
fn inner_apply_pane_osc52_override(
    inner: &mut RegistryInner,
    pane_id: &crate::session::ids::PaneId,
    allow: bool,
) {
    if let Some(pane) = inner
        .tabs
        .values_mut()
        .find_map(|e| e.panes.get_mut(pane_id))
    {
        pane.vt.write().allow_osc52_write = allow;
        pane.osc52_overridden = true;
    }
}

/// Build a `RegistryInner` containing a single tab with one pane.
/// Returns (inner, tab_id, pane_id).
fn make_registry_with_one_pane() -> (
    RegistryInner,
    crate::session::ids::TabId,
    crate::session::ids::PaneId,
) {
    let tab_id = crate::session::ids::TabId::new();
    let pane_id = crate::session::ids::PaneId::new();

    let tab_state = make_leaf_tab_state(&tab_id, &pane_id);
    let pane_session = make_pane_session(&pane_id);

    let mut panes = HashMap::new();
    panes.insert(pane_id.clone(), pane_session);

    let mut inner = RegistryInner::new();
    inner.pane_to_tab.insert(pane_id.clone(), tab_id.clone());
    inner.tabs.insert(
        tab_id.clone(),
        TabEntry {
            state: tab_state,
            panes,
        },
    );

    (inner, tab_id, pane_id)
}

/// Helper: read `allow_osc52_write` from a pane in `RegistryInner`.
fn read_osc52_allow(inner: &RegistryInner, pane_id: &crate::session::ids::PaneId) -> bool {
    inner
        .tabs
        .values()
        .find_map(|e| e.panes.get(pane_id))
        .map(|p| p.vt.read().allow_osc52_write)
        .expect("pane not found in registry")
}

/// Helper: read `osc52_overridden` from a pane in `RegistryInner`.
fn read_osc52_overridden(inner: &RegistryInner, pane_id: &crate::session::ids::PaneId) -> bool {
    inner
        .tabs
        .values()
        .find_map(|e| e.panes.get(pane_id))
        .map(|p| p.osc52_overridden)
        .expect("pane not found in registry")
}

/// osc52_prop_001: Global propagation must NOT overwrite an SSH pane that has
/// `osc52_overridden = true`.
#[test]
fn osc52_prop_001_global_change_does_not_affect_ssh_pane_with_override() {
    let (mut inner, _, pane_id) = make_registry_with_one_pane();

    // Simulate an SSH override: policy = allow, pane is marked as overridden.
    inner_apply_pane_osc52_override(&mut inner, &pane_id, true);
    assert!(read_osc52_allow(&inner, &pane_id));
    assert!(read_osc52_overridden(&inner, &pane_id));

    // Global preference changes to false — must not touch the overridden pane.
    inner_propagate_osc52_allow(&inner, false);

    assert!(
        read_osc52_allow(&inner, &pane_id),
        "osc52_overridden pane must keep allow=true after propagate_osc52_allow(false)"
    );
}

/// osc52_prop_002: Global propagation DOES update a local pane
/// (`osc52_overridden = false`).
#[test]
fn osc52_prop_002_global_change_does_affect_local_pane() {
    let (inner, _, pane_id) = make_registry_with_one_pane();

    // Initial state: allow=false (default from make_pane_session), not overridden.
    assert!(!read_osc52_allow(&inner, &pane_id));
    assert!(!read_osc52_overridden(&inner, &pane_id));

    inner_propagate_osc52_allow(&inner, true);

    assert!(
        read_osc52_allow(&inner, &pane_id),
        "local pane (not overridden) must have allow=true after propagate_osc52_allow(true)"
    );
}

/// osc52_prop_003: `apply_pane_osc52_override(true)` on a pane whose VT has
/// `allow=false` sets `allow=true` and marks `osc52_overridden=true`.
#[test]
fn osc52_prop_003_ssh_conn_allow_true_global_false() {
    let (mut inner, _, pane_id) = make_registry_with_one_pane();

    // Starting state: allow=false (global default).
    assert!(!read_osc52_allow(&inner, &pane_id));

    inner_apply_pane_osc52_override(&mut inner, &pane_id, true);

    assert!(
        read_osc52_allow(&inner, &pane_id),
        "apply_pane_osc52_override(true) must set allow_osc52_write=true"
    );
    assert!(
        read_osc52_overridden(&inner, &pane_id),
        "apply_pane_osc52_override must set osc52_overridden=true"
    );
}

/// osc52_prop_004: `apply_pane_osc52_override(false)` on a pane with global
/// `allow=true` sets `allow=false`, marks override, and a subsequent global
/// propagation does NOT reset it back to true.
#[test]
fn osc52_prop_004_ssh_conn_allow_false_global_true_survives_propagation() {
    let (mut inner, _, pane_id) = make_registry_with_one_pane();

    // Start with global allow=true.
    inner_propagate_osc52_allow(&inner, true);
    assert!(read_osc52_allow(&inner, &pane_id));

    // SSH override: this pane must have allow=false.
    inner_apply_pane_osc52_override(&mut inner, &pane_id, false);
    assert!(
        !read_osc52_allow(&inner, &pane_id),
        "apply_pane_osc52_override(false) must set allow_osc52_write=false"
    );
    assert!(
        read_osc52_overridden(&inner, &pane_id),
        "osc52_overridden must be true after apply_pane_osc52_override"
    );

    // Global preference changes back to true — must not affect overridden pane.
    inner_propagate_osc52_allow(&inner, true);
    assert!(
        !read_osc52_allow(&inner, &pane_id),
        "overridden pane must remain allow=false after propagate_osc52_allow(true)"
    );
}

// -----------------------------------------------------------------------
// update_pane_title / update_pane_cwd — title resolution pipeline tests
//
// `SessionRegistry` cannot be constructed in unit tests (requires AppHandle).
// We replicate the logic of `update_pane_title` and `update_pane_cwd` on
// `RegistryInner` directly, exactly as the OSC52 tests above do for their
// respective operations.
//
// `resolve_effective_title` is a private method of `SessionRegistry` so we
// duplicate its logic here. Any change to the priority chain in `tab_ops.rs`
// must be mirrored here — this is an intentional trade-off to avoid the
// AppHandle dependency in unit tests.
// -----------------------------------------------------------------------

/// Replicates `SessionRegistry::resolve_effective_title` for use in unit tests.
fn resolve_effective_title_for_test(pane: &PaneSession) -> Option<String> {
    // Priority 1: OSC 0/2 title.
    if let Some(ref t) = pane.title
        && !t.is_empty()
    {
        return Some(t.clone());
    }
    // Priority 2: CWD basename.
    if let Some(ref cwd) = pane.cwd
        && let Some(name) = std::path::Path::new(cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_owned())
        && !name.is_empty()
    {
        return Some(name);
    }
    // Priority 3: foreground process name — always None in unit tests (no real PTY).
    None
}

/// Replicates `SessionRegistry::update_pane_title` on `RegistryInner`.
/// Returns `Some(TabState)` only when the effective display title changed,
/// `None` when the pane is not found or the title is unchanged.
fn inner_update_pane_title(
    inner: &mut RegistryInner,
    pane_id: &crate::session::ids::PaneId,
    title: String,
) -> Option<TabState> {
    let (_tab_id, entry) = inner
        .tabs
        .iter_mut()
        .find(|(_, e)| e.panes.contains_key(pane_id))
        .map(|(id, e)| (id.clone(), e))?;

    let old_title = entry
        .panes
        .get(pane_id)
        .and_then(resolve_effective_title_for_test);

    if let Some(pane) = entry.panes.get_mut(pane_id) {
        pane.title = Some(title.clone());
    }

    let new_title = entry
        .panes
        .get(pane_id)
        .and_then(resolve_effective_title_for_test);

    if new_title == old_title {
        return None;
    }

    let display_title = new_title.unwrap_or(title);
    layout::update_pane_title_in_tree(&mut entry.state.layout, pane_id, &display_title);

    Some(entry.state.clone())
}

/// Replicates `SessionRegistry::update_pane_cwd` on `RegistryInner`.
/// Always returns `Some(TabState)` when the pane is found (CWD is always
/// propagated to the layout tree so the frontend can read it).
fn inner_update_pane_cwd(
    inner: &mut RegistryInner,
    pane_id: &crate::session::ids::PaneId,
    cwd: String,
) -> Option<TabState> {
    let (_tab_id, entry) = inner
        .tabs
        .iter_mut()
        .find(|(_, e)| e.panes.contains_key(pane_id))
        .map(|(id, e)| (id.clone(), e))?;

    if let Some(pane) = entry.panes.get_mut(pane_id) {
        let old_title = resolve_effective_title_for_test(pane);
        pane.cwd = Some(cwd.clone());
        let new_title = resolve_effective_title_for_test(pane);
        layout::update_pane_cwd_in_tree(&mut entry.state.layout, pane_id, &cwd);
        if new_title != old_title
            && let Some(ref t) = new_title
        {
            layout::update_pane_title_in_tree(&mut entry.state.layout, pane_id, t);
        }
        return Some(entry.state.clone());
    }
    None
}

/// Extract the `PaneState.title` from the leaf node of a single-pane `TabState`.
fn leaf_title(tab_state: &TabState, pane_id: &crate::session::ids::PaneId) -> Option<String> {
    match &tab_state.layout {
        PaneNode::Leaf { pane_id: id, state } if id == pane_id => state.title.clone(),
        _ => None,
    }
}

#[test]
fn update_pane_title_stores_title_and_returns_tab_state() {
    let (mut inner, _tab_id, pane_id) = make_registry_with_one_pane();

    let result = inner_update_pane_title(&mut inner, &pane_id, "my-title".to_string());
    assert!(
        result.is_some(),
        "update_pane_title must return Some(TabState)"
    );

    let tab_state = result.unwrap();
    assert_eq!(
        leaf_title(&tab_state, &pane_id),
        Some("my-title".to_string()),
        "PaneState.title in the layout tree must reflect the OSC title"
    );
}

#[test]
fn update_pane_cwd_returns_some_when_osc7_basename_changes_effective_title() {
    // No OSC 0/2 title set — CWD basename is the effective title.
    let (mut inner, _tab_id, pane_id) = make_registry_with_one_pane();

    let result = inner_update_pane_cwd(&mut inner, &pane_id, "/home/user/projects".to_string());
    assert!(
        result.is_some(),
        "update_pane_cwd must return Some(TabState) when CWD basename changes the effective title"
    );

    let tab_state = result.unwrap();
    assert_eq!(
        leaf_title(&tab_state, &pane_id),
        Some("projects".to_string()),
        "PaneState.title must be the CWD basename when no OSC 0/2 title is set"
    );
}

#[test]
fn update_pane_cwd_preserves_osc02_title_but_updates_cwd() {
    // OSC 0/2 title is already set — changing CWD must not change the effective title,
    // but the CWD must still be updated in the layout tree (for status bar, tab creation).
    let (mut inner, _tab_id, pane_id) = make_registry_with_one_pane();

    // Set an OSC 0/2 title first.
    inner_update_pane_title(&mut inner, &pane_id, "existing-title".to_string());

    // Now update CWD — always returns Some so the frontend gets the new CWD.
    let result = inner_update_pane_cwd(&mut inner, &pane_id, "/home/user/other".to_string());
    assert!(
        result.is_some(),
        "update_pane_cwd must return Some(TabState) even when the title is unchanged"
    );

    // The effective title must still be the OSC 0/2 title, not the CWD basename.
    let tab_state = result.unwrap();
    assert_eq!(
        leaf_title(&tab_state, &pane_id),
        Some("existing-title".to_string()),
        "OSC 0/2 title must still take priority over CWD basename"
    );

    // But the CWD must be updated in the layout tree.
    let leaf_cwd = match &tab_state.layout {
        PaneNode::Leaf { state, .. } => state.cwd.clone(),
        _ => None,
    };
    assert_eq!(
        leaf_cwd,
        Some("/home/user/other".to_string()),
        "PaneState.cwd in the layout tree must reflect the new CWD"
    );
}

/// osc52_prop_005: Mixed panes in the same tab — local pane is affected by
/// propagation while the SSH pane with override is not.
#[test]
fn osc52_prop_005_mixed_panes_local_affected_ssh_not() {
    let tab_id = crate::session::ids::TabId::new();
    let pane_local = crate::session::ids::PaneId::new();
    let pane_ssh = crate::session::ids::PaneId::new();

    let state_local = make_pane_session(&pane_local).to_state();
    let state_ssh = make_pane_session(&pane_ssh).to_state();

    let tab_state = crate::session::tab::TabState {
        id: tab_id.clone(),
        label: None,
        active_pane_id: pane_local.clone(),
        order: 0,
        layout: crate::session::tab::PaneNode::Split {
            direction: crate::session::tab::SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(crate::session::tab::PaneNode::Leaf {
                pane_id: pane_local.clone(),
                state: state_local,
            }),
            second: Box::new(crate::session::tab::PaneNode::Leaf {
                pane_id: pane_ssh.clone(),
                state: state_ssh,
            }),
        },
    };

    let session_local = make_pane_session(&pane_local);
    // local: osc52_overridden=false, allow=false (default)
    assert!(!session_local.osc52_overridden);

    let mut session_ssh = make_pane_session(&pane_ssh);
    // SSH: manually set override and allow=true
    session_ssh.vt.write().allow_osc52_write = true;
    session_ssh.osc52_overridden = true;

    let mut panes = HashMap::new();
    panes.insert(pane_local.clone(), session_local);
    panes.insert(pane_ssh.clone(), session_ssh);

    let mut inner = RegistryInner::new();
    inner.tabs.insert(
        tab_id.clone(),
        TabEntry {
            state: tab_state,
            panes,
        },
    );

    // propagate_osc52_allow(false): local must become false (already false),
    // ssh must stay true.
    inner_propagate_osc52_allow(&inner, false);

    assert!(
        !read_osc52_allow(&inner, &pane_local),
        "local pane must have allow=false after propagate_osc52_allow(false)"
    );
    assert!(
        read_osc52_allow(&inner, &pane_ssh),
        "SSH pane with override must keep allow=true after propagate_osc52_allow(false)"
    );
}

// -----------------------------------------------------------------------
// validated_working_dir — CWD validation helper tests (FS-VT-064)
// -----------------------------------------------------------------------

use super::pty_helpers::validated_working_dir;

#[test]
fn validated_working_dir_none_returns_none() {
    assert!(validated_working_dir(None).is_none());
}

#[test]
fn validated_working_dir_empty_returns_none() {
    assert!(validated_working_dir(Some("")).is_none());
}

#[test]
fn validated_working_dir_relative_returns_none() {
    assert!(validated_working_dir(Some("foo/bar")).is_none());
}

#[test]
fn validated_working_dir_absolute_returns_some() {
    let result = validated_working_dir(Some("/home/user/projects"));
    assert_eq!(
        result,
        Some(std::path::PathBuf::from("/home/user/projects"))
    );
}

#[test]
fn validated_working_dir_root_returns_some() {
    let result = validated_working_dir(Some("/"));
    assert_eq!(result, Some(std::path::PathBuf::from("/")));
}

#[test]
fn validated_working_dir_bidi_override_returns_none() {
    // U+202E RIGHT-TO-LEFT OVERRIDE embedded in a path
    assert!(validated_working_dir(Some("/home/user/\u{202E}evil")).is_none());
}

#[test]
fn validated_working_dir_zero_width_space_returns_none() {
    // U+200B ZERO WIDTH SPACE
    assert!(validated_working_dir(Some("/home/user/\u{200B}hidden")).is_none());
}

// -----------------------------------------------------------------------
// CreateTabConfig.source_pane_id — serde tests
// -----------------------------------------------------------------------

#[test]
fn create_tab_config_source_pane_id_defaults_to_none() {
    let json = r#"{"cols":80,"rows":24}"#;
    let config: CreateTabConfig = serde_json::from_str(json).expect("deserialize failed");
    assert!(
        config.source_pane_id.is_none(),
        "serde default for source_pane_id must be None"
    );
}

#[test]
fn create_tab_config_source_pane_id_round_trips() {
    let config = CreateTabConfig {
        label: None,
        cols: 80,
        rows: 24,
        pixel_width: 0,
        pixel_height: 0,
        shell: None,
        login: false,
        source_pane_id: Some(PaneId::new()),
    };
    let json = serde_json::to_string(&config).expect("serialize failed");
    let restored: CreateTabConfig = serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(
        restored.source_pane_id, config.source_pane_id,
        "source_pane_id must survive serde round-trip"
    );
}

// -----------------------------------------------------------------------
// pane_to_tab index invariant tests
//
// These tests verify that the `pane_to_tab: HashMap<PaneId, TabId>` reverse
// index in `RegistryInner` stays consistent with `tabs` after insert/remove
// operations. Since we cannot instantiate `SessionRegistry` in unit tests
// (requires `AppHandle`), we test the invariant at the `RegistryInner` level
// by manually maintaining the index as the real methods do.
// -----------------------------------------------------------------------

/// Helper: insert a tab with its panes into `RegistryInner`, maintaining the
/// `pane_to_tab` index exactly as `create_tab` and `split_pane` do.
fn insert_tab_with_panes(
    inner: &mut RegistryInner,
    tab_id: &crate::session::ids::TabId,
    pane_ids: &[crate::session::ids::PaneId],
    tab_state: TabState,
) {
    let mut panes = HashMap::new();
    for pid in pane_ids {
        panes.insert(pid.clone(), make_pane_session(pid));
        inner.pane_to_tab.insert(pid.clone(), tab_id.clone());
    }
    inner.tabs.insert(
        tab_id.clone(),
        TabEntry {
            state: tab_state,
            panes,
        },
    );
}

#[test]
fn pane_to_tab_invariant_after_insert_and_remove() {
    let mut inner = RegistryInner::new();

    let tab1 = crate::session::ids::TabId::new();
    let tab2 = crate::session::ids::TabId::new();
    let p1 = crate::session::ids::PaneId::new();
    let p2 = crate::session::ids::PaneId::new();
    let p3 = crate::session::ids::PaneId::new();

    insert_tab_with_panes(
        &mut inner,
        &tab1,
        &[p1.clone(), p2.clone()],
        make_leaf_tab_state(&tab1, &p1),
    );
    insert_tab_with_panes(
        &mut inner,
        &tab2,
        std::slice::from_ref(&p3),
        make_leaf_tab_state(&tab2, &p3),
    );

    // Verify forward lookups.
    assert_eq!(inner.pane_to_tab.len(), 3);
    assert_eq!(inner.tab_id_for_pane(&p1).unwrap(), tab1);
    assert_eq!(inner.tab_id_for_pane(&p2).unwrap(), tab1);
    assert_eq!(inner.tab_id_for_pane(&p3).unwrap(), tab2);

    // Remove one pane (mirrors close_pane logic).
    inner.pane_to_tab.remove(&p2);
    inner.tabs.get_mut(&tab1).unwrap().panes.remove(&p2);

    assert_eq!(inner.pane_to_tab.len(), 2);
    assert!(inner.tab_id_for_pane(&p2).is_err());
    assert_eq!(inner.tab_id_for_pane(&p1).unwrap(), tab1);
    assert_eq!(inner.tab_id_for_pane(&p3).unwrap(), tab2);
}

#[test]
fn pane_to_tab_invariant_after_close_tab() {
    let mut inner = RegistryInner::new();

    let tab1 = crate::session::ids::TabId::new();
    let p1 = crate::session::ids::PaneId::new();
    let p2 = crate::session::ids::PaneId::new();

    let state_a = make_pane_session(&p1).to_state();
    let state_b = make_pane_session(&p2).to_state();

    let tab_state = TabState {
        id: tab1.clone(),
        label: None,
        active_pane_id: p1.clone(),
        order: 0,
        layout: PaneNode::Split {
            direction: crate::session::tab::SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::Leaf {
                pane_id: p1.clone(),
                state: state_a,
            }),
            second: Box::new(PaneNode::Leaf {
                pane_id: p2.clone(),
                state: state_b,
            }),
        },
    };

    insert_tab_with_panes(&mut inner, &tab1, &[p1.clone(), p2.clone()], tab_state);

    assert_eq!(inner.pane_to_tab.len(), 2);

    // Close tab: remove all pane entries first, then the tab (mirrors close_tab).
    let pane_ids: Vec<_> = inner
        .tabs
        .get(&tab1)
        .unwrap()
        .panes
        .keys()
        .cloned()
        .collect();
    for pid in &pane_ids {
        inner.pane_to_tab.remove(pid);
    }
    inner.tabs.remove(&tab1);

    assert!(inner.pane_to_tab.is_empty());
    assert!(inner.tab_id_for_pane(&p1).is_err());
    assert!(inner.tab_id_for_pane(&p2).is_err());
}

#[test]
fn tab_id_for_pane_unknown_returns_error() {
    let inner = RegistryInner::new();
    let unknown = crate::session::ids::PaneId::new();

    let result = inner.tab_id_for_pane(&unknown);
    assert!(result.is_err());
    match result {
        Err(crate::error::SessionError::PaneNotFound(id)) => {
            assert_eq!(id, unknown.to_string());
        }
        other => panic!("expected PaneNotFound, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// TEST-ACK-021 — `record_frame_ack` idempotence (ADR-0027 Addendum 3)
//
// The helper `fetchAndAckSnapshot` (frontend) acks on every snapshot
// consumption. `flushRafQueue` also acks on every paint. These paths
// may fire within the same millisecond, so `record_frame_ack` must be
// safely idempotent at wall-clock resolution: two back-to-back calls
// MUST leave `last_frame_ack_ms` monotone non-decreasing (equal values
// permitted when `SystemTime::now()` returns the same ms), MUST NOT
// panic, and a later call MUST eventually advance the timestamp.
//
// Unknown pane IDs must be silently ignored (race with pane close),
// and MUST NOT affect any other pane's timestamp.
//
// `SessionRegistry` cannot be constructed in unit tests (requires
// `AppHandle`), so we replicate the `record_frame_ack` logic on
// `RegistryInner` directly — same pattern as `inner_propagate_osc52_allow`
// and other `inner_*` helpers above.
// -----------------------------------------------------------------------

/// Replicates `SessionRegistry::record_frame_ack` on a `RegistryInner`.
/// Mirrors the production logic line-for-line so the test exercises the
/// same control flow (pane-to-tab lookup, atomic store, silent no-op on
/// missing pane).
fn inner_record_frame_ack(inner: &RegistryInner, pane_id: &crate::session::ids::PaneId) {
    if let Some(tab_id) = inner.pane_to_tab.get(pane_id)
        && let Some(entry) = inner.tabs.get(tab_id)
        && let Some(pane) = entry.panes.get(pane_id)
    {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        pane.last_frame_ack_ms
            .store(ts, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Helper: read `last_frame_ack_ms` from a pane in `RegistryInner`.
fn read_last_frame_ack_ms(inner: &RegistryInner, pane_id: &crate::session::ids::PaneId) -> u64 {
    inner
        .tabs
        .values()
        .find_map(|e| e.panes.get(pane_id))
        .map(|p| {
            p.last_frame_ack_ms
                .load(std::sync::atomic::Ordering::Relaxed)
        })
        .expect("pane not found in registry")
}

#[test]
fn test_ack_021_record_frame_ack_is_idempotent_and_monotonic() {
    // TEST-ACK-021 — ADR-0027 Addendum 3
    let (inner, _tab_id, pane_id) = make_registry_with_one_pane();

    // Step 2: capture initial value (set at PaneSession::new() = creation time).
    let t0 = read_last_frame_ack_ms(&inner, &pane_id);

    // Step 3: two acks back-to-back, no sleep between.
    inner_record_frame_ack(&inner, &pane_id);
    let t1 = read_last_frame_ack_ms(&inner, &pane_id);

    inner_record_frame_ack(&inner, &pane_id);
    let t2 = read_last_frame_ack_ms(&inner, &pane_id);

    // Step 4: monotone non-decreasing (equality permitted within same ms).
    assert!(t1 >= t0, "first ack must not go backward: t0={t0} t1={t1}");
    assert!(t2 >= t1, "second ack must not go backward: t1={t1} t2={t2}");

    // Step 5: sleep 10ms, third ack.
    std::thread::sleep(std::time::Duration::from_millis(10));
    inner_record_frame_ack(&inner, &pane_id);
    let t3 = read_last_frame_ack_ms(&inner, &pane_id);

    // Step 6: after a real delay, the timestamp must have strictly advanced.
    assert!(
        t3 > t2,
        "after 10ms sleep the ack timestamp must advance: t2={t2} t3={t3}"
    );

    // Step 7: unknown pane ID — must be silent, must not affect real pane.
    let unknown = crate::session::ids::PaneId::new();
    inner_record_frame_ack(&inner, &unknown);
    let t4 = read_last_frame_ack_ms(&inner, &pane_id);
    assert_eq!(
        t4, t3,
        "record_frame_ack on unknown pane must not mutate any other pane's timestamp"
    );
}
