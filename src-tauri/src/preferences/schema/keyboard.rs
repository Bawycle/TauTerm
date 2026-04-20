// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

/// Keyboard shortcut preferences.
#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct KeyboardPrefs {
    // Keybinding overrides — populated in the full keyboard integration pass.
    // Using a map of action → key combo.
    pub bindings: std::collections::HashMap<String, String>,
}
