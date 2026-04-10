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
pub mod validation;

#[cfg(feature = "e2e-testing")]
pub mod pty_injectable;

#[cfg(feature = "e2e-testing")]
pub use pty_injectable::{InjectablePtyBackend, InjectableRegistry, create_injectable_pty_backend};

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
    ///
    /// `pixel_width` and `pixel_height` are the initial cell pixel dimensions
    /// passed to `TIOCSWINSZ` / SSH `pty-req`. Pass `0` when unknown.
    ///
    /// `working_directory` sets the initial working directory for the spawned
    /// process. `None` inherits the parent's working directory.
    #[allow(clippy::too_many_arguments)]
    fn open_session(
        &self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
        working_directory: Option<&std::path::Path>,
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

    /// Return the name of the current foreground process on this PTY master.
    ///
    /// Reads `/proc/{pgid}/comm` after calling `tcgetpgrp(master_fd)`. Returns
    /// `None` for session types that do not support it (SSH, stubs) or when the
    /// proc file cannot be read.
    fn foreground_process_name(&self) -> Option<String> {
        None
    }

    /// Return the process group ID of the current foreground process on this PTY master.
    ///
    /// Uses `tcgetpgrp(master_fd)` (Linux: TIOCGPGRP).
    /// Returns `Err` if the syscall fails or this session type does not support it.
    /// The default returns `Err` (SSH sessions and stub backends have no master fd).
    fn foreground_pgid(&self) -> Result<libc::pid_t, PtyError> {
        Err(PtyError::Io(std::io::Error::other(
            "foreground_pgid not supported on this session type",
        )))
    }

    /// Return the PID of the shell process spawned for this session.
    ///
    /// `None` if this session type does not track a shell PID (SSH sessions, stubs).
    fn shell_pid(&self) -> Option<u32> {
        None
    }

    /// Attempt a non-blocking wait on the child process to obtain its exit code.
    ///
    /// Returns `Some(exit_code)` where `exit_code` is:
    ///   - `Some(0)` for a clean exit
    ///   - `Some(n)` for a non-zero exit or a signal-killed process (best-effort code)
    ///   - `None` if the exit code could not be determined
    ///
    /// Returns `None` if the process has not yet exited or this session type
    /// does not support it (SSH sessions, stubs).
    fn try_wait_exit_code(&self) -> Option<Option<i32>> {
        None
    }

    /// Blocking wait for the child process to exit and obtain its exit code.
    ///
    /// Unlike `try_wait_exit_code`, this call blocks until the child has fully
    /// exited (zombie reaped). It must only be called after the PTY EOF has been
    /// observed, which guarantees the process is already dead — so the blocking
    /// wait returns in microseconds on Linux.
    ///
    /// Returns `Some(exit_code)` where `exit_code` is:
    ///   - `Some(0)` for a clean exit
    ///   - `Some(n)` for a non-zero exit
    ///   - `None` if the exit code could not be determined (signal kill, etc.)
    ///
    /// Returns `None` if this session type does not support it (SSH sessions, stubs).
    /// Default: delegates to `try_wait_exit_code` (best-effort fallback).
    fn wait_exit_code(&self) -> Option<Option<i32>> {
        self.try_wait_exit_code()
    }

    /// Get a shared reader handle for the PTY read task.
    ///
    /// Platform implementations that support a read task return `Some(...)`.
    /// The default returns `None` (no reader available).
    fn reader_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Read + Send>>>> {
        None
    }

    /// Get a shared writer handle for writing responses back to the PTY master.
    ///
    /// Used by the PTY read task to send DSR/DA/CPR responses without holding
    /// the `VtProcessor` write-lock. The default returns `None`.
    fn writer_handle(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>> {
        None
    }

    /// Return the injectable sender for this session, if this is an
    /// `InjectablePtySession`.
    ///
    /// The default returns `None`. Only `InjectablePtySession` returns `Some`.
    /// This method only exists when the `e2e-testing` feature is active.
    #[cfg(feature = "e2e-testing")]
    fn injectable_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
        None
    }
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
