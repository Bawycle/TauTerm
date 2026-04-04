// SPDX-License-Identifier: MPL-2.0

//! SSH connection state machine and russh client handler.
//!
//! Models the lifecycle of a single SSH session (§5.2 of ARCHITECTURE.md):
//! Connecting → Authenticating → Connected ↔ Disconnected | Closed.
//!
//! `TauTermSshHandler` implements `russh::client::Handler`. It holds the pane
//! context needed to emit `ssh-state-changed` events when the connection is
//! disconnected (keepalive timeout, network drop, remote close).
//!
//! The `KnownHostsStore` TOFU check is performed inside `check_server_key`.
//! On `Unknown` (first connection), a `host-key-prompt` event is emitted and
//! the connection is **rejected** — the frontend must call `accept_host_key`
//! before attempting to connect again. This is correct TOFU behavior: do not
//! accept silently.

use std::future::Future;

use russh::keys::{HashAlg, PublicKeyBase64};
use tauri::AppHandle;

use crate::error::SshError;
use crate::events::{emit_host_key_prompt, emit_ssh_state_changed};
use crate::events::{HostKeyPromptEvent, SshStateChangedEvent};
use crate::session::ids::PaneId;
use crate::ssh::{SshConnectionConfig, SshLifecycleState};
use crate::ssh::known_hosts::{KnownHostLookup, KnownHostsStore};

/// An active or pending SSH connection for one pane.
pub struct SshConnection {
    pub pane_id: PaneId,
    pub config: SshConnectionConfig,
    state: parking_lot::Mutex<SshLifecycleState>,
}

impl SshConnection {
    pub fn new(pane_id: PaneId, config: SshConnectionConfig) -> Self {
        Self {
            pane_id,
            config,
            state: parking_lot::Mutex::new(SshLifecycleState::Connecting),
        }
    }

    /// Get the current lifecycle state.
    pub fn state(&self) -> SshLifecycleState {
        self.state.lock().clone()
    }

    /// Transition to a new state.
    pub fn set_state(&self, new_state: SshLifecycleState) {
        *self.state.lock() = new_state;
    }
}

// ---------------------------------------------------------------------------
// russh client handler
// ---------------------------------------------------------------------------

/// russh `Handler` implementation for TauTerm SSH sessions.
///
/// Holds the pane ID and `AppHandle` needed to emit events. The known-hosts
/// store path is passed at construction time.
pub struct TauTermSshHandler {
    pub pane_id: PaneId,
    pub host: String,
    pub app: AppHandle,
    /// Path to TauTerm's known-hosts file. Uses `KnownHostsStore::default_path()`
    /// if not overridden (only overridden in tests).
    pub known_hosts_path: Option<std::path::PathBuf>,
}

impl TauTermSshHandler {
    pub fn new(pane_id: PaneId, config: &SshConnectionConfig, app: AppHandle) -> Self {
        Self {
            pane_id,
            host: config.host.clone(),
            app,
            known_hosts_path: None,
        }
    }
}

impl russh::client::Handler for TauTermSshHandler {
    type Error = SshError;

    /// TOFU host key verification (FS-SSH-011).
    ///
    /// - `Unknown` (first connection): emit `host-key-prompt` event with
    ///   `is_changed = false`, then **reject** the connection. The user must
    ///   confirm via the UI (`accept_host_key` command) and reconnect.
    /// - `Trusted` (key matches stored): accept silently.
    /// - `Mismatch` (key changed): emit `host-key-prompt` with `is_changed = true`,
    ///   then **reject**. Acceptance requires a deliberate non-default UI action.
    fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let pane_id = self.pane_id.clone();
        let host = self.host.clone();
        let app = self.app.clone();
        let known_hosts_path = self.known_hosts_path.clone();

        // Compute fingerprint (SHA-256) and key type for the prompt event.
        let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        let key_type = server_public_key.algorithm().as_str().to_string();

        // Encode raw key bytes for storage / comparison.
        // PublicKeyBase64::public_key_bytes() returns the wire-format bytes.
        let key_bytes: Vec<u8> = server_public_key.public_key_bytes();

        async move {
            let store_path = known_hosts_path
                .or_else(KnownHostsStore::default_path)
                .ok_or_else(|| {
                    SshError::Connection("Cannot determine known_hosts path".to_string())
                })?;

            let store = KnownHostsStore::new(store_path);

            let lookup = store
                .lookup(&host, &key_type, &key_bytes)
                .map_err(|e| SshError::Connection(format!("known_hosts I/O error: {e}")))?;

            match lookup {
                KnownHostLookup::Trusted(_) => {
                    // Key matches — proceed silently.
                    Ok(true)
                }
                KnownHostLookup::Unknown => {
                    // First connection — emit TOFU prompt and reject.
                    // The user must confirm via UI and reconnect.
                    emit_host_key_prompt(
                        &app,
                        HostKeyPromptEvent {
                            pane_id,
                            host,
                            key_type,
                            fingerprint,
                            is_changed: false,
                        },
                    );
                    Ok(false)
                }
                KnownHostLookup::Mismatch { .. } => {
                    // Key changed — potential MITM. Emit warning and reject.
                    // Default action is Reject (FS-SSH-011).
                    emit_host_key_prompt(
                        &app,
                        HostKeyPromptEvent {
                            pane_id,
                            host,
                            key_type,
                            fingerprint,
                            is_changed: true,
                        },
                    );
                    Ok(false)
                }
            }
        }
    }

    /// Called when the connection is terminated.
    ///
    /// Emits a `ssh-state-changed` event:
    /// - `Closed` for a clean server-initiated disconnect.
    /// - `Disconnected` for unexpected errors (keepalive timeout, network drop).
    fn disconnected(
        &mut self,
        reason: russh::client::DisconnectReason<Self::Error>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let pane_id = self.pane_id.clone();
        let app = self.app.clone();

        async move {
            let (new_state, reason_str, is_error) = match &reason {
                russh::client::DisconnectReason::ReceivedDisconnect(info) => (
                    SshLifecycleState::Closed,
                    Some(format!("Server disconnected: {:?}", info.reason_code)),
                    false,
                ),
                russh::client::DisconnectReason::Error(e) => (
                    SshLifecycleState::Disconnected,
                    Some(format!("Connection lost: {e:?}")),
                    true,
                ),
            };

            emit_ssh_state_changed(
                &app,
                SshStateChangedEvent {
                    pane_id,
                    state: new_state,
                    reason: reason_str,
                },
            );

            if is_error {
                // Re-propagate so russh's task join sees the error.
                match reason {
                    russh::client::DisconnectReason::Error(e) => Err(e),
                    russh::client::DisconnectReason::ReceivedDisconnect(_) => Ok(()),
                }
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ids::{ConnectionId, PaneId};

    fn make_config() -> SshConnectionConfig {
        SshConnectionConfig {
            id: ConnectionId::new(),
            label: "test-server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            identity_file: None,
            allow_osc52_write: false,
        }
    }

    // -----------------------------------------------------------------------
    // TEST-SSH-007 (partial) — SSH connection state machine transitions
    // FS-SSH-020
    // -----------------------------------------------------------------------

    #[test]
    fn new_connection_starts_in_connecting_state() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        assert_eq!(conn.state(), SshLifecycleState::Connecting);
    }

    #[test]
    fn set_state_transitions_to_authenticating() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Authenticating);
        assert_eq!(conn.state(), SshLifecycleState::Authenticating);
    }

    #[test]
    fn set_state_transitions_to_connected() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Connected);
        assert_eq!(conn.state(), SshLifecycleState::Connected);
    }

    #[test]
    fn set_state_transitions_to_disconnected() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Connected);
        conn.set_state(SshLifecycleState::Disconnected);
        assert_eq!(conn.state(), SshLifecycleState::Disconnected);
    }

    #[test]
    fn set_state_transitions_to_closed() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Closed);
        assert_eq!(conn.state(), SshLifecycleState::Closed);
    }

    #[test]
    fn full_lifecycle_sequence_transitions_correctly() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        // Connecting → Authenticating → Connected → Disconnected → Closed
        assert_eq!(conn.state(), SshLifecycleState::Connecting);
        conn.set_state(SshLifecycleState::Authenticating);
        assert_eq!(conn.state(), SshLifecycleState::Authenticating);
        conn.set_state(SshLifecycleState::Connected);
        assert_eq!(conn.state(), SshLifecycleState::Connected);
        conn.set_state(SshLifecycleState::Disconnected);
        assert_eq!(conn.state(), SshLifecycleState::Disconnected);
        conn.set_state(SshLifecycleState::Closed);
        assert_eq!(conn.state(), SshLifecycleState::Closed);
    }

    #[test]
    fn ssh_lifecycle_state_serializes_with_type_tag() {
        let json = serde_json::to_string(&SshLifecycleState::Connected).expect("serialize failed");
        assert!(json.contains("\"type\":\"connected\""), "got: {json}");
    }
}
