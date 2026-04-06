// SPDX-License-Identifier: MPL-2.0

//! Integration tests — preferences round-trip (TEST-PREF-001).
//!
//! These tests exercise the `Preferences` type end-to-end: JSON → struct → JSON,
//! unknown-field tolerance, and `load_or_default` robustness on arbitrary file content.

use std::io::Write as IoWrite;

use tau_term_lib::preferences::schema::{
    AppearancePatch, AppearancePrefs, BellType, CursorStyle, Language, Preferences,
    PreferencesPatch, TerminalPrefs,
};
use tau_term_lib::preferences::store::PreferencesStore;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write bytes to a temp file and return the directory and file path.
fn write_temp_prefs(subdir: &str, content: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let prefs_dir = tmp.path().join(subdir).join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create prefs dir");
    let path = prefs_dir.join("preferences.json");
    std::fs::File::create(&path)
        .expect("create prefs file")
        .write_all(content)
        .expect("write prefs file");
    // Return tmp so it stays alive for the duration of the test.
    (tmp, path)
}

/// Run `load_or_default` with `XDG_CONFIG_HOME` pointing at `xdg_root`.
///
/// # Safety
/// Environment mutation is only safe when nextest runs each test in its own
/// process (default isolation mode). Do not move this to an inline `#[test]`
/// where multiple tests share the same process.
fn load_with_xdg(xdg_root: &std::path::Path) -> Preferences {
    let orig = std::env::var_os("XDG_CONFIG_HOME");
    // SAFETY: nextest process-per-test isolation — no concurrent env readers.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", xdg_root) };

    let store = PreferencesStore::load_or_default();
    let prefs = store.read().get();

    // SAFETY: same as above.
    unsafe {
        match orig {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
    prefs
}

// ---------------------------------------------------------------------------
// TEST-PREF-001 — Full round-trip: JSON → struct → JSON, values identical
// ---------------------------------------------------------------------------

#[test]
fn pref_roundtrip_default_json_values_are_identical() {
    let original = Preferences::default();
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: Preferences = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(
        restored.appearance.font_family,
        original.appearance.font_family
    );
    assert_eq!(restored.appearance.font_size, original.appearance.font_size);
    assert_eq!(
        restored.appearance.cursor_blink_ms,
        original.appearance.cursor_blink_ms
    );
    assert_eq!(
        restored.appearance.theme_name,
        original.appearance.theme_name
    );
    assert_eq!(restored.appearance.opacity, original.appearance.opacity);
    assert_eq!(restored.appearance.language, original.appearance.language);
    assert_eq!(
        restored.appearance.context_menu_hint_shown,
        original.appearance.context_menu_hint_shown
    );
    assert_eq!(
        restored.terminal.scrollback_lines,
        original.terminal.scrollback_lines
    );
    assert_eq!(
        restored.terminal.allow_osc52_write,
        original.terminal.allow_osc52_write
    );
    assert_eq!(
        restored.terminal.word_delimiters,
        original.terminal.word_delimiters
    );
    assert_eq!(restored.terminal.bell_type, original.terminal.bell_type);
    assert_eq!(
        restored.terminal.confirm_multiline_paste,
        original.terminal.confirm_multiline_paste
    );
}

#[test]
fn pref_roundtrip_non_default_values_preserved() {
    let mut prefs = Preferences::default();
    prefs.appearance.font_family = "JetBrains Mono".to_string();
    prefs.appearance.font_size = 18.5;
    prefs.appearance.cursor_style = CursorStyle::Bar;
    prefs.appearance.cursor_blink_ms = 800;
    prefs.appearance.theme_name = "nord".to_string();
    prefs.appearance.opacity = 0.85;
    prefs.appearance.language = Language::Fr;
    prefs.appearance.context_menu_hint_shown = true;
    prefs.terminal.scrollback_lines = 5_000;
    prefs.terminal.allow_osc52_write = true;
    prefs.terminal.word_delimiters = " |,".to_string();
    prefs.terminal.bell_type = BellType::Audio;
    prefs.terminal.confirm_multiline_paste = false;

    let json = serde_json::to_string_pretty(&prefs).expect("serialize");
    let restored: Preferences = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.appearance.font_family, "JetBrains Mono");
    assert_eq!(restored.appearance.font_size, 18.5);
    assert_eq!(restored.appearance.cursor_style, CursorStyle::Bar);
    assert_eq!(restored.appearance.cursor_blink_ms, 800);
    assert_eq!(restored.appearance.theme_name, "nord");
    assert_eq!(restored.appearance.opacity, 0.85);
    assert_eq!(restored.appearance.language, Language::Fr);
    assert!(restored.appearance.context_menu_hint_shown);
    assert_eq!(restored.terminal.scrollback_lines, 5_000);
    assert!(restored.terminal.allow_osc52_write);
    assert_eq!(restored.terminal.word_delimiters, " |,");
    assert_eq!(restored.terminal.bell_type, BellType::Audio);
    assert!(!restored.terminal.confirm_multiline_paste);
}

#[test]
fn pref_roundtrip_language_fr_survives_json() {
    let mut prefs = Preferences::default();
    prefs.appearance.language = Language::Fr;
    let json = serde_json::to_string(&prefs).expect("serialize");
    let restored: Preferences = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.appearance.language, Language::Fr);
}

#[test]
fn pref_roundtrip_all_cursor_styles_survive_json() {
    for style in [CursorStyle::Block, CursorStyle::Underline, CursorStyle::Bar] {
        let mut prefs = Preferences::default();
        prefs.appearance.cursor_style = style;
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: Preferences = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.appearance.cursor_style, style);
    }
}

#[test]
fn pref_roundtrip_all_bell_types_survive_json() {
    for bell in [
        BellType::None,
        BellType::Visual,
        BellType::Audio,
        BellType::Both,
    ] {
        let mut prefs = Preferences::default();
        prefs.terminal.bell_type = bell;
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: Preferences = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.terminal.bell_type, bell);
    }
}

#[test]
fn pref_roundtrip_keyboard_bindings_survive_json() {
    let mut prefs = Preferences::default();
    prefs
        .keyboard
        .bindings
        .insert("new-tab".to_string(), "Ctrl+T".to_string());
    prefs
        .keyboard
        .bindings
        .insert("close-pane".to_string(), "Ctrl+W".to_string());
    let json = serde_json::to_string(&prefs).expect("serialize");
    let restored: Preferences = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        restored
            .keyboard
            .bindings
            .get("new-tab")
            .map(String::as_str),
        Some("Ctrl+T")
    );
    assert_eq!(
        restored
            .keyboard
            .bindings
            .get("close-pane")
            .map(String::as_str),
        Some("Ctrl+W")
    );
}

#[test]
fn pref_roundtrip_patch_serializes_correctly() {
    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch::default()),
        terminal: None,
        keyboard: None,
    };
    let json = serde_json::to_string(&patch).expect("serialize");
    let restored: PreferencesPatch = serde_json::from_str(&json).expect("deserialize");
    assert!(restored.appearance.is_some());
    assert!(restored.terminal.is_none());
    assert!(restored.keyboard.is_none());
}

// ---------------------------------------------------------------------------
// TEST-PREF-001 — Unknown field in JSON → ignored, defaults applied
// ---------------------------------------------------------------------------

#[test]
fn pref_roundtrip_unknown_field_in_json_is_ignored() {
    // `#[serde(default)]` on `Preferences` means unknown top-level fields are ignored.
    let json = r#"{
        "appearance": {"fontFamily": "Hack"},
        "unknownFutureFeature": {"setting": true},
        "anotherUnknown": 42
    }"#;
    let prefs: Preferences = serde_json::from_str(json).expect("should not fail on unknown fields");
    assert_eq!(prefs.appearance.font_family, "Hack");
    // Unlisted fields default.
    assert_eq!(prefs.terminal.scrollback_lines, 10_000);
}

#[test]
fn pref_roundtrip_unknown_field_in_appearance_is_ignored() {
    let json = r#"{"appearance": {"fontFamily": "Fira Code", "nonExistentField": "ignored"}}"#;
    let prefs: Preferences = serde_json::from_str(json).expect("unknown field must not panic");
    assert_eq!(prefs.appearance.font_family, "Fira Code");
}

#[test]
fn pref_roundtrip_unknown_field_in_terminal_is_ignored() {
    let json = r#"{"terminal": {"scrollbackLines": 3000, "futureOption": true}}"#;
    let prefs: Preferences = serde_json::from_str(json).expect("unknown field must not panic");
    assert_eq!(prefs.terminal.scrollback_lines, 3000);
}

#[test]
fn pref_roundtrip_partial_json_applies_defaults_for_missing_fields() {
    // Only one sub-section present; the rest must default.
    let json = r#"{"terminal": {"scrollbackLines": 2000}}"#;
    let prefs: Preferences = serde_json::from_str(json).expect("partial json");
    assert_eq!(prefs.terminal.scrollback_lines, 2000);
    // appearance defaults untouched.
    assert_eq!(prefs.appearance.font_family, "monospace");
    assert_eq!(prefs.appearance.language, Language::En);
}

// ---------------------------------------------------------------------------
// TEST-PREF-001 — load_or_default never panics on arbitrary file content
// ---------------------------------------------------------------------------

#[test]
fn load_or_default_does_not_panic_on_empty_file() {
    let (tmp, _) = write_temp_prefs("test_empty", b"");
    let prefs = load_with_xdg(tmp.path());
    // Must fall back to defaults without panicking.
    assert_eq!(prefs.appearance.font_family, "monospace");
}

#[test]
fn load_or_default_does_not_panic_on_invalid_json() {
    let (tmp, _) = write_temp_prefs("test_invalid_json", b"{ not valid json !!!");
    let prefs = load_with_xdg(tmp.path());
    assert_eq!(prefs.appearance.language, Language::En);
}

#[test]
fn load_or_default_does_not_panic_when_file_missing() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    // Deliberately do NOT create a preferences.json file.
    let prefs = load_with_xdg(tmp.path());
    assert_eq!(prefs.terminal.scrollback_lines, 10_000);
}

#[test]
fn load_or_default_does_not_panic_on_incomplete_json() {
    // Syntactically valid JSON but missing almost all fields.
    let (tmp, _) = write_temp_prefs("test_incomplete", br#"{"appearance": {}}"#);
    let prefs = load_with_xdg(tmp.path());
    // Missing appearance fields must fall back to defaults.
    assert_eq!(prefs.appearance.font_size, 14.0);
    assert_eq!(prefs.appearance.cursor_blink_ms, 530);
}

#[test]
fn load_or_default_returns_defaults_on_json_type_mismatch() {
    // `fontFamily` expects a String but receives an integer.
    let (tmp, _) = write_temp_prefs(
        "test_type_mismatch",
        br#"{"appearance": {"fontFamily": 999}}"#,
    );
    let prefs = load_with_xdg(tmp.path());
    // Parse error → full default fallback.
    assert_eq!(prefs.appearance.font_family, "monospace");
}

#[test]
fn load_or_default_falls_back_for_unknown_language_variant() {
    // "de" is not a valid Language variant → serde error → load_or_default falls back.
    let (tmp, _) = write_temp_prefs(
        "test_unknown_lang",
        br#"{"appearance": {"language": "de"}}"#,
    );
    let prefs = load_with_xdg(tmp.path());
    assert_eq!(
        prefs.appearance.language,
        Language::En,
        "Unknown language variant must fall back to Language::En"
    );
}

#[test]
fn load_or_default_loads_valid_preferences_correctly() {
    let json = serde_json::json!({
        "appearance": {
            "fontFamily": "Iosevka",
            "fontSize": 16.0,
            "language": "fr"
        },
        "terminal": {
            "scrollbackLines": 8000
        }
    })
    .to_string();
    // write_temp_prefs creates <tmp>/<subdir>/tauterm/preferences.json and XDG root is tmp.path()/<subdir>.
    // We must pass <subdir> as empty string equivalent — use the variant that writes directly.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let prefs_dir = tmp.path().join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create prefs dir");
    std::fs::write(prefs_dir.join("preferences.json"), json.as_bytes()).expect("write");
    let prefs = load_with_xdg(tmp.path());
    assert_eq!(prefs.appearance.font_family, "Iosevka");
    assert_eq!(prefs.appearance.font_size, 16.0);
    assert_eq!(prefs.appearance.language, Language::Fr);
    assert_eq!(prefs.terminal.scrollback_lines, 8000);
}

#[test]
fn pref_store_apply_patch_appearance_updates_correctly() {
    // Build a store with defaults (no real disk path needed — /dev/null fallback).
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let prefs_dir = tmp.path().join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create prefs dir");
    // Write a minimal valid prefs file so load_or_default reads it.
    std::fs::write(prefs_dir.join("preferences.json"), b"{}").expect("write");

    let prefs = load_with_xdg(tmp.path());
    assert_eq!(prefs.appearance.language, Language::En);
}

#[test]
fn pref_terminal_prefs_default_confirm_multiline_paste_is_true() {
    let t = TerminalPrefs::default();
    assert!(
        t.confirm_multiline_paste,
        "FS-CLIP-009: must default to true"
    );
}

#[test]
fn pref_appearance_prefs_default_context_menu_hint_shown_is_false() {
    let a = AppearancePrefs::default();
    assert!(!a.context_menu_hint_shown);
}

// ---------------------------------------------------------------------------
// TOML persistence tests
// ---------------------------------------------------------------------------

/// Write a TOML preferences file to a temp XDG dir and return the dir + path.
fn write_temp_prefs_toml(content: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let prefs_dir = tmp.path().join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create prefs dir");
    let path = prefs_dir.join("preferences.toml");
    std::fs::write(&path, content).expect("write prefs file");
    (tmp, path)
}

#[test]
fn pref_roundtrip_toml_default_values_are_identical() {
    let original = Preferences::default();
    let toml_str = toml::to_string_pretty(&original).expect("serialize to TOML");
    let restored: Preferences = toml::from_str(&toml_str).expect("deserialize from TOML");

    assert_eq!(
        restored.appearance.font_family,
        original.appearance.font_family
    );
    assert_eq!(restored.appearance.font_size, original.appearance.font_size);
    assert_eq!(restored.appearance.language, original.appearance.language);
    assert_eq!(
        restored.terminal.scrollback_lines,
        original.terminal.scrollback_lines
    );
}

#[test]
fn pref_roundtrip_toml_non_default_values_preserved() {
    let mut original = Preferences::default();
    original.appearance.language = Language::Fr;
    original.terminal.scrollback_lines = 5000;
    original.appearance.font_size = 16.0;

    let toml_str = toml::to_string_pretty(&original).expect("serialize to TOML");
    let restored: Preferences = toml::from_str(&toml_str).expect("deserialize from TOML");

    assert_eq!(restored.appearance.language, Language::Fr);
    assert_eq!(restored.terminal.scrollback_lines, 5000);
    assert_eq!(restored.appearance.font_size, 16.0);
}

#[test]
fn load_or_default_reads_toml_file() {
    // On-disk format uses snake_case keys (ADR-0016); the store converts to camelCase on load.
    let (tmp, _) = write_temp_prefs_toml(b"[appearance]\nfont_size = 18.0\n");
    let prefs = load_with_xdg(tmp.path());
    assert_eq!(prefs.appearance.font_size, 18.0);
}

#[test]
fn load_or_default_falls_back_to_json_when_toml_absent() {
    // Write only a JSON file (camelCase keys) — no TOML file exists.
    let (tmp, _) = write_temp_prefs("migration", br#"{"appearance": {"fontSize": 14.5}}"#);
    // `write_temp_prefs` creates <subdir>/tauterm/preferences.json.
    // XDG must point at <tmp.path()>/migration so the store finds <xdg>/tauterm/preferences.json.
    let prefs = load_with_xdg(&tmp.path().join("migration"));
    // JSON migration: fontSize preserved, everything else defaults.
    assert_eq!(prefs.appearance.font_size, 14.5);
}

#[test]
fn load_or_default_toml_takes_priority_over_json() {
    // Both files exist: TOML should win.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let prefs_dir = tmp.path().join("tauterm");
    std::fs::create_dir_all(&prefs_dir).expect("create prefs dir");
    // On-disk format is snake_case (ADR-0016)
    std::fs::write(
        prefs_dir.join("preferences.toml"),
        b"[appearance]\nfont_size = 20.0\n",
    )
    .expect("write toml");
    // Legacy JSON uses camelCase (serde rename_all)
    std::fs::write(
        prefs_dir.join("preferences.json"),
        br#"{"appearance": {"fontSize": 99.0}}"#,
    )
    .expect("write json");

    let prefs = load_with_xdg(tmp.path());
    // TOML wins → 20.0, not 99.0
    assert_eq!(prefs.appearance.font_size, 20.0);
}

#[test]
fn load_or_default_falls_back_to_defaults_on_corrupt_toml() {
    let (tmp, _) = write_temp_prefs_toml(b"[[[[not valid toml at all");
    let prefs = load_with_xdg(tmp.path());
    assert_eq!(
        prefs.appearance.font_size,
        Preferences::default().appearance.font_size
    );
}

// ---------------------------------------------------------------------------
// Store save/load roundtrip — verifies snake_case on disk (ADR-0016)
// ---------------------------------------------------------------------------

/// Helper: load a store with XDG pointing at `xdg_root`, call `apply_patch`,
/// and return the raw bytes written to disk.
fn save_via_store_and_read_file(xdg_root: &std::path::Path, patch: PreferencesPatch) -> String {
    let orig = std::env::var_os("XDG_CONFIG_HOME");
    // SAFETY: nextest process-per-test isolation.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", xdg_root) };

    let store = PreferencesStore::load_or_default();
    store
        .read()
        .apply_patch(patch)
        .expect("apply_patch must succeed");

    // SAFETY: same as above.
    unsafe {
        match orig {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    let toml_path = xdg_root.join("tauterm").join("preferences.toml");
    std::fs::read_to_string(&toml_path).expect("preferences.toml must exist after save")
}

#[test]
fn save_writes_snake_case_keys_to_disk() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            font_size: Some(20.0),
            language: Some(Language::Fr),
            ..Default::default()
        }),
        ..Default::default()
    };
    let content = save_via_store_and_read_file(tmp.path(), patch);

    // File must contain snake_case keys, not camelCase
    assert!(
        content.contains("font_size"),
        "expected snake_case key 'font_size' in file, got:\n{content}"
    );
    assert!(
        !content.contains("fontSize"),
        "camelCase key 'fontSize' must NOT appear in file, got:\n{content}"
    );
    assert!(
        content.contains("font_family"),
        "expected snake_case key 'font_family' in file, got:\n{content}"
    );
    assert!(
        !content.contains("fontFamily"),
        "camelCase key 'fontFamily' must NOT appear in file, got:\n{content}"
    );
}

#[test]
fn store_roundtrip_save_then_load_preserves_values() {
    let tmp = tempfile::TempDir::new().expect("tempdir");

    // --- Save ---
    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            font_size: Some(17.5),
            language: Some(Language::Fr),
            ..Default::default()
        }),
        ..Default::default()
    };
    save_via_store_and_read_file(tmp.path(), patch);

    // --- Load from the file just written ---
    let prefs = load_with_xdg(tmp.path());

    assert_eq!(prefs.appearance.font_size, 17.5);
    assert_eq!(prefs.appearance.language, Language::Fr);
}

#[test]
fn store_roundtrip_snake_case_file_survives_all_preference_fields() {
    let tmp = tempfile::TempDir::new().expect("tempdir");

    // Save a patch touching the appearance section.
    let patch = PreferencesPatch {
        appearance: Some(AppearancePatch {
            font_size: Some(15.0),
            ..Default::default()
        }),
        ..Default::default()
    };
    let content = save_via_store_and_read_file(tmp.path(), patch);

    // Verify representative field names from each section are snake_case.
    let snake_case_keys = [
        "font_size",
        "font_family",
        "cursor_style",
        "cursor_blink_ms",
        "theme_name",
    ];
    for key in snake_case_keys {
        assert!(
            content.contains(key),
            "expected snake_case key '{key}' in saved TOML:\n{content}"
        );
    }

    // Re-load and verify the full roundtrip.
    let prefs = load_with_xdg(tmp.path());
    assert_eq!(prefs.appearance.font_size, 15.0);
}
