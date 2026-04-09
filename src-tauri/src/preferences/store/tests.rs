// SPDX-License-Identifier: MPL-2.0

use super::{
    MAX_CONNECTIONS, PreferencesStore,
    schema_convert::{camel_to_snake, snake_to_camel},
};
use crate::preferences::schema::{AppearancePatch, Language, PreferencesPatch, UserTheme};

// -----------------------------------------------------------------------
// apply_patch — AppearancePatch merge semantics
// -----------------------------------------------------------------------

/// apply_patch with only `language` set must not modify `font_size`.
#[test]
fn apply_patch_language_only_does_not_touch_font_size() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    let default_font_size = store.read().get().appearance.font_size;

    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            language: Some(Language::Fr),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = store.read().apply_patch(patch).expect("apply_patch");

    assert_eq!(
        updated.appearance.language,
        Language::Fr,
        "language must be updated"
    );
    assert_eq!(
        updated.appearance.font_size, default_font_size,
        "font_size must be unchanged"
    );
}

/// apply_patch with only `font_size` set must not modify `language`.
#[test]
fn apply_patch_font_size_only_does_not_touch_language() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    let default_language = store.read().get().appearance.language;

    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            font_size: Some(22.0),
            ..Default::default()
        }),
        ..Default::default()
    };
    let updated = store.read().apply_patch(patch).expect("apply_patch");

    assert_eq!(
        updated.appearance.font_size, 22.0,
        "font_size must be updated"
    );
    assert_eq!(
        updated.appearance.language, default_language,
        "language must be unchanged"
    );
}

/// apply_patch with `appearance: None` must leave all appearance fields untouched.
#[test]
fn apply_patch_with_none_appearance_changes_nothing() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    let before = store.read().get().appearance.clone();

    let patch = PreferencesPatch::default(); // all None
    let updated = store.read().apply_patch(patch).expect("apply_patch");

    assert_eq!(updated.appearance.font_size, before.font_size);
    assert_eq!(updated.appearance.language, before.language);
    assert_eq!(updated.appearance.font_family, before.font_family);
    assert_eq!(updated.appearance.theme_name, before.theme_name);
    assert_eq!(updated.appearance.opacity, before.opacity);
}

// -----------------------------------------------------------------------
// camel_to_snake
// -----------------------------------------------------------------------

// camel_to_snake

#[test]
fn camel_to_snake_simple() {
    assert_eq!(camel_to_snake("fontSize"), "font_size");
    assert_eq!(camel_to_snake("scrollbackLines"), "scrollback_lines");
    assert_eq!(camel_to_snake("cursorBlinkMs"), "cursor_blink_ms");
}

#[test]
fn camel_to_snake_already_lower() {
    assert_eq!(camel_to_snake("language"), "language");
    assert_eq!(camel_to_snake("connections"), "connections");
}

#[test]
fn camel_to_snake_empty() {
    assert_eq!(camel_to_snake(""), "");
}

#[test]
fn camel_to_snake_single_char() {
    assert_eq!(camel_to_snake("x"), "x");
    assert_eq!(camel_to_snake("X"), "x");
}

#[test]
fn camel_to_snake_leading_upper() {
    // Leading uppercase: no underscore before the first char.
    assert_eq!(camel_to_snake("FontSize"), "font_size");
}

// snake_to_camel

#[test]
fn snake_to_camel_simple() {
    assert_eq!(snake_to_camel("font_size"), "fontSize");
    assert_eq!(snake_to_camel("scrollback_lines"), "scrollbackLines");
    assert_eq!(snake_to_camel("cursor_blink_ms"), "cursorBlinkMs");
}

#[test]
fn snake_to_camel_already_flat() {
    assert_eq!(snake_to_camel("language"), "language");
    assert_eq!(snake_to_camel("connections"), "connections");
}

#[test]
fn snake_to_camel_empty() {
    assert_eq!(snake_to_camel(""), "");
}

#[test]
fn snake_to_camel_single_char() {
    assert_eq!(snake_to_camel("x"), "x");
}

// Roundtrip: camel → snake → camel

// -----------------------------------------------------------------------
// SEC-PATH-005 — connection limit
// -----------------------------------------------------------------------

fn make_connection(label: &str) -> crate::ssh::SshConnectionConfig {
    use crate::preferences::types::{SshHost, SshLabel, SshUsername};
    crate::ssh::SshConnectionConfig {
        id: crate::session::ids::ConnectionId::new(),
        label: SshLabel::try_from(label.to_string()).unwrap(),
        host: SshHost::try_from("example.com".to_string()).unwrap(),
        port: 22,
        username: SshUsername::try_from("alice".to_string()).unwrap(),
        identity_file: None,
        allow_osc52_write: false,
        keepalive_interval_secs: None,
        keepalive_max_failures: None,
    }
}

/// SEC-PATH-005: save_connection must reject a new connection when the store
/// already holds MAX_CONNECTIONS entries.
#[test]
fn sec_path_005_save_connection_rejected_when_limit_reached() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    // Fill up to exactly MAX_CONNECTIONS.
    for i in 0..MAX_CONNECTIONS {
        let conn = make_connection(&format!("conn-{i}"));
        store
            .read()
            .save_connection(conn)
            .expect("save should succeed while under limit");
    }

    // One more must be rejected.
    let overflow = make_connection("overflow");
    let result = store.read().save_connection(overflow);
    assert!(
        result.is_err(),
        "SEC-PATH-005: adding a {}-th connection must fail",
        MAX_CONNECTIONS + 1
    );
    match result.unwrap_err() {
        crate::error::PreferencesError::Validation(msg) => {
            assert!(
                msg.contains("1000"),
                "Error message should mention the limit, got: {msg}"
            );
        }
        other => panic!("Expected Validation error, got: {other:?}"),
    }
}

/// SEC-PATH-005: updating an existing connection when already at MAX_CONNECTIONS
/// must succeed (it is not adding a new entry).
#[test]
fn sec_path_005_update_existing_connection_allowed_at_limit() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    // Fill to the limit, keeping the last connection's ID.
    let mut last_id = None;
    for i in 0..MAX_CONNECTIONS {
        let conn = make_connection(&format!("conn-{i}"));
        last_id = Some(conn.id.clone());
        store.read().save_connection(conn).expect("save");
    }

    // Update the last connection — must not fail (not a new entry).
    let mut updated = make_connection("updated-label");
    updated.id = last_id.unwrap();
    let result = store.read().save_connection(updated);
    assert!(
        result.is_ok(),
        "SEC-PATH-005: updating an existing connection at the limit must succeed"
    );
}

/// SEC-PATH-005: clamp_connections truncates an oversized list.
#[test]
fn sec_path_005_clamp_connections_truncates_oversized_list() {
    use super::io::clamp_connections;
    use crate::preferences::schema::Preferences;

    let mut prefs = Preferences::default();
    for i in 0..MAX_CONNECTIONS + 1 {
        prefs
            .connections
            .push(make_connection(&format!("conn-{i}")));
    }
    assert_eq!(prefs.connections.len(), MAX_CONNECTIONS + 1);

    let clamped = clamp_connections(prefs);
    assert_eq!(
        clamped.connections.len(),
        MAX_CONNECTIONS,
        "SEC-PATH-005: clamp_connections must truncate to MAX_CONNECTIONS"
    );
}

#[test]
fn conversion_roundtrip_camel_snake_camel() {
    let cases = [
        "fontSize",
        "scrollbackLines",
        "cursorBlinkMs",
        "fontFamily",
        "themeName",
        "allowOsc52Write",
        "wordDelimiters",
        "contextMenuHintShown",
    ];
    for original in cases {
        let snake = camel_to_snake(original);
        let restored = snake_to_camel(&snake);
        assert_eq!(
            restored, original,
            "roundtrip failed for '{original}': snake='{snake}', restored='{restored}'"
        );
    }
}

// -----------------------------------------------------------------------
// Built-in theme protection
// -----------------------------------------------------------------------

fn make_user_theme(name: &str) -> UserTheme {
    let black = "#000000".to_string();
    UserTheme {
        name: name.to_string(),
        palette: std::array::from_fn(|_| black.clone()),
        foreground: black.clone(),
        background: black.clone(),
        cursor_color: black.clone(),
        selection_bg: black.clone(),
        line_height: None,
    }
}

/// delete_theme must reject the built-in "umbra" theme.
#[test]
fn test_delete_theme_rejects_builtin_umbra() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    let result = store.read().delete_theme("umbra");
    assert!(result.is_err(), "deleting built-in theme 'umbra' must fail");
    match result.unwrap_err() {
        crate::error::PreferencesError::Validation(msg) => {
            assert!(
                msg.contains("umbra"),
                "error message should mention 'umbra', got: {msg}"
            );
        }
        other => panic!("expected Validation error, got: {other:?}"),
    }
}

/// delete_theme must reject the built-in "solstice" theme.
#[test]
fn test_delete_theme_rejects_builtin_solstice() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    let result = store.read().delete_theme("solstice");
    assert!(
        result.is_err(),
        "deleting built-in theme 'solstice' must fail"
    );
    match result.unwrap_err() {
        crate::error::PreferencesError::Validation(msg) => {
            assert!(
                msg.contains("solstice"),
                "error message should mention 'solstice', got: {msg}"
            );
        }
        other => panic!("expected Validation error, got: {other:?}"),
    }
}

/// delete_theme must reject the built-in "archipel" theme.
#[test]
fn test_delete_theme_rejects_builtin_archipel() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    let result = store.read().delete_theme("archipel");
    assert!(
        result.is_err(),
        "deleting built-in theme 'archipel' must fail"
    );
    match result.unwrap_err() {
        crate::error::PreferencesError::Validation(msg) => {
            assert!(
                msg.contains("archipel"),
                "error message should mention 'archipel', got: {msg}"
            );
        }
        other => panic!("expected Validation error, got: {other:?}"),
    }
}

/// delete_theme must allow deleting a user-defined theme.
#[test]
fn test_delete_theme_allows_user_defined_theme() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("preferences.toml");
    let store = PreferencesStore::new_with_defaults(path);

    // Save a user-defined theme first.
    let theme = make_user_theme("my-theme");
    store
        .read()
        .save_theme(theme)
        .expect("save_theme must succeed");

    // Verify it is present.
    let themes = store.read().get_themes();
    assert!(
        themes.iter().any(|t| t.name == "my-theme"),
        "theme 'my-theme' must be present after save"
    );

    // Delete it — must succeed.
    let result = store.read().delete_theme("my-theme");
    assert!(
        result.is_ok(),
        "deleting user-defined theme 'my-theme' must succeed"
    );

    // Verify it is gone.
    let themes_after = store.read().get_themes();
    assert!(
        !themes_after.iter().any(|t| t.name == "my-theme"),
        "theme 'my-theme' must be absent after delete"
    );
}
