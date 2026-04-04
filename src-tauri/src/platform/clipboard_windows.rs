// SPDX-License-Identifier: MPL-2.0

//! Windows clipboard backend stub — not supported in v1.

use crate::platform::ClipboardBackend;

#[derive(Default)]
pub struct WindowsClipboard {}

impl WindowsClipboard {
    pub fn new() -> Self {
        Self {}
    }
}

impl ClipboardBackend for WindowsClipboard {
    fn set_clipboard(&self, _text: &str) -> Result<(), String> {
        unimplemented!("Windows clipboard not supported in v1")
    }

    fn get_clipboard(&self) -> Result<String, String> {
        unimplemented!("Windows clipboard not supported in v1")
    }

    fn set_primary(&self, _text: &str) -> Result<(), String> {
        unimplemented!("Windows PRIMARY selection not supported in v1")
    }
}
