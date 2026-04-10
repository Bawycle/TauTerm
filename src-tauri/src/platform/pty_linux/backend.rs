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
        pixel_width: u16,
        pixel_height: u16,
        command: &str,
        args: &[&str],
        env: &[(&str, &str)],
        working_directory: Option<&std::path::Path>,
    ) -> Result<Box<dyn PtySession>, PtyError> {
        let pty_system = native_pty_system();

        let size = PtySize {
            rows,
            cols,
            pixel_width,
            pixel_height,
        };

        let pty_pair = pty_system
            .openpty(size)
            .map_err(|e| PtyError::Open(e.to_string()))?;

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        // Set the working directory for the child process when provided.
        if let Some(dir) = working_directory {
            cmd.cwd(dir);
        }

        // SECURITY: env_clear() erases the parent process environment snapshot
        // that portable-pty pre-populates in CommandBuilder::new(). Without this,
        // secrets present in TauTerm's process env (AWS_SECRET_ACCESS_KEY, GITHUB_TOKEN,
        // LD_PRELOAD, etc.) are silently forwarded to every child process (FS-PTY-011/L3).
        cmd.env_clear();

        // Inject system-identity vars required by FS-PTY-011 that callers don't supply.
        // Read from TauTerm's process env; apply safe defaults when absent.
        let lang = std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let user = std::env::var("USER").unwrap_or_default();
        let logname = std::env::var("LOGNAME").unwrap_or_default();
        let path =
            std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string());

        cmd.env("LANG", &lang);
        cmd.env("SHELL", &shell);
        cmd.env("HOME", &home);
        if !user.is_empty() {
            cmd.env("USER", &user);
        }
        if !logname.is_empty() {
            cmd.env("LOGNAME", &logname);
        }
        cmd.env("PATH", &path);

        // Caller-supplied allowlist (TERM, COLORTERM, LINES, COLUMNS, TERM_PROGRAM,
        // TERM_PROGRAM_VERSION, DISPLAY, WAYLAND_DISPLAY, DBUS_SESSION_BUS_ADDRESS).
        // Caller values take precedence — they overwrite any base var above if present.
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
