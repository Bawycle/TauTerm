// SPDX-License-Identifier: MPL-2.0

//! SSH prompt response Tauri commands.
//!
//! Commands: provide_credentials, accept_host_key, reject_host_key,
//!           dismiss_ssh_algorithm_warning.
//!
//! Security note (SEC-SSH-CH-002): `accept_host_key` does NOT accept key bytes
//! from the IPC payload. Key bytes are retrieved from `SshManager::pending_host_keys`
//! where they were stored by the Rust-side `check_server_key` callback. This
//! prevents a hostile frontend from injecting arbitrary key data.

use std::sync::Arc;

use tauri::State;
use zeroize::Zeroizing;

use crate::error::TauTermError;
use crate::events::{SshStateChangedEvent, emit_ssh_state_changed};
use crate::session::ids::PaneId;
use crate::ssh::known_hosts::KnownHostsStore;
use crate::ssh::manager::{Credentials, PassphraseInput};
use crate::ssh::{SshLifecycleState, SshManager};

#[tauri::command]
pub async fn provide_credentials(
    pane_id: PaneId,
    credentials: Credentials,
    ssh_manager: State<'_, Arc<SshManager>>,
) -> Result<(), TauTermError> {
    // SEC-CRED-004: reject oversized passwords before they reach the auth layer.
    if let Some(ref pw) = credentials.password
        && pw.len() > crate::commands::connection_cmds::MAX_PASSWORD_LEN
    {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "password exceeds maximum length.",
        ));
    }
    ssh_manager
        .provide_credentials(&pane_id, credentials)
        .map_err(|e| {
            TauTermError::with_detail(
                "SSH_CREDENTIALS_ERROR",
                "Failed to deliver credentials to SSH auth flow.",
                e.to_string(),
            )
        })
}

/// Accept the host key for a pane that is pending TOFU verification.
///
/// ## Security (SEC-SSH-CH-002)
/// The key bytes are retrieved from `SshManager::pending_host_keys` — they were
/// stored by the Rust-side `check_server_key` callback and are NOT taken from the
/// IPC payload. This prevents a hostile frontend from accepting an arbitrary key.
///
/// After persisting the key to `known_hosts`, returns `Ok(())`. The frontend
/// must then call `open_ssh_connection` again to reconnect (two-phase TOFU).
#[tauri::command]
pub async fn accept_host_key(
    pane_id: PaneId,
    ssh_manager: State<'_, Arc<SshManager>>,
) -> Result<(), TauTermError> {
    // Retrieve the pending host key — key bytes come from Rust, NOT from IPC.
    let (_, pending) = ssh_manager
        .pending_host_keys
        .remove(&pane_id)
        .ok_or_else(|| {
            TauTermError::new(
                "NO_PENDING_HOST_KEY",
                "No pending host key verification for this pane.",
            )
        })?;

    let store_path = KnownHostsStore::default_path().ok_or_else(|| {
        TauTermError::new(
            "CONFIG_DIR_UNAVAILABLE",
            "Cannot determine known_hosts path.",
        )
    })?;
    let store = KnownHostsStore::new(store_path);

    // For mismatch: remove old entry first, then warn in the audit log.
    if pending.is_mismatch {
        store
            .remove_entries_for_host(&pending.host, &pending.key_type)
            .map_err(|e| {
                TauTermError::with_detail(
                    "KNOWN_HOSTS_IO_ERROR",
                    "Failed to remove old host key entry.",
                    e.to_string(),
                )
            })?;

        // SEC-SSH-CH-004: emit a warning so host-key substitution attacks leave a
        // trace in the application log. Host and key type are logged; the raw key
        // bytes are not (verbose but not secret). Username and filesystem paths are
        // intentionally omitted per CLAUDE.md logging constraints.
        tracing::warn!(
            host = %pending.host,
            key_type = %pending.key_type,
            "SSH host key MISMATCH accepted by user — \
             old key removed and new key trusted (SEC-SSH-CH-004)"
        );
    }

    // Persist the new key.
    store
        .add_entry(&pending.host, &pending.key_type, &pending.key_bytes)
        .map_err(|e| {
            TauTermError::with_detail(
                "KNOWN_HOSTS_IO_ERROR",
                "Failed to save host key.",
                e.to_string(),
            )
        })?;

    // The frontend must call open_ssh_connection again (two-phase TOFU reconnect).
    Ok(())
}

/// Reject the host key for a pane — abort the pending connection.
#[tauri::command]
pub async fn reject_host_key(
    pane_id: PaneId,
    ssh_manager: State<'_, Arc<SshManager>>,
    app: tauri::AppHandle,
) -> Result<(), TauTermError> {
    // Discard the pending host key.
    ssh_manager.pending_host_keys.remove(&pane_id);

    // Emit a Closed state so the frontend knows the connection is dead.
    emit_ssh_state_changed(
        &app,
        SshStateChangedEvent {
            pane_id,
            state: SshLifecycleState::Closed,
        },
    );

    Ok(())
}

/// Provide a passphrase for an encrypted SSH private key (FS-SSH-019a).
///
/// Called by the frontend in response to a `passphrase-prompt` event.
/// The passphrase is forwarded to the connect task via the oneshot channel
/// stored in `SshManager::pending_passphrases`.
#[tauri::command]
pub async fn provide_passphrase(
    pane_id: PaneId,
    passphrase: String,
    save_in_keychain: bool,
    ssh_manager: State<'_, Arc<SshManager>>,
) -> Result<(), TauTermError> {
    // SEC-CRED-004: reject oversized passphrases — same limit as passwords.
    if passphrase.len() > crate::commands::connection_cmds::MAX_PASSWORD_LEN {
        return Err(TauTermError::new(
            "VALIDATION_ERROR",
            "passphrase exceeds maximum length.",
        ));
    }
    let entry = ssh_manager
        .pending_passphrases
        .remove(&pane_id)
        .ok_or_else(|| {
            TauTermError::new(
                "PANE_NOT_FOUND",
                "No pending passphrase prompt for this pane.",
            )
        })?;
    // Ignore send errors — the connect task may have timed out or been cancelled.
    let _ = entry.1.sender.send(PassphraseInput {
        passphrase: Zeroizing::new(passphrase),
        save_in_keychain,
    });
    Ok(())
}

/// Dismiss an SSH algorithm warning for a pane.
///
/// This command is a no-op in v1 — the warning is purely informational and
/// requires no server-side action. It exists so the frontend can call it and
/// receive an Ok without causing an IPC error.
#[tauri::command]
pub async fn dismiss_ssh_algorithm_warning(pane_id: PaneId) -> Result<(), TauTermError> {
    // No persistent state to clear — the warning is UI-only.
    let _ = pane_id;
    Ok(())
}
