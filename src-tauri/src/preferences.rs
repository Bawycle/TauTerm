// SPDX-License-Identifier: MPL-2.0

//! Preferences module — user preferences persistence and schema.
//!
//! Preferences are stored at `~/.config/tauterm/preferences.json`.
//! The `PreferencesStore` is injected as `State<Arc<RwLock<PreferencesStore>>>`.

pub mod schema;
pub mod store;

pub use schema::{
    AppearancePrefs, BellType, CursorStyle, KeyboardPrefs, Language, Preferences, PreferencesPatch,
    TerminalPrefs, UserTheme,
};
pub use store::PreferencesStore;
