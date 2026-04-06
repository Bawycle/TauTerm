// SPDX-License-Identifier: MPL-2.0

//! SSH keepalive configuration.
//!
//! TauTerm delegates keepalive to `russh`'s built-in mechanism: `client::Config`
//! accepts `keepalive_interval` and `keepalive_max`. When the configured number
//! of consecutive keepalive requests go unanswered, `russh` closes the connection
//! and calls `Handler::disconnected()` on the client handler — which transitions
//! the `SshConnection` state machine to `Disconnected` and emits the
//! `ssh-state-changed` event via `AppHandle`.
//!
//! The constants here are the single source of truth for FS-SSH-020 values.
//! They are used in `manager.rs` when building `client::Config`.

use std::time::Duration;

/// Default keepalive interval (FS-SSH-020: 30 seconds).
pub const SSH_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

/// Number of consecutive keepalive misses before declaring the connection lost (FS-SSH-020).
pub const SSH_KEEPALIVE_MAX_MISSES: u32 = 3;

/// Build a `russh::client::Config` with keepalive enabled.
///
/// `interval` defaults to `SSH_KEEPALIVE_INTERVAL` and `max_misses` defaults to
/// `SSH_KEEPALIVE_MAX_MISSES`. Pass `None` to use the defaults.
pub fn make_client_config(
    interval: Option<Duration>,
    max_misses: Option<usize>,
) -> std::sync::Arc<russh::client::Config> {
    std::sync::Arc::new(russh::client::Config {
        keepalive_interval: Some(interval.unwrap_or(SSH_KEEPALIVE_INTERVAL)),
        keepalive_max: max_misses.unwrap_or(SSH_KEEPALIVE_MAX_MISSES as usize),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TEST-SSH-007 (partial) — keepalive constants (FS-SSH-020)
    // -----------------------------------------------------------------------

    #[test]
    fn keepalive_max_misses_is_three() {
        // FS-SSH-020 requires exactly 3 consecutive missed keepalives.
        assert_eq!(
            SSH_KEEPALIVE_MAX_MISSES, 3,
            "FS-SSH-020 requires exactly 3 consecutive missed keepalives before disconnect"
        );
    }

    #[test]
    fn keepalive_default_interval_is_30_seconds() {
        // FS-SSH-020 requires a default interval of 30 seconds.
        assert_eq!(
            SSH_KEEPALIVE_INTERVAL,
            Duration::from_secs(30),
            "FS-SSH-020 requires a default keepalive interval of 30 seconds"
        );
    }

    #[test]
    fn make_client_config_defaults_match_fs_ssh_020() {
        let config = make_client_config(None, None);
        assert_eq!(
            config.keepalive_interval,
            Some(SSH_KEEPALIVE_INTERVAL),
            "Default config must use SSH_KEEPALIVE_INTERVAL"
        );
        assert_eq!(
            config.keepalive_max, SSH_KEEPALIVE_MAX_MISSES as usize,
            "Default config must use SSH_KEEPALIVE_MAX_MISSES"
        );
    }

    #[test]
    fn make_client_config_custom_values() {
        let custom_interval = Duration::from_secs(60);
        let config = make_client_config(Some(custom_interval), Some(5));
        assert_eq!(config.keepalive_interval, Some(custom_interval));
        assert_eq!(config.keepalive_max, 5);
    }

    // -----------------------------------------------------------------------
    // FS-SSH-020 — per-connection keepalive overrides
    // -----------------------------------------------------------------------

    /// keepalive_interval_secs in SshConnectionConfig is forwarded to make_client_config.
    #[test]
    fn per_connection_interval_override_applied() {
        let custom_interval = Duration::from_secs(120);
        let config = make_client_config(Some(custom_interval), None);
        assert_eq!(
            config.keepalive_interval,
            Some(custom_interval),
            "Per-connection interval override must be forwarded to russh config"
        );
        // max falls back to default
        assert_eq!(config.keepalive_max, SSH_KEEPALIVE_MAX_MISSES as usize);
    }

    /// keepalive_max_failures in SshConnectionConfig is forwarded to make_client_config.
    #[test]
    fn per_connection_max_failures_override_applied() {
        let config = make_client_config(None, Some(7));
        assert_eq!(
            config.keepalive_max, 7,
            "Per-connection max failures override must be forwarded to russh config"
        );
        // interval falls back to default
        assert_eq!(config.keepalive_interval, Some(SSH_KEEPALIVE_INTERVAL));
    }

    /// Both overrides applied simultaneously.
    #[test]
    fn per_connection_both_overrides_applied() {
        let interval = Duration::from_secs(45);
        let config = make_client_config(Some(interval), Some(2));
        assert_eq!(config.keepalive_interval, Some(interval));
        assert_eq!(config.keepalive_max, 2);
    }

    /// Zero max_failures is accepted (edge case — disables keepalive).
    #[test]
    fn per_connection_zero_max_failures_accepted() {
        let config = make_client_config(None, Some(0));
        assert_eq!(config.keepalive_max, 0);
    }
}
