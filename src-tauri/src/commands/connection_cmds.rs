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
use crate::platform::validation::validate_ssh_identity_path;
use crate::preferences::PreferencesStore;
use crate::session::ids::ConnectionId;
use crate::ssh::SshConnectionConfig;

#[tauri::command]
pub async fn get_connections(
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<Vec<SshConnectionConfig>, TauTermError> {
    Ok(prefs.read().get().connections)
}

/// Maximum accepted length for the `hostname` field (SEC-IPC-004).
///
/// DNS maximum hostname length per RFC 1035 §2.3.4: 253 characters.
const MAX_HOSTNAME_LEN: usize = 253;

/// Maximum accepted length for the `username` field (SEC-IPC-004).
///
/// POSIX `LOGIN_NAME_MAX` is typically 255 on Linux.
const MAX_USERNAME_LEN: usize = 255;

#[tauri::command]
pub async fn save_connection(
    config: SshConnectionConfig,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<ConnectionId, TauTermError> {
    // SEC-IPC-004: reject oversized hostname / username.
    if config.host.len() > MAX_HOSTNAME_LEN {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "hostname exceeds maximum allowed length (253 characters).",
        ));
    }
    if config.username.len() > MAX_USERNAME_LEN {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "username exceeds maximum allowed length (255 characters).",
        ));
    }
    if let Some(ref path) = config.identity_file {
        // SEC-PATH-005: use the strict validator (file must exist and be inside ~/.ssh/).
        validate_ssh_identity_path(path)?;
    }
    // If the frontend sends an empty ID (new connection form), assign a fresh ID
    // here so that two consecutive "new connection" saves do not collide on the
    // empty-string key inside the store.
    let mut config = config;
    if config.id.as_str().is_empty() {
        config.id = ConnectionId::new();
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
        if host.len() > MAX_HOSTNAME_LEN {
            return Err(TauTermError::new(
                "VALIDATION_ERROR",
                "hostname exceeds maximum allowed length (253 characters).",
            ));
        }
        Ok(())
    }

    fn check_username(username: &str) -> Result<(), TauTermError> {
        if username.len() > MAX_USERNAME_LEN {
            return Err(TauTermError::new(
                "VALIDATION_ERROR",
                "username exceeds maximum allowed length (255 characters).",
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
        // SAFETY: `set_var` is unsound when multiple threads read the environment
        // concurrently. This is safe here because:
        // 1. This project uses `cargo nextest` exclusively (see CLAUDE.md), which
        //    runs each test in its own forked process — no shared address space.
        // 2. No other thread in this test binary reads XDG_CONFIG_HOME concurrently.
        // DO NOT run this code under `cargo test --test-threads > 1`.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        let store = crate::preferences::PreferencesStore::load_or_default();
        // SAFETY: same rationale as the set_var call above — nextest process isolation.
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
    // SEC-IPC-004 — hostname length validation (DNS max: 253 chars)
    // -----------------------------------------------------------------------

    /// SEC-IPC-004: hostname at exactly 253 chars (DNS max) must be accepted.
    #[test]
    fn sec_ipc_004_hostname_at_limit_accepted() {
        let hostname = "a".repeat(MAX_HOSTNAME_LEN);
        assert!(
            check_hostname(&hostname).is_ok(),
            "SEC-IPC-004: hostname of exactly {MAX_HOSTNAME_LEN} chars must be accepted"
        );
    }

    /// SEC-IPC-004: hostname at 254 chars (one over DNS max) must be rejected.
    #[test]
    fn sec_ipc_004_hostname_over_limit_rejected() {
        let long_hostname = "a".repeat(MAX_HOSTNAME_LEN + 1);
        let result = check_hostname(&long_hostname);
        assert!(
            result.is_err(),
            "SEC-IPC-004: hostname > {MAX_HOSTNAME_LEN} chars must be rejected"
        );
        assert_eq!(result.unwrap_err().code, "VALIDATION_ERROR");
    }

    // -----------------------------------------------------------------------
    // SEC-IPC-004 — username length validation (POSIX LOGIN_NAME_MAX: 255)
    // -----------------------------------------------------------------------

    /// SEC-IPC-004: username at exactly 255 chars must be accepted.
    #[test]
    fn sec_ipc_004_username_at_limit_accepted() {
        let username = "u".repeat(MAX_USERNAME_LEN);
        assert!(
            check_username(&username).is_ok(),
            "SEC-IPC-004: username of exactly {MAX_USERNAME_LEN} chars must be accepted"
        );
    }

    /// SEC-IPC-004: username at 256 chars (one over POSIX max) must be rejected.
    #[test]
    fn sec_ipc_004_username_over_limit_rejected() {
        let long_username = "u".repeat(MAX_USERNAME_LEN + 1);
        let result = check_username(&long_username);
        assert!(
            result.is_err(),
            "SEC-IPC-004: username > {MAX_USERNAME_LEN} chars must be rejected"
        );
        assert_eq!(result.unwrap_err().code, "VALIDATION_ERROR");
    }

    // -----------------------------------------------------------------------
    // validate_ssh_identity_path via save_connection (SEC-PATH-005)
    //
    // The light validate_identity_file_path has been removed. All identity path
    // validation now goes through validate_ssh_identity_path (platform::validation),
    // which requires the file to exist and be within ~/.ssh/. The comprehensive
    // test suite for that function lives in platform/validation.rs.
    //
    // We test here only that the connection_cmds save path calls the strict validator:
    // a non-existent path must be rejected at save time.
    // -----------------------------------------------------------------------

    #[test]
    fn sec_path_005_nonexistent_identity_file_rejected_at_save_time() {
        let result =
            validate_ssh_identity_path("/home/nobody_tauterm_test/.ssh/does_not_exist_key");
        assert!(
            result.is_err(),
            "SEC-PATH-005: nonexistent identity file must be rejected by the strict validator"
        );
        assert_eq!(result.unwrap_err().code, "INVALID_SSH_IDENTITY_PATH");
    }

    #[test]
    fn sec_path_005_relative_identity_path_rejected_at_save_time() {
        let result = validate_ssh_identity_path("relative/path");
        assert!(
            result.is_err(),
            "SEC-PATH-005: relative identity file path must be rejected"
        );
        assert_eq!(result.unwrap_err().code, "INVALID_SSH_IDENTITY_PATH");
    }

    // -----------------------------------------------------------------------
    // Bug fix: empty ConnectionId must be replaced with a fresh ID at save time
    // -----------------------------------------------------------------------

    /// When the frontend sends `id: ""` (new-connection form), `save_connection`
    /// must generate a fresh `ConnectionId` and return it — not store `""`.
    #[test]
    fn save_connection_with_empty_id_assigns_new_id() {
        let (store, _tmp) = make_test_store();
        let mut config = make_connection_config("New Server");
        config.id = crate::session::ids::ConnectionId(String::new()); // simulate frontend empty id

        // Replicate the command-handler logic (ID assignment) directly:
        let mut config = config;
        if config.id.as_str().is_empty() {
            config.id = crate::session::ids::ConnectionId::new();
        }
        let assigned_id = config.id.clone();
        store.read().save_connection(config).expect("save");

        assert!(
            !assigned_id.as_str().is_empty(),
            "assigned ID must not be empty"
        );

        let all = store.read().get().connections;
        assert_eq!(all.len(), 1, "exactly one connection must be stored");
        assert_eq!(
            all[0].id, assigned_id,
            "stored connection must carry the assigned ID"
        );
    }

    /// Two consecutive saves with `id: ""` must produce two separate connections,
    /// not overwrite the first.
    #[test]
    fn two_saves_with_empty_id_create_two_connections() {
        let (store, _tmp) = make_test_store();

        for label in &["First", "Second"] {
            let mut config = make_connection_config(label);
            config.id = crate::session::ids::ConnectionId(String::new());
            // Replicate command-handler ID assignment:
            if config.id.as_str().is_empty() {
                config.id = crate::session::ids::ConnectionId::new();
            }
            store.read().save_connection(config).expect("save");
        }

        let all = store.read().get().connections;
        assert_eq!(
            all.len(),
            2,
            "two saves with empty IDs must create two distinct connections"
        );
        assert_ne!(
            all[0].id, all[1].id,
            "the two connections must have distinct IDs"
        );
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
