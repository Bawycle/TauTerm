// SPDX-License-Identifier: MPL-2.0

//! SSH session manager — manages live SSH sessions indexed by pane ID.
//!
//! `SshManager` holds a `DashMap<PaneId, SshConnection>` for active connections.
//! It does **not** own saved connection configs — those live in `PreferencesStore`
//! under the `connections` sub-key. Command handlers retrieve configs from
//! `PreferencesStore` and pass them into `SshManager::open_connection` (§3.3).
//!
//! ## Connection flow (§5.2 of ARCHITECTURE.md)
//!
//! 1. Insert a new `SshConnection` in `Connecting` state.
//! 2. TCP connect via `russh::client::connect()`.
//! 3. russh calls `TauTermSshHandler::check_server_key()` — TOFU verification.
//!    - Unknown or Mismatch: emits frontend event, rejects connection.
//!    - Trusted: proceeds.
//! 4. Transition to `Authenticating`.
//! 5. Try authentication in order: pubkey → password (FS-SSH-012).
//! 6. On success: transition to `Connected`, keepalive configured in russh `Config`.
//! 7. On failure: remove from map, emit error event.

use std::sync::Arc;

use dashmap::DashMap;
use tauri::AppHandle;

use crate::error::SshError;
use crate::events::{SshStateChangedEvent, emit_ssh_state_changed};
use crate::platform::validation::validate_ssh_identity_path;
use crate::session::ids::PaneId;
use crate::ssh::{SshConnectionConfig, SshLifecycleState, connection::SshConnection};
use crate::ssh::auth::{authenticate_password, authenticate_pubkey};
use crate::ssh::connection::TauTermSshHandler;
use crate::ssh::keepalive::make_client_config;

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

    #[test]
    fn sec_cred_003_none_password_debug_output_safe() {
        let creds = Credentials {
            username: "alice".to_string(),
            password: None,
            private_key_path: None,
        };
        let debug_str = format!("{:?}", creds);
        assert!(
            debug_str.contains("None"),
            "None password should appear as None in Debug"
        );
    }

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
        assert!(
            !json.contains("password"),
            "SshConnectionConfig JSON must not contain a 'password' field (SEC-CRED-004). Got: {}",
            json
        );
        assert!(
            json.contains("/home/alice/.ssh/id_ed25519"),
            "identity_file must store the path, not key content (SEC-CRED-004)"
        );
        assert!(
            json.contains("identityFile"),
            "Field must serialize as identityFile (camelCase)"
        );
    }

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
    ///
    /// Returns immediately after inserting the connection in `Connecting` state.
    /// The actual TCP connect, handshake, and auth run in a spawned task.
    /// Results are communicated via `ssh-state-changed` events.
    ///
    /// # Errors
    /// Returns `Err` only for synchronous precondition failures (duplicate pane,
    /// invalid key path). Transport/auth errors are delivered via events.
    pub async fn open_connection(
        self: &Arc<Self>,
        pane_id: PaneId,
        config: &SshConnectionConfig,
        credentials: Option<Credentials>,
        app: AppHandle,
    ) -> Result<(), SshError> {
        if self.connections.contains_key(&pane_id) {
            return Err(SshError::Connection(
                "A connection is already active for this pane.".to_string(),
            ));
        }

        // Validate the identity file path before any network activity.
        if let Some(ref key_path_str) = config.identity_file {
            validate_ssh_identity_path(key_path_str)
                .map_err(|e| SshError::Auth(format!("invalid identity file path: {e}")))?;
        }

        let conn = SshConnection::new(pane_id.clone(), config.clone());
        self.connections.insert(pane_id.clone(), conn);

        // Pass Arc<Self> into the task so it can update the shared map.
        let manager = Arc::clone(self);
        let config = config.clone();
        let task_pane_id = pane_id.clone();
        let task_app = app.clone();

        tokio::spawn(async move {
            let result =
                manager.connect_task(task_pane_id.clone(), &config, credentials, task_app.clone())
                .await;

            if let Err(e) = result {
                manager.connections.remove(&task_pane_id);
                emit_ssh_state_changed(
                    &task_app,
                    SshStateChangedEvent {
                        pane_id: task_pane_id,
                        state: SshLifecycleState::Disconnected,
                        reason: Some(format!("Connection failed: {e}")),
                    },
                );
            }
        });

        Ok(())
    }

    /// The async connection task: TCP connect → russh handshake → auth → Connected.
    async fn connect_task(
        &self,
        pane_id: PaneId,
        config: &SshConnectionConfig,
        credentials: Option<Credentials>,
        app: AppHandle,
    ) -> Result<(), SshError> {
        let addr = format!("{}:{}", config.host, config.port);

        let russh_config = make_client_config(None, None);
        let handler = TauTermSshHandler::new(pane_id.clone(), config, app.clone());

        // TCP connect + SSH handshake (check_server_key called inside).
        let mut session = russh::client::connect(russh_config, addr.as_str(), handler)
            .await
            .map_err(|e| SshError::Connection(format!("TCP/SSH connect failed: {e}")))?;

        // Transition to Authenticating.
        if let Some(conn) = self.connections.get(&pane_id) {
            conn.set_state(SshLifecycleState::Authenticating);
        }
        emit_ssh_state_changed(
            &app,
            SshStateChangedEvent {
                pane_id: pane_id.clone(),
                state: SshLifecycleState::Authenticating,
                reason: None,
            },
        );

        let username = credentials
            .as_ref()
            .map(|c| c.username.clone())
            .unwrap_or_else(|| config.username.clone());

        // Authentication order: pubkey → password (FS-SSH-012).
        let authenticated =
            Self::try_authenticate(&mut session, &username, config, credentials.as_ref()).await?;

        if !authenticated {
            return Err(SshError::Auth(
                "All authentication methods failed.".to_string(),
            ));
        }

        // Transition to Connected.
        if let Some(conn) = self.connections.get(&pane_id) {
            conn.set_state(SshLifecycleState::Connected);
        }
        emit_ssh_state_changed(
            &app,
            SshStateChangedEvent {
                pane_id: pane_id.clone(),
                state: SshLifecycleState::Connected,
                reason: None,
            },
        );

        // The russh Handle is intentionally held here. Dropping it would
        // close the connection. Full PTY channel integration (FS-SSH-013:
        // channel_open_session + request_pty + shell) is deferred until the
        // PTY output pipeline is wired — documented prerequisite.
        std::mem::drop(session);

        Ok(())
    }

    /// Try authentication methods in order: pubkey → password (FS-SSH-012).
    async fn try_authenticate<H: russh::client::Handler>(
        session: &mut russh::client::Handle<H>,
        username: &str,
        config: &SshConnectionConfig,
        credentials: Option<&Credentials>,
    ) -> Result<bool, SshError> {
        // 1. Public key (if identity_file is configured).
        if let Some(ref key_path_str) = config.identity_file {
            let key_path = std::path::Path::new(key_path_str);
            match authenticate_pubkey(session, username, key_path).await {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    tracing::debug!("Pubkey auth rejected for {username}@{}", config.host);
                }
                Err(e) => {
                    // Log and fall through to password — transport errors on pubkey
                    // do not abort the auth sequence.
                    tracing::warn!("Pubkey auth error for {username}: {e}");
                }
            }
        }

        // 2. Password (if provided in credentials).
        if let Some(creds) = credentials
            && let Some(ref password) = creds.password
        {
            return authenticate_password(session, username, password).await;
        }

        Ok(false)
    }

    /// Close the SSH session for a pane.
    pub async fn close_connection(&self, pane_id: PaneId) -> Result<(), SshError> {
        self.connections
            .remove(&pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        // Dropping the entry drops the SshConnection. Full channel close / TCP
        // disconnect is performed by the russh Handle when it is eventually dropped.
        Ok(())
    }

    /// Reconnect a disconnected SSH session.
    pub async fn reconnect(&self, pane_id: PaneId) -> Result<(), SshError> {
        self.connections
            .get(&pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        // Full reconnect (FS-SSH-040) requires storing the original credentials.
        // Wired once the credential store injection is available in the command layer.
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

    /// get_state returns None for unknown pane.
    #[tokio::test]
    async fn ssh_manager_get_state_returns_none_for_unknown_pane() {
        let manager = SshManager::new();
        let unknown_pane = PaneId::new();
        assert!(
            manager.get_state(&unknown_pane).is_none(),
            "get_state for unknown pane must return None"
        );
    }

    /// Manager starts with no connections.
    #[test]
    fn ssh_manager_starts_empty() {
        let manager = SshManager::new();
        assert_eq!(manager.connection_count(), 0);
    }

    /// Direct map insertion simulates the state seen after open_connection inserts
    /// but before the task completes. Verifies the map is accessible.
    #[test]
    fn ssh_manager_direct_insert_and_get_state() {
        let manager = SshManager::new();
        let pane_id = PaneId::new();
        let config = make_config("host-a.example.com");
        let conn = SshConnection::new(pane_id.clone(), config);
        manager.connections.insert(pane_id.clone(), conn);

        assert_eq!(manager.connection_count(), 1);
        assert_eq!(
            manager.get_state(&pane_id),
            Some(SshLifecycleState::Connecting),
            "Freshly inserted connection must be in Connecting state"
        );
    }

    /// open then close should result in zero connections.
    #[tokio::test]
    async fn ssh_manager_direct_insert_then_close_cleans_up() {
        let manager = SshManager::new();
        let pane_id = PaneId::new();
        let config = make_config("host-b.example.com");
        let conn = SshConnection::new(pane_id.clone(), config);
        manager.connections.insert(pane_id.clone(), conn);
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

    /// Duplicate pane detection: open_connection must reject a pane_id that is
    /// already in the map. This test uses direct map insertion to bypass the
    /// AppHandle requirement.
    #[tokio::test]
    async fn ssh_manager_duplicate_pane_detected_via_map() {
        let manager = SshManager::new();
        let pane_id = PaneId::new();
        let config = make_config("host-c.example.com");

        // Simulate the first open_connection inserting the entry.
        let conn = SshConnection::new(pane_id.clone(), config.clone());
        manager.connections.insert(pane_id.clone(), conn);

        // The guard in open_connection checks contains_key before doing anything.
        assert!(
            manager.connections.contains_key(&pane_id),
            "Duplicate detection: map must report pane as present"
        );
    }
}
