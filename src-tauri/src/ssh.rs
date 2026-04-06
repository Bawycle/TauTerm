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

use crate::session::ids::ConnectionId;

/// Saved SSH connection configuration (persisted in preferences.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConnectionConfig {
    pub id: ConnectionId,
    pub label: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// Path to a private key file. Validated for path traversal (§8.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<String>,
    /// Per-connection OSC 52 write policy override.
    #[serde(default)]
    pub allow_osc52_write: bool,
    /// Override the keepalive probe interval for this connection (seconds).
    /// Falls back to `SSH_KEEPALIVE_INTERVAL` (30 s) when absent (FS-SSH-020).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keepalive_interval_secs: Option<u64>,
    /// Override the maximum number of consecutive unanswered keepalive probes
    /// before the connection is declared lost (FS-SSH-020).
    /// Falls back to `SSH_KEEPALIVE_MAX_MISSES` (3) when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keepalive_max_failures: Option<u32>,
}

/// SSH session lifecycle state (§5.2 of ARCHITECTURE.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SshLifecycleState {
    /// TCP connection in progress.
    Connecting,
    /// SSH handshake and credential exchange.
    Authenticating,
    /// Session is active.
    Connected,
    /// Session lost due to network drop or keepalive timeout.
    Disconnected,
    /// User-initiated close or remote exit with code 0.
    Closed,
}
