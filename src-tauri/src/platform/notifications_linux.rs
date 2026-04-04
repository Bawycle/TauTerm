// SPDX-License-Identifier: MPL-2.0

//! Linux notification backend — D-Bus `org.freedesktop.Notifications`.
//!
//! Falls back to a no-op if D-Bus is unavailable (§7.4).
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
    fn notify(&self, _title: &str, _body: &str) {
        // TODO: implement via D-Bus org.freedesktop.Notifications.
        // No-op if D-Bus is unavailable.
    }
}
