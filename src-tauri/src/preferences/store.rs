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
                tracing::info!("Loaded preferences from {}", path.display());
                prefs
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse preferences ({}), using defaults: {e}",
                    path.display()
                );
                Preferences::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!("No preferences file found, using defaults.");
            Preferences::default()
        }
        Err(e) => {
            tracing::warn!(
                "Could not read preferences file ({}), using defaults: {e}",
                path.display()
            );
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
