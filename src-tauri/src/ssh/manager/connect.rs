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
        // E2E: hold in `Connecting` state long enough for WebdriverIO to observe
        // the connecting overlay (UXD §7.5.2). Fires once per armed delay, then
        // resets. No-op in production builds.
        #[cfg(feature = "e2e-testing")]
        if let Some(ms) = crate::commands::testing::consume_ssh_connect_delay() {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        let addr = format!("{}:{}", config.host, config.port);

        // Use per-connection keepalive overrides when present (FS-SSH-020).
        let keepalive_interval = config
            .keepalive_interval_secs
            .map(std::time::Duration::from_secs);
        let keepalive_max = config.keepalive_max_failures.map(|n| n as usize);
        let russh_config = make_client_config(keepalive_interval, keepalive_max);
        let handler =
            TauTermSshHandler::new(pane_id.clone(), config, app.clone()).with_manager(self);

        // SECURITY: russh::client::connect() calls TauTermSshHandler::check_server_key()
        // during SSH key-exchange, before returning the Handle. If check_server_key returns
        // Ok(false) — which TauTermSshHandler does for Unknown and Mismatch host keys —
        // russh aborts the handshake with Error::UnknownKey and connect() returns Err.
        // The Handle<TauTermSshHandler> is therefore ONLY obtained when the host key is
        // trusted. Auth methods (authenticate_password, authenticate_publickey, etc.) can
        // only be called on a Handle, making it architecturally impossible to attempt auth
        // before host key validation passes.
        //
        // Regression test: src/ssh/manager/tests/host_key_sequencing.rs
        // (SEC-HOSTKEY-SEQ-001, SEC-HOSTKEY-SEQ-002)
        //
        // Wrapped in a 5-second timeout because russh 0.60.0 has no connect_timeout in
        // client::Config. Without this, a DROPped port (firewall) blocks indefinitely.
        let mut session = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            russh::client::connect(russh_config, addr.as_str(), handler),
        )
        .await
        .map_err(|_| SshError::Connection("TCP connect timed out".to_string()))?
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

        // FS-SSH-019a: Resolve passphrase for encrypted identity files before SSH auth.
        let resolved_passphrase: Option<zeroize::Zeroizing<String>> =
            if let Some(ref key_path_str) = config.identity_file {
                let key_path = std::path::Path::new(key_path_str);
                if crate::ssh::auth::key_needs_passphrase(key_path) {
                    const MAX_PASSPHRASE_ATTEMPTS: u32 = 3;
                    const PASSPHRASE_PROMPT_TIMEOUT_SECS: u64 = 120;
                    let key_label = key_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("private key")
                        .to_string();

                    let mut passphrase_attempt: u32 = 0;
                    let mut resolved: Option<zeroize::Zeroizing<String>> = None;

                    // 1. Try keychain first.
                    if let Ok(Some(stored)) =
                        self.credential_manager.get_passphrase(key_path_str).await
                    {
                        // Verify the stored passphrase actually decrypts the key.
                        if russh::keys::load_secret_key(key_path, Some(stored.as_str())).is_ok() {
                            resolved = Some(zeroize::Zeroizing::new(stored));
                        }
                    }

                    // 2. If not resolved, prompt the user.
                    while resolved.is_none() && passphrase_attempt < MAX_PASSPHRASE_ATTEMPTS {
                        passphrase_attempt += 1;
                        let (tx, rx) = tokio::sync::oneshot::channel::<super::PassphraseInput>();
                        self.pending_passphrases
                            .insert(pane_id.clone(), super::PendingPassphrase { sender: tx });
                        crate::events::emit_passphrase_prompt(
                            &app,
                            crate::events::PassphrasePromptEvent {
                                pane_id: pane_id.clone(),
                                key_path_label: key_label.clone(),
                                failed: passphrase_attempt > 1,
                                is_keychain_available: self.credential_manager.is_available(),
                            },
                        );
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(PASSPHRASE_PROMPT_TIMEOUT_SECS),
                            rx,
                        )
                        .await
                        {
                            Ok(Ok(input)) => {
                                if russh::keys::load_secret_key(
                                    key_path,
                                    Some(input.passphrase.as_str()),
                                )
                                .is_ok()
                                {
                                    if input.save_in_keychain
                                        && let Err(e) = self
                                            .credential_manager
                                            .store_passphrase(
                                                key_path_str,
                                                input.passphrase.as_str(),
                                            )
                                            .await
                                    {
                                        // SECURITY: `{e}` is a CredentialError from the
                                        // platform store — it must not include the keychain
                                        // key (which contains the identity file path).
                                        // TauTerm's own code never embeds the key in `e`;
                                        // the underlying secret-service error is opaque.
                                        tracing::warn!(
                                            "Failed to save passphrase to keychain: {e}"
                                        );
                                    }
                                    resolved = Some(input.passphrase.clone());
                                }
                                // Wrong passphrase: loop to re-prompt.
                            }
                            Ok(Err(_)) => {
                                // User cancelled (sender dropped).
                                self.pending_passphrases.remove(&pane_id);
                                return Err(SshError::Auth(
                                    "passphrase prompt cancelled by user".to_string(),
                                ));
                            }
                            Err(_) => {
                                self.pending_passphrases.remove(&pane_id);
                                return Err(SshError::Auth(
                                    "passphrase prompt timed out".to_string(),
                                ));
                            }
                        }
                    }

                    if resolved.is_none() {
                        return Err(SshError::Auth(
                            "Maximum passphrase attempts exceeded.".to_string(),
                        ));
                    }
                    resolved
                } else {
                    None
                }
            } else {
                None
            };

        // Authentication retry loop: try → on failure emit credential-prompt → wait → retry.
        // Max 3 attempts; 120 s timeout per prompt (FS-SSH-015/016/017).
        const MAX_AUTH_ATTEMPTS: u32 = 3;
        const CREDENTIAL_PROMPT_TIMEOUT_SECS: u64 = 120;

        let mut current_credentials = credentials;
        let mut attempt: u32 = 0;

        loop {
            attempt += 1;
            tracing::debug!(
                pane_id = %pane_id,
                attempt,
                has_credentials = current_credentials.is_some(),
                "connect_task: starting auth attempt"
            );
            let authenticated = Self::try_authenticate(
                &mut session,
                &username,
                config,
                current_credentials.as_ref(),
                resolved_passphrase.as_ref().map(|p| p.as_str()),
            )
            .await?;
            tracing::debug!(
                pane_id = %pane_id,
                attempt,
                authenticated,
                "connect_task: try_authenticate returned"
            );

            // Extract keychain-save intent before dropping credentials (FS-CRED-003).
            let password_for_keychain: Option<zeroize::Zeroizing<String>> =
                current_credentials.as_ref().and_then(|c| {
                    if c.save_in_keychain {
                        c.password
                            .as_ref()
                            .map(|p| zeroize::Zeroizing::new(p.clone()))
                    } else {
                        None
                    }
                });
            // SECURITY (FS-CRED-003): drop credentials immediately after each auth attempt
            // so ZeroizeOnDrop wipes password bytes. `Credentials` derives `ZeroizeOnDrop`
            // which zeroes all fields — including `password: Option<String>` — on drop.
            // `password_for_keychain` carries a separate `Zeroizing<String>` copy.
            // Using `drop()` here moves the value, which triggers the Drop impl
            // and avoids the `unused_assignments` lint that `= None` would produce.
            drop(current_credentials);

            if authenticated {
                if let Some(ref password) = password_for_keychain
                    && let Err(e) = self
                        .credential_manager
                        .store_password(&config.id.to_string(), &username, password)
                        .await
                {
                    tracing::warn!("Failed to save password to keychain: {e}");
                }
                break;
            }

            if attempt >= MAX_AUTH_ATTEMPTS {
                return Err(SshError::Auth(
                    "Maximum authentication attempts exceeded.".to_string(),
                ));
            }

            let (tx, rx) = tokio::sync::oneshot::channel::<super::Credentials>();
            self.pending_credentials
                .insert(pane_id.clone(), super::PendingCredentials { sender: tx });
            tracing::debug!(
                pane_id = %pane_id,
                attempt,
                "connect_task: emitting credential-prompt event"
            );
            crate::events::emit_credential_prompt(
                &app,
                crate::events::CredentialPromptEvent {
                    pane_id: pane_id.clone(),
                    host: config.host.clone(),
                    username: username.clone(),
                    prompt: None,
                    failed: attempt > 1,
                    is_keychain_available: self.credential_manager.is_available(),
                },
            );

            match tokio::time::timeout(
                std::time::Duration::from_secs(CREDENTIAL_PROMPT_TIMEOUT_SECS),
                rx,
            )
            .await
            {
                Ok(Ok(creds)) => {
                    tracing::debug!(pane_id = %pane_id, "connect_task: auth input received from user prompt");
                    current_credentials = Some(creds);
                }
                Ok(Err(_)) => {
                    // Sender dropped — user cancelled the dialog.
                    tracing::warn!(pane_id = %pane_id, "connect_task: credential prompt cancelled (sender dropped)");
                    return Err(SshError::Auth("cancelled by user".to_string()));
                }
                Err(_) => {
                    // Timeout expired without a response.
                    tracing::warn!(pane_id = %pane_id, "connect_task: credential prompt timed out");
                    self.pending_credentials.remove(&pane_id);
                    return Err(SshError::Auth("credential prompt timed out".to_string()));
                }
            }
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
    ///
    /// `passphrase` is forwarded to `authenticate_pubkey` for encrypted identity files (FS-SSH-019a).
    async fn try_authenticate<H: russh::client::Handler>(
        session: &mut russh::client::Handle<H>,
        username: &str,
        config: &SshConnectionConfig,
        credentials: Option<&Credentials>,
        passphrase: Option<&str>,
    ) -> Result<bool, SshError> {
        // 1. Public key (if identity_file is configured).
        if let Some(ref key_path_str) = config.identity_file {
            let key_path = std::path::Path::new(key_path_str);
            match authenticate_pubkey(session, username, key_path, passphrase).await {
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
