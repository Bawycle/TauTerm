// SPDX-License-Identifier: MPL-2.0

//! SSH keepalive task.
//!
//! Runs a per-connection Tokio timer that sends keepalive probes.
//! After `SSH_KEEPALIVE_MAX_MISSES` consecutive missed probes (FS-SSH-020),
//! the connection is considered lost and transitions to `Disconnected`.

// Stub — full implementation in SSH integration pass.

/// Number of consecutive keepalive misses before declaring the connection lost (FS-SSH-020).
pub const SSH_KEEPALIVE_MAX_MISSES: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TEST-SSH-007 (partial) — keepalive constant (FS-SSH-020)
    // The full keepalive timer test requires a mock transport.
    // -----------------------------------------------------------------------

    #[test]
    fn keepalive_max_misses_is_three() {
        // TEST-SSH-007 step 1: FS-SSH-020 requires 3 consecutive missed keepalives.
        assert_eq!(
            SSH_KEEPALIVE_MAX_MISSES,
            3,
            "FS-SSH-020 requires exactly 3 consecutive missed keepalives before disconnect"
        );
    }
}
