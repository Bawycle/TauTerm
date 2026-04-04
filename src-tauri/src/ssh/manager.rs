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
