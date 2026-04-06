// SPDX-License-Identifier: MPL-2.0

//! Preferences schema — all user preferences and their default values.
//!
//! `Preferences` is the top-level struct persisted to `~/.config/tauterm/preferences.json`.
//! All nested types implement `Serialize`, `Deserialize`, and `Default`.
//!
//! The `Language` field MUST be an enum — never a free `String` across IPC (FS-I18N-006,
//! CLAUDE.md constraint).

use serde::{Deserialize, Serialize};

use crate::ssh::SshConnectionConfig;

/// Top-level user preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub appearance: AppearancePrefs,
    pub terminal: TerminalPrefs,
    pub keyboard: KeyboardPrefs,
    /// Saved SSH connections. Authoritative source — `SshManager` reads/writes these.
    pub connections: Vec<SshConnectionConfig>,
    /// User-defined themes.
    pub themes: Vec<UserTheme>,
}

/// Appearance-related preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppearancePrefs {
    pub font_family: String,
    pub font_size: f32,
    pub cursor_style: CursorStyle,
    /// Cursor blink period in milliseconds (FS-VT-032). Default: 530ms.
    pub cursor_blink_ms: u32,
    /// Name of the active theme.
    pub theme_name: String,
    /// Background opacity (0.0–1.0).
    pub opacity: f32,
    /// UI language (FS-I18N-006: enum, not free String).
    pub language: Language,
    /// Whether the context menu hint has been shown at least once.
    /// Used to suppress the first-use onboarding hint after the user has seen it.
    #[serde(default)]
    pub context_menu_hint_shown: bool,
    /// Whether the window should be in full-screen mode (FS-FULL-009).
    /// `#[serde(default)]` ensures existing preferences files without this field
    /// deserialize successfully with `false`.
    #[serde(default)]
    pub fullscreen: bool,
}

impl Default for AppearancePrefs {
    fn default() -> Self {
        Self {
            font_family: "monospace".to_string(),
            font_size: 14.0,
            cursor_style: CursorStyle::default(),
            cursor_blink_ms: 530,
            theme_name: "umbra".to_string(),
            opacity: 1.0,
            language: Language::default(),
            context_menu_hint_shown: false,
            fullscreen: false,
        }
    }
}

/// Terminal behavior preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminalPrefs {
    /// Maximum scrollback lines per pane.
    pub scrollback_lines: usize,
    /// Allow OSC 52 clipboard write for local sessions.
    pub allow_osc52_write: bool,
    /// Characters treated as word delimiters for double-click selection.
    pub word_delimiters: String,
    /// Bell notification type.
    pub bell_type: BellType,
    /// Show a confirmation dialog before pasting multi-line text when bracketed
    /// paste is inactive (FS-CLIP-009). Default: `true`. The user can disable
    /// it via the "Don't ask again" toggle in the paste dialog.
    #[serde(default = "default_confirm_multiline_paste")]
    pub confirm_multiline_paste: bool,
}

fn default_confirm_multiline_paste() -> bool {
    true
}

impl Default for TerminalPrefs {
    fn default() -> Self {
        Self {
            scrollback_lines: 10_000,
            allow_osc52_write: false,
            word_delimiters: r#" \t|"'`&()*,;<=>[]{}~"#.to_string(),
            bell_type: BellType::default(),
            confirm_multiline_paste: true,
        }
    }
}

/// Keyboard shortcut preferences.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct KeyboardPrefs {
    // Keybinding overrides — populated in the full keyboard integration pass.
    // Using a map of action → key combo.
    pub bindings: std::collections::HashMap<String, String>,
}

/// Partial update for appearance preferences — only the fields provided are changed.
///
/// Using a dedicated patch type (instead of `Option<AppearancePrefs>` in `PreferencesPatch`)
/// allows field-by-field updates without read-before-write: e.g. changing the language
/// without knowing or sending the current font size.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppearancePatch {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub cursor_style: Option<CursorStyle>,
    pub cursor_blink_ms: Option<u32>,
    pub theme_name: Option<String>,
    pub opacity: Option<f32>,
    pub language: Option<Language>,
    pub context_menu_hint_shown: Option<bool>,
    pub fullscreen: Option<bool>,
}

/// A partial preferences update (only the fields the user changed).
/// All fields are optional so the frontend can send minimal payloads.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PreferencesPatch {
    pub appearance: Option<AppearancePatch>,
    pub terminal: Option<TerminalPrefs>,
    pub keyboard: Option<KeyboardPrefs>,
}

// ---------------------------------------------------------------------------
// Cursor style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

// ---------------------------------------------------------------------------
// Bell type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BellType {
    /// No bell notification.
    None,
    /// Visual flash (default).
    #[default]
    Visual,
    /// System audio bell.
    Audio,
    /// Both visual and audio.
    Both,
}

// ---------------------------------------------------------------------------
// Language — MUST be an enum, never a free String (FS-I18N-006)
// ---------------------------------------------------------------------------

/// Supported UI languages. Extend this enum when adding new locales.
/// This type is used across IPC — it MUST NOT be replaced with a plain `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    #[default]
    En,
    Fr,
}

// ---------------------------------------------------------------------------
// User theme
// ---------------------------------------------------------------------------

/// A user-defined color theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTheme {
    pub name: String,
    /// ANSI palette: 16 colors (0–15). Each entry is an RGB hex string (e.g., "#1e1e2e").
    pub palette: [String; 16],
    pub foreground: String,
    pub background: String,
    pub cursor_color: String,
    pub selection_bg: String,
    /// Terminal line height multiplier (FS-THEME-010). Range: 1.0–2.0.
    /// `None` means use the global default (`--line-height-terminal` token).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Default values ---

    #[test]
    fn default_preferences_has_expected_appearance_defaults() {
        let prefs = Preferences::default();
        assert_eq!(prefs.appearance.font_family, "monospace");
        assert_eq!(prefs.appearance.font_size, 14.0);
        assert_eq!(prefs.appearance.cursor_blink_ms, 530);
        assert_eq!(prefs.appearance.theme_name, "umbra");
        assert_eq!(prefs.appearance.opacity, 1.0);
        assert_eq!(prefs.appearance.language, Language::En);
    }

    #[test]
    fn default_preferences_has_expected_terminal_defaults() {
        let prefs = Preferences::default();
        assert_eq!(prefs.terminal.scrollback_lines, 10_000);
        assert!(!prefs.terminal.allow_osc52_write);
        assert_eq!(prefs.terminal.bell_type, BellType::Visual);
        assert!(
            prefs.terminal.confirm_multiline_paste,
            "FS-CLIP-009: default must be true"
        );
    }

    #[test]
    fn confirm_multiline_paste_round_trips_through_json() {
        let mut prefs = Preferences::default();
        prefs.terminal.confirm_multiline_paste = false;
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: Preferences = serde_json::from_str(&json).expect("deserialize");
        assert!(!restored.terminal.confirm_multiline_paste);
    }

    #[test]
    fn confirm_multiline_paste_defaults_to_true_when_absent_from_json() {
        // Old preferences files without this field should default to true (FS-CLIP-009).
        let json = r#"{"terminal":{"scrollbackLines":5000}}"#;
        let prefs: Preferences = serde_json::from_str(json).expect("deserialize");
        assert!(
            prefs.terminal.confirm_multiline_paste,
            "Missing field must default to true"
        );
    }

    #[test]
    fn default_preferences_has_empty_connections_and_themes() {
        let prefs = Preferences::default();
        assert!(prefs.connections.is_empty());
        assert!(prefs.themes.is_empty());
    }

    // --- Serialization round-trip ---

    #[test]
    fn preferences_serializes_and_deserializes_to_identical_value() {
        let original = Preferences::default();
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: Preferences = serde_json::from_str(&json).expect("deserialize failed");

        // Spot-check key fields across sections.
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
        assert_eq!(restored.terminal.bell_type, original.terminal.bell_type);
    }

    #[test]
    fn preferences_round_trip_preserves_language_enum() {
        let mut prefs = Preferences::default();
        prefs.appearance.language = Language::Fr;
        let json = serde_json::to_string(&prefs).expect("serialize failed");
        let restored: Preferences = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(restored.appearance.language, Language::Fr);
    }

    // --- Language enum serialization (FS-I18N-006) ---

    #[test]
    fn language_en_serializes_to_camel_case_string() {
        let json = serde_json::to_string(&Language::En).expect("serialize failed");
        assert_eq!(json, "\"en\"");
    }

    #[test]
    fn language_fr_serializes_to_camel_case_string() {
        let json = serde_json::to_string(&Language::Fr).expect("serialize failed");
        assert_eq!(json, "\"fr\"");
    }

    #[test]
    fn language_deserializes_from_lowercase_string() {
        let en: Language = serde_json::from_str("\"en\"").expect("deserialize failed");
        let fr: Language = serde_json::from_str("\"fr\"").expect("deserialize failed");
        assert_eq!(en, Language::En);
        assert_eq!(fr, Language::Fr);
    }

    // --- CursorStyle ---

    #[test]
    fn cursor_style_default_is_block() {
        assert_eq!(CursorStyle::default(), CursorStyle::Block);
    }

    // --- BellType ---

    #[test]
    fn bell_type_default_is_visual() {
        assert_eq!(BellType::default(), BellType::Visual);
    }

    // --- PreferencesPatch (partial update) ---

    #[test]
    fn preferences_patch_default_has_all_none_fields() {
        let patch = PreferencesPatch::default();
        assert!(patch.appearance.is_none());
        assert!(patch.terminal.is_none());
        assert!(patch.keyboard.is_none());
    }

    // -----------------------------------------------------------------------
    // SEC-IPC-005 — Language enum rejects unknown variants at IPC boundary
    // -----------------------------------------------------------------------

    /// SEC-IPC-005: Unknown language string "de" must fail serde deserialization.
    /// This prevents arbitrary string injection via the language field.
    #[test]
    fn sec_ipc_005_unknown_language_variant_de_rejected() {
        let result: Result<Language, _> = serde_json::from_str("\"de\"");
        assert!(
            result.is_err(),
            "Unknown language variant 'de' must fail deserialization (SEC-IPC-005)"
        );
    }

    /// SEC-IPC-005: Empty string language must fail deserialization.
    #[test]
    fn sec_ipc_005_empty_string_language_rejected() {
        let result: Result<Language, _> = serde_json::from_str("\"\"");
        assert!(
            result.is_err(),
            "Empty string language must fail deserialization (SEC-IPC-005)"
        );
    }

    /// SEC-IPC-005: SQL injection payload as language value must fail.
    #[test]
    fn sec_ipc_005_language_sql_injection_payload_rejected() {
        let result: Result<Language, _> =
            serde_json::from_str("\"en'; DROP TABLE preferences; --\"");
        assert!(
            result.is_err(),
            "SQL injection payload as language must be rejected (SEC-IPC-005)"
        );
    }

    /// SEC-IPC-005: Preferences deserialization with unknown language in JSON
    /// must fail — not silently fall back to a default (serde strict mode).
    ///
    /// NOTE: `#[serde(default)]` on AppearancePrefs means a missing field uses
    /// the default value. However a present-but-invalid variant MUST still fail.
    #[test]
    fn sec_ipc_005_preferences_with_unknown_language_fails_deserialization() {
        let json = r#"{"appearance":{"language":"zz"}}"#;
        let result: Result<Preferences, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Preferences with unknown language variant must fail deserialization (SEC-IPC-005)"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-I18N-004 — unknown language in preferences falls back to En
    // FS-I18N-005, FS-I18N-006
    //
    // The fallback MUST NOT happen at the serde level (that would weaken
    // SEC-IPC-005). It must happen in PreferencesStore::load_or_default(),
    // which catches deserialization errors and applies field-level defaults.
    // The tests below document the expected store behaviour and are BLOCKED
    // until load_or_default() is implemented.
    // -----------------------------------------------------------------------

    /// TEST-I18N-004 (FS-I18N-005, FS-I18N-006):
    /// When preferences.json contains an unknown language variant, `load_or_default()`
    /// must detect the deserialization error and fall back to `Language::En`.
    ///
    /// The `Language` serde deserializer stays strict (SEC-IPC-005) — the fallback
    /// occurs at the store level, not inside serde.
    #[test]
    fn i18n_004_preferences_store_falls_back_to_en_for_unknown_language() {
        use crate::preferences::store::PreferencesStore;
        use std::io::Write;

        // Write a preferences file with an unknown language variant.
        // preferences_path() resolves to: {XDG_CONFIG_HOME}/tauterm/preferences.json
        let tmp_dir = std::env::temp_dir().join("tauterm_i18n_004_test");
        let prefs_dir = tmp_dir.join("tauterm");
        std::fs::create_dir_all(&prefs_dir).expect("create tmp dir");
        let prefs_path = prefs_dir.join("preferences.json");
        {
            let mut f = std::fs::File::create(&prefs_path).expect("create tmp prefs");
            f.write_all(br#"{"appearance":{"language":"de"}}"#)
                .expect("write tmp prefs");
        }

        // Point the store at this file via XDG_CONFIG_HOME override.
        // load_or_default() uses preferences_path() → dirs_or_home() which reads XDG_CONFIG_HOME.
        // Temporarily override XDG_CONFIG_HOME so load_or_default picks up our file.
        // NOTE: env mutation in tests is only safe in single-threaded context.
        // This test is self-contained and does not run concurrently with other env tests.
        let orig_xdg = std::env::var_os("XDG_CONFIG_HOME");
        // SAFETY: `set_var` is unsound when multiple threads read the environment
        // concurrently. This is safe here because:
        // 1. This project uses `cargo nextest` exclusively (see CLAUDE.md), which
        //    runs each test in its own forked process — no shared address space.
        // 2. No other thread in this test binary reads XDG_CONFIG_HOME concurrently.
        // DO NOT run this code under `cargo test --test-threads > 1`.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &tmp_dir) };

        let store = PreferencesStore::load_or_default();
        let prefs = store.read().get();

        // Restore env.
        // SAFETY: same rationale as the set_var call above — nextest process isolation.
        unsafe {
            match orig_xdg {
                Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp_dir);

        assert_eq!(
            prefs.appearance.language,
            Language::En,
            "TEST-I18N-004: Unknown language in preferences.json must fall back to Language::En"
        );
    }

    #[test]
    fn preferences_patch_round_trips_through_json() {
        let patch = PreferencesPatch {
            appearance: Some(AppearancePatch::default()),
            terminal: None,
            keyboard: None,
        };
        let json = serde_json::to_string(&patch).expect("serialize failed");
        let restored: PreferencesPatch = serde_json::from_str(&json).expect("deserialize failed");
        assert!(restored.appearance.is_some());
        assert!(restored.terminal.is_none());
    }

    // --- AppearancePatch ---

    /// AppearancePatch::default() must have all fields set to None.
    #[test]
    fn appearance_patch_default_has_all_none_fields() {
        let patch = AppearancePatch::default();
        assert!(patch.font_family.is_none());
        assert!(patch.font_size.is_none());
        assert!(patch.cursor_style.is_none());
        assert!(patch.cursor_blink_ms.is_none());
        assert!(patch.theme_name.is_none());
        assert!(patch.opacity.is_none());
        assert!(patch.language.is_none());
        assert!(patch.context_menu_hint_shown.is_none());
        assert!(patch.fullscreen.is_none());
    }

    // --- Fullscreen preference (FS-FULL-009) ---

    /// AppearancePrefs::default() must have fullscreen = false.
    #[test]
    fn fullscreen_field_defaults_to_false() {
        let prefs = AppearancePrefs::default();
        assert!(!prefs.fullscreen, "fullscreen must default to false");
    }

    /// fullscreen = true round-trips through JSON.
    #[test]
    fn fullscreen_round_trips_through_json() {
        let mut prefs = Preferences::default();
        prefs.appearance.fullscreen = true;
        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: Preferences = serde_json::from_str(&json).expect("deserialize");
        assert!(
            restored.appearance.fullscreen,
            "fullscreen must survive JSON round-trip"
        );
    }

    /// Existing preferences JSON without the `fullscreen` field must deserialize
    /// with `fullscreen = false` (backward-compatibility, FS-FULL-009).
    #[test]
    fn fullscreen_absent_from_json_defaults_to_false() {
        let json = r#"{"appearance":{"fontSize":16.0}}"#;
        let prefs: Preferences = serde_json::from_str(json).expect("deserialize");
        assert!(
            !prefs.appearance.fullscreen,
            "Missing fullscreen field must default to false"
        );
    }

    /// AppearancePatch with fullscreen = Some(true) serializes the field correctly.
    #[test]
    fn appearance_patch_fullscreen_serializes_correctly() {
        let patch = AppearancePatch {
            fullscreen: Some(true),
            ..Default::default()
        };
        let json = serde_json::to_string(&patch).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(
            value.get("fullscreen").and_then(|v| v.as_bool()),
            Some(true),
            "fullscreen must serialize to true"
        );
    }

    /// AppearancePatch with only language set serializes to a JSON object
    /// containing the `language` field and no others that are present-and-non-null.
    #[test]
    fn appearance_patch_language_only_serializes_correctly() {
        let patch = AppearancePatch {
            language: Some(Language::Fr),
            ..Default::default()
        };
        let json = serde_json::to_string(&patch).expect("serialize failed");
        let value: serde_json::Value = serde_json::from_str(&json).expect("deserialize failed");

        // language field must be present with the correct value.
        assert_eq!(
            value.get("language").and_then(|v| v.as_str()),
            Some("fr"),
            "language must serialize to \"fr\""
        );
    }
}
