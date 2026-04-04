// SPDX-License-Identifier: MPL-2.0

//! Linux PTY backend — `portable-pty` crate (`UnixPtySystem`).
//!
//! Wraps `portable-pty`'s `UnixPtySystem` to implement the `PtyBackend` and
//! `PtySession` traits. The master fd is exposed as a `tokio::io::unix::AsyncFd`
//! for the PTY read task (§7.1 of ARCHITECTURE.md).

use crate::error::PtyError;
use crate::platform::{PtyBackend, PtySession};

pub struct LinuxPtyBackend {}

impl LinuxPtyBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl PtyBackend for LinuxPtyBackend {
    fn open_session(
        &self,
        _cols: u16,
        _rows: u16,
        _command: &str,
        _args: &[&str],
        _env: &[(&str, &str)],
    ) -> Result<Box<dyn PtySession>, PtyError> {
        // TODO: implement using portable-pty UnixPtySystem.
        // 1. Create PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }
        // 2. UnixPtySystem::default().openpty(size)
        // 3. Build CommandBuilder with command, args, env (TERM, COLORTERM, LINES, COLUMNS)
        // 4. master_pty.spawn_command(cmd)
        // 5. Extract AsyncFd from master for use in PtyReadTask
        // 6. Return LinuxPtySession wrapping master + child
        Err(PtyError::Open(
            "PTY backend not yet implemented.".to_string(),
        ))
    }
}

pub struct LinuxPtySession {
    // TODO: master: Box<dyn portable_pty::MasterPty>,
    // TODO: child: Box<dyn portable_pty::Child>,
}

impl PtySession for LinuxPtySession {
    fn write(&mut self, _data: &[u8]) -> Result<(), PtyError> {
        todo!("LinuxPtySession::write")
    }

    fn resize(
        &mut self,
        _cols: u16,
        _rows: u16,
        _pixel_width: u16,
        _pixel_height: u16,
    ) -> Result<(), PtyError> {
        todo!("LinuxPtySession::resize")
    }

    fn close(self: Box<Self>) {
        // Drop the master fd — kernel delivers SIGHUP to the foreground process group (§7.1).
    }
}
