// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use super::language::Language;

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
