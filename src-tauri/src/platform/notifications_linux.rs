// SPDX-License-Identifier: MPL-2.0

//! Linux notification backend — D-Bus `org.freedesktop.Notifications`.
//!
//! Sends desktop notifications via `notify-rust`, which uses the D-Bus
//! `org.freedesktop.Notifications` interface. Falls back to a no-op if
//! D-Bus is unavailable or the notification daemon is not running (§7.4).
//! Triggered by `VtProcessor` on BEL (0x07) in a non-active pane (FS-VT-090).

use crate::platform::NotificationBackend;

#[derive(Default)]
pub struct LinuxNotifications {}

impl LinuxNotifications {
    pub fn new() -> Self {
        Self {}
    }
}

impl NotificationBackend for LinuxNotifications {
    fn notify(&self, title: &str, body: &str) {
        // notify-rust sends via D-Bus; errors are logged and swallowed so the
        // caller (VtProcessor bell handler) is never disrupted by missing D-Bus.
        let result = notify_rust::Notification::new()
            .summary(title)
            .body(body)
            .appname("TauTerm")
            // Use a generic terminal icon; falls back gracefully if unavailable.
            .icon("utilities-terminal")
            .timeout(notify_rust::Timeout::Milliseconds(4000))
            .show();

        if let Err(e) = result {
            tracing::debug!("Desktop notification unavailable (D-Bus absent?): {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `notify` does not panic in a headless / no-D-Bus environment.
    /// The notification may fail silently — that is the expected behaviour.
    #[test]
    fn notify_does_not_panic_in_headless_env() {
        let backend = LinuxNotifications::new();
        // Must not panic even when D-Bus / notification daemon is absent.
        backend.notify("TauTerm Test", "Bell event from pane");
    }
}
