// SPDX-License-Identifier: MPL-2.0

//! SSH connection config management Tauri commands.
//!
//! Commands: get_connections, save_connection, update_connection, delete_connection,
//!           duplicate_connection, store_connection_password.
//!
//! Connection configs are authoritative in `PreferencesStore` (§8.1).

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::State;

use crate::credentials::CredentialManager;
use crate::error::TauTermError;
use crate::preferences::PreferencesStore;
use crate::session::ids::ConnectionId;
use crate::ssh::SshConnectionConfig;

#[tauri::command]
#[specta::specta]
pub async fn get_connections(
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<Vec<SshConnectionConfig>, TauTermError> {
    Ok(prefs.read().get().connections)
}

/// Maximum accepted length for the `username` field (SEC-IPC-004).
///
/// POSIX `LOGIN_NAME_MAX` is typically 255 on Linux.
/// Still used by `store_connection_password` which takes a raw `String` username
/// (not a `SshUsername` newtype) because the keychain API is not coupled to the
/// connection config schema.
const MAX_USERNAME_LEN: usize = 255;

/// Maximum accepted length for the `password` field (SEC-CRED-004).
pub(crate) const MAX_PASSWORD_LEN: usize = 4096;

/// Maximum accepted length for the `hostname` field (SEC-IPC-004).
///
/// DNS maximum hostname length per RFC 1035 §2.3.4: 253 characters.
/// Kept for test helpers below — hostname validation is handled by `SshHost::try_from`
/// at deserialization time in production paths.
#[cfg(test)]
const MAX_HOSTNAME_LEN: usize = 253;

#[tauri::command]
#[specta::specta]
pub async fn save_connection(
    config: SshConnectionConfig,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<ConnectionId, TauTermError> {
    // SEC-IPC-004: hostname and username length / format are validated upstream by
    // SshHost::try_from and SshUsername::try_from at serde deserialization time —
    // serde rejects invalid payloads before this handler is called. The manual
    // length checks that were here are therefore redundant and have been removed.
    //
    // SEC-PATH-005: Structural validation (absolute, no traversal, no control chars, ≤4096 bytes)
    // is enforced by SshIdentityPath::try_from at IPC deserialization time.
    // File existence and ~/.ssh/ boundary are checked at connection time in
    // lifecycle.rs::open_connection_inner. No existence check here — user must
    // be able to save a config before the key file exists on disk.
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

/// Returns the full `SshConnectionConfig` of the new connection rather than just its ID,
/// allowing the frontend to update its connection list in a single round-trip.
/// The IPC spec table lists `ConnectionId` as the return type, but returning the full
/// config is strictly better UX and avoids a redundant `get_connections` call.
#[tauri::command]
#[specta::specta]
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
#[specta::specta]
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

/// SEC-CRED-004: Store a connection password in the OS keychain.
///
/// Called by the frontend after `save_connection` succeeds, when the user has
/// entered a password in the connection form. The password is stored under the
/// key `tauterm:ssh:{connection_id}:{username}` via the platform credential store.
///
/// Validation rejects empty IDs, empty passwords, and passwords exceeding
/// `MAX_PASSWORD_LEN` bytes. Errors are non-fatal from the frontend's perspective
/// (keychain daemon may be unavailable), but are reported as typed errors so the
/// caller can decide.
#[tauri::command]
#[specta::specta]
pub async fn store_connection_password(
    connection_id: ConnectionId,
    username: String,
    password: String,
    cred_manager: State<'_, Arc<CredentialManager>>,
) -> Result<(), TauTermError> {
    if connection_id.as_str().is_empty() {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "connection_id must not be empty.",
        ));
    }
    if password.is_empty() {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "password must not be empty.",
        ));
    }
    if password.len() > MAX_PASSWORD_LEN {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "password exceeds maximum length.",
        ));
    }
    // SEC-IPC-004: same limit as save_connection to prevent keychain key corruption.
    if username.len() > MAX_USERNAME_LEN {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "username exceeds maximum allowed length.",
        ));
    }
    cred_manager
        .store_password(connection_id.as_str(), &username, &password)
        .await
        .map_err(|e| {
            TauTermError::with_detail(
                "CREDENTIAL_ERROR",
                "Failed to store password.",
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

    /// Replicates the validation logic of `store_connection_password` so it can
    /// be tested without a live Tauri runtime or a `State<_>` injection.
    fn check_store_password(connection_id: &str, password: &str) -> Result<(), TauTermError> {
        if connection_id.is_empty() {
            return Err(TauTermError::new(
                "VALIDATION_ERROR",
                "connection_id must not be empty.",
            ));
        }
        if password.is_empty() {
            return Err(TauTermError::new(
                "VALIDATION_ERROR",
                "password must not be empty.",
            ));
        }
        if password.len() > MAX_PASSWORD_LEN {
            return Err(TauTermError::new(
                "VALIDATION_ERROR",
                "password exceeds maximum length.",
            ));
        }
        Ok(())
    }

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
        use crate::preferences::types::{SshHost, SshLabel, SshUsername};
        crate::ssh::SshConnectionConfig {
            id: ConnectionId::new(),
            label: SshLabel::try_from(label.to_string()).unwrap(),
            host: SshHost::try_from("example.com".to_string()).unwrap(),
            port: 22,
            username: SshUsername::try_from("alice".to_string()).unwrap(),
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
    // SshIdentityPath structural validation at serde time (SEC-PATH-005)
    //
    // Structural checks (absolute path, no traversal, no control chars, ≤4096 bytes)
    // are now enforced by SshIdentityPath::try_from at IPC deserialization time.
    // File existence and ~/.ssh/ boundary remain checked at connection time in
    // lifecycle.rs::open_connection_inner.
    // -----------------------------------------------------------------------

    #[test]
    fn sec_path_005_traversal_identity_path_rejected_at_serde() {
        use crate::preferences::types::SshIdentityPath;
        assert!(
            SshIdentityPath::try_from("/home/user/../.ssh/id_rsa".to_string()).is_err(),
            "SEC-PATH-005: path with '..' traversal must be rejected by SshIdentityPath::try_from"
        );
    }

    #[test]
    fn sec_path_005_relative_identity_path_rejected_at_serde() {
        use crate::preferences::types::SshIdentityPath;
        assert!(
            SshIdentityPath::try_from("relative/path".to_string()).is_err(),
            "SEC-PATH-005: relative identity file path must be rejected by SshIdentityPath::try_from"
        );
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

    // -----------------------------------------------------------------------
    // SEC-CRED-004 — store_connection_password validation
    // -----------------------------------------------------------------------

    /// SEC-CRED-004: empty connection_id must be rejected with VALIDATION_ERROR.
    #[test]
    fn store_connection_password_empty_id_rejected() {
        let result = check_store_password("", "s3cr3t");
        assert!(result.is_err(), "empty connection_id must be rejected");
        assert_eq!(result.unwrap_err().code, "VALIDATION_ERROR");
    }

    /// SEC-CRED-004: empty password must be rejected with VALIDATION_ERROR.
    #[test]
    fn store_connection_password_empty_password_rejected() {
        let result = check_store_password("conn-abc", "");
        assert!(result.is_err(), "empty password must be rejected");
        assert_eq!(result.unwrap_err().code, "VALIDATION_ERROR");
    }

    /// SEC-CRED-004: password exceeding MAX_PASSWORD_LEN (4097 chars) must be rejected.
    #[test]
    fn store_connection_password_over_limit_rejected() {
        let long_password = "x".repeat(MAX_PASSWORD_LEN + 1);
        let result = check_store_password("conn-abc", &long_password);
        assert!(
            result.is_err(),
            "password > {MAX_PASSWORD_LEN} bytes must be rejected"
        );
        assert_eq!(result.unwrap_err().code, "VALIDATION_ERROR");
    }
}
