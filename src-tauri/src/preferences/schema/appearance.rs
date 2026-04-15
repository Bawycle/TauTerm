// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use super::language::Language;
use crate::preferences::types::{FontFamily, ThemeName};

// ---------------------------------------------------------------------------
// Cursor style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

impl CursorStyle {
    /// Convert to a DECSCUSR parameter value (blinking variant, FS-VT-030).
    ///
    /// Blink on/off is controlled at runtime by `cursor_blink` (DECSET ?12 /
    /// DECRST ?12). Using blinking codes here ensures that when an application
    /// resets via DECSCUSR 0 the terminal returns to the preferred blinking
    /// shape, consistent with the default `cursor_blink=true` in `VtProcessor`.
    ///
    /// Mapping:
    /// - `Block`     → 1 (blinking block)
    /// - `Underline` → 3 (blinking underline)
    /// - `Bar`       → 5 (blinking bar)
    pub fn to_decscusr(self) -> u8 {
        match self {
            CursorStyle::Block => 1,
            CursorStyle::Underline => 3,
            CursorStyle::Bar => 5,
        }
    }
}

// ---------------------------------------------------------------------------
// Bell type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
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
// Fullscreen chrome behavior
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FullscreenChromeBehavior {
    /// Tab bar and status bar auto-hide when entering fullscreen (default).
    #[default]
    AutoHide,
    /// Tab bar and status bar remain visible in fullscreen.
    AlwaysVisible,
}

/// Appearance-related preferences.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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
    /// Hide the mouse cursor while the user is typing in the terminal (UI-2).
    /// Restored immediately on `mousemove`. Default: `true`.
    #[serde(default = "default_hide_cursor_while_typing")]
    pub hide_cursor_while_typing: bool,
    /// Whether the pane title bar is visible. Default: `true`.
    #[serde(default = "default_show_pane_title_bar")]
    pub show_pane_title_bar: bool,
    /// How the tab bar and status bar behave in fullscreen mode.
    /// `#[serde(default)]` ensures existing preferences files without this field
    /// deserialize successfully with `FullscreenChromeBehavior::AutoHide`.
    #[serde(default)]
    pub fullscreen_chrome_behavior: FullscreenChromeBehavior,
}

fn default_hide_cursor_while_typing() -> bool {
    true
}

fn default_show_pane_title_bar() -> bool {
    true
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
            hide_cursor_while_typing: true,
            show_pane_title_bar: true,
            fullscreen_chrome_behavior: FullscreenChromeBehavior::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_pane_title_bar_defaults_to_true() {
        let prefs = AppearancePrefs::default();
        assert!(prefs.show_pane_title_bar);
    }
}
