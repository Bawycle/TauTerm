// SPDX-License-Identifier: MPL-2.0

//! Windows notification backend stub — not supported in v1.

use crate::platform::NotificationBackend;

pub struct WindowsNotifications {}

impl WindowsNotifications {
    pub fn new() -> Self {
        Self {}
    }
}

impl NotificationBackend for WindowsNotifications {
    fn notify(&self, _title: &str, _body: &str) {
        unimplemented!("Windows notifications not supported in v1")
    }
}
