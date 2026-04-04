// SPDX-License-Identifier: MPL-2.0

//! SSH session manager — manages live SSH sessions indexed by pane ID.
//!
//! `SshManager` holds a `DashMap<PaneId, SshConnection>` for active connections.
//! It does **not** own saved connection configs — those live in `PreferencesStore`
//! under the `connections` sub-key. Command handlers retrieve configs from
//! `PreferencesStore` and pass them into `SshManager::open_connection` (§3.3).

use std::sync::Arc;

use dashmap::DashMap;

use crate::error::SshError;
use crate::session::ids::PaneId;
use crate::ssh::{SshConnectionConfig, SshLifecycleState, connection::SshConnection};

/// Manages all live SSH sessions.
pub struct SshManager {
    connections: DashMap<PaneId, SshConnection>,
}

/// Credentials for SSH authentication.
///
/// SECURITY: `Debug` is implemented manually to redact sensitive fields.
/// Never use `#[derive(Debug)]` on this struct — it would expose passwords in logs.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key_path: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &self.password.as_deref().map(|_| "<redacted>"))
            .field(
                "private_key_path",
                &self.private_key_path.as_deref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SEC-CRED-003 — Credentials::Debug redacts password and private_key_path
    // -----------------------------------------------------------------------

    /// SEC-CRED-003: Password must appear as "<redacted>" in Debug output, not in clear.
    #[test]
    fn sec_cred_003_password_redacted_in_debug_output() {
        let creds = Credentials {
            username: "alice".to_string(),
            password: Some("hunter2".to_string()),
            private_key_path: None,
        };
        let debug_str = format!("{:?}", creds);
        assert!(
            !debug_str.contains("hunter2"),
            "Password must not appear in Debug output (SEC-CRED-003). Got: {}",
            debug_str
        );
        assert!(
            debug_str.contains("<redacted>"),
            "Debug output must contain '<redacted>' for password (SEC-CRED-003). Got: {}",
            debug_str
        );
    }

    /// SEC-CRED-003: None password must not produce a false "redacted" marker but must
    /// still not leak anything unexpected.
    #[test]
    fn sec_cred_003_none_password_debug_output_safe() {
        let creds = Credentials {
            username: "alice".to_string(),
            password: None,
            private_key_path: None,
        };
        let debug_str = format!("{:?}", creds);
        // No password value at all — must not contain any real secret string.
        // The debug output for None is "None" which is safe.
        assert!(
            debug_str.contains("None"),
            "None password should appear as None in Debug"
        );
    }

    /// SEC-CRED-003: private_key_path must be redacted in Debug output (FINDING-001).
    /// The path may reveal filesystem layout or SSH key names — treat as sensitive.
    #[test]
    fn sec_cred_003_private_key_path_redacted_in_debug() {
        let creds = Credentials {
            username: "alice".to_string(),
            password: None,
            private_key_path: Some("/home/alice/.ssh/id_ed25519".to_string()),
        };
        let debug_str = format!("{:?}", creds);
        assert!(
            !debug_str.contains("/home/alice/.ssh/id_ed25519"),
            "private_key_path must NOT appear in Debug output (SEC-CRED-003 / FINDING-001). Got: {}",
            debug_str
        );
        assert!(
            debug_str.contains("<redacted>"),
            "Debug output must contain '<redacted>' for private_key_path (SEC-CRED-003). Got: {}",
            debug_str
        );
    }

    // -----------------------------------------------------------------------
    // SEC-CRED-004 — SshConnectionConfig does not contain password field
    // -----------------------------------------------------------------------

    /// SEC-CRED-004: SshConnectionConfig serialized to JSON must not contain
    /// any password or private key content — only a file path reference.
    #[test]
    fn sec_cred_004_ssh_connection_config_no_password_in_json() {
        use crate::session::ids::ConnectionId;
        use crate::ssh::SshConnectionConfig;

        let config = SshConnectionConfig {
            id: ConnectionId::new(),
            label: "My Server".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "alice".to_string(),
            identity_file: Some("/home/alice/.ssh/id_ed25519".to_string()),
            allow_osc52_write: false,
        };

        let json = serde_json::to_string(&config).expect("serialize failed");

        // No password field at all in the JSON.
        assert!(
            !json.contains("password"),
            "SshConnectionConfig JSON must not contain a 'password' field (SEC-CRED-004). Got: {}",
            json
        );
        // identity_file stores a path, not key content.
        assert!(
            json.contains("/home/alice/.ssh/id_ed25519"),
            "identity_file must store the path, not key content (SEC-CRED-004)"
        );
        // Confirm the field name is identityFile (camelCase) as per serde config.
        assert!(
            json.contains("identityFile"),
            "Field must serialize as identityFile (camelCase)"
        );
    }

    /// SEC-CRED-004: SshConnectionConfig with no identity_file must omit the field.
    #[test]
    fn sec_cred_004_ssh_connection_config_identity_file_skipped_when_none() {
        use crate::session::ids::ConnectionId;
        use crate::ssh::SshConnectionConfig;

        let config = SshConnectionConfig {
            id: ConnectionId::new(),
            label: "Password server".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "bob".to_string(),
            identity_file: None,
            allow_osc52_write: false,
        };

        let json = serde_json::to_string(&config).expect("serialize failed");
        assert!(
            !json.contains("identityFile"),
            "identityFile field must be omitted when None (skip_serializing_if)"
        );
    }
}

impl SshManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: DashMap::new(),
        })
    }

    /// Begin connecting an SSH session for the given pane.
    pub async fn open_connection(
        &self,
        pane_id: PaneId,
        config: &SshConnectionConfig,
        _credentials: Option<Credentials>,
    ) -> Result<(), SshError> {
        if self.connections.contains_key(&pane_id) {
            return Err(SshError::Connection(
                "A connection is already active for this pane.".to_string(),
            ));
        }
        let conn = SshConnection::new(pane_id.clone(), config.clone());
        self.connections.insert(pane_id, conn);
        // TODO: initiate async connect flow (TCP → russh handshake → auth).
        Ok(())
    }

    /// Close the SSH session for a pane.
    pub async fn close_connection(&self, pane_id: PaneId) -> Result<(), SshError> {
        self.connections
            .remove(&pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        // TODO: send channel close / TCP close.
        Ok(())
    }

    /// Reconnect a disconnected SSH session.
    pub async fn reconnect(&self, pane_id: PaneId) -> Result<(), SshError> {
        self.connections
            .get(&pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        // TODO: trigger reconnect state machine.
        Ok(())
    }

    /// Get the current lifecycle state of an SSH session.
    pub fn get_state(&self, pane_id: &PaneId) -> Option<SshLifecycleState> {
        self.connections.get(pane_id).map(|c| c.state())
    }

    /// Number of active connections currently tracked.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

#[cfg(test)]
mod manager_tests {
    use super::*;
    use crate::session::ids::ConnectionId;

    fn make_config(host: &str) -> SshConnectionConfig {
        SshConnectionConfig {
            id: ConnectionId::new(),
            label: "test".to_string(),
            host: host.to_string(),
            port: 22,
            username: "user".to_string(),
            identity_file: None,
            allow_osc52_write: false,
        }
    }

    // -----------------------------------------------------------------------
    // TEST-SSH-UNIT-001 steps 6-8 — SshManager guard conditions
    // -----------------------------------------------------------------------

    /// Duplicate open_connection for same pane_id must return an error.
    #[tokio::test]
    async fn ssh_manager_rejects_duplicate_pane_connection() {
        let manager = SshManager::new();
        let pane_id = PaneId::new();
        let config = make_config("host-a.example.com");

        // First open: succeeds.
        let result = manager
            .open_connection(pane_id.clone(), &config, None)
            .await;
        assert!(result.is_ok(), "First open must succeed");

        // Second open for same pane: must fail.
        let result2 = manager
            .open_connection(pane_id.clone(), &config, None)
            .await;
        assert!(
            result2.is_err(),
            "Duplicate open for same pane must be rejected (TEST-SSH-UNIT-001 step 6)"
        );
        // The connection count must stay at 1 (not 2).
        assert_eq!(manager.connection_count(), 1);
    }

    /// close_connection on unknown pane_id must return PaneNotFound.
    #[tokio::test]
    async fn ssh_manager_close_unknown_pane_returns_error() {
        let manager = SshManager::new();
        let unknown_pane = PaneId::new();

        let result = manager.close_connection(unknown_pane).await;
        assert!(
            result.is_err(),
            "close_connection on unknown pane must return error (TEST-SSH-UNIT-001 step 7)"
        );
        match result.unwrap_err() {
            SshError::PaneNotFound(_) => {}
            other => panic!("Expected PaneNotFound, got {other:?}"),
        }
    }

    /// reconnect on unknown pane_id must return PaneNotFound.
    #[tokio::test]
    async fn ssh_manager_reconnect_unknown_pane_returns_error() {
        let manager = SshManager::new();
        let unknown_pane = PaneId::new();

        let result = manager.reconnect(unknown_pane).await;
        assert!(
            result.is_err(),
            "reconnect on unknown pane must return error (TEST-SSH-UNIT-001 step 8)"
        );
        match result.unwrap_err() {
            SshError::PaneNotFound(_) => {}
            other => panic!("Expected PaneNotFound, got {other:?}"),
        }
    }

    /// open then close should result in zero connections.
    #[tokio::test]
    async fn ssh_manager_open_then_close_cleans_up() {
        let manager = SshManager::new();
        let pane_id = PaneId::new();
        let config = make_config("host-b.example.com");

        manager
            .open_connection(pane_id.clone(), &config, None)
            .await
            .expect("open must succeed");
        assert_eq!(manager.connection_count(), 1);

        manager
            .close_connection(pane_id)
            .await
            .expect("close must succeed");
        assert_eq!(
            manager.connection_count(),
            0,
            "connection map must be empty after close"
        );
    }

    /// get_state returns Some for an active pane, None for unknown.
    #[tokio::test]
    async fn ssh_manager_get_state_returns_none_for_unknown_pane() {
        let manager = SshManager::new();
        let unknown_pane = PaneId::new();
        assert!(
            manager.get_state(&unknown_pane).is_none(),
            "get_state for unknown pane must return None"
        );
    }

    /// New connection starts in Connecting state.
    #[tokio::test]
    async fn ssh_manager_new_connection_in_connecting_state() {
        let manager = SshManager::new();
        let pane_id = PaneId::new();
        let config = make_config("host-c.example.com");

        manager
            .open_connection(pane_id.clone(), &config, None)
            .await
            .expect("open must succeed");

        let state = manager.get_state(&pane_id);
        assert_eq!(
            state,
            Some(SshLifecycleState::Connecting),
            "New connection must start in Connecting state (FS-SSH-010)"
        );
    }
}
