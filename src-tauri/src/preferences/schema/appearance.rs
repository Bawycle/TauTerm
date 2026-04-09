// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use super::language::Language;
use crate::preferences::types::{FontFamily, ThemeName};

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

impl CursorStyle {
    /// Convert to a DECSCUSR parameter value (steady variant; blinking controlled
    /// separately by `cursor_blink_ms`).
    ///
    /// Mapping:
    /// - `Block`     → 2 (steady block)
    /// - `Underline` → 4 (steady underline)
    /// - `Bar`       → 6 (steady bar)
    pub fn to_decscusr(self) -> u8 {
        match self {
            CursorStyle::Block => 2,
            CursorStyle::Underline => 4,
            CursorStyle::Bar => 6,
        }
    }
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
    pub font_family: FontFamily,
    pub font_size: f32,
    pub cursor_style: CursorStyle,
    /// Cursor blink period in milliseconds (FS-VT-032). Default: 530ms.
    pub cursor_blink_ms: u32,
    /// Name of the active theme.
    pub theme_name: ThemeName,
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
            font_family: FontFamily::monospace(),
            font_size: 14.0,
            cursor_style: CursorStyle::default(),
            cursor_blink_ms: 530,
            theme_name: ThemeName::umbra(),
            opacity: 1.0,
            language: Language::default(),
            context_menu_hint_shown: false,
            fullscreen: false,
        }
    }
}
