// SPDX-License-Identifier: MPL-2.0

//! Integration tests — IPC command handler input validation and error paths.
//!
//! Tests coverage:
//!
//! - `session_cmds`: invalid `TabId`/`PaneId` inputs propagate to typed
//!   `TauTermError` values with the correct `code` (SEC-IPC-002).
//! - `preferences_cmds`: `apply_patch` with valid/invalid inputs, `save_theme`
//!   / `delete_theme` round-trip on a temporary store.
//! - `ssh_cmds` / `ssh_prompt_cmds`: error-path conversions from `SessionError`
//!   and `SshError` to `TauTermError` (no real SSH server required).
//!
//! ## Why `SessionRegistry` is not instantiated here
//!
//! `SessionRegistry::new()` requires an `AppHandle<Wry>`, which in turn requires
//! a live display (X11/Wayland). The session command handlers are thin wrappers
//! over `SessionRegistry` methods; the registry's error propagation is validated
//! via the `From<SessionError> for TauTermError` conversion, which is exercised
//! directly without a real registry instance.
//!
//! The full behavioral integration (topology mutations, pane lifecycle) is covered
//! by `session_registry_topology.rs` (data types) and by the E2E test suite
//! (real PTY + Tauri app).

use tau_term_lib::{
    error::{PreferencesError, SessionError, SshError, TauTermError},
    preferences::{
        schema::{
            AppearancePatch, BellType, CursorStyle, Language, PreferencesPatch, TerminalPatch,
            UserTheme,
        },
        store::PreferencesStore,
    },
    session::ids::{PaneId, TabId},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a temporary `PreferencesStore` backed by a temp directory.
/// Returns the store and the `TempDir` guard (dropped at end of test).
fn temp_store() -> (
    std::sync::Arc<parking_lot::RwLock<PreferencesStore>>,
    tempfile::TempDir,
) {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let prefs_dir = tmp.path().join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create prefs dir");
    let path = prefs_dir.join("preferences.toml");

    // Use load_or_default with XDG override so the store writes to a temp path.
    // The XDG override is safe because nextest runs each integration test in its
    // own process.
    let orig = std::env::var_os("XDG_CONFIG_HOME");
    // SAFETY: nextest process-per-test isolation guarantees no concurrent env readers.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
    let store = PreferencesStore::load_or_default();
    unsafe {
        match orig {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
    let _ = path; // kept for documentation
    (store, tmp)
}

fn make_user_theme(name: &str) -> UserTheme {
    UserTheme {
        name: name.to_string(),
        palette: std::array::from_fn(|i| format!("#{:06x}", i * 0x111111)),
        foreground: "#cdd6f4".to_string(),
        background: "#1e1e2e".to_string(),
        cursor_color: "#f5e0dc".to_string(),
        selection_bg: "#45475a".to_string(),
        line_height: None,
    }
}

// ---------------------------------------------------------------------------
// ICH-ERR-001 — SessionError variants → TauTermError codes
// ---------------------------------------------------------------------------

/// ICH-ERR-001a: TabNotFound → "INVALID_TAB_ID"
#[test]
fn ich_err_001a_tab_not_found_maps_to_invalid_tab_id() {
    let fake_id = TabId::new().to_string();
    let err = SessionError::TabNotFound(fake_id.clone());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "INVALID_TAB_ID");
    assert!(
        tt.detail.as_deref().unwrap_or("").contains(&fake_id),
        "detail must contain the offending TabId: {tt:?}"
    );
}

/// ICH-ERR-001b: PaneNotFound → "INVALID_PANE_ID"
#[test]
fn ich_err_001b_pane_not_found_maps_to_invalid_pane_id() {
    let fake_id = PaneId::new().to_string();
    let err = SessionError::PaneNotFound(fake_id.clone());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "INVALID_PANE_ID");
    assert!(tt.detail.as_deref().unwrap_or("").contains(&fake_id));
}

/// ICH-ERR-001c: PaneNotRunning → "PANE_NOT_RUNNING"
#[test]
fn ich_err_001c_pane_not_running_maps_correctly() {
    let fake_id = PaneId::new().to_string();
    let err = SessionError::PaneNotRunning(fake_id);
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "PANE_NOT_RUNNING");
}

/// ICH-ERR-001d: InvalidShellPath → "INVALID_SHELL_PATH"
#[test]
fn ich_err_001d_invalid_shell_path_maps_correctly() {
    let err = SessionError::InvalidShellPath("/not/a/shell".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "INVALID_SHELL_PATH");
}

/// ICH-ERR-001e: PtySpawn failure → "PTY_SPAWN_FAILED"
#[test]
fn ich_err_001e_pty_spawn_failure_maps_correctly() {
    let err = SessionError::PtySpawn("fork failed".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "PTY_SPAWN_FAILED");
}

// ---------------------------------------------------------------------------
// ICH-ERR-002 — SshError variants → TauTermError codes
// ---------------------------------------------------------------------------

/// ICH-ERR-002a: SshError::Connection → "SSH_CONNECTION_FAILED"
#[test]
fn ich_err_002a_ssh_connection_failed_maps_correctly() {
    let err = SshError::Connection("refused".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "SSH_CONNECTION_FAILED");
}

/// ICH-ERR-002b: SshError::Auth → "SSH_AUTH_FAILED"
#[test]
fn ich_err_002b_ssh_auth_failed_maps_correctly() {
    let err = SshError::Auth("wrong password".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "SSH_AUTH_FAILED");
}

/// ICH-ERR-002c: SshError::HostKey → "SSH_HOST_KEY_REJECTED"
#[test]
fn ich_err_002c_ssh_host_key_rejected_maps_correctly() {
    let err = SshError::HostKey("mismatch".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "SSH_HOST_KEY_REJECTED");
}

/// ICH-ERR-002d: SshError::PaneNotFound → "INVALID_PANE_ID"
#[test]
fn ich_err_002d_ssh_pane_not_found_maps_to_invalid_pane_id() {
    let id = PaneId::new().to_string();
    let err = SshError::PaneNotFound(id.clone());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "INVALID_PANE_ID");
    assert!(tt.detail.as_deref().unwrap_or("").contains(&id));
}

/// ICH-ERR-002e: SshError::Transport → "SSH_TRANSPORT_ERROR"
#[test]
fn ich_err_002e_ssh_transport_error_maps_correctly() {
    let err = SshError::Transport("keepalive timeout".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "SSH_TRANSPORT_ERROR");
}

// ---------------------------------------------------------------------------
// ICH-ERR-003 — PreferencesError variants → TauTermError codes
// ---------------------------------------------------------------------------

/// ICH-ERR-003a: PreferencesError::Io → "PREF_IO_ERROR"
#[test]
fn ich_err_003a_prefs_io_error_maps_correctly() {
    let err = PreferencesError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "no write access",
    ));
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "PREF_IO_ERROR");
}

/// ICH-ERR-003b: PreferencesError::Parse → "PREF_PARSE_ERROR"
#[test]
fn ich_err_003b_prefs_parse_error_maps_correctly() {
    let err = PreferencesError::Parse("unexpected token".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "PREF_PARSE_ERROR");
}

/// ICH-ERR-003c: PreferencesError::Validation → "PREF_INVALID_VALUE"
#[test]
fn ich_err_003c_prefs_validation_error_maps_correctly() {
    let err = PreferencesError::Validation("opacity out of range".to_string());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "PREF_INVALID_VALUE");
}

// ---------------------------------------------------------------------------
// ICH-PREF-001 — preferences_cmds: get / patch / theme CRUD on a real store
//
// These tests exercise the logic that `preferences_cmds.rs` delegates to
// `PreferencesStore`. The command handlers are thin wrappers — validating the
// store ensures end-to-end correctness for the IPC layer.
// ---------------------------------------------------------------------------

/// ICH-PREF-001a: get_preferences returns defaults on a fresh store.
#[test]
fn ich_pref_001a_get_preferences_returns_defaults() {
    let (store, _tmp) = temp_store();
    let prefs = store.read().get();
    assert_eq!(prefs.appearance.font_family, "monospace");
    assert_eq!(prefs.terminal.scrollback_lines, 10_000);
}

/// ICH-PREF-001b: update_preferences — patch font_family only; other fields unchanged.
#[test]
fn ich_pref_001b_patch_font_family_leaves_other_fields_intact() {
    let (store, _tmp) = temp_store();
    let original_scrollback = store.read().get().terminal.scrollback_lines;

    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            font_family: Some("JetBrains Mono".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = store
        .read()
        .apply_patch(patch)
        .expect("apply_patch must succeed");

    assert_eq!(updated.appearance.font_family, "JetBrains Mono");
    assert_eq!(
        updated.terminal.scrollback_lines, original_scrollback,
        "scrollback_lines must not change when only font_family is patched"
    );
}

/// ICH-PREF-001c: patch language (Language enum — FS-I18N-006).
#[test]
fn ich_pref_001c_patch_language_sets_language_field() {
    let (store, _tmp) = temp_store();
    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            language: Some(Language::Fr),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = store.read().apply_patch(patch).expect("apply_patch");
    assert_eq!(updated.appearance.language, Language::Fr);
}

/// ICH-PREF-001d: patch terminal prefs — scrollback_lines.
#[test]
fn ich_pref_001d_patch_terminal_scrollback() {
    let (store, _tmp) = temp_store();
    let patch = PreferencesPatch {
        terminal: Some(TerminalPatch {
            scrollback_lines: Some(5_000),
            allow_osc52_write: Some(true),
            word_delimiters: Some(" ".to_string()),
            bell_type: Some(BellType::None),
            confirm_multiline_paste: Some(false),
        }),
        ..Default::default()
    };
    let updated = store.read().apply_patch(patch).expect("apply_patch");
    assert_eq!(updated.terminal.scrollback_lines, 5_000);
    assert!(updated.terminal.allow_osc52_write);
}

/// ICH-PREF-001e: patch cursor_style to Underline.
#[test]
fn ich_pref_001e_patch_cursor_style() {
    let (store, _tmp) = temp_store();
    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            cursor_style: Some(CursorStyle::Underline),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = store.read().apply_patch(patch).expect("apply_patch");
    assert_eq!(updated.appearance.cursor_style, CursorStyle::Underline);
}

// ---------------------------------------------------------------------------
// ICH-THEME-001 — save_theme / delete_theme
// ---------------------------------------------------------------------------

/// ICH-THEME-001a: save a theme and retrieve it via get_themes.
#[test]
fn ich_theme_001a_save_and_retrieve_theme() {
    let (store, _tmp) = temp_store();
    let theme = make_user_theme("umbra-custom");
    store.read().save_theme(theme.clone()).expect("save_theme");

    let themes = store.read().get_themes();
    assert!(
        themes.iter().any(|t| t.name == "umbra-custom"),
        "saved theme must appear in get_themes: {themes:?}"
    );
}

/// ICH-THEME-001b: saving a theme with the same name updates the existing entry.
#[test]
fn ich_theme_001b_save_theme_updates_existing() {
    let (store, _tmp) = temp_store();
    let theme1 = make_user_theme("my-theme");
    store.read().save_theme(theme1).expect("save initial");

    let mut theme2 = make_user_theme("my-theme");
    theme2.foreground = "#ffffff".to_string();
    store.read().save_theme(theme2).expect("save update");

    let themes = store.read().get_themes();
    let found: Vec<_> = themes.iter().filter(|t| t.name == "my-theme").collect();
    assert_eq!(found.len(), 1, "only one entry with that name must exist");
    assert_eq!(found[0].foreground, "#ffffff");
}

/// ICH-THEME-001c: delete_theme removes the named theme.
#[test]
fn ich_theme_001c_delete_theme_removes_entry() {
    let (store, _tmp) = temp_store();
    store
        .read()
        .save_theme(make_user_theme("to-delete"))
        .expect("save");
    store.read().delete_theme("to-delete").expect("delete");

    let themes = store.read().get_themes();
    assert!(
        !themes.iter().any(|t| t.name == "to-delete"),
        "deleted theme must not appear in get_themes"
    );
}

/// ICH-THEME-001d: delete_theme on non-existent name is a no-op (returns Ok).
#[test]
fn ich_theme_001d_delete_nonexistent_theme_is_noop() {
    let (store, _tmp) = temp_store();
    // Must not error out on a non-existent theme.
    let result = store.read().delete_theme("no-such-theme");
    assert!(
        result.is_ok(),
        "deleting a non-existent theme must return Ok: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// ICH-SERIAL-001 — TauTermError is fully serializable (SEC-IPC-002)
// ---------------------------------------------------------------------------

/// ICH-SERIAL-001: every `TauTermError` is serializable to JSON with
/// a `code` field discriminable by the frontend.
#[test]
fn ich_serial_001_tauterm_error_serializable_with_code() {
    let err = TauTermError::with_detail("INVALID_TAB_ID", "Tab not found.", "abc-def");
    let json = serde_json::to_string(&err).expect("serialize");
    let val: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(val["code"], "INVALID_TAB_ID");
    assert_eq!(val["detail"], "abc-def");
    // message is present and non-empty
    assert!(!val["message"].as_str().unwrap_or("").is_empty());
}

/// ICH-SERIAL-001b: TauTermError without detail omits `detail` in JSON.
#[test]
fn ich_serial_001b_tauterm_error_without_detail_omits_detail_field() {
    let err = TauTermError::new("SOME_ERROR", "Something went wrong.");
    let json = serde_json::to_string(&err).expect("serialize");
    let val: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(
        val.get("detail").is_none() || val["detail"].is_null(),
        "detail field must be absent when not set: {val}"
    );
}

// ---------------------------------------------------------------------------
// ICH-INPUT-001 — ID validation: malformed IDs produce typed errors
// ---------------------------------------------------------------------------

/// ICH-INPUT-001a: empty string as TabId produces TabNotFound with the empty
/// string in the error detail — the handler must NOT panic.
#[test]
fn ich_input_001a_empty_tab_id_produces_typed_error() {
    let err = SessionError::TabNotFound(String::new());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "INVALID_TAB_ID");
    // detail contains the empty string (not a crash)
    assert!(tt.detail.is_some());
}

/// ICH-INPUT-001b: unicode string as PaneId produces PaneNotFound correctly.
#[test]
fn ich_input_001b_unicode_pane_id_produces_typed_error() {
    let unicode_id = "éàü-🐧-漢字".to_string();
    let err = SessionError::PaneNotFound(unicode_id.clone());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "INVALID_PANE_ID");
    assert!(tt.detail.as_deref().unwrap_or("").contains(&unicode_id));
}

/// ICH-INPUT-001c: very long ID string does not truncate or panic.
#[test]
fn ich_input_001c_very_long_id_does_not_panic() {
    let long_id = "x".repeat(10_000);
    let err = SessionError::TabNotFound(long_id.clone());
    let tt: TauTermError = err.into();
    assert_eq!(tt.code, "INVALID_TAB_ID");
    // detail must contain the full ID (no silent truncation)
    assert_eq!(tt.detail.as_deref().unwrap_or(""), &long_id);
}
