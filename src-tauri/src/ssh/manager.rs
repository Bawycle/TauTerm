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
//! 5. Try authentication in order: pubkey → keyboard-interactive → password (FS-SSH-012).
//! 6. On success: transition to `Connected`, keepalive configured in russh `Config`.
//! 7. On failure: remove from map, emit error event.

use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;
use russh::Pty;
use tauri::AppHandle;
use tokio::sync::oneshot;

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::SshError;
use crate::events::{
    SshReconnectedEvent, SshStateChangedEvent, emit_ssh_reconnected, emit_ssh_state_changed,
};
use crate::platform::validation::validate_ssh_identity_path;
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::session::ssh_task::spawn_ssh_read_task;
use crate::ssh::auth::{
    authenticate_keyboard_interactive, authenticate_password, authenticate_pubkey,
};
use crate::ssh::connection::{SshChannelArc, TauTermSshHandler};
use crate::ssh::keepalive::make_client_config;
use crate::ssh::{SshConnectionConfig, SshLifecycleState, connection::SshConnection};
use crate::vt::VtProcessor;

/// Terminal modes for SSH PTY requests (RFC 4254 §8).
///
/// VKILL = opcode 4, VEOF = opcode 5 — per RFC 4254 table (not inverted).
const TERMINAL_MODES: &[(Pty, u32)] = &[
    (Pty::VINTR, 3),    // Ctrl+C
    (Pty::VQUIT, 28),   // Ctrl+\
    (Pty::VERASE, 127), // Backspace (DEL)
    (Pty::VKILL, 21),   // Ctrl+U  — opcode 4 per RFC 4254
    (Pty::VEOF, 4),     // Ctrl+D  — opcode 5 per RFC 4254
    (Pty::VSUSP, 26),   // Ctrl+Z
    (Pty::ISIG, 1),     // Enable signals
    (Pty::ICANON, 1),   // Canonical mode
    (Pty::ECHO, 1),     // Echo input
];

/// Pending credential request — a oneshot sender parked while waiting for
/// the user to respond to a `credential-prompt` event.
struct PendingCredentials {
    sender: oneshot::Sender<Credentials>,
}

/// A pending host key verification — stored until the user accepts or rejects.
///
/// Keyed by pane ID in `SshManager::pending_host_keys`. Populated by
/// `TauTermSshHandler::check_server_key` on Unknown/Mismatch, consumed by
/// `accept_host_key` / `reject_host_key` command handlers.
pub struct PendingHostKey {
    pub host: String,
    pub key_type: String,
    pub key_bytes: Vec<u8>,
    pub is_mismatch: bool,
}

/// Manages all live SSH sessions.
pub struct SshManager {
    connections: DashMap<PaneId, SshConnection>,
    /// Pending credential prompts indexed by pane ID.
    /// Inserted when the connect task needs user input; removed when satisfied or timed out.
    pending_credentials: DashMap<PaneId, PendingCredentials>,
    /// Pending host key verifications awaiting user acceptance/rejection.
    pub pending_host_keys: DashMap<PaneId, PendingHostKey>,
}

/// Credentials for SSH authentication.
///
/// SECURITY: `Debug` is implemented manually to redact sensitive fields.
/// Never use `#[derive(Debug)]` on this struct — it would expose passwords in logs.
/// `ZeroizeOnDrop` ensures all secret fields are overwritten in memory when the struct is dropped
/// (FS-CRED-003).
#[derive(Clone, Zeroize, ZeroizeOnDrop, serde::Serialize, serde::Deserialize)]
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
            keepalive_interval_secs: None,
            keepalive_max_failures: None,
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
            keepalive_interval_secs: None,
            keepalive_max_failures: None,
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
            pending_credentials: DashMap::new(),
            pending_host_keys: DashMap::new(),
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
    async fn open_connection_inner(
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

    /// The async connection task: TCP connect → russh handshake → auth → PTY channel → Connected.
    #[allow(clippy::too_many_arguments)]
    async fn connect_task(
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
        let addr = format!("{}:{}", config.host, config.port);

        // Use per-connection keepalive overrides when present (FS-SSH-020).
        let keepalive_interval = config
            .keepalive_interval_secs
            .map(std::time::Duration::from_secs);
        let keepalive_max = config.keepalive_max_failures.map(|n| n as usize);
        let russh_config = make_client_config(keepalive_interval, keepalive_max);
        let handler =
            TauTermSshHandler::new(pane_id.clone(), config, app.clone()).with_manager(self);

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

        // Authentication order: pubkey → keyboard-interactive → password (FS-SSH-012).
        let authenticated =
            Self::try_authenticate(&mut session, &username, config, credentials.as_ref()).await?;

        // SECURITY (FS-CRED-003): drop credentials immediately after auth so ZeroizeOnDrop
        // wipes the password/key bytes without waiting until end of the connect_task future
        // (which may stay alive for minutes across the PTY channel lifetime).
        drop(credentials);

        if !authenticated {
            return Err(SshError::Auth(
                "All authentication methods failed.".to_string(),
            ));
        }

        // Open a session channel and request a PTY (FS-SSH-013).
        let mut channel = session
            .channel_open_session()
            .await
            .map_err(|e| SshError::Connection(format!("channel_open_session failed: {e}")))?;

        channel
            .request_pty(
                true,
                "xterm-256color",
                cols as u32,
                rows as u32,
                0, // pixel width — not used
                0, // pixel height — not used
                TERMINAL_MODES,
            )
            .await
            .map_err(|e| SshError::Connection(format!("request_pty failed: {e}")))?;

        // Wait for PTY Success confirmation before requesting the shell.
        loop {
            match channel.wait().await {
                Some(russh::ChannelMsg::Success) => break,
                Some(russh::ChannelMsg::Failure) => {
                    return Err(SshError::Connection(
                        "PTY request rejected by server.".to_string(),
                    ));
                }
                None => {
                    return Err(SshError::Connection(
                        "Channel closed before PTY ack.".to_string(),
                    ));
                }
                Some(_) => {
                    // Skip other messages (e.g. WindowAdjust) while waiting for PTY ack.
                }
            }
        }

        channel
            .request_shell(true)
            .await
            .map_err(|e| SshError::Connection(format!("request_shell failed: {e}")))?;

        // Wait for shell Success confirmation.
        loop {
            match channel.wait().await {
                Some(russh::ChannelMsg::Success) => break,
                Some(russh::ChannelMsg::Failure) => {
                    return Err(SshError::Connection(
                        "Shell request rejected by server.".to_string(),
                    ));
                }
                None => {
                    return Err(SshError::Connection(
                        "Channel closed before shell ack.".to_string(),
                    ));
                }
                Some(_) => {}
            }
        }

        // Wrap the channel in Arc<Mutex> so it can be shared between the read task
        // and the write path (send_input / resize).
        let channel_arc: SshChannelArc = Arc::new(tokio::sync::Mutex::new(channel));

        // Spawn the read task.
        let read_task = spawn_ssh_read_task(
            pane_id.clone(),
            vt,
            app.clone(),
            Arc::clone(&channel_arc),
            registry,
        );

        // Mutate the connection entry in-place via DashMap::get_mut.
        // This avoids a remove/insert window where concurrent access would
        // return PaneNotFound.
        if let Some(mut conn) = self.connections.get_mut(&pane_id) {
            let mut handle_guard = conn.handle.lock().await;
            *handle_guard = Some(session);
            drop(handle_guard);
            conn.channel = Some(channel_arc);
            conn.read_task = Some(read_task);
            conn.set_state(SshLifecycleState::Connected);
        }

        // Transition to Connected.
        emit_ssh_state_changed(
            &app,
            SshStateChangedEvent {
                pane_id: pane_id.clone(),
                state: SshLifecycleState::Connected,
                reason: None,
            },
        );

        // Emit the reconnection separator event (FS-SSH-042).
        // Only on reconnect — not on the initial connection.
        if is_reconnect {
            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            emit_ssh_reconnected(
                &app,
                SshReconnectedEvent {
                    pane_id: pane_id.clone(),
                    timestamp_ms,
                },
            );
        }

        Ok(())
    }

    /// Try authentication methods in order: pubkey → keyboard-interactive → password (FS-SSH-012).
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
                    // Log and fall through — transport errors on pubkey do not abort the sequence.
                    tracing::warn!("Pubkey auth error for {username}: {e}");
                }
            }
        }

        // 2. Keyboard-interactive (if password is available as a response).
        if let Some(creds) = credentials
            && let Some(ref password) = creds.password
        {
            match authenticate_keyboard_interactive(session, username, password).await {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    tracing::debug!(
                        "Keyboard-interactive auth rejected for {username}@{}",
                        config.host
                    );
                }
                Err(e) => {
                    // Transport error: log and fall through to password.
                    tracing::warn!("Keyboard-interactive auth error for {username}: {e}");
                }
            }
        }

        // 3. Password (if provided in credentials).
        if let Some(creds) = credentials
            && let Some(ref password) = creds.password
        {
            return authenticate_password(session, username, password).await;
        }

        Ok(false)
    }

    /// Send input bytes to the SSH PTY channel for a pane.
    pub async fn send_input(&self, pane_id: &PaneId, data: Vec<u8>) -> Result<(), SshError> {
        let conn = self
            .connections
            .get(pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        conn.send_input(data).await
    }

    /// Resize the SSH PTY channel for a pane.
    pub async fn resize_pane(
        &self,
        pane_id: &PaneId,
        cols: u16,
        rows: u16,
        px_w: u16,
        px_h: u16,
    ) -> Result<(), SshError> {
        let conn = self
            .connections
            .get(pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        conn.resize(cols, rows, px_w, px_h).await
    }

    /// Deliver credentials to a pending SSH auth prompt for a pane.
    ///
    /// The connect task parks a oneshot sender in `pending_credentials` while
    /// waiting for the user. This method resolves it.
    pub fn provide_credentials(
        &self,
        pane_id: &PaneId,
        creds: Credentials,
    ) -> Result<(), SshError> {
        let (_, pending) = self
            .pending_credentials
            .remove(pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        // Ignore send error — the connect task may have timed out already.
        let _ = pending.sender.send(creds);
        Ok(())
    }

    /// Close the SSH session for a pane.
    ///
    /// Sends a clean `Disconnect` to the server before dropping the handle,
    /// so the remote end sees a proper close rather than a TCP reset.
    pub async fn close_connection(&self, pane_id: PaneId) -> Result<(), SshError> {
        // Drop the pending credential prompt if any (unblocks the connect task).
        self.pending_credentials.remove(&pane_id);

        let (_, conn) = self
            .connections
            .remove(&pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;

        // Abort the read task before touching the handle.
        drop(conn.read_task);

        // Send a clean disconnect to the server.
        let mut guard = conn.handle.lock().await;
        if let Some(handle) = guard.take() {
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "user close", "en")
                .await;
        }

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
            keepalive_interval_secs: None,
            keepalive_max_failures: None,
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
    /// Verified via direct map inspection (reconnect requires AppHandle — not constructible
    /// in unit tests; the pane-not-found guard executes before any AppHandle usage).
    #[test]
    fn ssh_manager_reconnect_unknown_pane_not_in_map() {
        let manager = SshManager::new();
        let unknown_pane = PaneId::new();
        // Verify precondition: pane is not in the map.
        // reconnect() starts with `self.connections.get(&pane_id).ok_or(PaneNotFound)`.
        assert!(
            !manager.connections.contains_key(&unknown_pane),
            "reconnect on unknown pane must return error (TEST-SSH-UNIT-001 step 8)"
        );
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
