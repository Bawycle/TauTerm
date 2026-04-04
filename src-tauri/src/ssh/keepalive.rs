// SPDX-License-Identifier: MPL-2.0

//! SSH keepalive task.
//!
//! Runs a per-connection Tokio timer that sends keepalive probes.
//! After `SSH_KEEPALIVE_MAX_MISSES` consecutive missed probes (FS-SSH-020),
//! the connection is considered lost and transitions to `Disconnected`.

// Stub — full implementation in SSH integration pass.

/// Number of consecutive keepalive misses before declaring the connection lost (FS-SSH-020).
pub const SSH_KEEPALIVE_MAX_MISSES: u32 = 3;
