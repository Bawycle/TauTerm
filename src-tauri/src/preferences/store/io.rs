// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use crate::preferences::schema::Preferences;

use super::MAX_CONNECTIONS;
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
/// structs use `#[serde(rename_all = "camelCase")]`.  We bridge this gap by:
/// 1. Parsing the TOML into a generic `toml::Value` (snake_case keys).
/// 2. Renaming all keys from snake_case to camelCase.
/// 3. Re-serializing the renamed `toml::Value` to a TOML string.
/// 4. Parsing that string as `Preferences` (serde now finds camelCase keys).
///
/// Steps 3–4 are a small overhead, but the file is tiny and startup-only.
pub(super) fn parse_toml_prefs(content: &str) -> Preferences {
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
