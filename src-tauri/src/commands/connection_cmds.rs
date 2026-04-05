// SPDX-License-Identifier: MPL-2.0

//! SSH connection config management Tauri commands.
//!
//! Commands: get_connections, save_connection, update_connection, delete_connection.
//!
//! Connection configs are authoritative in `PreferencesStore` (§8.1).

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::State;

use crate::error::TauTermError;
use crate::preferences::PreferencesStore;
use crate::session::ids::ConnectionId;
use crate::ssh::SshConnectionConfig;

/// Validate an identity file path received over IPC (FINDING-004 / SEC-PATH-005).
///
/// Rules:
/// - Must be absolute (no relative paths that could resolve unexpectedly).
/// - Must not contain `..` components (path traversal prevention).
/// - Must not contain null bytes (null injection prevention).
fn validate_identity_file_path(path: &str) -> Result<(), TauTermError> {
    if path.contains('\0') {
        return Err(TauTermError::new(
            "INVALID_PATH",
            "Identity file path must not contain null bytes.",
        ));
    }
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err(TauTermError::new(
            "INVALID_PATH",
            "Identity file path must be absolute.",
        ));
    }
    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(TauTermError::new(
            "INVALID_PATH",
            "Identity file path must not contain '..' components.",
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_connections(
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<Vec<SshConnectionConfig>, TauTermError> {
    Ok(prefs.read().get().connections)
}

#[tauri::command]
pub async fn save_connection(
    config: SshConnectionConfig,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<ConnectionId, TauTermError> {
    if let Some(ref path) = config.identity_file {
        validate_identity_file_path(path)?;
    }
    let id = config.id.clone();
    prefs.read().save_connection(config).map_err(|e| {
        TauTermError::with_detail(
            "PREFERENCES_ERROR",
            "Failed to save connection.",
            e.to_string(),
        )
    })?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_connection(
    connection_id: ConnectionId,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<(), TauTermError> {
    prefs.read().delete_connection(&connection_id).map_err(|e| {
        TauTermError::with_detail(
            "PREFERENCES_ERROR",
            "Failed to delete connection.",
            e.to_string(),
        )
    })
}
