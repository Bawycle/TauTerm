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
        std::fs::write(&self.path, toml_str)?;
        Ok(())
    }
}

/// Determine the preferences file path (TOML format).
fn preferences_path() -> Result<PathBuf, PreferencesError> {
    let config_dir = dirs_or_home()?;
    Ok(config_dir.join("tauterm").join("preferences.toml"))
}

/// Load preferences from disk, returning defaults on any parse/IO error.
///
/// Load order:
/// 1. `preferences.toml` — primary format (snake_case keys).
/// 2. `preferences.json` — legacy fallback for migration; its camelCase keys
///    are directly compatible with the serde attributes on `Preferences`.
/// 3. If neither file exists, return `Preferences::default()`.
fn load_from_disk(path: &PathBuf) -> Preferences {
    // --- Primary: TOML (snake_case keys on disk) ---
    match std::fs::read_to_string(path) {
        Ok(content) => {
            return parse_toml_prefs(&content);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Fall through to JSON migration check.
        }
        Err(e) => {
            tracing::warn!("Could not read preferences.toml, using defaults: {e}");
            return Preferences::default();
        }
    }

    // --- Migration: JSON legacy file ---
    let json_path = path.with_extension("json");
    match std::fs::read_to_string(&json_path) {
        Ok(content) => match serde_json::from_str::<Preferences>(&content) {
            Ok(prefs) => {
                tracing::info!(
                    "Migrating preferences from preferences.json to preferences.toml \
                     (TOML will be written on next save)"
                );
                prefs
            }
            Err(e) => {
                tracing::warn!("Found preferences.json but failed to parse it, using defaults: {e}");
                Preferences::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!("No preferences file found, using defaults.");
            Preferences::default()
        }
        Err(e) => {
            tracing::warn!("Could not read preferences.json, using defaults: {e}");
            Preferences::default()
        }
    }
}

/// Parse a TOML string (with snake_case keys) into `Preferences`.
///
/// The on-disk format uses snake_case keys (TOML convention), while the Rust
/// structs use `#[serde(rename_all = "camelCase")]`.  We bridge this gap by:
/// 1. Parsing the TOML into a generic `toml::Value` (snake_case keys).
/// 2. Renaming all keys from snake_case to camelCase.
/// 3. Re-serializing the renamed `toml::Value` to a TOML string.
/// 4. Parsing that string as `Preferences` (serde now finds camelCase keys).
///
/// Steps 3–4 are a small overhead, but the file is tiny and startup-only.
fn parse_toml_prefs(content: &str) -> Preferences {
    let snake_value: toml::Value = match toml::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse preferences.toml, using defaults: {e}");
            return Preferences::default();
        }
    };
    let camel_value = rename_toml_keys(snake_value, snake_to_camel);
    // Re-serialize with camelCase keys, then parse as Preferences.
    let camel_toml = match toml::to_string(&camel_value) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to re-serialize preferences for camelCase pass, using defaults: {e}");
            return Preferences::default();
        }
    };
    match toml::from_str::<Preferences>(&camel_toml) {
        Ok(prefs) => {
            tracing::info!("Loaded preferences from preferences.toml");
            prefs
        }
        Err(e) => {
            tracing::warn!("Failed to deserialize preferences.toml after key rename, using defaults: {e}");
            Preferences::default()
        }
    }
}

/// Recursively rename all table keys in a `toml::Value` using `rename_fn`.
fn rename_toml_keys(value: toml::Value, rename_fn: fn(&str) -> String) -> toml::Value {
    match value {
        toml::Value::Table(table) => {
            let renamed = table
                .into_iter()
                .map(|(k, v)| (rename_fn(&k), rename_toml_keys(v, rename_fn)))
                .collect();
            toml::Value::Table(renamed)
        }
        toml::Value::Array(arr) => {
            toml::Value::Array(arr.into_iter().map(|v| rename_toml_keys(v, rename_fn)).collect())
        }
        other => other,
    }
}

/// Convert a camelCase identifier to snake_case.
///
/// Examples: `fontSize` → `font_size`, `scrollbackLines` → `scrollback_lines`.
fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.char_indices() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// Convert a snake_case identifier to camelCase.
///
/// Examples: `font_size` → `fontSize`, `scrollback_lines` → `scrollbackLines`.
fn snake_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut next_upper = false;
    for ch in s.chars() {
        if ch == '_' {
            next_upper = true;
        } else if next_upper {
            out.extend(ch.to_uppercase());
            next_upper = false;
        } else {
            out.push(ch);
        }
    }
    out
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

// ---------------------------------------------------------------------------
// Unit tests — key conversion functions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{camel_to_snake, snake_to_camel};

    // camel_to_snake

    #[test]
    fn camel_to_snake_simple() {
        assert_eq!(camel_to_snake("fontSize"), "font_size");
        assert_eq!(camel_to_snake("scrollbackLines"), "scrollback_lines");
        assert_eq!(camel_to_snake("cursorBlinkMs"), "cursor_blink_ms");
    }

    #[test]
    fn camel_to_snake_already_lower() {
        assert_eq!(camel_to_snake("language"), "language");
        assert_eq!(camel_to_snake("connections"), "connections");
    }

    #[test]
    fn camel_to_snake_empty() {
        assert_eq!(camel_to_snake(""), "");
    }

    #[test]
    fn camel_to_snake_single_char() {
        assert_eq!(camel_to_snake("x"), "x");
        assert_eq!(camel_to_snake("X"), "x");
    }

    #[test]
    fn camel_to_snake_leading_upper() {
        // Leading uppercase: no underscore before the first char.
        assert_eq!(camel_to_snake("FontSize"), "font_size");
    }

    // snake_to_camel

    #[test]
    fn snake_to_camel_simple() {
        assert_eq!(snake_to_camel("font_size"), "fontSize");
        assert_eq!(snake_to_camel("scrollback_lines"), "scrollbackLines");
        assert_eq!(snake_to_camel("cursor_blink_ms"), "cursorBlinkMs");
    }

    #[test]
    fn snake_to_camel_already_flat() {
        assert_eq!(snake_to_camel("language"), "language");
        assert_eq!(snake_to_camel("connections"), "connections");
    }

    #[test]
    fn snake_to_camel_empty() {
        assert_eq!(snake_to_camel(""), "");
    }

    #[test]
    fn snake_to_camel_single_char() {
        assert_eq!(snake_to_camel("x"), "x");
    }

    // Roundtrip: camel → snake → camel

    #[test]
    fn conversion_roundtrip_camel_snake_camel() {
        let cases = [
            "fontSize",
            "scrollbackLines",
            "cursorBlinkMs",
            "fontFamily",
            "themeeName",
            "allowOsc52Write",
            "wordDelimiters",
            "contextMenuHintShown",
        ];
        for original in cases {
            let snake = camel_to_snake(original);
            let restored = snake_to_camel(&snake);
            assert_eq!(
                restored, original,
                "roundtrip failed for '{original}': snake='{snake}', restored='{restored}'"
            );
        }
    }
}
