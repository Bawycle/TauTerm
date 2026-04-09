// SPDX-License-Identifier: MPL-2.0

//! Preferences store — load/save from disk with schema validation.
//!
//! Preferences are stored as TOML at `~/.config/tauterm/preferences.toml`
//! using **snake_case** keys (e.g. `font_size`, `scrollback_lines`).  This
//! follows the TOML/POSIX convention for configuration files and is
//! independent of the IPC naming convention (camelCase JSON).
//!
//! The key-conversion bridge works as follows:
//! - **Save**: serialize `Preferences` via serde (camelCase) → `toml::Value`
//!   → rename keys camelCase → snake_case → `toml::to_string_pretty`.
//! - **Load**: `toml::from_str::<toml::Value>` (snake_case) → rename keys
//!   snake_case → camelCase → serialize back to TOML string → `toml::from_str`
//!   as `Preferences`.  The extra serialize/parse round-trip is negligible for
//!   the small preferences file (< 50 KB, startup-only).
//!
//! On load failure (corrupt file, missing fields), a logged fallback to
//! defaults is applied — this is an expected filesystem condition, not a
//! programming error (§9.1 of ARCHITECTURE.md).
//!
//! **Migration:** if `preferences.toml` is absent but `preferences.json`
//! exists (left over from a previous installation), the JSON file is parsed
//! (its camelCase keys are already compatible with serde) and its content is
//! used as the initial preferences.  The TOML file is written on the next
//! `save_to_disk` call.  The legacy JSON file is not deleted automatically.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::error::PreferencesError;
use crate::preferences::schema::{Preferences, PreferencesPatch, UserTheme};

mod io;
mod path;
mod schema_convert;
#[cfg(test)]
mod tests;
mod validation;

use io::load_from_disk;
use path::preferences_path;
use schema_convert::{camel_to_snake, rename_toml_keys};
use validation::validate_and_clamp;

/// Maximum number of saved SSH connections (SEC-PATH-005).
///
/// Prevents DoS via a malformed or adversarially crafted preferences file that
/// contains an unbounded list of connections, which would exhaust memory or
/// cause excessive IPC payload sizes.
const MAX_CONNECTIONS: usize = 1_000;

/// Names of built-in themes that must never be deleted by the user.
const BUILT_IN_THEME_NAMES: &[&str] = &["umbra", "solstice", "archipel"];

/// The preferences store — thread-safe, injected as `State<Arc<RwLock<PreferencesStore>>>`.
pub struct PreferencesStore {
    prefs: RwLock<Preferences>,
    path: PathBuf,
}

impl PreferencesStore {
    /// Load preferences from disk, falling back to defaults on any error.
    pub fn load() -> Result<Arc<RwLock<Self>>, PreferencesError> {
        let path = preferences_path()?;
        let mut prefs = load_from_disk(&path);
        validate_and_clamp(&mut prefs);
        Ok(Arc::new(RwLock::new(Self {
            prefs: RwLock::new(prefs),
            path,
        })))
    }

    /// Load preferences from the given path, falling back to `Preferences::default()`
    /// on ANY error (file not found, parse error, unknown field, etc.).
    ///
    /// This is the entry point for preference loading at startup (FS-PREF-001,
    /// FS-I18N-005). The `Language` serde deserializer remains strict (SEC-IPC-005);
    /// the fallback occurs here, at the store level, not inside serde.
    ///
    /// Returns a ready-to-use `Arc<RwLock<PreferencesStore>>` — never fails.
    pub fn load_or_default() -> Arc<RwLock<Self>> {
        match preferences_path() {
            Ok(path) => {
                let mut prefs = load_from_disk(&path);
                validate_and_clamp(&mut prefs);
                Arc::new(RwLock::new(Self {
                    prefs: RwLock::new(prefs),
                    path,
                }))
            }
            Err(e) => {
                tracing::warn!("Could not determine preferences path (using defaults): {e}");
                // Use a fallback path that will never be written to in this state.
                let fallback_path = PathBuf::from("/dev/null");
                Arc::new(RwLock::new(Self {
                    prefs: RwLock::new(Preferences::default()),
                    path: fallback_path,
                }))
            }
        }
    }

    /// Get a clone of the current preferences.
    pub fn get(&self) -> Preferences {
        self.prefs.read().clone()
    }

    /// Apply a partial update and persist to disk.
    ///
    /// `AppearancePatch` is merged field-by-field so that updating a single field
    /// (e.g. `language`) does not clobber the rest of `AppearancePrefs`.
    pub fn apply_patch(&self, patch: PreferencesPatch) -> Result<Preferences, PreferencesError> {
        let mut prefs = self.prefs.write();
        if let Some(patch) = patch.appearance {
            let a = &mut prefs.appearance;
            if let Some(v) = patch.font_family {
                a.font_family = v;
            }
            if let Some(v) = patch.font_size {
                a.font_size = v;
            }
            if let Some(v) = patch.cursor_style {
                a.cursor_style = v;
            }
            if let Some(v) = patch.cursor_blink_ms {
                a.cursor_blink_ms = v;
            }
            if let Some(v) = patch.theme_name {
                a.theme_name = v;
            }
            if let Some(v) = patch.opacity {
                a.opacity = v;
            }
            if let Some(v) = patch.language {
                a.language = v;
            }
            if let Some(v) = patch.context_menu_hint_shown {
                a.context_menu_hint_shown = v;
            }
            if let Some(v) = patch.fullscreen {
                a.fullscreen = v;
            }
        }
        if let Some(terminal) = patch.terminal {
            let t = &mut prefs.terminal;
            if let Some(v) = terminal.scrollback_lines {
                t.scrollback_lines = v;
            }
            if let Some(v) = terminal.allow_osc52_write {
                t.allow_osc52_write = v;
            }
            if let Some(v) = terminal.word_delimiters {
                t.word_delimiters = v;
            }
            if let Some(v) = terminal.bell_type {
                t.bell_type = v;
            }
            if let Some(v) = terminal.confirm_multiline_paste {
                t.confirm_multiline_paste = v;
            }
        }
        if let Some(keyboard) = patch.keyboard
            && let Some(v) = keyboard.bindings
        {
            prefs.keyboard.bindings = v;
        }
        validate_and_clamp(&mut prefs);
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)?;
        Ok(updated)
    }

    /// Get all user themes.
    pub fn get_themes(&self) -> Vec<UserTheme> {
        self.prefs.read().themes.clone()
    }

    /// Save or update a user theme.
    pub fn save_theme(&self, theme: UserTheme) -> Result<(), PreferencesError> {
        let mut prefs = self.prefs.write();
        if let Some(existing) = prefs.themes.iter_mut().find(|t| t.name == theme.name) {
            *existing = theme;
        } else {
            prefs.themes.push(theme);
        }
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)
    }

    /// Delete a user theme by name.
    ///
    /// Returns `Err(PreferencesError::Validation)` if `name` matches a built-in
    /// theme (`umbra`, `solstice`, `archipel`) — those are not user-owned and
    /// must never be removed.
    pub fn delete_theme(&self, name: &str) -> Result<(), PreferencesError> {
        if BUILT_IN_THEME_NAMES.contains(&name) {
            return Err(PreferencesError::Validation(format!(
                "Built-in theme '{name}' cannot be deleted"
            )));
        }
        let mut prefs = self.prefs.write();
        prefs.themes.retain(|t| t.name != name);
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)
    }

    /// Save or update an SSH connection config.
    ///
    /// Returns `Err(PreferencesError::Validation)` if saving a **new** connection
    /// would exceed `MAX_CONNECTIONS` (SEC-PATH-005).
    pub fn save_connection(
        &self,
        config: crate::ssh::SshConnectionConfig,
    ) -> Result<(), PreferencesError> {
        let mut prefs = self.prefs.write();
        if let Some(existing) = prefs.connections.iter_mut().find(|c| c.id == config.id) {
            // Updating an existing connection — no count check needed.
            *existing = config;
        } else {
            // New connection — enforce the limit before inserting.
            if prefs.connections.len() >= MAX_CONNECTIONS {
                return Err(PreferencesError::Validation(format!(
                    "Cannot save more than {MAX_CONNECTIONS} SSH connections."
                )));
            }
            prefs.connections.push(config);
        }
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)
    }

    /// Delete an SSH connection config by ID.
    pub fn delete_connection(
        &self,
        id: &crate::session::ids::ConnectionId,
    ) -> Result<(), PreferencesError> {
        let mut prefs = self.prefs.write();
        prefs.connections.retain(|c| &c.id != id);
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)
    }

    /// Duplicate an SSH connection config by ID (FS-SSH-033).
    ///
    /// Creates a copy of the connection with a fresh `ConnectionId` and a label
    /// suffixed with " (copy)". Returns the newly created config, or `None` if
    /// the source ID is not found.
    pub fn duplicate_connection(
        &self,
        id: &crate::session::ids::ConnectionId,
    ) -> Result<Option<crate::ssh::SshConnectionConfig>, PreferencesError> {
        let mut prefs = self.prefs.write();
        let Some(source) = prefs.connections.iter().find(|c| &c.id == id).cloned() else {
            return Ok(None);
        };
        let mut copy = source.clone();
        copy.id = crate::session::ids::ConnectionId::new();
        copy.label = format!("{} (copy)", source.label);
        prefs.connections.push(copy.clone());
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)?;
        Ok(Some(copy))
    }

    /// Persist the window full-screen state (FS-FULL-009).
    ///
    /// Called by `toggle_fullscreen` immediately after the OS transition so the
    /// preference survives application restarts.
    pub fn set_fullscreen(&self, value: bool) -> Result<(), PreferencesError> {
        let mut prefs = self.prefs.write();
        prefs.appearance.fullscreen = value;
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)
    }

    /// Mark that the context menu hint has been shown to the user.
    ///
    /// This is a one-way latch — it never resets to `false` once set.
    /// Idempotent: calling it multiple times is safe.
    pub fn mark_context_menu_used(&self) -> Result<(), PreferencesError> {
        let mut prefs = self.prefs.write();
        if prefs.appearance.context_menu_hint_shown {
            // Already set — avoid unnecessary disk write.
            return Ok(());
        }
        prefs.appearance.context_menu_hint_shown = true;
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)
    }

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    /// Construct a store backed by `path` with default preferences — for unit tests only.
    ///
    /// Avoids environment mutation (`XDG_CONFIG_HOME`) in inline `#[cfg(test)]` modules
    /// where nextest does not guarantee process-per-test isolation.
    #[cfg(test)]
    pub fn new_with_defaults(path: PathBuf) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            prefs: RwLock::new(Preferences::default()),
            path,
        }))
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn save_to_disk(&self, prefs: &Preferences) -> Result<(), PreferencesError> {
        // Serialize to toml::Value (keys are camelCase from #[serde(rename_all = "camelCase")])
        let camel_value =
            toml::Value::try_from(prefs).map_err(|e| PreferencesError::Parse(e.to_string()))?;
        // Convert keys to snake_case for conventional TOML config file format.
        // This is independent of the IPC naming convention (camelCase JSON).
        let snake_value = rename_toml_keys(camel_value, camel_to_snake);
        let toml_str = toml::to_string_pretty(&snake_value)
            .map_err(|e| PreferencesError::Parse(e.to_string()))?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Atomic write: write to a temp file then rename (ADR-0012, ADR-0016).
        // Prevents file corruption on power loss or process kill mid-write.
        let tmp_path = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp_path, toml_str)?;
        std::fs::rename(&tmp_path, &self.path)?;
        Ok(())
    }
}
