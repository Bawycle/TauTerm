// SPDX-License-Identifier: MPL-2.0

//! Central error type for the TauTerm backend.
//!
//! `TauTermError` is the uniform error envelope returned by all `#[tauri::command]`
//! handlers. It maps to the TypeScript `TauTermError` interface on the frontend:
//! `{ code, message, detail? }`.
//!
//! Internal module errors use `thiserror`-derived types. Command handlers convert
//! them to `TauTermError` via `From` impls or explicit mapping.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Uniform error envelope returned by all Tauri command handlers.
///
/// Serialized to JSON as `{ "code": "...", "message": "...", "detail": "..." }`.
#[derive(Debug, Clone, Serialize, Deserialize, Error, specta::Type)]
#[error("{code}: {message}")]
pub struct TauTermError {
    /// Machine-readable error code (upper-case, underscore-separated, module-prefixed).
    /// Examples: `PTY_SPAWN_FAILED`, `INVALID_PANE_ID`, `SSH_KEEPALIVE_TIMEOUT`.
    pub code: String,
    /// Human-readable summary suitable for display to non-technical users (FS-UX-001).
    pub message: String,
    /// Optional technical detail: raw OS error, exit code, system message.
    pub detail: Option<String>,
}

impl TauTermError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(
        code: impl Into<String>,
        message: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: Some(detail.into()),
        }
    }
}

impl From<anyhow::Error> for TauTermError {
    fn from(err: anyhow::Error) -> Self {
        TauTermError::with_detail(
            "INTERNAL_ERROR",
            "An internal error occurred.",
            format!("{err:#}"),
        )
    }
}

/// PTY-specific errors.
#[derive(Debug, Error)]
pub enum PtyError {
    #[error("Failed to open PTY: {0}")]
    Open(String),
    #[error("Failed to spawn child process: {0}")]
    Spawn(String),
    #[error("I/O error on PTY master: {0}")]
    Io(#[from] std::io::Error),
    #[error("PTY resize failed: {0}")]
    Resize(String),
}

impl From<PtyError> for TauTermError {
    fn from(err: PtyError) -> Self {
        match &err {
            PtyError::Open(msg) => {
                TauTermError::with_detail("PTY_OPEN_FAILED", "Failed to open terminal.", msg)
            }
            PtyError::Spawn(msg) => TauTermError::with_detail(
                "PTY_SPAWN_FAILED",
                "Failed to start the shell process.",
                msg,
            ),
            PtyError::Io(e) => {
                TauTermError::with_detail("PTY_IO_ERROR", "Terminal I/O error.", e.to_string())
            }
            PtyError::Resize(msg) => {
                TauTermError::with_detail("PTY_RESIZE_FAILED", "Failed to resize terminal.", msg)
            }
        }
    }
}

/// Session management errors.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Tab not found: {0}")]
    TabNotFound(String),
    #[error("Pane not found: {0}")]
    PaneNotFound(String),
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
    /// Pane exists but is not in Running state (e.g., Spawning, Terminated, Closed).
    #[error("Pane is not running: {0}")]
    PaneNotRunning(String),
    /// PTY I/O error during write or resize on a live session.
    #[error("PTY I/O error: {0}")]
    PtyIo(String),
    /// Shell path validation failed.
    #[error("Invalid shell path: {0}")]
    InvalidShellPath(String),
    /// PTY spawn failed.
    #[error("PTY spawn failed: {0}")]
    PtySpawn(String),
}

impl From<SessionError> for TauTermError {
    fn from(err: SessionError) -> Self {
        match &err {
            SessionError::TabNotFound(id) => {
                TauTermError::with_detail("INVALID_TAB_ID", "Tab not found.", id)
            }
            SessionError::PaneNotFound(id) => {
                TauTermError::with_detail("INVALID_PANE_ID", "Pane not found.", id)
            }
            SessionError::Pty(e) => {
                TauTermError::from(PtyError::from(std::io::Error::other(e.to_string())))
            }
            SessionError::PaneNotRunning(id) => {
                TauTermError::with_detail("PANE_NOT_RUNNING", "Pane is not in running state.", id)
            }
            SessionError::PtyIo(msg) => {
                TauTermError::with_detail("PTY_IO_ERROR", "Terminal I/O error.", msg)
            }
            SessionError::InvalidShellPath(msg) => TauTermError::with_detail(
                "INVALID_SHELL_PATH",
                "Invalid shell executable path.",
                msg,
            ),
            SessionError::PtySpawn(msg) => TauTermError::with_detail(
                "PTY_SPAWN_FAILED",
                "Failed to start the shell process.",
                msg,
            ),
        }
    }
}

/// SSH-specific errors.
#[derive(Debug, Error)]
pub enum SshError {
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("Host key verification failed: {0}")]
    HostKey(String),
    #[error("SSH I/O error: {0}")]
    Io(String),
    #[error("Pane not found: {0}")]
    PaneNotFound(String),
    /// No pending credential prompt for this pane — either it was never requested
    /// or it has already been consumed / timed out.
    #[error("No pending credentials for pane: {0}")]
    NoPendingCredentials(String),
    /// Transport-level russh error (keepalive timeout, protocol violation, etc.).
    #[error("SSH transport error: {0}")]
    Transport(String),
}

/// Required by `russh::client::Handler::Error` bound: `From<russh::Error>`.
///
/// russh calls this conversion when an internal transport error occurs during
/// the handler callbacks (keepalive timeout, protocol violation). We map it to
/// `SshError::Transport` so the error propagates through our state machine.
impl From<russh::Error> for SshError {
    fn from(e: russh::Error) -> Self {
        SshError::Transport(e.to_string())
    }
}

impl From<SshError> for TauTermError {
    fn from(err: SshError) -> Self {
        match &err {
            SshError::Connection(msg) => TauTermError::with_detail(
                "SSH_CONNECTION_FAILED",
                "Failed to connect to the SSH server.",
                msg,
            ),
            SshError::Auth(msg) => {
                TauTermError::with_detail("SSH_AUTH_FAILED", "SSH authentication failed.", msg)
            }
            SshError::HostKey(msg) => TauTermError::with_detail(
                "SSH_HOST_KEY_REJECTED",
                "The server's host key could not be verified.",
                msg,
            ),
            SshError::Io(msg) => {
                TauTermError::with_detail("SSH_IO_ERROR", "SSH connection I/O error.", msg)
            }
            SshError::PaneNotFound(id) => {
                TauTermError::with_detail("INVALID_PANE_ID", "Pane not found.", id)
            }
            SshError::NoPendingCredentials(id) => TauTermError::with_detail(
                "NO_PENDING_CREDENTIALS",
                "No pending credential prompt for this pane.",
                id,
            ),
            SshError::Transport(msg) => {
                TauTermError::with_detail("SSH_TRANSPORT_ERROR", "SSH transport error.", msg)
            }
        }
    }
}

/// Preferences errors.
#[derive(Debug, Error)]
pub enum PreferencesError {
    #[error("Failed to read preferences file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse preferences: {0}")]
    Parse(String),
    #[error("Invalid preference value: {0}")]
    Validation(String),
    #[error("Timed out waiting for preferences lock")]
    LockTimeout,
}

impl From<PreferencesError> for TauTermError {
    fn from(err: PreferencesError) -> Self {
        match &err {
            PreferencesError::Io(e) => TauTermError::with_detail(
                "PREF_IO_ERROR",
                "Failed to read preferences.",
                e.to_string(),
            ),
            PreferencesError::Parse(msg) => {
                TauTermError::with_detail("PREF_PARSE_ERROR", "Failed to parse preferences.", msg)
            }
            PreferencesError::Validation(msg) => {
                TauTermError::with_detail("PREF_INVALID_VALUE", "Invalid preference value.", msg)
            }
            PreferencesError::LockTimeout => TauTermError::new(
                "PREF_LOCK_TIMEOUT",
                "Timed out waiting for preferences lock.",
            ),
        }
    }
}

/// Credential store errors.
#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Credential store unavailable: {0}")]
    Unavailable(String),
    #[error("Credential not found: {0}")]
    NotFound(String),
    #[error("Credential store I/O error: {0}")]
    Io(String),
}

impl From<CredentialError> for TauTermError {
    fn from(err: CredentialError) -> Self {
        match &err {
            CredentialError::Unavailable(msg) => TauTermError::with_detail(
                "CRED_STORE_UNAVAILABLE",
                "The credential store is unavailable.",
                msg,
            ),
            CredentialError::NotFound(key) => {
                TauTermError::with_detail("CRED_NOT_FOUND", "No saved credentials found.", key)
            }
            CredentialError::Io(msg) => {
                TauTermError::with_detail("CRED_IO_ERROR", "Credential store I/O error.", msg)
            }
        }
    }
}
