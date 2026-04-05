// SPDX-License-Identifier: MPL-2.0

//! Preferences store — load/save from disk with schema validation.
//!
//! Preferences are stored as JSON at `~/.config/tauterm/preferences.json`.
//! On load failure (corrupt file, missing fields), a logged fallback to
//! defaults is applied — this is an expected filesystem condition, not a
//! programming error (§9.1 of ARCHITECTURE.md).

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::error::PreferencesError;
use crate::preferences::schema::{Preferences, PreferencesPatch, UserTheme};

/// The preferences store — thread-safe, injected as `State<Arc<RwLock<PreferencesStore>>>`.
pub struct PreferencesStore {
    prefs: RwLock<Preferences>,
    path: PathBuf,
}

impl PreferencesStore {
    /// Load preferences from disk, falling back to defaults on any error.
    pub fn load() -> Result<Arc<RwLock<Self>>, PreferencesError> {
        let path = preferences_path()?;
        let prefs = load_from_disk(&path);
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
                let prefs = load_from_disk(&path);
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
    pub fn apply_patch(&self, patch: PreferencesPatch) -> Result<Preferences, PreferencesError> {
        let mut prefs = self.prefs.write();
        if let Some(appearance) = patch.appearance {
            prefs.appearance = appearance;
        }
        if let Some(terminal) = patch.terminal {
            prefs.terminal = terminal;
        }
        if let Some(keyboard) = patch.keyboard {
            prefs.keyboard = keyboard;
        }
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
    pub fn delete_theme(&self, name: &str) -> Result<(), PreferencesError> {
        let mut prefs = self.prefs.write();
        prefs.themes.retain(|t| t.name != name);
        let updated = prefs.clone();
        drop(prefs);
        self.save_to_disk(&updated)
    }

    /// Save or update an SSH connection config.
    pub fn save_connection(
        &self,
        config: crate::ssh::SshConnectionConfig,
    ) -> Result<(), PreferencesError> {
        let mut prefs = self.prefs.write();
        if let Some(existing) = prefs.connections.iter_mut().find(|c| c.id == config.id) {
            *existing = config;
        } else {
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
    // Internal helpers
    // -----------------------------------------------------------------------

    fn save_to_disk(&self, prefs: &Preferences) -> Result<(), PreferencesError> {
        let json = serde_json::to_string_pretty(prefs)
            .map_err(|e| PreferencesError::Parse(e.to_string()))?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

/// Determine the preferences file path.
fn preferences_path() -> Result<PathBuf, PreferencesError> {
    let config_dir = dirs_or_home()?;
    Ok(config_dir.join("tauterm").join("preferences.json"))
}

/// Load preferences from disk, returning defaults on any parse/IO error.
fn load_from_disk(path: &PathBuf) -> Preferences {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<Preferences>(&content) {
            Ok(prefs) => {
                tracing::info!("Loaded preferences from preferences.json");
                prefs
            }
            Err(e) => {
                tracing::warn!("Failed to parse preferences.json, using defaults: {e}",);
                Preferences::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!("No preferences file found, using defaults.");
            Preferences::default()
        }
        Err(e) => {
            tracing::warn!("Could not read preferences.json, using defaults: {e}",);
            Preferences::default()
        }
    }
}

/// Get the XDG config directory, falling back to `~/.config`.
fn dirs_or_home() -> Result<PathBuf, PreferencesError> {
    // Try XDG_CONFIG_HOME first, then fall back to $HOME/.config.
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        return Ok(dir);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| PreferencesError::Io(std::io::Error::other("HOME not set")))?;
    Ok(home.join(".config"))
}
