// SPDX-License-Identifier: MPL-2.0

mod environment;
mod io_operations;
mod lifecycle;

use std::io::Read;
use std::sync::{Arc, Mutex};

use crate::error::PtyError;
use crate::platform::{PtyBackend, PtySession};

use super::backend::LinuxPtyBackend;

/// Helper: open a real PTY session with /bin/sh.
pub(super) fn open_sh_session(cols: u16, rows: u16) -> Result<Box<dyn PtySession>, PtyError> {
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

/// Open a real PTY session with the given command/args/env.
///
/// Returns `Box<dyn PtySession>`. Callers use `PtySession::reader_handle()`
/// (or `as_linux_pty()`) to access the underlying reader — no raw-pointer
/// downcast required.
pub(super) fn open_linux_session_with_env(
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
pub(super) fn read_until_timeout(
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
