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
use std::sync::Arc;

use russh::keys::{HashAlg, PublicKeyBase64};
use tauri::AppHandle;

use crate::error::SshError;
use crate::events::{HostKeyPromptEvent, SshStateChangedEvent};
use crate::events::{emit_host_key_prompt, emit_ssh_state_changed};
use crate::session::ids::PaneId;
use crate::session::ssh_task::SshTaskHandle;
use crate::ssh::algorithms::check_server_key_algorithm;
use crate::ssh::known_hosts::{KnownHostLookup, KnownHostsStore};
use crate::ssh::{SshConnectionConfig, SshLifecycleState};

/// Write-side of an SSH PTY channel, shared between the connection and the pane.
///
/// Wrapped in `Arc<tokio::sync::Mutex<...>>` because:
/// - The read task needs the full `Channel` (via `wait()`).
/// - The write path needs it too (via `data()`).
///
/// Both hold the same Arc; writes are serialized by the async mutex.
pub type SshChannelArc = Arc<tokio::sync::Mutex<russh::Channel<russh::client::Msg>>>;

/// An active or pending SSH connection for one pane.
pub struct SshConnection {
    pub pane_id: PaneId,
    pub config: SshConnectionConfig,
    state: parking_lot::Mutex<SshLifecycleState>,
    /// The russh client handle — kept alive to prevent connection teardown.
    pub handle: tokio::sync::Mutex<Option<russh::client::Handle<TauTermSshHandler>>>,
    /// Shared SSH channel — `None` until the PTY channel is opened.
    pub channel: Option<SshChannelArc>,
    /// Handle to the async read task. Dropped to abort it.
    pub read_task: Option<SshTaskHandle>,
}

impl SshConnection {
    pub fn new(pane_id: PaneId, config: SshConnectionConfig) -> Self {
        Self {
            pane_id,
            config,
            state: parking_lot::Mutex::new(SshLifecycleState::Connecting),
            handle: tokio::sync::Mutex::new(None),
            channel: None,
            read_task: None,
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

    /// Send bytes to the remote PTY via the SSH channel.
    ///
    /// Returns `Err` if no channel is open or if the send fails.
    pub async fn send_input(&self, data: Vec<u8>) -> Result<(), SshError> {
        let channel = self
            .channel
            .as_ref()
            .ok_or_else(|| SshError::Connection("SSH channel not open.".to_string()))?;
        let ch = channel.lock().await;
        ch.data(data.as_slice())
            .await
            .map_err(|e| SshError::Connection(format!("channel write error: {e}")))
    }

    /// Send a window-resize request to the remote PTY.
    pub async fn resize(&self, cols: u16, rows: u16, px_w: u16, px_h: u16) -> Result<(), SshError> {
        let channel = self
            .channel
            .as_ref()
            .ok_or_else(|| SshError::Connection("SSH channel not open.".to_string()))?;
        let ch = channel.lock().await;
        ch.window_change(cols as u32, rows as u32, px_w as u32, px_h as u32)
            .await
            .map_err(|e| SshError::Connection(format!("window_change error: {e}")))
    }
}

// ---------------------------------------------------------------------------
// russh client handler
// ---------------------------------------------------------------------------

/// russh `Handler` implementation for TauTerm SSH sessions.
///
/// Holds the pane ID and `AppHandle` needed to emit events. The known-hosts
/// store path is passed at construction time.
///
/// `manager` is an `Arc<SshManager>` weak reference used to store pending host
/// key verifications in the manager's `pending_host_keys` map, so that
/// `accept_host_key` / `reject_host_key` command handlers can retrieve them.
pub struct TauTermSshHandler {
    pub pane_id: PaneId,
    /// Connection config ID — carried in TOFU events so the frontend can reopen
    /// the connection after the user accepts the host key.
    pub connection_id: crate::session::ids::ConnectionId,
    pub host: String,
    pub app: AppHandle,
    /// Path to TauTerm's known-hosts file. Uses `KnownHostsStore::default_path()`
    /// if not overridden (only overridden in tests).
    pub known_hosts_path: Option<std::path::PathBuf>,
    /// Weak reference to the SSH manager for storing pending host key prompts.
    pub manager: Option<std::sync::Weak<crate::ssh::manager::SshManager>>,
}

impl TauTermSshHandler {
    pub fn new(pane_id: PaneId, config: &SshConnectionConfig, app: AppHandle) -> Self {
        Self {
            pane_id,
            connection_id: config.id.clone(),
            host: config.host.to_string(),
            app,
            known_hosts_path: None,
            manager: None,
        }
    }

    /// Attach a weak reference to the SSH manager for pending host key storage.
    pub fn with_manager(mut self, manager: &Arc<crate::ssh::manager::SshManager>) -> Self {
        self.manager = Some(Arc::downgrade(manager));
        self
    }
}

/// Classify a russh disconnect reason into a lifecycle state and message.
///
/// Pure function extracted from `TauTermSshHandler::disconnected` so that the
/// mapping logic can be unit-tested without an `AppHandle` (TEST-SSH-UNIT-004).
///
/// Returns `(new_state, reason_message, is_error)`:
/// - `Closed`        / `false` — clean server-initiated disconnect (SSH_DISCONNECT).
/// - `Disconnected`  / `true`  — unexpected error: keepalive timeout, network drop, etc.
pub(crate) fn classify_disconnect_reason(
    reason: &russh::client::DisconnectReason<SshError>,
) -> (SshLifecycleState, Option<String>, bool) {
    match reason {
        russh::client::DisconnectReason::ReceivedDisconnect(info) => (
            SshLifecycleState::Closed,
            Some(format!("Server disconnected: {:?}", info.reason_code)),
            false,
        ),
        russh::client::DisconnectReason::Error(e) => {
            let reason_str = format!("Connection lost: {e:?}");
            (
                SshLifecycleState::Disconnected {
                    reason: Some(reason_str.clone()),
                },
                Some(reason_str),
                true,
            )
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
        let connection_id = self.connection_id.clone();
        let host = self.host.clone();
        let app = self.app.clone();
        let known_hosts_path = self.known_hosts_path.clone();
        let manager_weak = self.manager.clone();

        // Compute fingerprint (SHA-256) and key type for the prompt event.
        let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        let key_type = server_public_key.algorithm().as_str().to_string();

        // Encode raw key bytes for storage / comparison.
        // PublicKeyBase64::public_key_bytes() returns the wire-format bytes.
        let key_bytes: Vec<u8> = server_public_key.public_key_bytes();

        async move {
            // Emit a warning before the TOFU check so the user is informed even
            // if the connection is ultimately rejected (FS-SSH-014).
            check_server_key_algorithm(&app, &pane_id, &key_type);

            let store_path = known_hosts_path
                .or_else(KnownHostsStore::default_path)
                .ok_or_else(|| {
                    SshError::Connection("Cannot determine known_hosts path".to_string())
                })?;

            let store = KnownHostsStore::new(store_path);

            let lookup = store
                .lookup_with_system_fallback(&host, &key_type, &key_bytes, None)
                .map_err(|e| SshError::Connection(format!("known_hosts I/O error: {e}")))?;

            match lookup {
                KnownHostLookup::Trusted(_) => {
                    // Key matches — proceed silently.
                    Ok(true)
                }
                KnownHostLookup::Unknown => {
                    // First connection — store pending host key and emit TOFU prompt.
                    // The user must confirm via UI (`accept_host_key`) and reconnect.
                    if let Some(ref weak) = manager_weak
                        && let Some(mgr) = weak.upgrade()
                    {
                        mgr.pending_host_keys.insert(
                            pane_id.clone(),
                            crate::ssh::manager::PendingHostKey {
                                host: host.clone(),
                                key_type: key_type.clone(),
                                key_bytes: key_bytes.clone(),
                                is_mismatch: false,
                            },
                        );
                    }
                    emit_host_key_prompt(
                        &app,
                        HostKeyPromptEvent {
                            pane_id,
                            connection_id,
                            host,
                            key_type,
                            fingerprint,
                            is_changed: false,
                        },
                    );
                    Ok(false)
                }
                KnownHostLookup::Mismatch { .. } => {
                    // Key changed — potential MITM. Store pending key and emit warning.
                    // Default action is Reject (FS-SSH-011).
                    if let Some(ref weak) = manager_weak
                        && let Some(mgr) = weak.upgrade()
                    {
                        mgr.pending_host_keys.insert(
                            pane_id.clone(),
                            crate::ssh::manager::PendingHostKey {
                                host: host.clone(),
                                key_type: key_type.clone(),
                                key_bytes: key_bytes.clone(),
                                is_mismatch: true,
                            },
                        );
                    }
                    emit_host_key_prompt(
                        &app,
                        HostKeyPromptEvent {
                            pane_id,
                            connection_id,
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
            let (new_state, reason_str, is_error) = classify_disconnect_reason(&reason);

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
    use russh::client::{DisconnectReason, RemoteDisconnectInfo};

    // -----------------------------------------------------------------------
    // TEST-SSH-UNIT-004 — classify_disconnect_reason (FS-SSH-020)
    //
    // Context: russh owns the keepalive timer and the failure counter.
    // The application cannot intercept individual keepalive misses — it only
    // receives a final `disconnected(DisconnectReason::Error(...))` callback
    // after the configured number of consecutive misses (SSH_KEEPALIVE_MAX_MISSES).
    //
    // `tokio::time::pause()/advance()` is therefore not applicable here: there
    // is no applicative sleep to accelerate.
    //
    // What IS testable without AppHandle is the pure classification function
    // `classify_disconnect_reason`, which maps the disconnect reason to the
    // correct lifecycle state and error flag.  These tests cover all branches
    // of that function and verify that a keepalive-equivalent error reason
    // produces `Disconnected` (not `Closed`).
    // -----------------------------------------------------------------------

    /// A clean server-initiated disconnect maps to Closed, is_error = false.
    #[test]
    fn classify_received_disconnect_maps_to_closed() {
        let reason: DisconnectReason<SshError> =
            DisconnectReason::ReceivedDisconnect(RemoteDisconnectInfo {
                reason_code: russh::Disconnect::ByApplication,
                message: "user logout".to_string(),
                lang_tag: "en".to_string(),
            });
        let (state, msg, is_error) = classify_disconnect_reason(&reason);
        assert_eq!(
            state,
            SshLifecycleState::Closed,
            "ReceivedDisconnect must map to Closed (TEST-SSH-UNIT-004)"
        );
        assert!(!is_error, "ReceivedDisconnect must not be flagged as error");
        assert!(
            msg.as_deref().unwrap_or("").contains("ByApplication"),
            "reason message must include reason_code, got: {msg:?}"
        );
    }

    /// An unexpected transport error maps to Disconnected, is_error = true.
    /// This is the path taken by russh when keepalive misses exceed the maximum.
    #[test]
    fn classify_transport_error_maps_to_disconnected() {
        let reason: DisconnectReason<SshError> =
            DisconnectReason::Error(SshError::Connection("keepalive timeout".to_string()));
        let (state, msg, is_error) = classify_disconnect_reason(&reason);
        assert!(
            matches!(state, SshLifecycleState::Disconnected { .. }),
            "Transport error must map to Disconnected (TEST-SSH-UNIT-004 — keepalive path)"
        );
        assert!(
            is_error,
            "Transport error must be flagged as is_error = true"
        );
        assert!(
            msg.as_deref().unwrap_or("").contains("keepalive timeout"),
            "reason message must include error detail, got: {msg:?}"
        );
    }

    /// ConnectionLost disconnect code also maps to Closed (it is a ReceivedDisconnect).
    #[test]
    fn classify_received_disconnect_connection_lost_maps_to_closed() {
        let reason: DisconnectReason<SshError> =
            DisconnectReason::ReceivedDisconnect(RemoteDisconnectInfo {
                reason_code: russh::Disconnect::ConnectionLost,
                message: String::new(),
                lang_tag: String::new(),
            });
        let (state, _msg, is_error) = classify_disconnect_reason(&reason);
        assert_eq!(state, SshLifecycleState::Closed);
        assert!(!is_error);
    }

    /// Auth error (e.g. all methods failed) maps to Disconnected.
    #[test]
    fn classify_auth_error_maps_to_disconnected() {
        let reason: DisconnectReason<SshError> =
            DisconnectReason::Error(SshError::Auth("all methods failed".to_string()));
        let (state, _msg, is_error) = classify_disconnect_reason(&reason);
        assert!(matches!(state, SshLifecycleState::Disconnected { .. }));
        assert!(is_error);
    }

    fn make_config() -> SshConnectionConfig {
        use crate::preferences::types::{SshHost, SshLabel, SshUsername};
        SshConnectionConfig {
            id: ConnectionId::new(),
            label: SshLabel::try_from("test-server".to_string()).unwrap(),
            host: SshHost::try_from("192.168.1.1".to_string()).unwrap(),
            port: 22,
            username: SshUsername::try_from("admin".to_string()).unwrap(),
            identity_file: None,
            allow_osc52_write: false,
            keepalive_interval_secs: None,
            keepalive_max_failures: None,
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
        conn.set_state(SshLifecycleState::Disconnected { reason: None });
        assert!(matches!(
            conn.state(),
            SshLifecycleState::Disconnected { .. }
        ));
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
        conn.set_state(SshLifecycleState::Disconnected { reason: None });
        assert!(matches!(
            conn.state(),
            SshLifecycleState::Disconnected { .. }
        ));
        conn.set_state(SshLifecycleState::Closed);
        assert_eq!(conn.state(), SshLifecycleState::Closed);
    }

    #[test]
    fn ssh_lifecycle_state_serializes_with_type_tag() {
        let json = serde_json::to_string(&SshLifecycleState::Connected).expect("serialize failed");
        assert!(json.contains("\"type\":\"connected\""), "got: {json}");
    }

    /// `Disconnected { reason: Some(...) }` must serialize with `"type":"disconnected"`
    /// and a `"reason"` field so the frontend can display it.
    #[test]
    fn disconnected_with_reason_serializes_correctly() {
        let state = SshLifecycleState::Disconnected {
            reason: Some("keepalive timeout".to_string()),
        };
        let json = serde_json::to_string(&state).expect("serialize failed");
        assert!(
            json.contains("\"type\":\"disconnected\""),
            "type tag must be 'disconnected'; got: {json}"
        );
        assert!(
            json.contains("\"reason\":\"keepalive timeout\""),
            "reason must be present; got: {json}"
        );
    }

    /// `Disconnected { reason: None }` must serialize without a `"reason"` field
    /// (skip_serializing_if on Option — but this depends on serde derive for struct variants).
    #[test]
    fn disconnected_without_reason_serializes_with_null_reason() {
        let state = SshLifecycleState::Disconnected { reason: None };
        let json = serde_json::to_string(&state).expect("serialize failed");
        assert!(
            json.contains("\"type\":\"disconnected\""),
            "type tag must be 'disconnected'; got: {json}"
        );
    }
}
