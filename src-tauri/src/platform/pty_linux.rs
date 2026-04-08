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

mod backend;
mod session;

pub use backend::LinuxPtyBackend;
pub use session::LinuxPtySession;

#[cfg(test)]
mod tests;
