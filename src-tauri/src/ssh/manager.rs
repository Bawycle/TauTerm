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

mod connect;
mod io_ops;
mod lifecycle;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::oneshot;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::session::ids::PaneId;
use crate::ssh::connection::SshConnection;

/// Terminal modes for SSH PTY requests (RFC 4254 §8).
///
/// VKILL = opcode 4, VEOF = opcode 5 — per RFC 4254 table (not inverted).
pub(super) const TERMINAL_MODES: &[(russh::Pty, u32)] = &[
    (russh::Pty::VINTR, 3),    // Ctrl+C
    (russh::Pty::VQUIT, 28),   // Ctrl+\
    (russh::Pty::VERASE, 127), // Backspace (DEL)
    (russh::Pty::VKILL, 21),   // Ctrl+U  — opcode 4 per RFC 4254
    (russh::Pty::VEOF, 4),     // Ctrl+D  — opcode 5 per RFC 4254
    (russh::Pty::VSUSP, 26),   // Ctrl+Z
    (russh::Pty::ISIG, 1),     // Enable signals
    (russh::Pty::ICANON, 1),   // Canonical mode
    (russh::Pty::ECHO, 1),     // Echo input
];

/// Pending credential request — a oneshot sender parked while waiting for
/// the user to respond to a `credential-prompt` event.
pub(super) struct PendingCredentials {
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

/// Pending passphrase request — a oneshot sender parked while waiting for
/// the user to respond to a `passphrase-prompt` event (FS-SSH-019a).
pub struct PendingPassphrase {
    pub sender: oneshot::Sender<PassphraseInput>,
}

/// User-supplied passphrase response for an encrypted SSH private key (FS-SSH-019a).
///
/// The `passphrase` field is `Zeroizing<String>`, which implements `Drop` and
/// zeroes the underlying string bytes when the value is dropped. This provides
/// the memory-safety guarantee without preventing ownership transfer of the struct.
pub struct PassphraseInput {
    pub passphrase: zeroize::Zeroizing<String>,
    pub save_in_keychain: bool,
}

/// Manages all live SSH sessions.
pub struct SshManager {
    pub(super) connections: DashMap<PaneId, SshConnection>,
    /// Pending credential prompts indexed by pane ID.
    /// Inserted when the connect task needs user input; removed when satisfied or timed out.
    pub(super) pending_credentials: DashMap<PaneId, PendingCredentials>,
    /// Pending passphrase prompts for encrypted private keys (FS-SSH-019a).
    /// Inserted when the connect task needs a passphrase; removed when satisfied or timed out.
    pub pending_passphrases: DashMap<PaneId, PendingPassphrase>,
    /// Pending host key verifications awaiting user acceptance/rejection.
    pub pending_host_keys: DashMap<PaneId, PendingHostKey>,
    /// Credential manager for OS keychain access (store/retrieve passwords).
    pub(super) credential_manager: Arc<crate::credentials::CredentialManager>,
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
    /// Whether the user wants the password saved to the OS keychain (FS-CRED-007).
    #[serde(default)]
    pub save_in_keychain: bool,
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
            .field("save_in_keychain", &self.save_in_keychain)
            .finish()
    }
}
