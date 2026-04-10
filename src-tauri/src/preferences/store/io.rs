// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use crate::preferences::schema::Preferences;

use super::MAX_CONNECTIONS;
use super::migration;
use super::schema_convert::{rename_toml_keys, snake_to_camel};

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
pub(super) fn load_from_disk(path: &PathBuf) -> Preferences {
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
        Ok(content) => parse_json_prefs(&content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("No preferences file found, using defaults.");
            None
        }
        Err(e) => {
            tracing::warn!("Could not read preferences.json, using defaults: {e}");
            None
        }
    };

    clamp_connections(migrated.unwrap_or_default())
}

/// Truncate the connections list to `MAX_CONNECTIONS` if it exceeds the limit
/// (SEC-PATH-005 — DoS via malformed prefs).
///
/// `identity_file` path sanitisation is no longer needed here: `SshIdentityPath`
/// serde deserialization rejects non-absolute or traversal paths at parse time,
/// so any `SshConnectionConfig` that reaches this function already carries a
/// structurally valid path (or `None`).
pub(super) fn clamp_connections(mut prefs: Preferences) -> Preferences {
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
/// structs use `#[serde(rename_all = "camelCase")]`. This function bridges the
/// gap by:
/// 1. Parsing the TOML string into a `toml::Value` (snake_case keys).
/// 2. Renaming all keys from snake_case to camelCase.
/// 3. Converting the renamed `toml::Value` to a `serde_json::Value`.
/// 4. Running the migration pipeline (`migration::migrate`).
/// 5. Deserializing the migrated `serde_json::Value` into `Preferences`.
///
/// The TOML → JSON → typed struct round-trip is a small overhead but the
/// preferences file is tiny (< 50 KB) and is only read at startup.
pub(super) fn parse_toml_prefs(content: &str) -> Preferences {
    let snake_value: toml::Value = match toml::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse preferences.toml, using defaults: {e}");
            return Preferences::default();
        }
    };
    let camel_toml_value = rename_toml_keys(snake_value, snake_to_camel);

    // Convert toml::Value (camelCase) → serde_json::Value so that migration
    // can operate on the canonical JSON representation.
    let json_value: serde_json::Value = match serde_json::to_value(&camel_toml_value) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "Failed to convert preferences TOML to JSON for migration, using defaults: {e}"
            );
            return Preferences::default();
        }
    };

    // Run the schema migration pipeline.
    let migrated = migration::migrate(json_value);

    // Deserialize the migrated JSON value into the typed Preferences struct.
    match serde_json::from_value::<Preferences>(migrated) {
        Ok(prefs) => {
            tracing::info!("Loaded preferences from preferences.toml");
            prefs
        }
        Err(e) => {
            tracing::warn!(
                "Failed to deserialize preferences after migration, using defaults: {e}"
            );
            Preferences::default()
        }
    }
}

/// Parse a JSON string (camelCase keys, legacy format) into `Preferences`.
///
/// Applies the same migration pipeline as the TOML path so that legacy JSON
/// preferences are also stamped with the current schema version on next save.
pub(super) fn parse_json_prefs(content: &str) -> Option<Preferences> {
    let json_value: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Found preferences.json but failed to parse it, using defaults: {e}");
            return None;
        }
    };
    let migrated = migration::migrate(json_value);
    match serde_json::from_value::<Preferences>(migrated) {
        Ok(p) => {
            tracing::info!(
                "Migrating preferences from preferences.json to preferences.toml \
                 (TOML will be written on next save)"
            );
            Some(p)
        }
        Err(e) => {
            tracing::warn!(
                "Found preferences.json but deserialization failed, using defaults: {e}"
            );
            None
        }
    }
}
