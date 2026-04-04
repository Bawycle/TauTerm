// SPDX-License-Identifier: MPL-2.0

//! macOS clipboard backend stub — not supported in v1.

use crate::platform::ClipboardBackend;

#[derive(Default)]
pub struct MacOsClipboard {}

impl MacOsClipboard {
    pub fn new() -> Self {
        Self {}
    }
}

impl ClipboardBackend for MacOsClipboard {
    fn set_clipboard(&self, _text: &str) -> Result<(), String> {
        unimplemented!("macOS clipboard not supported in v1")
    }

    fn get_clipboard(&self) -> Result<String, String> {
        unimplemented!("macOS clipboard not supported in v1")
    }

    fn set_primary(&self, _text: &str) -> Result<(), String> {
        unimplemented!("macOS PRIMARY selection not supported in v1")
    }
}
