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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for Preferences {
    fn default() -> Self {
        Self {
            appearance: AppearancePrefs::default(),
            terminal: TerminalPrefs::default(),
            keyboard: KeyboardPrefs::default(),
            connections: Vec::new(),
            themes: Vec::new(),
        }
    }
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
}

impl Default for TerminalPrefs {
    fn default() -> Self {
        Self {
            scrollback_lines: 10_000,
            allow_osc52_write: false,
            word_delimiters: r#" \t|"'`&()*,;<=>[]{}~"#.to_string(),
            bell_type: BellType::default(),
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

/// A partial preferences update (only the fields the user changed).
/// All fields are optional so the frontend can send minimal payloads.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PreferencesPatch {
    pub appearance: Option<AppearancePrefs>,
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
        assert_eq!(restored.appearance.font_family, original.appearance.font_family);
        assert_eq!(restored.appearance.font_size, original.appearance.font_size);
        assert_eq!(restored.appearance.language, original.appearance.language);
        assert_eq!(restored.terminal.scrollback_lines, original.terminal.scrollback_lines);
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

    #[test]
    #[ignore = "TEST-I18N-004: PreferencesStore::load_or_default() not yet implemented (BLOCKED)"]
    fn i18n_004_preferences_store_falls_back_to_en_for_unknown_language() {
        // When preferences.json contains `"language": "de"`, load_or_default()
        // must: (1) detect the deserialization error, (2) substitute Language::En,
        // (3) return a valid Preferences struct without crashing.
        // Unblocked when store::load_or_default() is implemented.
    }

    #[test]
    fn preferences_patch_round_trips_through_json() {
        let patch = PreferencesPatch {
            appearance: Some(AppearancePrefs::default()),
            terminal: None,
            keyboard: None,
        };
        let json = serde_json::to_string(&patch).expect("serialize failed");
        let restored: PreferencesPatch = serde_json::from_str(&json).expect("deserialize failed");
        assert!(restored.appearance.is_some());
        assert!(restored.terminal.is_none());
    }
}
