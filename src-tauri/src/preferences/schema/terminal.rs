// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use super::appearance::BellType;
use crate::preferences::types::WordDelimiters;

pub(super) fn default_confirm_multiline_paste() -> bool {
    true
}

/// Terminal behavior preferences.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminalPrefs {
    /// Maximum scrollback lines per pane.
    #[specta(type = f64)]
    pub scrollback_lines: usize,
    /// Allow OSC 52 clipboard write for local sessions.
    pub allow_osc52_write: bool,
    /// Characters treated as word delimiters for double-click selection.
    pub word_delimiters: WordDelimiters,
    /// Bell notification type.
    pub bell_type: BellType,
    /// Show a confirmation dialog before pasting multi-line text when bracketed
    /// paste is inactive (FS-CLIP-009). Default: `true`. The user can disable
    /// it via the "Don't ask again" toggle in the paste dialog.
    #[serde(default = "default_confirm_multiline_paste")]
    pub confirm_multiline_paste: bool,
}

impl Default for TerminalPrefs {
    fn default() -> Self {
        Self {
            scrollback_lines: 10_000,
            allow_osc52_write: false,
            word_delimiters: WordDelimiters::try_from(r#" \t|"'`&()*,;<=>[]{}~"#.to_string())
                .expect("default word_delimiters is valid"),
            bell_type: BellType::default(),
            confirm_multiline_paste: true,
        }
    }
}
