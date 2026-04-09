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
            pending_passphrases: dashmap::DashMap::new(),
            pending_host_keys: dashmap::DashMap::new(),
            credential_manager: Arc::new(crate::credentials::CredentialManager::new()),
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
        // SEC-PATH-005: structural checks (absolute, no traversal, no control chars, ≤4096 bytes)
        // are already enforced by SshIdentityPath::try_from at IPC deserialization time.
        // validate_ssh_identity_path adds the runtime checks: file existence and ~/.ssh/ boundary.
        if let Some(ref key_path) = config.identity_file {
            crate::platform::validation::validate_ssh_identity_path(key_path)
                .map_err(|e| SshError::Auth(format!("invalid identity file path: {e}")))?;
        }

        let conn = SshConnection::new(pane_id.clone(), config.clone());
        self.connections.insert(pane_id.clone(), conn);

        // Notify the frontend immediately so it can show the connecting overlay
        // (UXD §7.5.2). The task will emit Authenticating once the handshake
        // completes. Without this event the frontend stays at null sshState
        // until Authenticating fires (after TCP + SSH handshake), which may be
        // too late to show any feedback before the credential dialog appears.
        crate::events::emit_ssh_state_changed(
            &app,
            crate::events::SshStateChangedEvent {
                pane_id: pane_id.clone(),
                state: SshLifecycleState::Connecting,
                reason: None,
            },
        );

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
                // Defensive cleanup: if connect_task failed while a credential or
                // passphrase prompt was in flight (transport error, timeout, etc.),
                // remove stale entries so they don't accumulate.
                manager.pending_credentials.remove(&task_pane_id);
                manager.pending_passphrases.remove(&task_pane_id);
                let reason_str = format!("Connection failed: {e}");
                crate::events::emit_ssh_state_changed(
                    &task_app,
                    crate::events::SshStateChangedEvent {
                        pane_id: task_pane_id,
                        state: SshLifecycleState::Disconnected {
                            reason: Some(reason_str.clone()),
                        },
                        reason: Some(reason_str),
                    },
                );
            }
        });

        Ok(())
    }

    /// Look up credentials for a reconnect attempt from the OS keychain.
    ///
    /// Returns `None` if the keychain is unavailable or if no password is stored
    /// for this connection — `open_connection_inner` will then emit a
    /// `credential-prompt` event and wait for the user to supply credentials.
    ///
    /// This method is `pub(super)` to allow unit testing without a live Tauri
    /// runtime (see `tests/manager_tests.rs`).
    pub(super) async fn resolve_reconnect_credentials(
        &self,
        config: &SshConnectionConfig,
    ) -> Option<Credentials> {
        if !self.credential_manager.is_available() {
            return None;
        }
        match self
            .credential_manager
            .get_password(&config.id.to_string(), &config.username)
            .await
        {
            Ok(Some(password)) => Some(Credentials {
                username: config.username.to_string(),
                password: Some(password),
                private_key_path: config.identity_file.as_deref().map(str::to_owned),
                save_in_keychain: false,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("Keychain lookup on reconnect failed: {e}");
                None
            }
        }
    }

    /// Reconnect a disconnected SSH session (FS-SSH-040).
    ///
    /// Retrieves the saved config from the existing connection entry, removes it,
    /// and re-inserts it in `Connecting` state. Credentials are resolved from the
    /// OS keychain via `resolve_reconnect_credentials` so the user is not prompted
    /// again if a password was previously stored (SEC-SSH-CH-007).
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

        // Attempt to resolve credentials from the keychain. If unavailable or no
        // password stored, `connect_task` will emit a `credential-prompt` event.
        // `is_reconnect = true` causes a `ssh-reconnected` separator event to be
        // emitted once the new session reaches Connected state (FS-SSH-042).
        let credentials = self.resolve_reconnect_credentials(&config).await;
        self.open_connection_inner(
            pane_id,
            &config,
            credentials,
            app,
            vt,
            cols,
            rows,
            registry,
            true,
        )
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

    /// Remove all tracking state for a pane without waiting for the underlying
    /// connection to close naturally.
    ///
    /// Used by `inject_ssh_disconnect` (e2e-testing only) to clean up before
    /// emitting a synthetic `Disconnected` event.  Any background `connect_task`
    /// that later calls `connections.remove` will silently find nothing to remove.
    #[cfg(feature = "e2e-testing")]
    pub fn purge_pane(&self, pane_id: &PaneId) {
        self.connections.remove(pane_id);
        self.pending_credentials.remove(pane_id);
    }
}
