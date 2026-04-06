// SPDX-License-Identifier: MPL-2.0

//! SSH connection config management Tauri commands.
//!
//! Commands: get_connections, save_connection, update_connection, delete_connection,
//!           duplicate_connection.
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

/// Maximum accepted length for `hostname` and `username` fields (SEC-IPC-004).
const MAX_FIELD_LEN: usize = 10_000;

#[tauri::command]
pub async fn save_connection(
    config: SshConnectionConfig,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<ConnectionId, TauTermError> {
    // SEC-IPC-004: reject oversized hostname / username.
    if config.host.len() > MAX_FIELD_LEN {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "hostname exceeds maximum allowed length.",
        ));
    }
    if config.username.len() > MAX_FIELD_LEN {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "username exceeds maximum allowed length.",
        ));
    }
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
pub async fn duplicate_connection(
    connection_id: ConnectionId,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<SshConnectionConfig, TauTermError> {
    prefs
        .read()
        .duplicate_connection(&connection_id)
        .map_err(|e| {
            TauTermError::with_detail(
                "PREFERENCES_ERROR",
                "Failed to duplicate connection.",
                e.to_string(),
            )
        })?
        .ok_or_else(|| {
            TauTermError::new(
                "CONNECTION_NOT_FOUND",
                "The specified SSH connection was not found.",
            )
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers — exercise validation logic without a live Tauri runtime.
    // -----------------------------------------------------------------------

    fn check_hostname(host: &str) -> Result<(), TauTermError> {
        if host.len() > MAX_FIELD_LEN {
            return Err(TauTermError::new(
                "VALIDATION_ERROR",
                "hostname exceeds maximum allowed length.",
            ));
        }
        Ok(())
    }

    fn check_username(username: &str) -> Result<(), TauTermError> {
        if username.len() > MAX_FIELD_LEN {
            return Err(TauTermError::new(
                "VALIDATION_ERROR",
                "username exceeds maximum allowed length.",
            ));
        }
        Ok(())
    }

    /// Create an isolated `PreferencesStore` backed by a fresh temp directory.
    ///
    /// Uses `XDG_CONFIG_HOME` override. Each call returns a completely empty store
    /// so tests are independent of the user's real preferences file.
    fn make_test_store() -> (
        std::sync::Arc<parking_lot::RwLock<crate::preferences::PreferencesStore>>,
        tempfile::TempDir,
    ) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let orig_xdg = std::env::var_os("XDG_CONFIG_HOME");
        // SAFETY: nextest runs each test in its own process.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        let store = crate::preferences::PreferencesStore::load_or_default();
        // SAFETY: same as above.
        unsafe {
            match orig_xdg {
                Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
        // Return TempDir so it stays alive for the test duration.
        (store, tmp)
    }

    fn make_connection_config(label: &str) -> crate::ssh::SshConnectionConfig {
        crate::ssh::SshConnectionConfig {
            id: ConnectionId::new(),
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

    // -----------------------------------------------------------------------
    // SEC-IPC-004 — oversized hostname / username rejected
    // -----------------------------------------------------------------------

    /// SEC-IPC-004: hostname > 10 000 chars must be rejected.
    #[test]
    fn sec_ipc_004_hostname_over_limit_rejected() {
        let long_hostname = "a".repeat(MAX_FIELD_LEN + 1);
        let result = check_hostname(&long_hostname);
        assert!(
            result.is_err(),
            "SEC-IPC-004: hostname > {MAX_FIELD_LEN} chars must be rejected"
        );
        assert_eq!(result.unwrap_err().code, "VALIDATION_ERROR");
    }

    /// SEC-IPC-004: hostname at exactly MAX_FIELD_LEN chars must be accepted.
    #[test]
    fn sec_ipc_004_hostname_at_limit_accepted() {
        let hostname = "a".repeat(MAX_FIELD_LEN);
        assert!(
            check_hostname(&hostname).is_ok(),
            "SEC-IPC-004: hostname of exactly {MAX_FIELD_LEN} chars must be accepted"
        );
    }

    /// SEC-IPC-004: username > 10 000 chars must be rejected.
    #[test]
    fn sec_ipc_004_username_over_limit_rejected() {
        let long_username = "u".repeat(MAX_FIELD_LEN + 1);
        let result = check_username(&long_username);
        assert!(
            result.is_err(),
            "SEC-IPC-004: username > {MAX_FIELD_LEN} chars must be rejected"
        );
        assert_eq!(result.unwrap_err().code, "VALIDATION_ERROR");
    }

    /// SEC-IPC-004: username at exactly MAX_FIELD_LEN chars must be accepted.
    #[test]
    fn sec_ipc_004_username_at_limit_accepted() {
        let username = "u".repeat(MAX_FIELD_LEN);
        assert!(
            check_username(&username).is_ok(),
            "SEC-IPC-004: username of exactly {MAX_FIELD_LEN} chars must be accepted"
        );
    }

    // -----------------------------------------------------------------------
    // validate_identity_file_path unit tests (SEC-PATH-005)
    // -----------------------------------------------------------------------

    #[test]
    fn identity_file_path_null_byte_rejected() {
        let result = validate_identity_file_path("/home/user/.ssh/id\0_ed25519");
        assert!(result.is_err(), "Null byte in path must be rejected");
        assert_eq!(result.unwrap_err().code, "INVALID_PATH");
    }

    #[test]
    fn identity_file_path_relative_rejected() {
        let result = validate_identity_file_path("relative/path");
        assert!(result.is_err(), "Relative path must be rejected");
    }

    #[test]
    fn identity_file_path_with_parent_dir_rejected() {
        let result = validate_identity_file_path("/home/user/../.ssh/id_ed25519");
        assert!(result.is_err(), "'..' component must be rejected");
    }

    #[test]
    fn identity_file_path_valid_absolute_accepted() {
        let result = validate_identity_file_path("/home/user/.ssh/id_ed25519");
        assert!(result.is_ok(), "Valid absolute path must be accepted");
    }

    // -----------------------------------------------------------------------
    // FS-SSH-033 — duplicate_connection via PreferencesStore
    // -----------------------------------------------------------------------

    /// FS-SSH-033: duplicate_connection creates a copy with a new ID.
    #[test]
    fn fs_ssh_033_duplicate_connection_creates_copy_with_new_id() {
        let (store, _tmp) = make_test_store();
        let original = make_connection_config("My Server");
        let original_id = original.id.clone();
        store
            .read()
            .save_connection(original.clone())
            .expect("save");

        let copy = store
            .read()
            .duplicate_connection(&original_id)
            .expect("duplicate_connection must not fail")
            .expect("connection must be found");

        assert_ne!(copy.id, original_id, "Duplicate must have a different ID");
        assert_eq!(copy.host, original.host, "host must be copied");
        assert_eq!(copy.port, original.port, "port must be copied");
        assert_eq!(copy.username, original.username, "username must be copied");
    }

    /// FS-SSH-033: duplicate label is suffixed with " (copy)".
    #[test]
    fn fs_ssh_033_duplicate_connection_label_has_copy_suffix() {
        let (store, _tmp) = make_test_store();
        let original = make_connection_config("Production");
        let original_id = original.id.clone();
        store.read().save_connection(original).expect("save");

        let copy = store
            .read()
            .duplicate_connection(&original_id)
            .expect("duplicate_connection must not fail")
            .expect("connection must be found");

        assert_eq!(
            copy.label, "Production (copy)",
            "FS-SSH-033: duplicate label must be '<original> (copy)'"
        );
    }

    /// FS-SSH-033: duplicate_connection on unknown ID returns None.
    #[test]
    fn fs_ssh_033_duplicate_unknown_connection_returns_none() {
        let (store, _tmp) = make_test_store();
        let unknown_id = ConnectionId::new();
        let result = store
            .read()
            .duplicate_connection(&unknown_id)
            .expect("no I/O error expected");
        assert!(
            result.is_none(),
            "FS-SSH-033: duplicate of unknown ID must return None"
        );
    }

    /// FS-SSH-033: duplicated connection is stored in the preferences list.
    #[test]
    fn fs_ssh_033_duplicate_connection_is_persisted_in_store() {
        let (store, _tmp) = make_test_store();
        let original = make_connection_config("Staging");
        let original_id = original.id.clone();
        store.read().save_connection(original).expect("save");

        let copy = store
            .read()
            .duplicate_connection(&original_id)
            .expect("no error")
            .expect("found");

        let all = store.read().get().connections;
        assert_eq!(all.len(), 2, "Both original and copy must be in the store");
        assert!(
            all.iter().any(|c| c.id == copy.id),
            "Duplicate must be findable in the connections list"
        );
    }
}
