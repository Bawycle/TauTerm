// SPDX-License-Identifier: MPL-2.0

//! macOS notification backend stub — not supported in v1.

use crate::platform::NotificationBackend;

pub struct MacOsNotifications {}

impl MacOsNotifications {
    pub fn new() -> Self {
        Self {}
    }
}

impl NotificationBackend for MacOsNotifications {
    fn notify(&self, _title: &str, _body: &str) {
        unimplemented!("macOS notifications not supported in v1")
    }
}
