// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

/// A user-defined color theme.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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
    #[serde(default)]
    pub line_height: Option<f32>,
}
