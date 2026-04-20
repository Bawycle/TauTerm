// SPDX-License-Identifier: MPL-2.0

//! SSH module — remote terminal sessions via the SSH protocol.
//!
//! Re-exports the public API used by command handlers and the session registry.
//! The `SshManager` holds live sessions only; saved connection configs are
//! stored in `PreferencesStore` (§3.3 / §8.1 of ARCHITECTURE.md).

pub mod algorithms;
pub mod auth;
pub mod connection;
pub mod keepalive;
pub mod known_hosts;
pub mod manager;

pub use manager::{Credentials, SshManager};

use serde::{Deserialize, Serialize};

use crate::preferences::types::{SshHost, SshIdentityPath, SshLabel, SshUsername};
use crate::session::ids::ConnectionId;

/// Saved SSH connection configuration (persisted in preferences.json).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SshConnectionConfig {
    pub id: ConnectionId,
    pub label: SshLabel,
    pub host: SshHost,
    pub port: u16,
    pub username: SshUsername,
    /// Path to a private key file.
    ///
    /// Structural validation (absolute, no traversal, no control chars, ≤4096 bytes) is enforced
    /// by `SshIdentityPath::try_from` at IPC deserialization time (SEC-PATH-005).
    /// File existence and `~/.ssh/` boundary are checked at connection time in
    /// `lifecycle.rs::open_connection_inner`.
    pub identity_file: Option<SshIdentityPath>,
    /// Per-connection OSC 52 write policy override.
    #[serde(default)]
    pub allow_osc52_write: bool,
    /// Override the keepalive probe interval for this connection (seconds).
    /// Falls back to `SSH_KEEPALIVE_INTERVAL` (30 s) when absent (FS-SSH-020).
    #[specta(type = Option<f64>)]
    pub keepalive_interval_secs: Option<u64>,
    /// Override the maximum number of consecutive unanswered keepalive probes
    /// before the connection is declared lost (FS-SSH-020).
    /// Falls back to `SSH_KEEPALIVE_MAX_MISSES` (3) when absent.
    pub keepalive_max_failures: Option<u32>,
}

/// SSH session lifecycle state (§5.2 of ARCHITECTURE.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SshLifecycleState {
    /// TCP connection in progress.
    Connecting,
    /// SSH handshake and credential exchange.
    Authenticating,
    /// Session is active.
    Connected,
    /// Session lost due to network drop or keepalive timeout.
    ///
    /// `reason` carries a human-readable explanation (e.g. "keepalive timeout",
    /// "Connection lost: …"). The frontend reads it directly from this variant —
    /// `SshStateChangedEvent` carries no separate top-level reason field.
    Disconnected { reason: Option<String> },
    /// User-initiated close or remote exit with code 0.
    Closed,
}
