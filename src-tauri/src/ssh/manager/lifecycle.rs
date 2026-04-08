// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::AppHandle;

use crate::error::SshError;
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::ssh::{SshConnectionConfig, SshLifecycleState, connection::SshConnection};
use crate::vt::VtProcessor;

use super::{Credentials, SshManager};

impl SshManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: dashmap::DashMap::new(),
            pending_credentials: dashmap::DashMap::new(),
            pending_host_keys: dashmap::DashMap::new(),
        })
    }

    /// Begin connecting an SSH session for the given pane.
    ///
    /// Returns immediately after inserting the connection in `Connecting` state.
    /// The actual TCP connect, handshake, and auth run in a spawned task.
    /// Results are communicated via `ssh-state-changed` events.
    ///
    /// `vt` — the pane's shared `VtProcessor`, used by the SSH read task to
    /// process terminal output and emit `screen-update` events.
    ///
    /// `is_reconnect` — when `true`, emits a `ssh-reconnected` separator event
    /// upon successful connection (FS-SSH-042).
    ///
    /// # Errors
    /// Returns `Err` only for synchronous precondition failures (duplicate pane,
    /// invalid key path). Transport/auth errors are delivered via events.
    #[allow(clippy::too_many_arguments)]
    pub async fn open_connection(
        self: &Arc<Self>,
        pane_id: PaneId,
        config: &SshConnectionConfig,
        credentials: Option<Credentials>,
        app: AppHandle,
        vt: Arc<RwLock<VtProcessor>>,
        cols: u16,
        rows: u16,
        registry: Arc<SessionRegistry>,
    ) -> Result<(), SshError> {
        self.open_connection_inner(
            pane_id,
            config,
            credentials,
            app,
            vt,
            cols,
            rows,
            registry,
            /* is_reconnect */ false,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn open_connection_inner(
        self: &Arc<Self>,
        pane_id: PaneId,
        config: &SshConnectionConfig,
        credentials: Option<Credentials>,
        app: AppHandle,
        vt: Arc<RwLock<VtProcessor>>,
        cols: u16,
        rows: u16,
        registry: Arc<SessionRegistry>,
        is_reconnect: bool,
    ) -> Result<(), SshError> {
        if self.connections.contains_key(&pane_id) {
            return Err(SshError::Connection(
                "A connection is already active for this pane.".to_string(),
            ));
        }

        // Validate the identity file path before any network activity.
        if let Some(ref key_path_str) = config.identity_file {
            crate::platform::validation::validate_ssh_identity_path(key_path_str)
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
            let result = manager
                .connect_task(
                    task_pane_id.clone(),
                    &config,
                    credentials,
                    task_app.clone(),
                    vt,
                    cols,
                    rows,
                    registry,
                    is_reconnect,
                )
                .await;

            if let Err(e) = result {
                manager.connections.remove(&task_pane_id);
                crate::events::emit_ssh_state_changed(
                    &task_app,
                    crate::events::SshStateChangedEvent {
                        pane_id: task_pane_id,
                        state: SshLifecycleState::Disconnected,
                        reason: Some(format!("Connection failed: {e}")),
                    },
                );
            }
        });

        Ok(())
    }

    /// Reconnect a disconnected SSH session (FS-SSH-040).
    ///
    /// Retrieves the saved config from the existing connection entry, removes it,
    /// and re-inserts it in `Connecting` state so the next `open_connection` call
    /// will succeed. The caller (command handler) must then call `open_connection`
    /// with fresh credentials retrieved from the OS keychain (SEC-SSH-CH-007 —
    /// no credential caching in memory).
    ///
    /// Returns the config that should be passed to the subsequent `open_connection`
    /// call.
    pub async fn reconnect(
        self: &Arc<Self>,
        pane_id: PaneId,
        app: AppHandle,
        vt: Arc<RwLock<VtProcessor>>,
        cols: u16,
        rows: u16,
        registry: Arc<SessionRegistry>,
    ) -> Result<(), SshError> {
        // Retrieve the config from the existing (disconnected) entry.
        let config = {
            let entry = self
                .connections
                .get(&pane_id)
                .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
            entry.config.clone()
        };

        // Remove the stale connection entry.
        self.connections.remove(&pane_id);
        // Also discard any stale pending host key for this pane.
        self.pending_host_keys.remove(&pane_id);

        // Re-open the connection. Credentials are None — the connect task will
        // prompt the user via the credential-prompt event if needed.
        // `is_reconnect = true` causes a `ssh-reconnected` separator event to be
        // emitted once the new session reaches Connected state (FS-SSH-042).
        self.open_connection_inner(pane_id, &config, None, app, vt, cols, rows, registry, true)
            .await
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
