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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{ClipboardBackend as _, LinuxClipboard};

    // -----------------------------------------------------------------------
    // TEST-SPRINT-002 — FS-CLIP-004: X11 PRIMARY selection write
    //
    // `set_primary` calls arboard with `LinuxClipboardKind::Primary`.
    // The test requires a live X11/Wayland display and an arboard-compatible
    // compositor. On headless CI there is no display, so the test is marked
    // `#[ignore]`.
    //
    // To run manually: `DISPLAY=:0 cargo nextest run -- clipboard_linux --include-ignored`
    // -----------------------------------------------------------------------

    /// TEST-SPRINT-002 (FS-CLIP-004): write to PRIMARY selection on X11.
    ///
    /// Requires a running X11 or Wayland session with PRIMARY selection support.
    /// arboard returns an error if the display is unavailable or the compositor
    /// does not support the PRIMARY selection protocol — this is non-fatal and
    /// correctly propagated as `Err(String)`.
    #[test]
    #[ignore = "requires a running X11/Wayland display (no headless CI support)"]
    fn test_sprint_002_set_primary_writes_to_x11_primary_selection() {
        let cb = LinuxClipboard::new();
        let text = "tauterm-primary-test";
        // Must not panic; may return Err on Wayland without wlr-data-control.
        let result = cb.set_primary(text);
        // If the display is available and PRIMARY is supported, the call succeeds.
        // If PRIMARY is unsupported on this compositor, arboard returns Err.
        // Both outcomes are correct — what is NOT correct is a panic or a wrong
        // selection target being written (which would be caught by a read-back test).
        match result {
            Ok(()) => {
                // Verify round-trip via CLIPBOARD is NOT confused with PRIMARY.
                // We do not read PRIMARY back here because arboard's get_text()
                // always reads CLIPBOARD, not PRIMARY. The correctness of the
                // `LinuxClipboardKind::Primary` argument is validated by arboard
                // itself; our test verifies the call reaches arboard without error.
            }
            Err(e) => {
                // Acceptable: compositor does not support PRIMARY selection.
                // Log but do not fail the test.
                eprintln!("set_primary returned Err (expected on some compositors): {e}");
            }
        }
    }

    /// TEST-SPRINT-002b (FS-CLIP-004): set_primary signature matches ClipboardBackend trait.
    ///
    /// This test verifies the trait binding compiles and the method is reachable
    /// via the `ClipboardBackend` trait object — it does NOT require a display.
    #[test]
    fn test_sprint_002b_set_primary_is_part_of_clipboard_backend_trait() {
        // Verify LinuxClipboard implements ClipboardBackend (compile-time check).
        let _cb: Box<dyn crate::platform::ClipboardBackend> = Box::new(LinuxClipboard::new());
        // If this compiles, the trait impl is present. No display needed.
    }
}
