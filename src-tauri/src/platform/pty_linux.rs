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
        assert!(
            result.is_ok(),
            "write small payload must succeed: {:?}",
            result.err()
        );
    }

    // --- FPL-W-002: write 64 KiB payload succeeds ---
    #[test]
    fn fpl_w_002_write_max_payload_succeeds() {
        let mut session = open_sh_session(80, 24).expect("open session");
        let payload = vec![b'a'; 65_536];
        let result = session.write(&payload);
        assert!(
            result.is_ok(),
            "write 64 KiB must succeed: {:?}",
            result.err()
        );
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
        assert!(
            result.is_ok(),
            "resize with pixel dims must succeed: {:?}",
            result.err()
        );
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
                let abort = jh.abort_handle();
                let handle = PtyTaskHandle::from_abort_handle(abort);
                drop(handle);
                jh.await
            });
        // Task was aborted — should return Err(JoinError::Cancelled)
        assert!(task.is_err(), "task must be cancelled after handle drop");
    }

    // -----------------------------------------------------------------------
    // Integration test harness — PTY round-trip helpers
    // -----------------------------------------------------------------------

    /// Open a real PTY session with the given command/args/env.
    ///
    /// Returns `Box<dyn PtySession>`. Callers use `PtySession::reader_handle()`
    /// (or `as_linux_pty()`) to access the underlying reader — no raw-pointer
    /// downcast required.
    fn open_linux_session_with_env(
        cols: u16,
        rows: u16,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Box<dyn PtySession> {
        let backend = LinuxPtyBackend::new();
        backend
            .open_session(cols, rows, command, args, env)
            .expect("open_linux_session_with_env: open_session failed")
    }

    /// Read bytes from a PTY reader until the `expected` substring appears in
    /// the accumulated output, or until `timeout` expires.
    ///
    /// Returns `Some(accumulated_output)` if found, `None` on timeout.
    ///
    /// Uses a dedicated thread + channel to enforce the timeout without tokio.
    fn read_until_timeout(
        reader: Arc<Mutex<Box<dyn Read + Send>>>,
        expected: &str,
        timeout: std::time::Duration,
    ) -> Option<String> {
        let expected = expected.to_string();
        let (tx, rx) = std::sync::mpsc::channel::<String>();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut accumulated = String::new();
            loop {
                let n = {
                    let mut r = reader.lock().expect("reader lock poisoned");
                    match r.read(&mut buf) {
                        Ok(0) => break, // EOF
                        Ok(n) => n,
                        Err(_) => break,
                    }
                };
                let chunk = String::from_utf8_lossy(&buf[..n]);
                accumulated.push_str(&chunk);
                if accumulated.contains(&expected) {
                    let _ = tx.send(accumulated);
                    return;
                }
            }
        });

        rx.recv_timeout(timeout).ok()
    }

    // -----------------------------------------------------------------------
    // FPL-S-004 to FPL-S-009 — Environment variable injection (FS-PTY-011)
    // -----------------------------------------------------------------------

    /// FPL-S-004: TERM must be set to "xterm-256color" in the child process.
    #[test]
    fn fpl_s_004_env_term_is_xterm_256color() {
        let rows: u16 = 24;
        let cols: u16 = 80;
        let session = open_linux_session_with_env(
            cols,
            rows,
            "/bin/sh",
            &["-c", "printenv TERM; exit"],
            &[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")],
        );
        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");
        let output =
            read_until_timeout(reader, "xterm-256color", std::time::Duration::from_secs(5));
        assert!(
            output.is_some(),
            "FPL-S-004: TERM=xterm-256color must appear in child process output"
        );
    }

    /// FPL-S-005: COLORTERM must be set to "truecolor" in the child process.
    #[test]
    fn fpl_s_005_env_colorterm_is_truecolor() {
        let session = open_linux_session_with_env(
            80,
            24,
            "/bin/sh",
            &["-c", "printenv COLORTERM; exit"],
            &[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")],
        );
        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");
        let output = read_until_timeout(reader, "truecolor", std::time::Duration::from_secs(5));
        assert!(
            output.is_some(),
            "FPL-S-005: COLORTERM=truecolor must appear in child process output"
        );
    }

    /// FPL-S-006: LINES must match the rows passed to open_session.
    #[test]
    fn fpl_s_006_env_lines_matches_rows() {
        let rows: u16 = 30;
        let cols: u16 = 80;
        let session = open_linux_session_with_env(
            cols,
            rows,
            "/bin/sh",
            &["-c", "printenv LINES; exit"],
            &[
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("LINES", &rows.to_string()),
                ("COLUMNS", &cols.to_string()),
            ],
        );
        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");
        let output = read_until_timeout(reader, "30", std::time::Duration::from_secs(5));
        assert!(
            output.is_some(),
            "FPL-S-006: LINES=30 must appear in child process output"
        );
    }

    /// FPL-S-007: COLUMNS must match the cols passed to open_session.
    #[test]
    fn fpl_s_007_env_columns_matches_cols() {
        let rows: u16 = 24;
        let cols: u16 = 132;
        let session = open_linux_session_with_env(
            cols,
            rows,
            "/bin/sh",
            &["-c", "printenv COLUMNS; exit"],
            &[
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("LINES", &rows.to_string()),
                ("COLUMNS", &cols.to_string()),
            ],
        );
        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");
        let output = read_until_timeout(reader, "132", std::time::Duration::from_secs(5));
        assert!(
            output.is_some(),
            "FPL-S-007: COLUMNS=132 must appear in child process output"
        );
    }

    /// FPL-S-008: TERM_PROGRAM must be set to "TauTerm" in the child process.
    #[test]
    fn fpl_s_008_env_term_program_is_tauterm() {
        let session = open_linux_session_with_env(
            80,
            24,
            "/bin/sh",
            &["-c", "printenv TERM_PROGRAM; exit"],
            &[
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("TERM_PROGRAM", "TauTerm"),
            ],
        );
        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");
        let output = read_until_timeout(reader, "TauTerm", std::time::Duration::from_secs(5));
        assert!(
            output.is_some(),
            "FPL-S-008: TERM_PROGRAM=TauTerm must appear in child process output"
        );
    }

    /// FPL-S-009: TERM_PROGRAM_VERSION must be set in the child process.
    #[test]
    fn fpl_s_009_env_term_program_version_is_set() {
        let version = env!("CARGO_PKG_VERSION");
        let session = open_linux_session_with_env(
            80,
            24,
            "/bin/sh",
            &["-c", "printenv TERM_PROGRAM_VERSION; exit"],
            &[
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("TERM_PROGRAM", "TauTerm"),
                ("TERM_PROGRAM_VERSION", version),
            ],
        );
        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");
        let output = read_until_timeout(reader, version, std::time::Duration::from_secs(5));
        assert!(
            output.is_some(),
            "FPL-S-009: TERM_PROGRAM_VERSION={version} must appear in child process output"
        );
    }

    // -----------------------------------------------------------------------
    // FPL-W-003 — Write after session close returns an error
    // -----------------------------------------------------------------------

    /// FPL-W-003: Closing the PTY session (via Drop) must not panic, and
    /// subsequent read on the master reader must return EOF or an error (no data).
    ///
    /// Background: On Linux PTY, writing to the master fd after the child exits
    /// does not reliably return EIO — the kernel may buffer the write in the
    /// character device ring. The observable "dead fd" condition on a PTY master is
    /// read-side: once the slave fd is closed (child exited), reading the master
    /// returns EIO. We test this read-side behaviour here, not write-side.
    #[test]
    fn fpl_w_003_read_after_child_exit_returns_eof_or_error() {
        let backend = LinuxPtyBackend::new();
        let session = backend
            .open_session(
                80,
                24,
                "/bin/sh",
                &["-c", "exit 0"], // shell exits immediately
                &[("TERM", "xterm-256color")],
            )
            .expect("open session");

        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");

        // The reader must eventually return EOF or EIO after the child exits.
        // We poll until we get a 0-byte read or an error.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut got_eof = false;
        {
            let mut r = reader.lock().expect("lock");
            let mut buf = [0u8; 64];
            while std::time::Instant::now() < deadline {
                match r.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        got_eof = true;
                        break;
                    }
                    Ok(_) => {}
                }
            }
        }

        assert!(
            got_eof,
            "FPL-W-003: reading from the PTY master after child exit must return EOF or error"
        );

        // Dropping the session must not panic.
        drop(session);
    }

    // -----------------------------------------------------------------------
    // FPL-W-004 — Master write is readable via reader_handle (round-trip)
    // -----------------------------------------------------------------------

    /// FPL-W-004: Bytes written to the PTY (simulating keyboard input `echo`) are
    /// echoed back through the master reader, validating the full round-trip path
    /// used by the production PTY read task.
    ///
    /// We use `echo` via the shell to produce predictable output on the master.
    #[test]
    fn fpl_w_004_write_master_readable_via_reader_handle() {
        let backend = LinuxPtyBackend::new();
        let session = backend
            .open_session(
                80,
                24,
                "/bin/sh",
                &["-c", "echo FPL_W_004_MARKER; sleep 5"],
                &[("TERM", "xterm-256color")],
            )
            .expect("open session");

        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");

        let output = read_until_timeout(
            reader,
            "FPL_W_004_MARKER",
            std::time::Duration::from_secs(5),
        );
        assert!(
            output.is_some(),
            "FPL-W-004: 'FPL_W_004_MARKER' must be readable via reader_handle after shell echo"
        );
    }

    // -----------------------------------------------------------------------
    // FPL-C-001 — close() delivers SIGHUP to the child process
    // -----------------------------------------------------------------------

    /// FPL-C-001: Closing the PTY session (dropping the master fd) must deliver
    /// SIGHUP to the foreground process group (FS-PTY-007).
    ///
    /// Strategy: spawn a shell that traps SIGHUP and writes a marker to a temp file,
    /// then close the session and verify the file exists.
    ///
    /// Using a file instead of reading from the reader: all master-side fds must be
    /// closed before the kernel delivers SIGHUP. Holding the reader Arc open (which
    /// is a cloned master fd) prevents delivery. A temp file is the clean solution.
    #[test]
    fn fpl_c_001_close_delivers_sighup_to_child() {
        use std::path::PathBuf;

        // Unique marker file for this test run.
        let marker_path =
            PathBuf::from(format!("/tmp/tauterm_fpl_c_001_{}.txt", std::process::id()));
        // Cleanup if left from a previous failed run.
        let _ = std::fs::remove_file(&marker_path);

        let script = format!(
            "trap 'echo SIGHUP_RECEIVED > {path}; exit 0' HUP; echo READY; sleep 30 & wait $!",
            path = marker_path.display()
        );

        let backend = LinuxPtyBackend::new();
        let session = backend
            .open_session(
                80,
                24,
                "/bin/sh",
                &["-c", &script],
                &[("TERM", "xterm-256color")],
            )
            .expect("open session");

        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");

        // Wait for "READY" to confirm the shell trap is installed.
        let ready = read_until_timeout(reader.clone(), "READY", std::time::Duration::from_secs(5));
        assert!(
            ready.is_some(),
            "FPL-C-001: child must print READY before we close"
        );

        // Explicitly drop the reader Arc so it releases its cloned master fd.
        // All master-side fds must be closed for SIGHUP to be delivered.
        drop(reader);

        // Brief pause to ensure the shell has entered the wait loop.
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Drop the session — closes all remaining master fds → kernel delivers
        // SIGHUP to the foreground process group (the shell, which is `wait`-ing).
        drop(session);

        // Poll for the marker file (written by the SIGHUP trap).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut sighup_received = false;
        while std::time::Instant::now() < deadline {
            if marker_path.exists() {
                sighup_received = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Cleanup.
        let _ = std::fs::remove_file(&marker_path);

        assert!(
            sighup_received,
            "FPL-C-001: child must write marker file SIGHUP_RECEIVED after master fd close"
        );
    }

    // -----------------------------------------------------------------------
    // FPL-R-005 — resize() delivers SIGWINCH to the child process
    // -----------------------------------------------------------------------

    /// FPL-R-005: Resizing the PTY must deliver SIGWINCH to the foreground process
    /// group so that the child can update its layout (FS-PTY-009).
    ///
    /// Strategy: spawn a shell that traps SIGWINCH and prints a marker, then resize
    /// and verify the marker appears in the output.
    ///
    /// The `while true; do sleep 1; done` loop keeps the shell as the foreground
    /// process so that TIOCSWINSZ delivers SIGWINCH to the shell (not to a `sleep`
    /// exec-optimised into the shell's place).
    #[test]
    fn fpl_r_005_resize_delivers_sigwinch_to_child() {
        let backend = LinuxPtyBackend::new();
        let mut session = backend
            .open_session(
                80,
                24,
                "/bin/sh",
                &[
                    "-c",
                    "trap 'echo SIGWINCH_RECEIVED' WINCH; echo READY; while true; do sleep 1; done",
                ],
                &[("TERM", "xterm-256color")],
            )
            .expect("open session");

        let reader = session
            .reader_handle()
            .expect("LinuxPtySession must have a reader");

        // Wait for the child to signal it's ready (trap is installed).
        let ready = read_until_timeout(reader.clone(), "READY", std::time::Duration::from_secs(5));
        assert!(
            ready.is_some(),
            "FPL-R-005: child must print READY before resize"
        );

        // Brief pause to ensure the shell has started its wait loop before we resize.
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Resize the PTY — TIOCSWINSZ delivers SIGWINCH to the foreground process group.
        session.resize(120, 40, 0, 0).expect("resize must succeed");

        // The child's WINCH trap handler should print SIGWINCH_RECEIVED.
        let output = read_until_timeout(
            reader,
            "SIGWINCH_RECEIVED",
            std::time::Duration::from_secs(5),
        );
        assert!(
            output.is_some(),
            "FPL-R-005: child must print SIGWINCH_RECEIVED after PTY resize"
        );
    }

    // SPL-RM-001: fd leak test deferred — /proc/self/fd count is unstable in
    // parallel nextest runs (inter-test fd pollution from concurrent threads).
    // To verify manually: run `cargo nextest run fpl_s_001 --no-capture` in isolation
    // and compare /proc/self/fd before and after.
}
