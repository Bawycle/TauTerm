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

pub use appearance::{AppearancePrefs, BellType, CursorStyle, FullscreenChromeBehavior};
pub use keyboard::KeyboardPrefs;
pub use language::Language;
pub use patch::{AppearancePatch, KeyboardPatch, PreferencesPatch, TerminalPatch};
pub use terminal::TerminalPrefs;
pub use theme::UserTheme;

use serde::{Deserialize, Serialize};

use crate::ssh::SshConnectionConfig;

/// Current schema version stamped into every saved preferences file.
///
/// Increment this constant and add a migration step in
/// `store::migration::migrate` whenever a structural or breaking change is made
/// to the `Preferences` schema.
pub const PREFS_SCHEMA_VERSION: u32 = 1;

/// Top-level user preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    /// Schema version stamped at save time (FS-PREF-schema, ARCH-3).
    /// Absence of the field (`None` after deserialization) is treated as v0
    /// by the migration layer. Defaults to 0 so the `Default` impl never needs
    /// to know the current version — the store stamps it on every save.
    #[serde(default)]
    pub schema_version: u32,
    pub appearance: AppearancePrefs,
    pub terminal: TerminalPrefs,
    pub keyboard: KeyboardPrefs,
    /// Saved SSH connections. Authoritative source — `SshManager` reads/writes these.
    pub connections: Vec<SshConnectionConfig>,
    /// User-defined themes.
    pub themes: Vec<UserTheme>,
}
