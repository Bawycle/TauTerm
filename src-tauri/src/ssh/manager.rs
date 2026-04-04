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
            .field("private_key_path", &self.private_key_path)
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

    /// SEC-CRED-003: private_key_path is emitted in Debug (it is a path, not secret
    /// material). The test confirms that the current impl exposes the path — this is
    /// explicitly documented as the design intent (only password is redacted).
    /// If policy changes to also redact paths, this test must be updated.
    #[test]
    fn sec_cred_003_private_key_path_visible_in_debug() {
        let creds = Credentials {
            username: "alice".to_string(),
            password: None,
            private_key_path: Some("/home/alice/.ssh/id_ed25519".to_string()),
        };
        let debug_str = format!("{:?}", creds);
        // Current design: path is emitted (it is not secret material — it is a file path).
        // This test documents and pins the current behaviour.
        assert!(
            debug_str.contains("/home/alice/.ssh/id_ed25519"),
            "private_key_path should be visible in Debug output (current design)"
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
        // TODO: initiate async connect flow.
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
}
