// SPDX-License-Identifier: MPL-2.0

//! Pane session — owns a PTY task handle and a `VtProcessor`.
//!
//! Each pane corresponds to one terminal session (local PTY or SSH channel).
//! The `VtProcessor` is wrapped in `Arc<RwLock<...>>` so the PTY read task
//! can hold a reference independently of the registry's lock (§6.2 of ARCHITECTURE.md).
//!
//! `PaneSession` now also holds the `PtySession` handle for write/resize operations
//! and the `PtyTaskHandle` that drives the async read loop.

use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::platform::PtySession;
use crate::session::ssh_task::SshTaskHandle;
use crate::session::{ids::PaneId, lifecycle::PaneLifecycleState, pty_task::PtyTaskHandle};
use crate::ssh::SshLifecycleState;
use crate::ssh::connection::SshChannelArc;
use crate::vt::VtProcessor;

/// Serializable pane state — sent to the frontend via IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneState {
    pub pane_id: PaneId,
    pub lifecycle: PaneLifecycleState,
    /// Title from OSC sequences. `None` until first OSC title received.
    pub title: Option<String>,
    /// SSH session state. `None` for local PTY panes.
    pub ssh_state: Option<SshLifecycleState>,
    /// Current scroll offset in scrollback lines (0 = bottom/live view).
    pub scroll_offset: i64,
}

/// Live pane session data (not serialized — kept in the registry).
pub struct PaneSession {
    pub id: PaneId,
    /// VT processor shared with the PTY read task.
    pub vt: Arc<RwLock<VtProcessor>>,
    pub lifecycle: PaneLifecycleState,
    pub title: Option<String>,
    /// `Some` if this pane is connected via SSH.
    pub ssh_state: Option<SshLifecycleState>,
    /// Active PTY session. `None` for SSH panes or before spawn completes.
    pub pty_session: Option<Box<dyn PtySession>>,
    /// Handle to the running PTY read task. Dropped to abort the task.
    pub pty_task: Option<PtyTaskHandle>,
    /// SSH channel, present only for SSH panes (mutually exclusive with `pty_session`).
    pub ssh_channel: Option<SshChannelArc>,
    /// Handle to the running SSH read task (for SSH panes). Dropped to abort.
    pub ssh_task: Option<SshTaskHandle>,
    /// Current scroll offset in scrollback lines (0 = bottom/live view, positive = scrolled up).
    pub scroll_offset: i64,
    /// When `true`, this pane's OSC 52 write policy was set from a per-connection
    /// `SshConnectionConfig.allow_osc52_write` override and must not be overwritten
    /// by global preference propagation (arch §8.2).
    pub osc52_overridden: bool,
}

impl PaneSession {
    /// Create a new pane in `Spawning` state (no PTY yet).
    ///
    /// `scrollback_lines` is the maximum scrollback capacity for this pane,
    /// read from `preferences.terminal.scrollback_lines` at creation time (FS-SB-002).
    ///
    /// `initial_cursor_shape` is the DECSCUSR-encoded initial cursor shape (0–6),
    /// derived from `preferences.appearance.cursor_style`. Applications can override
    /// this at any time via DECSCUSR — this only sets the *starting* value.
    ///
    /// `allow_osc52_write` is read from `preferences.terminal.allow_osc52_write`.
    pub fn new(
        id: PaneId,
        cols: u16,
        rows: u16,
        scrollback_lines: usize,
        initial_cursor_shape: u8,
        allow_osc52_write: bool,
    ) -> Self {
        Self {
            vt: Arc::new(RwLock::new(VtProcessor::new(
                cols,
                rows,
                scrollback_lines,
                initial_cursor_shape,
                allow_osc52_write,
            ))),
            lifecycle: PaneLifecycleState::Spawning,
            title: None,
            ssh_state: None,
            pty_session: None,
            pty_task: None,
            ssh_channel: None,
            ssh_task: None,
            scroll_offset: 0,
            osc52_overridden: false,
            id,
        }
    }

    /// Snapshot serializable state for IPC.
    pub fn to_state(&self) -> PaneState {
        PaneState {
            pane_id: self.id.clone(),
            lifecycle: self.lifecycle.clone(),
            title: self.title.clone(),
            ssh_state: self.ssh_state.clone(),
            scroll_offset: self.scroll_offset,
        }
    }

    /// Write bytes to the PTY master (keyboard input → shell).
    ///
    /// Returns `Err` if the pane is not in `Running` state or has no PTY session.
    pub fn write_input(&mut self, data: &[u8]) -> Result<(), crate::error::SessionError> {
        if !self.lifecycle.is_active() {
            return Err(crate::error::SessionError::PaneNotRunning(
                self.id.to_string(),
            ));
        }
        match self.pty_session.as_mut() {
            Some(pty) => pty
                .write(data)
                .map_err(|e| crate::error::SessionError::PtyIo(e.to_string())),
            None => Err(crate::error::SessionError::PaneNotRunning(
                self.id.to_string(),
            )),
        }
    }

    /// Resize the PTY (TIOCSWINSZ + SIGWINCH) and the VtProcessor grid.
    pub fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<(), crate::error::SessionError> {
        // Resize VtProcessor first (updates the internal grid).
        {
            let mut vt = self.vt.write();
            vt.resize(cols, rows);
        }
        // Resize PTY master (delivers SIGWINCH to the foreground process group).
        if let Some(pty) = self.pty_session.as_mut() {
            pty.resize(cols, rows, pixel_width, pixel_height)
                .map_err(|e| crate::error::SessionError::PtyIo(e.to_string()))?;
        }
        Ok(())
    }
}
