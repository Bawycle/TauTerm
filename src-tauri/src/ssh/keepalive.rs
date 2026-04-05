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
}
