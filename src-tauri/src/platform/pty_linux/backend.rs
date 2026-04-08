// SPDX-License-Identifier: MPL-2.0

use std::os::unix::io::RawFd;
use std::sync::{Arc, Mutex};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::error::PtyError;
use crate::platform::{PtyBackend, PtySession};

use super::session::LinuxPtySession;

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

        // Extract the shell PID and master fd before moving the master into the Arc.
        // process_id() returns the pid of the spawned child (the shell).
        let shell_pid = child.process_id();

        // as_raw_fd() returns the underlying fd for the master PTY.
        // Valid as long as the Arc<Mutex<master>> is alive.
        let master_fd: RawFd = pty_pair
            .master
            .as_raw_fd()
            .ok_or_else(|| PtyError::Open("master PTY has no raw fd".into()))?;

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
            master_fd,
            shell_pid,
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
            _child: Arc::new(Mutex::new(child)),
        }))
    }
}
