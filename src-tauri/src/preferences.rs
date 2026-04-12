// SPDX-License-Identifier: MPL-2.0

//! Preferences module — user preferences persistence and schema.
//!
//! Preferences are stored at `~/.config/tauterm/preferences.json`.
//! The `PreferencesStore` is injected as `State<Arc<RwLock<PreferencesStore>>>`.

pub mod schema;
pub mod store;
pub mod types;
pub mod watcher;

pub use schema::{
    AppearancePatch, AppearancePrefs, BellType, CursorStyle, KeyboardPrefs, Language, Preferences,
    PreferencesPatch, TerminalPrefs, UserTheme,
};
pub use store::PreferencesStore;
pub use types::{
    FontFamily, SshHost, SshIdentityPath, SshLabel, SshUsername, ThemeName, WordDelimiters,
};
