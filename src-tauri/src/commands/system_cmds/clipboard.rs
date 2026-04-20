// SPDX-License-Identifier: MPL-2.0

use crate::error::TauTermError;

/// Maximum clipboard text size: 16 MiB.
///
/// Protects against clipboard-based DoS where a process in a pane triggers
/// a large clipboard write via OSC 52 or direct IPC call.
pub(super) const MAX_CLIPBOARD_LEN: usize = 16 * 1024 * 1024;

#[tauri::command]
#[specta::specta]
pub async fn copy_to_clipboard(text: String) -> Result<(), TauTermError> {
    if text.len() > MAX_CLIPBOARD_LEN {
        return Err(TauTermError::new(
            "CLIPBOARD_TOO_LARGE",
            "Clipboard text exceeds maximum allowed size.",
        ));
    }
    tokio::task::spawn_blocking(move || {
        let mut cb = arboard::Clipboard::new().map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_UNAVAILABLE",
                "Could not access the system clipboard.",
                e.to_string(),
            )
        })?;

        // Write to CLIPBOARD (Ctrl+C/V selection).
        cb.set_text(&text).map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_WRITE_FAILED",
                "Failed to write to clipboard.",
                e.to_string(),
            )
        })?;

        // On Linux/X11 and Wayland, also write to PRIMARY so that
        // middle-click paste works as expected (FS-CLIP-006).
        #[cfg(target_os = "linux")]
        {
            use arboard::{LinuxClipboardKind, SetExtLinux as _};
            // Non-fatal: PRIMARY may be unsupported on some Wayland compositors.
            // Log a warning but do not propagate the error to the caller.
            if let Err(e) = cb
                .set()
                .clipboard(LinuxClipboardKind::Primary)
                .text(text.clone())
            {
                tracing::warn!("Failed to write to X11 PRIMARY selection: {e}");
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| {
        TauTermError::with_detail("INTERNAL_ERROR", "Clipboard task failed.", e.to_string())
    })?
}

#[tauri::command]
#[specta::specta]
pub async fn get_clipboard() -> Result<String, TauTermError> {
    tokio::task::spawn_blocking(|| {
        let mut cb = arboard::Clipboard::new().map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_UNAVAILABLE",
                "Could not access the system clipboard.",
                e.to_string(),
            )
        })?;
        cb.get_text().map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_READ_FAILED",
                "Failed to read from clipboard.",
                e.to_string(),
            )
        })
    })
    .await
    .map_err(|e| {
        TauTermError::with_detail("INTERNAL_ERROR", "Clipboard task failed.", e.to_string())
    })?
}
