// SPDX-License-Identifier: MPL-2.0

//! Preferences schema — all user preferences and their default values.
//!
//! `Preferences` is the top-level struct persisted to `~/.config/tauterm/preferences.toml`.
//! All nested types implement `Serialize`, `Deserialize`, and `Default`.
//!
//! The `Language` field MUST be an enum — never a free `String` across IPC (FS-I18N-006,
//! CLAUDE.md constraint).

mod appearance;
mod keyboard;
mod language;
mod patch;
mod terminal;
mod theme;

#[cfg(test)]
mod tests;

pub use appearance::{AppearancePrefs, BellType, CursorStyle};
pub use keyboard::KeyboardPrefs;
pub use language::Language;
pub use patch::{AppearancePatch, KeyboardPatch, PreferencesPatch, TerminalPatch};
pub use terminal::TerminalPrefs;
pub use theme::UserTheme;

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
