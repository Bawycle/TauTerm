// SPDX-License-Identifier: MPL-2.0

//! Platform Abstraction Layer (PAL) — trait definitions and factory functions.
//!
//! Defines the four PAL traits (`PtyBackend`, `CredentialStore`, `ClipboardBackend`,
//! `NotificationBackend`) and their platform-specific factory functions.
//!
//! Platform dispatch (`#[cfg(target_os = ...)]`) lives here, not in sub-files
//! (§3.2 of ARCHITECTURE.md).
//!
//! v1 targets Linux only. macOS and Windows stubs are included for future
//! cross-platform support (§1.3).

pub mod clipboard_linux;
pub mod credentials_linux;
pub mod notifications_linux;
pub mod pty_linux;

// Future platform stubs (currently unreachable in v1).
pub mod clipboard_macos;
pub mod credentials_macos;
pub mod notifications_macos;
pub mod pty_macos;

pub mod clipboard_windows;
pub mod credentials_windows;
pub mod notifications_windows;
pub mod pty_windows;

use crate::error::{CredentialError, PtyError};

// ---------------------------------------------------------------------------
// PTY Backend
// ---------------------------------------------------------------------------

/// Trait for PTY backend operations.
/// Implemented by `platform/pty_linux.rs` on Linux.
pub trait PtyBackend: Send + Sync {
    /// Open a new PTY pair and spawn a shell process.
    fn open_session(
        &self,
        cols: u16,
        rows: u16,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<Box<dyn PtySession>, PtyError>;
}

/// Trait for an active PTY session.
pub trait PtySession: Send + Sync {
    /// Write bytes to the PTY master (keyboard input → shell).
    fn write(&mut self, data: &[u8]) -> Result<(), PtyError>;

    /// Resize the PTY (TIOCSWINSZ + SIGWINCH).
    fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<(), PtyError>;

    /// Close the PTY, delivering SIGHUP to the foreground process group.
    fn close(self: Box<Self>);
}

// ---------------------------------------------------------------------------
// Credential Store
// ---------------------------------------------------------------------------

/// Trait for the OS keychain / secret store.
pub trait CredentialStore: Send + Sync {
    /// Returns `true` if the credential store is available on this system.
    fn is_available(&self) -> bool;

    /// Store a secret (password, passphrase).
    fn store(&self, key: &str, secret: &[u8]) -> Result<(), CredentialError>;

    /// Retrieve a secret. Returns `None` if not found.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CredentialError>;

    /// Delete a stored secret.
    fn delete(&self, key: &str) -> Result<(), CredentialError>;
}

// ---------------------------------------------------------------------------
// Clipboard Backend
// ---------------------------------------------------------------------------

/// Trait for clipboard access.
pub trait ClipboardBackend: Send + Sync {
    /// Write text to the CLIPBOARD selection.
    fn set_clipboard(&self, text: &str) -> Result<(), String>;

    /// Read the CLIPBOARD selection.
    fn get_clipboard(&self) -> Result<String, String>;

    /// Write text to the X11/Wayland PRIMARY selection (middle-click paste).
    fn set_primary(&self, text: &str) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// Notification Backend
// ---------------------------------------------------------------------------

/// Trait for system desktop notifications.
pub trait NotificationBackend: Send + Sync {
    /// Send a notification. No-op if the notification system is unavailable.
    fn notify(&self, title: &str, body: &str);
}

// ---------------------------------------------------------------------------
// Factory functions — platform dispatch
// ---------------------------------------------------------------------------

/// Create the platform-specific PTY backend.
pub fn create_pty_backend() -> Box<dyn PtyBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(pty_linux::LinuxPtyBackend::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(pty_macos::MacOsPtyBackend::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(pty_windows::WindowsPtyBackend::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("Unsupported platform — no PTY backend available.");
    }
}

/// Create the platform-specific credential store.
pub fn create_credential_store() -> Box<dyn CredentialStore> {
    #[cfg(target_os = "linux")]
    {
        Box::new(credentials_linux::LinuxCredentialStore::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(credentials_macos::MacOsCredentialStore::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(credentials_windows::WindowsCredentialStore::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("Unsupported platform — no credential store available.");
    }
}

/// Create the platform-specific clipboard backend.
pub fn create_clipboard_backend() -> Box<dyn ClipboardBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(clipboard_linux::LinuxClipboard::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(clipboard_macos::MacOsClipboard::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(clipboard_windows::WindowsClipboard::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("Unsupported platform — no clipboard backend available.");
    }
}

/// Create the platform-specific notification backend.
pub fn create_notification_backend() -> Box<dyn NotificationBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(notifications_linux::LinuxNotifications::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(notifications_macos::MacOsNotifications::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(notifications_windows::WindowsNotifications::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("Unsupported platform — no notification backend available.");
    }
}
