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
///
/// After loading, the connections list is truncated to `MAX_CONNECTIONS` if it
/// exceeds that limit (SEC-PATH-005 — DoS via malformed prefs).
fn load_from_disk(path: &PathBuf) -> Preferences {
    // --- Primary: TOML (snake_case keys on disk) ---
    match std::fs::read_to_string(path) {
        Ok(content) => {
            return clamp_connections(parse_toml_prefs(&content));
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
    let migrated: Option<Preferences> = match std::fs::read_to_string(&json_path) {
        Ok(content) => match serde_json::from_str::<Preferences>(&content) {
            Ok(p) => {
                tracing::info!(
                    "Migrating preferences from preferences.json to preferences.toml \
                     (TOML will be written on next save)"
                );
                Some(p)
            }
            Err(e) => {
                tracing::warn!(
                    "Found preferences.json but failed to parse it, using defaults: {e}"
                );
                None
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!("No preferences file found, using defaults.");
            None
        }
        Err(e) => {
            tracing::warn!("Could not read preferences.json, using defaults: {e}");
            None
        }
    };

    clamp_connections(migrated.unwrap_or_default())
}

/// Truncate the connections list to `MAX_CONNECTIONS` if it exceeds the limit.
///
/// A malformed preferences file could contain an unbounded list of connections.
/// This guard prevents DoS via memory exhaustion or excessively large IPC payloads
/// (SEC-PATH-005).
fn clamp_connections(mut prefs: Preferences) -> Preferences {
    if prefs.connections.len() > MAX_CONNECTIONS {
        tracing::warn!(
            "preferences file contains {} connections, truncating to {MAX_CONNECTIONS} \
             (SEC-PATH-005)",
            prefs.connections.len()
        );
        prefs.connections.truncate(MAX_CONNECTIONS);
    }
    prefs
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
            tracing::warn!(
                "Failed to re-serialize preferences for camelCase pass, using defaults: {e}"
            );
            return Preferences::default();
        }
    };
    match toml::from_str::<Preferences>(&camel_toml) {
        Ok(prefs) => {
            tracing::info!("Loaded preferences from preferences.toml");
            prefs
        }
        Err(e) => {
            tracing::warn!(
                "Failed to deserialize preferences.toml after key rename, using defaults: {e}"
            );
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
        toml::Value::Array(arr) => toml::Value::Array(
            arr.into_iter()
                .map(|v| rename_toml_keys(v, rename_fn))
                .collect(),
        ),
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
// Unit tests — apply_patch merge semantics and key conversion functions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{MAX_CONNECTIONS, PreferencesStore, camel_to_snake, snake_to_camel};
    use crate::preferences::schema::{AppearancePatch, Language, PreferencesPatch, UserTheme};

    // -----------------------------------------------------------------------
    // apply_patch — AppearancePatch merge semantics
    // -----------------------------------------------------------------------

    /// apply_patch with only `language` set must not modify `font_size`.
    #[test]
    fn apply_patch_language_only_does_not_touch_font_size() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        let default_font_size = store.read().get().appearance.font_size;

        let patch = PreferencesPatch {
            appearance: Some(AppearancePatch {
                language: Some(Language::Fr),
                ..Default::default()
            }),
            ..Default::default()
        };
        let updated = store.read().apply_patch(patch).expect("apply_patch");

        assert_eq!(
            updated.appearance.language,
            Language::Fr,
            "language must be updated"
        );
        assert_eq!(
            updated.appearance.font_size, default_font_size,
            "font_size must be unchanged"
        );
    }

    /// apply_patch with only `font_size` set must not modify `language`.
    #[test]
    fn apply_patch_font_size_only_does_not_touch_language() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        let default_language = store.read().get().appearance.language;

        let patch = PreferencesPatch {
            appearance: Some(AppearancePatch {
                font_size: Some(22.0),
                ..Default::default()
            }),
            ..Default::default()
        };
        let updated = store.read().apply_patch(patch).expect("apply_patch");

        assert_eq!(
            updated.appearance.font_size, 22.0,
            "font_size must be updated"
        );
        assert_eq!(
            updated.appearance.language, default_language,
            "language must be unchanged"
        );
    }

    /// apply_patch with `appearance: None` must leave all appearance fields untouched.
    #[test]
    fn apply_patch_with_none_appearance_changes_nothing() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        let before = store.read().get().appearance.clone();

        let patch = PreferencesPatch::default(); // all None
        let updated = store.read().apply_patch(patch).expect("apply_patch");

        assert_eq!(updated.appearance.font_size, before.font_size);
        assert_eq!(updated.appearance.language, before.language);
        assert_eq!(updated.appearance.font_family, before.font_family);
        assert_eq!(updated.appearance.theme_name, before.theme_name);
        assert_eq!(updated.appearance.opacity, before.opacity);
    }

    // -----------------------------------------------------------------------
    // camel_to_snake
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // SEC-PATH-005 — connection limit
    // -----------------------------------------------------------------------

    fn make_connection(label: &str) -> crate::ssh::SshConnectionConfig {
        crate::ssh::SshConnectionConfig {
            id: crate::session::ids::ConnectionId::new(),
            label: label.to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "alice".to_string(),
            identity_file: None,
            allow_osc52_write: false,
            keepalive_interval_secs: None,
            keepalive_max_failures: None,
        }
    }

    /// SEC-PATH-005: save_connection must reject a new connection when the store
    /// already holds MAX_CONNECTIONS entries.
    #[test]
    fn sec_path_005_save_connection_rejected_when_limit_reached() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        // Fill up to exactly MAX_CONNECTIONS.
        for i in 0..MAX_CONNECTIONS {
            let conn = make_connection(&format!("conn-{i}"));
            store
                .read()
                .save_connection(conn)
                .expect("save should succeed while under limit");
        }

        // One more must be rejected.
        let overflow = make_connection("overflow");
        let result = store.read().save_connection(overflow);
        assert!(
            result.is_err(),
            "SEC-PATH-005: adding a {}-th connection must fail",
            MAX_CONNECTIONS + 1
        );
        match result.unwrap_err() {
            crate::error::PreferencesError::Validation(msg) => {
                assert!(
                    msg.contains("1000"),
                    "Error message should mention the limit, got: {msg}"
                );
            }
            other => panic!("Expected Validation error, got: {other:?}"),
        }
    }

    /// SEC-PATH-005: updating an existing connection when already at MAX_CONNECTIONS
    /// must succeed (it is not adding a new entry).
    #[test]
    fn sec_path_005_update_existing_connection_allowed_at_limit() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        // Fill to the limit, keeping the last connection's ID.
        let mut last_id = None;
        for i in 0..MAX_CONNECTIONS {
            let conn = make_connection(&format!("conn-{i}"));
            last_id = Some(conn.id.clone());
            store.read().save_connection(conn).expect("save");
        }

        // Update the last connection — must not fail (not a new entry).
        let mut updated = make_connection("updated-label");
        updated.id = last_id.unwrap();
        let result = store.read().save_connection(updated);
        assert!(
            result.is_ok(),
            "SEC-PATH-005: updating an existing connection at the limit must succeed"
        );
    }

    /// SEC-PATH-005: clamp_connections truncates an oversized list.
    #[test]
    fn sec_path_005_clamp_connections_truncates_oversized_list() {
        use super::clamp_connections;
        use crate::preferences::schema::Preferences;

        let mut prefs = Preferences::default();
        for i in 0..MAX_CONNECTIONS + 1 {
            prefs
                .connections
                .push(make_connection(&format!("conn-{i}")));
        }
        assert_eq!(prefs.connections.len(), MAX_CONNECTIONS + 1);

        let clamped = clamp_connections(prefs);
        assert_eq!(
            clamped.connections.len(),
            MAX_CONNECTIONS,
            "SEC-PATH-005: clamp_connections must truncate to MAX_CONNECTIONS"
        );
    }

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

    // -----------------------------------------------------------------------
    // Built-in theme protection
    // -----------------------------------------------------------------------

    fn make_user_theme(name: &str) -> UserTheme {
        let black = "#000000".to_string();
        UserTheme {
            name: name.to_string(),
            palette: std::array::from_fn(|_| black.clone()),
            foreground: black.clone(),
            background: black.clone(),
            cursor_color: black.clone(),
            selection_bg: black.clone(),
            line_height: None,
        }
    }

    /// delete_theme must reject the built-in "umbra" theme.
    #[test]
    fn test_delete_theme_rejects_builtin_umbra() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        let result = store.read().delete_theme("umbra");
        assert!(result.is_err(), "deleting built-in theme 'umbra' must fail");
        match result.unwrap_err() {
            crate::error::PreferencesError::Validation(msg) => {
                assert!(
                    msg.contains("umbra"),
                    "error message should mention 'umbra', got: {msg}"
                );
            }
            other => panic!("expected Validation error, got: {other:?}"),
        }
    }

    /// delete_theme must reject the built-in "solstice" theme.
    #[test]
    fn test_delete_theme_rejects_builtin_solstice() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        let result = store.read().delete_theme("solstice");
        assert!(
            result.is_err(),
            "deleting built-in theme 'solstice' must fail"
        );
        match result.unwrap_err() {
            crate::error::PreferencesError::Validation(msg) => {
                assert!(
                    msg.contains("solstice"),
                    "error message should mention 'solstice', got: {msg}"
                );
            }
            other => panic!("expected Validation error, got: {other:?}"),
        }
    }

    /// delete_theme must reject the built-in "archipel" theme.
    #[test]
    fn test_delete_theme_rejects_builtin_archipel() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        let result = store.read().delete_theme("archipel");
        assert!(
            result.is_err(),
            "deleting built-in theme 'archipel' must fail"
        );
        match result.unwrap_err() {
            crate::error::PreferencesError::Validation(msg) => {
                assert!(
                    msg.contains("archipel"),
                    "error message should mention 'archipel', got: {msg}"
                );
            }
            other => panic!("expected Validation error, got: {other:?}"),
        }
    }

    /// delete_theme must allow deleting a user-defined theme.
    #[test]
    fn test_delete_theme_allows_user_defined_theme() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = tmp.path().join("preferences.toml");
        let store = PreferencesStore::new_with_defaults(path);

        // Save a user-defined theme first.
        let theme = make_user_theme("my-theme");
        store
            .read()
            .save_theme(theme)
            .expect("save_theme must succeed");

        // Verify it is present.
        let themes = store.read().get_themes();
        assert!(
            themes.iter().any(|t| t.name == "my-theme"),
            "theme 'my-theme' must be present after save"
        );

        // Delete it — must succeed.
        let result = store.read().delete_theme("my-theme");
        assert!(
            result.is_ok(),
            "deleting user-defined theme 'my-theme' must succeed"
        );

        // Verify it is gone.
        let themes_after = store.read().get_themes();
        assert!(
            !themes_after.iter().any(|t| t.name == "my-theme"),
            "theme 'my-theme' must be absent after delete"
        );
    }
}
