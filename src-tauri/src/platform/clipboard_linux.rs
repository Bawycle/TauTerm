// SPDX-License-Identifier: MPL-2.0

//! Linux clipboard backend — `arboard` crate (X11 + Wayland).
//!
//! Handles both CLIPBOARD (Ctrl+C/V) and X11 PRIMARY selection (§7.3).

use crate::platform::ClipboardBackend;

#[derive(Default)]
pub struct LinuxClipboard {}

impl LinuxClipboard {
    pub fn new() -> Self {
        Self {}
    }
}

impl ClipboardBackend for LinuxClipboard {
    fn set_clipboard(&self, _text: &str) -> Result<(), String> {
        // TODO: implement via arboard crate.
        Err("Clipboard not yet implemented.".to_string())
    }

    fn get_clipboard(&self) -> Result<String, String> {
        // TODO: implement via arboard crate.
        Err("Clipboard not yet implemented.".to_string())
    }

    fn set_primary(&self, _text: &str) -> Result<(), String> {
        // TODO: implement via arboard SetExtX11 or x11-clipboard crate.
        Err("PRIMARY selection not yet implemented.".to_string())
    }
}
