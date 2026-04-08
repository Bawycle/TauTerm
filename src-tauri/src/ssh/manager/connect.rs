// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::AppHandle;

use crate::error::SshError;
use crate::events::{
    SshReconnectedEvent, SshStateChangedEvent, emit_ssh_reconnected, emit_ssh_state_changed,
};
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::session::ssh_task::spawn_ssh_read_task;
use crate::ssh::auth::{
    authenticate_keyboard_interactive, authenticate_password, authenticate_pubkey,
};
use crate::ssh::connection::{SshChannelArc, TauTermSshHandler};
use crate::ssh::keepalive::make_client_config;
use crate::ssh::{SshConnectionConfig, SshLifecycleState};
use crate::vt::VtProcessor;

use super::{Credentials, SshManager, TERMINAL_MODES};

impl SshManager {
    /// The async connection task: TCP connect → russh handshake → auth → PTY channel → Connected.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn connect_task(
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
}
