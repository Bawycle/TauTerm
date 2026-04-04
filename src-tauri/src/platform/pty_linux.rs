// SPDX-License-Identifier: MPL-2.0

//! Linux PTY backend — `portable-pty` crate (`UnixPtySystem`).
//!
//! Wraps `portable-pty`'s `UnixPtySystem` to implement the `PtyBackend` and
//! `PtySession` traits. The master fd is exposed as a `tokio::io::unix::AsyncFd`
//! for the PTY read task (§7.1 of ARCHITECTURE.md).
//!
//! Login shell (FS-PTY-013): the first tab passes `--login` to the shell command.
//! This is controlled by the `login` flag in `open_session` via the `args` slice.
//!
//! Environment (FS-PTY-011, FS-PTY-012): mandatory variables are injected by
//! `SessionRegistry::create_tab` before calling `open_session`. The env slice
//! passed here is forwarded verbatim to the child process.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::error::PtyError;
use crate::platform::{PtyBackend, PtySession};

// ---------------------------------------------------------------------------
// LinuxPtyBackend
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct LinuxPtyBackend {}

impl LinuxPtyBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl PtyBackend for LinuxPtyBackend {
    fn open_session(
        &self,
        cols: u16,
        rows: u16,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<Box<dyn PtySession>, PtyError> {
        let pty_system = native_pty_system();

        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty_pair = pty_system
            .openpty(size)
            .map_err(|e| PtyError::Open(e.to_string()))?;

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        for (key, val) in env {
            cmd.env(key, val);
        }

        // Spawn the child on the slave side.
        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // The slave fd is closed in the parent after spawning (portable-pty handles this).
        // Drop the slave explicitly so the fd is released in the parent process.
        drop(pty_pair.slave);

        // Get a writer for sending input to the PTY master.
        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Open(e.to_string()))?;

        // Get a reader for reading PTY output.
        let reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Open(e.to_string()))?;

        Ok(Box::new(LinuxPtySession {
            master: Arc::new(Mutex::new(pty_pair.master)),
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
            _child: Arc::new(Mutex::new(child)),
        }))
    }
}

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
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    _child: Arc<Mutex<Box<dyn portable_pty::Child + Send>>>,
}

impl LinuxPtySession {
    /// Borrow the reader for the PTY read task.
    ///
    /// Returns a clone of the `Arc<Mutex<...>>` so the read task can hold it
    /// independently of the registry's write lock.
    pub fn reader_handle(&self) -> Arc<Mutex<Box<dyn Read + Send>>> {
        self.reader.clone()
    }
}

impl PtySession for LinuxPtySession {
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
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: open a real PTY session with /bin/sh.
    fn open_sh_session(cols: u16, rows: u16) -> Result<Box<dyn PtySession>, PtyError> {
        let backend = LinuxPtyBackend::new();
        backend.open_session(
            cols,
            rows,
            "/bin/sh",
            &[],
            &[
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("LINES", &rows.to_string()),
                ("COLUMNS", &cols.to_string()),
            ],
        )
    }

    // --- FPL-S-001: open_session with /bin/sh succeeds ---
    #[test]
    fn fpl_s_001_open_session_bin_sh_succeeds() {
        let result = open_sh_session(80, 24);
        assert!(
            result.is_ok(),
            "open_session(/bin/sh) must succeed on Linux. Error: {:?}",
            result.err()
        );
    }

    // --- FPL-S-002: open_session with nonexistent command returns Err(Spawn) ---
    #[test]
    fn fpl_s_002_open_session_nonexistent_command_returns_err() {
        let backend = LinuxPtyBackend::new();
        let result = backend.open_session(80, 24, "/nonexistent_shell_tauterm_test", &[], &[]);
        assert!(
            result.is_err(),
            "open_session with nonexistent command must return Err"
        );
    }

    // --- FPL-S-003: two concurrent sessions have independent master fds ---
    #[test]
    fn fpl_s_003_two_sessions_are_independent() {
        let s1 = open_sh_session(80, 24);
        let s2 = open_sh_session(80, 24);
        assert!(s1.is_ok(), "first session must open");
        assert!(s2.is_ok(), "second session must open");
        // Both sessions are alive simultaneously — no fd collision.
    }

    // --- FPL-W-001: write small payload succeeds ---
    #[test]
    fn fpl_w_001_write_small_payload_succeeds() {
        let mut session = open_sh_session(80, 24).expect("open session");
        let result = session.write(b"ls\n");
        assert!(result.is_ok(), "write small payload must succeed: {:?}", result.err());
    }

    // --- FPL-W-002: write 64 KiB payload succeeds ---
    #[test]
    fn fpl_w_002_write_max_payload_succeeds() {
        let mut session = open_sh_session(80, 24).expect("open session");
        let payload = vec![b'a'; 65_536];
        let result = session.write(&payload);
        assert!(result.is_ok(), "write 64 KiB must succeed: {:?}", result.err());
    }

    // --- FPL-R-001: resize returns Ok ---
    #[test]
    fn fpl_r_001_resize_succeeds() {
        let mut session = open_sh_session(80, 24).expect("open session");
        let result = session.resize(120, 40, 0, 0);
        assert!(result.is_ok(), "resize must succeed: {:?}", result.err());
    }

    // --- FPL-R-002: resize with pixel dimensions succeeds ---
    #[test]
    fn fpl_r_002_resize_with_pixel_dims_succeeds() {
        let mut session = open_sh_session(80, 24).expect("open session");
        let result = session.resize(80, 24, 960, 480);
        assert!(result.is_ok(), "resize with pixel dims must succeed: {:?}", result.err());
    }

    // --- FPL-R-003: degenerate resize (0,0) succeeds without panic ---
    #[test]
    fn fpl_r_003_resize_degenerate_zero_does_not_panic() {
        let mut session = open_sh_session(80, 24).expect("open session");
        // Degenerate size — result may be Ok or Err depending on kernel,
        // but must not panic.
        let _ = session.resize(0, 0, 0, 0);
    }

    // --- FPL-C-003: dropping PtyTaskHandle aborts the task ---
    // (Tested structurally: PtyTaskHandle::drop calls abort())
    #[test]
    fn fpl_c_003_pty_task_handle_drop_aborts_task() {
        use crate::session::pty_task::PtyTaskHandle;
        // Verify that PtyTaskHandle::Drop calls abort — we test via
        // creating a trivial task and confirming it is cancelled on drop.
        let task = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let jh = tokio::spawn(async {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                });
                let handle = PtyTaskHandle { abort: jh.abort_handle() };
                drop(handle);
                jh.await
            });
        // Task was aborted — should return Err(JoinError::Cancelled)
        assert!(task.is_err(), "task must be cancelled after handle drop");
    }
}
