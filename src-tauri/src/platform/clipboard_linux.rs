// SPDX-License-Identifier: MPL-2.0

//! Linux clipboard backend — `arboard` crate (X11 + Wayland).
//!
//! Implements `ClipboardBackend` for both CLIPBOARD (Ctrl+C/V) and
//! X11/Wayland PRIMARY selection (middle-click paste, FS-CLIP-006).

use arboard::{LinuxClipboardKind, SetExtLinux as _};

use crate::platform::ClipboardBackend;

#[derive(Default)]
pub struct LinuxClipboard {}

impl LinuxClipboard {
    pub fn new() -> Self {
        Self {}
    }
}

impl ClipboardBackend for LinuxClipboard {
    fn set_clipboard(&self, text: &str) -> Result<(), String> {
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        cb.set_text(text).map_err(|e| e.to_string())
    }

    fn get_clipboard(&self) -> Result<String, String> {
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        cb.get_text().map_err(|e| e.to_string())
    }

    /// Write to the X11/Wayland PRIMARY selection (middle-click paste).
    ///
    /// On Wayland, PRIMARY requires compositor support for wlr-data-control v2
    /// or the zwp_primary_selection_device_manager protocol. arboard returns
    /// an error if the compositor does not support it; this is non-fatal and
    /// the caller receives the error so it can warn as appropriate.
    fn set_primary(&self, text: &str) -> Result<(), String> {
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        cb.set()
            .clipboard(LinuxClipboardKind::Primary)
            .text(text.to_owned())
            .map_err(|e| e.to_string())
    }
}
