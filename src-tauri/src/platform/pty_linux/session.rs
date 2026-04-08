// SPDX-License-Identifier: MPL-2.0

use std::io::{Read, Write};
use std::os::unix::io::RawFd;
use std::sync::{Arc, Mutex};

use libc;
use portable_pty::PtySize;

use crate::error::PtyError;
use crate::platform::PtySession;

// ---------------------------------------------------------------------------
// LinuxPtySession
// ---------------------------------------------------------------------------

/// An active local PTY session on Linux.
///
/// Wraps `portable-pty`'s master PTY and child process handles.
/// `write()` sends bytes to the PTY master (keyboard input → shell).
/// `resize()` issues `TIOCSWINSZ` + SIGWINCH via `portable-pty`'s resize API.
/// `close()` / `Drop` closes the master fd — the kernel delivers SIGHUP to
/// the foreground process group (§7.1 of ARCHITECTURE.md, FS-PTY-007).
pub struct LinuxPtySession {
    pub(super) master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    /// Raw fd of the master PTY — used for `tcgetpgrp` (FS-PTY-008).
    /// Valid as long as `master` is alive.
    pub(super) master_fd: RawFd,
    /// PID of the shell process spawned on the slave side.
    pub(super) shell_pid: Option<u32>,
    pub(super) writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub(super) reader: Arc<Mutex<Box<dyn Read + Send>>>,
    pub(super) _child: Arc<Mutex<Box<dyn portable_pty::Child + Send>>>,
}

impl LinuxPtySession {
    /// Borrow the reader for the PTY read task.
    ///
    /// Returns a clone of the `Arc<Mutex<...>>` so the read task can hold it
    /// independently of the registry's write lock.
    pub fn reader_handle(&self) -> Arc<Mutex<Box<dyn Read + Send>>> {
        self.reader.clone()
    }

    /// Borrow the writer for the PTY read task (DSR/DA/CPR responses).
    pub fn writer_handle(&self) -> Arc<Mutex<Box<dyn Write + Send>>> {
        self.writer.clone()
    }
}

impl PtySession for LinuxPtySession {
    fn reader_handle(&self) -> Option<Arc<Mutex<Box<dyn Read + Send>>>> {
        Some(self.reader.clone())
    }

    fn writer_handle(&self) -> Option<Arc<Mutex<Box<dyn Write + Send>>>> {
        Some(self.writer.clone())
    }

    fn write(&mut self, data: &[u8]) -> Result<(), PtyError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| PtyError::Io(std::io::Error::other(e.to_string())))?;
        writer.write_all(data).map_err(PtyError::Io)
    }

    fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<(), PtyError> {
        let master = self
            .master
            .lock()
            .map_err(|e| PtyError::Resize(e.to_string()))?;
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width,
                pixel_height,
            })
            .map_err(|e| PtyError::Resize(e.to_string()))
    }

    fn close(self: Box<Self>) {
        // Drop self — Arc refcounts reach zero, master fd is dropped, kernel
        // delivers SIGHUP to the foreground process group (FS-PTY-007).
        // portable-pty's MasterPty Drop impl closes the underlying fd.
    }

    fn foreground_pgid(&self) -> Result<libc::pid_t, PtyError> {
        // SAFETY: master_fd is a valid open file descriptor owned by this
        // LinuxPtySession (kept alive by the Arc<Mutex<master>>). tcgetpgrp
        // is a pure read syscall with no memory-safety implications.
        let pgid = unsafe { libc::tcgetpgrp(self.master_fd) };
        if pgid == -1 {
            Err(PtyError::Io(std::io::Error::last_os_error()))
        } else {
            Ok(pgid)
        }
    }

    fn shell_pid(&self) -> Option<u32> {
        self.shell_pid
    }

    fn try_wait_exit_code(&self) -> Option<Option<i32>> {
        let mut child = self._child.lock().ok()?;
        let status = child.try_wait().ok()??;
        // ExitStatus::exit_code() returns u32; cast safely (capped at i32::MAX).
        let code = i32::try_from(status.exit_code()).unwrap_or(i32::MAX);
        Some(Some(code))
    }

    fn wait_exit_code(&self) -> Option<Option<i32>> {
        let mut child = self._child.lock().ok()?;
        let status = child.wait().ok()?;
        let code = i32::try_from(status.exit_code()).unwrap_or(i32::MAX);
        Some(Some(code))
    }
}
