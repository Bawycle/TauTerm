// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::appearance::{BellType, CursorStyle, FullscreenChromeBehavior};
use super::language::Language;
use crate::preferences::types::{FontFamily, ThemeName, WordDelimiters};

/// Partial update for appearance preferences — only the fields provided are changed.
///
/// Using a dedicated patch type (instead of `Option<AppearancePrefs>` in `PreferencesPatch`)
/// allows field-by-field updates without read-before-write: e.g. changing the language
/// without knowing or sending the current font size.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppearancePatch {
    pub font_family: Option<FontFamily>,
    pub font_size: Option<f32>,
    pub cursor_style: Option<CursorStyle>,
    pub cursor_blink_ms: Option<u32>,
    pub theme_name: Option<ThemeName>,
    pub opacity: Option<f32>,
    pub language: Option<Language>,
    pub context_menu_hint_shown: Option<bool>,
    pub fullscreen: Option<bool>,
    pub hide_cursor_while_typing: Option<bool>,
    pub show_pane_title_bar: Option<bool>,
    pub fullscreen_chrome_behavior: Option<FullscreenChromeBehavior>,
}

/// Partial update for terminal preferences — only the fields provided are changed.
///
/// Mirrors `AppearancePatch`: field-level merging avoids overwriting unrelated
/// settings when, e.g., only `scrollback_lines` is changed from the UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminalPatch {
    pub scrollback_lines: Option<usize>,
    pub allow_osc52_write: Option<bool>,
    pub word_delimiters: Option<WordDelimiters>,
    pub bell_type: Option<BellType>,
    pub confirm_multiline_paste: Option<bool>,
}

/// Partial update for keyboard preferences — only the fields provided are changed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeyboardPatch {
    pub bindings: Option<HashMap<String, String>>,
}

/// A partial preferences update (only the fields the user changed).
/// All fields are optional so the frontend can send minimal payloads.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PreferencesPatch {
    pub appearance: Option<AppearancePatch>,
    pub terminal: Option<TerminalPatch>,
    pub keyboard: Option<KeyboardPatch>,
}
