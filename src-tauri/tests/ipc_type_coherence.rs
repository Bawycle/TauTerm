// SPDX-License-Identifier: MPL-2.0

//! Integration tests — IPC type serialization coherence (TEST-IPC-001–004).
//!
//! Verifies that every backend→frontend event type is correctly serializable,
//! that `TauTermError` is serializable and discriminable by `code`, and that
//! `SessionError`/`SshError`/`PreferencesError` convert to typed `TauTermError`
//! values (not panic, not bare strings).
//!
//! SEC-IPC-002: Invalid PaneId/TabId in handlers must produce a typed error
//! with a known `code`. Since Tauri command handlers cannot be invoked without
//! a live AppHandle, we test the `From<SessionError> for TauTermError` conversion
//! which is the mechanism handlers rely on.

use tau_term_lib::{
    error::{PreferencesError, SessionError, SshError, TauTermError},
    events::types::{
        CellAttrsDto, CellUpdate, ColorDto, CredentialPromptEvent, CursorState, HostKeyPromptEvent,
        ModeStateChangedEvent, NotificationChangedEvent, PaneNotificationDto, ScreenUpdateEvent,
        ScrollPositionChangedEvent, SessionChangeType, SessionStateChangedEvent,
        SshStateChangedEvent,
    },
    session::{
        ids::{ConnectionId, PaneId, TabId},
        lifecycle::PaneLifecycleState,
        pane::PaneState,
        tab::{PaneNode, SplitDirection, TabState},
    },
    ssh::SshLifecycleState,
    vt::modes::{MouseEncoding, MouseReportingMode},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_pane_id() -> PaneId {
    PaneId::new()
}

fn make_tab_id() -> TabId {
    TabId::new()
}

fn make_pane_state(id: PaneId) -> PaneState {
    PaneState {
        pane_id: id,
        lifecycle: PaneLifecycleState::Running,
        title: None,
        cwd: None,
        label: None,
        ssh_state: None,
        scroll_offset: 0,
    }
}

fn make_leaf_tab(order: u32) -> TabState {
    let tab_id = make_tab_id();
    let pane_id = make_pane_id();
    let state = make_pane_state(pane_id.clone());
    let layout = PaneNode::Leaf {
        pane_id: pane_id.clone(),
        state,
    };
    TabState {
        id: tab_id,
        label: None,
        active_pane_id: pane_id,
        order,
        layout,
    }
}

fn make_cursor_state() -> CursorState {
    CursorState {
        row: 0,
        col: 0,
        visible: true,
        shape: 0,
        blink: false,
    }
}

fn make_cell_attrs() -> CellAttrsDto {
    CellAttrsDto {
        fg: None,
        bg: None,
        bold: false,
        dim: false,
        italic: false,
        underline: 0,
        blink: false,
        inverse: false,
        hidden: false,
        strikethrough: false,
        underline_color: None,
    }
}

// ---------------------------------------------------------------------------
// TEST-IPC-001 — All event types serialize without panic
// ---------------------------------------------------------------------------

#[test]
fn ipc_session_state_changed_tab_created_serializes() {
    let event = SessionStateChangedEvent {
        change_type: SessionChangeType::TabCreated,
        tab: Some(make_leaf_tab(0)),
        active_tab_id: None,
        closed_tab_id: None,
    };
    let json = serde_json::to_string(&event).expect("SessionStateChangedEvent must serialize");
    assert!(
        json.contains("tabCreated") || json.contains("tab-created"),
        "change_type tag missing: {json}"
    );
}

#[test]
fn ipc_session_state_changed_tab_closed_serializes() {
    let tab_id = make_tab_id();
    let event = SessionStateChangedEvent {
        change_type: SessionChangeType::TabClosed,
        tab: None,
        active_tab_id: Some(tab_id.0.clone()),
        closed_tab_id: Some(tab_id.clone()),
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        json.contains("tab-closed") || json.contains("tabClosed"),
        "got: {json}"
    );
}

#[test]
fn ipc_session_state_changed_all_change_types_serialize() {
    let variants = [
        SessionChangeType::TabCreated,
        SessionChangeType::TabClosed,
        SessionChangeType::TabReordered,
        SessionChangeType::ActiveTabChanged,
        SessionChangeType::ActivePaneChanged,
        SessionChangeType::PaneMetadataChanged,
    ];
    for variant in variants {
        let event = SessionStateChangedEvent {
            change_type: variant,
            tab: None,
            active_tab_id: None,
            closed_tab_id: None,
        };
        // Must serialize without panic.
        let json = serde_json::to_string(&event).expect("serialize SessionChangeType variant");
        assert!(!json.is_empty());
    }
}

#[test]
fn ipc_ssh_state_changed_connecting_serializes() {
    let event = SshStateChangedEvent {
        pane_id: make_pane_id(),
        state: SshLifecycleState::Connecting,
        reason: None,
    };
    let json = serde_json::to_string(&event).expect("SshStateChangedEvent must serialize");
    assert!(
        json.contains("paneId") || json.contains("pane_id"),
        "paneId missing: {json}"
    );
}

#[test]
fn ipc_ssh_state_changed_disconnected_with_reason_serializes() {
    let event = SshStateChangedEvent {
        pane_id: make_pane_id(),
        state: SshLifecycleState::Disconnected {
            reason: Some("connection reset by peer".to_string()),
        },
        reason: Some("connection reset by peer".to_string()),
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        json.contains("reason") || json.contains("connection reset"),
        "reason missing: {json}"
    );
}

#[test]
fn ipc_screen_update_event_serializes() {
    let pane_id = make_pane_id();
    let event = ScreenUpdateEvent {
        pane_id: pane_id.clone(),
        cells: vec![CellUpdate {
            row: 0,
            col: 0,
            content: "A".to_string(),
            width: 1,
            attrs: make_cell_attrs(),
            hyperlink: None,
        }],
        cursor: make_cursor_state(),
        scrollback_lines: 0,
        is_full_redraw: false,
        cols: 80,
        rows: 24,
        scroll_offset: 0,
    };
    let json = serde_json::to_string(&event).expect("ScreenUpdateEvent must serialize");
    assert!(json.contains("cells"));
    assert!(json.contains("cursor"));
}

#[test]
fn ipc_screen_update_empty_cells_serializes() {
    let event = ScreenUpdateEvent {
        pane_id: make_pane_id(),
        cells: vec![],
        cursor: make_cursor_state(),
        scrollback_lines: 100,
        is_full_redraw: false,
        cols: 80,
        rows: 24,
        scroll_offset: 0,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(json.contains("\"cells\":[]"));
}

#[test]
fn ipc_mode_state_changed_event_serializes() {
    let event = ModeStateChangedEvent {
        pane_id: make_pane_id(),
        decckm: false,
        deckpam: false,
        mouse_reporting: MouseReportingMode::None,
        mouse_encoding: MouseEncoding::X10,
        focus_events: false,
        bracketed_paste: true,
    };
    let json = serde_json::to_string(&event).expect("ModeStateChangedEvent must serialize");
    assert!(
        json.contains("\"mouseReporting\":\"none\""),
        "mouseReporting must serialize to \"none\": {json}"
    );
    assert!(
        json.contains("\"mouseEncoding\":\"x10\""),
        "mouseEncoding must serialize to \"x10\": {json}"
    );
    assert!(
        json.contains("bracketed") || json.contains("Paste"),
        "bracketed_paste field missing: {json}"
    );
}

#[test]
fn ipc_scroll_position_changed_event_serializes() {
    let event = ScrollPositionChangedEvent {
        pane_id: make_pane_id(),
        offset: 42,
        scrollback_lines: 1000,
    };
    let json = serde_json::to_string(&event).expect("ScrollPositionChangedEvent must serialize");
    assert!(json.contains("42"));
    assert!(json.contains("1000"));
}

#[test]
fn ipc_credential_prompt_event_serializes() {
    let event = CredentialPromptEvent {
        pane_id: make_pane_id(),
        host: "example.com".to_string(),
        username: "alice".to_string(),
        prompt: Some("Password:".to_string()),
        failed: false,
        is_keychain_available: false,
    };
    let json = serde_json::to_string(&event).expect("CredentialPromptEvent must serialize");
    assert!(json.contains("example.com"));
    assert!(json.contains("alice"));
}

#[test]
fn ipc_credential_prompt_event_without_prompt_serializes() {
    let event = CredentialPromptEvent {
        pane_id: make_pane_id(),
        host: "host.local".to_string(),
        username: "bob".to_string(),
        prompt: None,
        failed: false,
        is_keychain_available: false,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    // skip_serializing_if = None → "prompt" key must be absent.
    assert!(
        !json.contains("\"prompt\""),
        "absent prompt must be skipped: {json}"
    );
}

#[test]
fn ipc_host_key_prompt_event_serializes() {
    let event = HostKeyPromptEvent {
        pane_id: make_pane_id(),
        connection_id: ConnectionId::new(),
        host: "example.com".to_string(),
        key_type: "ed25519".to_string(),
        fingerprint: "SHA256:abc123".to_string(),
        is_changed: false,
    };
    let json = serde_json::to_string(&event).expect("HostKeyPromptEvent must serialize");
    assert!(json.contains("ed25519"));
    assert!(json.contains("SHA256:abc123"));
}

#[test]
fn ipc_notification_changed_bell_serializes() {
    let event = NotificationChangedEvent {
        tab_id: make_tab_id(),
        pane_id: make_pane_id(),
        notification: Some(PaneNotificationDto::Bell),
    };
    let json = serde_json::to_string(&event).expect("NotificationChangedEvent Bell must serialize");
    assert!(
        json.contains("bell") || json.contains("Bell"),
        "bell variant missing: {json}"
    );
}

#[test]
fn ipc_notification_changed_process_exited_serializes() {
    let event = NotificationChangedEvent {
        tab_id: make_tab_id(),
        pane_id: make_pane_id(),
        notification: Some(PaneNotificationDto::ProcessExited {
            exit_code: Some(0),
            signal_name: None,
        }),
    };
    let json = serde_json::to_string(&event).expect("serialize ProcessExited");
    assert!(
        json.contains("processExited") || json.contains("process") || json.contains("exit"),
        "got: {json}"
    );
}

#[test]
fn ipc_notification_changed_cleared_serializes() {
    // `notification: None` represents a cleared notification (e.g. user dismissed bell).
    // `NotificationChangedEvent.notification` does NOT carry `skip_serializing_if`,
    // so `None` serializes as `null` — the frontend uses the `null` value to clear state.
    let event = NotificationChangedEvent {
        tab_id: make_tab_id(),
        pane_id: make_pane_id(),
        notification: None,
    };
    let json = serde_json::to_string(&event).expect("serialize None notification");
    // Verify round-trip: `null` must deserialize back to `None`.
    let restored: NotificationChangedEvent =
        serde_json::from_str(&json).expect("deserialize cleared notification");
    assert!(
        restored.notification.is_none(),
        "Cleared notification must round-trip as None: {json}"
    );
    // The key must be present (serialized as null) to signal the clear to the frontend.
    assert!(
        json.contains("notification"),
        "notification key must be present: {json}"
    );
}

// ---------------------------------------------------------------------------
// TEST-IPC-002 — ColorDto variants all serialize with discriminant tag
// ---------------------------------------------------------------------------

#[test]
fn ipc_color_dto_default_serializes_with_type_tag() {
    let c = ColorDto::Default;
    let json = serde_json::to_string(&c).expect("serialize");
    assert!(
        json.contains("\"type\":\"default\"") || json.contains("default"),
        "got: {json}"
    );
}

#[test]
fn ipc_color_dto_ansi_serializes() {
    let c = ColorDto::Ansi { index: 2 };
    let json = serde_json::to_string(&c).expect("serialize");
    assert!(json.contains("\"index\":2"), "got: {json}");
}

#[test]
fn ipc_color_dto_rgb_serializes() {
    let c = ColorDto::Rgb {
        r: 255,
        g: 128,
        b: 0,
    };
    let json = serde_json::to_string(&c).expect("serialize");
    assert!(json.contains("255"));
    assert!(json.contains("128"));
}

#[test]
fn ipc_cell_attrs_with_colors_round_trips() {
    let attrs = CellAttrsDto {
        fg: Some(ColorDto::Rgb { r: 255, g: 0, b: 0 }),
        bg: Some(ColorDto::Ansi { index: 0 }),
        bold: true,
        dim: false,
        italic: true,
        underline: 1,
        blink: false,
        inverse: false,
        hidden: false,
        strikethrough: false,
        underline_color: None,
    };
    let json = serde_json::to_string(&attrs).expect("serialize CellAttrsDto");
    let restored: CellAttrsDto = serde_json::from_str(&json).expect("deserialize CellAttrsDto");
    assert!(restored.bold);
    assert!(restored.italic);
    assert_eq!(restored.underline, 1);
}

// ---------------------------------------------------------------------------
// TEST-IPC-003 — TauTermError is serializable with code/message/detail
// ---------------------------------------------------------------------------

#[test]
fn ipc_tau_term_error_serializes_with_code_and_message() {
    let err = TauTermError::new("PTY_SPAWN_FAILED", "Failed to start the shell.");
    let json = serde_json::to_string(&err).expect("TauTermError must serialize");
    assert!(json.contains("PTY_SPAWN_FAILED"), "code missing: {json}");
    assert!(json.contains("Failed to start"), "message missing: {json}");
}

#[test]
fn ipc_tau_term_error_with_detail_serializes() {
    let err = TauTermError::with_detail("INVALID_PANE_ID", "Pane not found.", "pane-xyz");
    let json = serde_json::to_string(&err).expect("serialize with detail");
    assert!(json.contains("INVALID_PANE_ID"));
    assert!(json.contains("pane-xyz"));
}

#[test]
fn ipc_tau_term_error_without_detail_omits_detail_key() {
    let err = TauTermError::new("SOME_ERROR", "Some message.");
    let json = serde_json::to_string(&err).expect("serialize");
    assert!(
        !json.contains("\"detail\""),
        "detail must be absent when None: {json}"
    );
}

#[test]
fn ipc_tau_term_error_round_trips_through_json() {
    let err = TauTermError::with_detail(
        "SSH_AUTH_FAILED",
        "Authentication failed.",
        "wrong password",
    );
    let json = serde_json::to_string(&err).expect("serialize");
    let restored: TauTermError = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.code, "SSH_AUTH_FAILED");
    assert_eq!(restored.detail, Some("wrong password".to_string()));
}

// ---------------------------------------------------------------------------
// SEC-IPC-002 — Invalid IDs → typed TauTermError (not panic)
// ---------------------------------------------------------------------------

#[test]
fn ipc_invalid_pane_id_session_error_converts_to_typed_tau_term_error() {
    let fake_id = "not-a-real-pane-id".to_string();
    let err: SessionError = SessionError::PaneNotFound(fake_id.clone());
    let tau_err = TauTermError::from(err);

    assert_eq!(
        tau_err.code, "INVALID_PANE_ID",
        "PaneNotFound must map to INVALID_PANE_ID code (SEC-IPC-002)"
    );
    assert_eq!(tau_err.detail, Some(fake_id));
}

#[test]
fn ipc_invalid_tab_id_session_error_converts_to_typed_tau_term_error() {
    let fake_id = "not-a-real-tab-id".to_string();
    let err = SessionError::TabNotFound(fake_id.clone());
    let tau_err = TauTermError::from(err);

    assert_eq!(
        tau_err.code, "INVALID_TAB_ID",
        "TabNotFound must map to INVALID_TAB_ID code (SEC-IPC-002)"
    );
    assert_eq!(tau_err.detail, Some(fake_id));
}

#[test]
fn ipc_session_error_pane_not_running_converts_correctly() {
    let err = SessionError::PaneNotRunning("pane-123".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "PANE_NOT_RUNNING");
}

#[test]
fn ipc_session_error_invalid_shell_path_converts_correctly() {
    let err = SessionError::InvalidShellPath("/not/valid".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "INVALID_SHELL_PATH");
}

#[test]
fn ipc_session_error_pty_spawn_converts_correctly() {
    let err = SessionError::PtySpawn("exec failed".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "PTY_SPAWN_FAILED");
}

#[test]
fn ipc_ssh_error_connection_converts_to_typed_error() {
    let err = SshError::Connection("timeout".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "SSH_CONNECTION_FAILED");
}

#[test]
fn ipc_ssh_error_auth_failed_converts_correctly() {
    let err = SshError::Auth("bad credentials".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "SSH_AUTH_FAILED");
}

#[test]
fn ipc_ssh_error_host_key_rejected_converts_correctly() {
    let err = SshError::HostKey("key mismatch".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "SSH_HOST_KEY_REJECTED");
}

#[test]
fn ipc_ssh_error_pane_not_found_converts_correctly() {
    let err = SshError::PaneNotFound("pane-abc".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "INVALID_PANE_ID");
}

#[test]
fn ipc_preferences_error_io_converts_correctly() {
    let io_err = std::io::Error::other("disk full");
    let err = PreferencesError::Io(io_err);
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "PREF_IO_ERROR");
}

#[test]
fn ipc_preferences_error_parse_converts_correctly() {
    let err = PreferencesError::Parse("unexpected token at line 5".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "PREF_PARSE_ERROR");
}

#[test]
fn ipc_preferences_error_validation_converts_correctly() {
    let err = PreferencesError::Validation("fontSize out of range".to_string());
    let tau_err = TauTermError::from(err);
    assert_eq!(tau_err.code, "PREF_INVALID_VALUE");
}

// ---------------------------------------------------------------------------
// TEST-IPC-004 — All event payloads include required pane_id / tab_id fields
// ---------------------------------------------------------------------------

#[test]
fn ipc_screen_update_payload_includes_pane_id() {
    let pane_id = make_pane_id();
    let event = ScreenUpdateEvent {
        pane_id: pane_id.clone(),
        cells: vec![],
        cursor: make_cursor_state(),
        scrollback_lines: 0,
        is_full_redraw: false,
        cols: 80,
        rows: 24,
        scroll_offset: 0,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    // Verify the pane ID value is present in the serialized payload.
    assert!(
        json.contains(pane_id.as_str()),
        "pane_id must be present in ScreenUpdateEvent payload: {json}"
    );
}

#[test]
fn ipc_notification_event_payload_includes_both_tab_id_and_pane_id() {
    let tab_id = make_tab_id();
    let pane_id = make_pane_id();
    let event = NotificationChangedEvent {
        tab_id: tab_id.clone(),
        pane_id: pane_id.clone(),
        notification: Some(PaneNotificationDto::Bell),
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        json.contains(tab_id.as_str()),
        "tab_id must be present in NotificationChangedEvent: {json}"
    );
    assert!(
        json.contains(pane_id.as_str()),
        "pane_id must be present in NotificationChangedEvent: {json}"
    );
}

#[test]
fn ipc_scroll_position_event_includes_pane_id() {
    let pane_id = make_pane_id();
    let event = ScrollPositionChangedEvent {
        pane_id: pane_id.clone(),
        offset: 10,
        scrollback_lines: 500,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        json.contains(pane_id.as_str()),
        "pane_id must be present in ScrollPositionChangedEvent: {json}"
    );
}

#[test]
fn ipc_ssh_state_event_includes_pane_id() {
    let pane_id = make_pane_id();
    let event = SshStateChangedEvent {
        pane_id: pane_id.clone(),
        state: SshLifecycleState::Connected,
        reason: None,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        json.contains(pane_id.as_str()),
        "pane_id must be present in SshStateChangedEvent: {json}"
    );
}

// ---------------------------------------------------------------------------
// Additional: SplitDirection serialization
// ---------------------------------------------------------------------------

#[test]
fn ipc_split_direction_serializes_as_camel_case() {
    let h = SplitDirection::Horizontal;
    let v = SplitDirection::Vertical;
    let json_h = serde_json::to_string(&h).expect("serialize Horizontal");
    let json_v = serde_json::to_string(&v).expect("serialize Vertical");
    assert!(json_h.contains("horizontal"), "got: {json_h}");
    assert!(json_v.contains("vertical"), "got: {json_v}");
}

// ---------------------------------------------------------------------------
// P-IPC1 — CellAttrsDto skip_serializing_if for default boolean/u8 fields
// ---------------------------------------------------------------------------

/// P-IPC1: CellAttrsDto with all-default attribute values must not emit any
/// of the boolean or underline keys in JSON — they are skipped by
/// `skip_serializing_if = "is_false"` / `skip_serializing_if = "is_zero"`.
#[test]
fn cell_attrs_dto_default_booleans_absent_from_json() {
    let attrs = CellAttrsDto {
        fg: None,
        bg: None,
        bold: false,
        dim: false,
        italic: false,
        underline: 0,
        blink: false,
        inverse: false,
        hidden: false,
        strikethrough: false,
        underline_color: None,
    };
    let json = serde_json::to_string(&attrs).expect("serialize CellAttrsDto");
    for key in &[
        "\"bold\"",
        "\"dim\"",
        "\"italic\"",
        "\"underline\"",
        "\"blink\"",
        "\"inverse\"",
        "\"hidden\"",
        "\"strikethrough\"",
    ] {
        assert!(
            !json.contains(key),
            "default field {key} must be absent from JSON; got: {json}"
        );
    }
}

/// P-IPC1: Non-default values must be present in the serialized JSON.
#[test]
fn cell_attrs_dto_nondefault_values_present_in_json() {
    let attrs = CellAttrsDto {
        fg: None,
        bg: None,
        bold: true,
        dim: false,
        italic: true,
        underline: 2,
        blink: false,
        inverse: false,
        hidden: false,
        strikethrough: false,
        underline_color: None,
    };
    let json = serde_json::to_string(&attrs).expect("serialize CellAttrsDto");
    assert!(
        json.contains("\"bold\":true"),
        "bold:true must be present; got: {json}"
    );
    assert!(
        json.contains("\"italic\":true"),
        "italic:true must be present; got: {json}"
    );
    assert!(
        json.contains("\"underline\":2"),
        "underline:2 must be present; got: {json}"
    );
}

/// P-IPC1: Deserializing `"{}"` (all fields absent) must yield all-default values.
/// `#[serde(default)]` is required for this round-trip to work.
#[test]
fn cell_attrs_dto_roundtrip_from_minimal_json() {
    let attrs: CellAttrsDto =
        serde_json::from_str("{}").expect("deserialize minimal CellAttrsDto");
    assert!(!attrs.bold, "bold must default to false");
    assert!(!attrs.dim, "dim must default to false");
    assert!(!attrs.italic, "italic must default to false");
    assert_eq!(attrs.underline, 0, "underline must default to 0");
    assert!(!attrs.blink, "blink must default to false");
    assert!(!attrs.inverse, "inverse must default to false");
    assert!(!attrs.hidden, "hidden must default to false");
    assert!(!attrs.strikethrough, "strikethrough must default to false");
}
