// SPDX-License-Identifier: MPL-2.0

//! Pane state accessors: VT, dims, snapshot, title, search, lifecycle,
//! termination, and foreground process detection.

use std::sync::Arc;

use libc;

use crate::error::SessionError;
use crate::session::{
    ids::{PaneId, TabId},
    tab::TabState,
};
use crate::vt::screen_buffer::ScreenSnapshot;

use super::SessionRegistry;

impl SessionRegistry {
    /// Get the `VtProcessor` Arc for a pane (used by SSH connection to wire the read task).
    pub fn get_pane_vt(
        &self,
        pane_id: &PaneId,
    ) -> Result<Arc<parking_lot::RwLock<crate::vt::VtProcessor>>, SessionError> {
        let inner = self.inner.read();
        let pane = inner
            .tabs
            .values()
            .find_map(|e| e.panes.get(pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        Ok(pane.vt.clone())
    }

    /// Get the current dimensions (cols, rows) of a pane.
    pub fn get_pane_dims(&self, pane_id: &PaneId) -> Result<(u16, u16), SessionError> {
        let inner = self.inner.read();
        let pane = inner
            .tabs
            .values()
            .find_map(|e| e.panes.get(pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let vt = pane.vt.read();
        let meta = vt.get_screen_meta();
        Ok((meta.cols, meta.rows))
    }

    /// Get a full screen snapshot for `get_pane_screen_snapshot`.
    pub fn get_pane_snapshot(&self, pane_id: &PaneId) -> Result<ScreenSnapshot, SessionError> {
        let inner = self.inner.read();
        let pane = inner
            .tabs
            .values()
            .find_map(|e| e.panes.get(pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let vt = pane.vt.read();
        Ok(vt.get_snapshot())
    }

    /// Search the scrollback buffer of a pane.
    pub fn search_pane(
        &self,
        pane_id: &PaneId,
        query: &crate::vt::SearchQuery,
    ) -> Result<Vec<crate::vt::SearchMatch>, SessionError> {
        let inner = self.inner.read();
        let pane = inner
            .tabs
            .values()
            .find_map(|e| e.panes.get(pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let vt = pane.vt.read();
        Ok(vt.search(query))
    }

    /// Returns `true` if `pane_id` is the currently active pane of its tab.
    ///
    /// Returns `false` if the pane does not exist or belongs to a background tab
    /// (i.e. another tab is the active tab). Only the active pane of the active tab
    /// is considered "in the foreground" for notification purposes (FS-NOTIF-001).
    pub fn is_active_pane(&self, pane_id: &PaneId) -> bool {
        let inner = self.inner.read();
        // Find the tab containing this pane.
        let Some((tab_id, entry)) = inner
            .tabs
            .iter()
            .find(|(_, e)| e.panes.contains_key(pane_id))
        else {
            return false;
        };
        // The pane must be the active pane of its tab AND the tab must be the active tab.
        entry.state.active_pane_id == *pane_id && inner.active_tab_id.as_ref() == Some(tab_id)
    }

    /// Returns the `TabId` and `TabState` for the tab containing `pane_id`, if found.
    pub fn get_tab_state_for_pane(&self, pane_id: &PaneId) -> Option<(TabId, TabState)> {
        let inner = self.inner.read();
        inner
            .tabs
            .iter()
            .find(|(_, e)| e.panes.contains_key(pane_id))
            .map(|(tab_id, e)| (tab_id.clone(), e.state.clone()))
    }

    /// Transition a pane to `Terminated` state and return its `(exit_code, signal_name)`.
    ///
    /// Called by the PTY emit task after EOF: queries the child process exit status
    /// via `PtySession::wait_exit_code()`, sets `pane.lifecycle = Terminated`,
    /// and returns the exit info for the `ProcessExited` notification.
    ///
    /// This method may block briefly (microseconds) while waiting for the zombie
    /// to be reaped. It must only be called after PTY EOF has been observed.
    ///
    /// Returns `(None, None)` if the pane is not found.
    pub fn mark_pane_terminated(&self, pane_id: &PaneId) -> (Option<i32>, Option<String>) {
        let mut inner = self.inner.write();
        let Some(pane) = inner
            .tabs
            .values_mut()
            .find_map(|e| e.panes.get_mut(pane_id))
        else {
            return (None, None);
        };
        let exit_code = pane
            .pty_session
            .as_ref()
            .and_then(|s| s.wait_exit_code())
            .flatten();
        pane.lifecycle = crate::session::lifecycle::PaneLifecycleState::Terminated {
            exit_code,
            error: None,
        };
        // signal_name detection deferred (requires extending PaneLifecycleState)
        (exit_code, None)
    }

    /// Returns `(exit_code, signal_name)` for a pane that is in `Terminated` state.
    ///
    /// `exit_code` is `None` when the process was killed by a signal.
    /// `signal_name` is `None` when termination was a clean or error exit — the
    /// `PaneLifecycleState::Terminated::error` field carries a human-readable
    /// description, not a parseable signal name, so it is not forwarded here.
    ///
    /// Returns `None` if the pane is not found or is not yet in `Terminated` state.
    /// The PTY read task calls this after EOF to build the `ProcessExited` notification
    /// (FS-NOTIF-002, FS-PTY-005).
    pub fn get_pane_termination_info(
        &self,
        pane_id: &PaneId,
    ) -> Option<(Option<i32>, Option<String>)> {
        let inner = self.inner.read();
        inner
            .tabs
            .values()
            .find_map(|e| e.panes.get(pane_id))
            .and_then(|pane| {
                if let crate::session::lifecycle::PaneLifecycleState::Terminated {
                    exit_code, ..
                } = &pane.lifecycle
                {
                    // `exit_code` is `None` when the child was killed by a signal.
                    // We cannot recover the signal name from `PaneLifecycleState`
                    // without extending it — signal_name is left None for now.
                    Some((*exit_code, None))
                } else {
                    None
                }
            })
    }

    /// Returns whether a non-shell foreground process is active on a pane's PTY.
    ///
    /// Logic (FS-PTY-008):
    /// - Returns `Ok(false)` if the pane is not found, not in `Running` state,
    ///   or has no local PTY session (SSH pane).
    /// - Calls `tcgetpgrp(master_fd)` to obtain the foreground PGID.
    /// - Returns `Ok(true)` if the foreground PGID differs from the shell's PID.
    ///
    /// The `tcgetpgrp` call is confined to `PtySession::foreground_pgid()` in the
    /// platform layer (`platform/pty_linux.rs`) — no `unsafe` in this method.
    ///
    /// In e2e-testing builds: if an injected foreground process name has been set for
    /// `pane_id` via `inject_foreground_process`, returns `Ok(true)` immediately
    /// without consulting the real PTY.
    pub fn has_foreground_process(
        &self,
        pane_id: &PaneId,
    ) -> Result<bool, crate::error::SessionError> {
        // E2E test shortcut: honour injected foreground state before touching the PTY.
        #[cfg(feature = "e2e-testing")]
        if self.injected_foreground.contains_key(pane_id) {
            return Ok(true);
        }
        let inner = self.inner.read();
        let pane = inner.tabs.values().find_map(|e| e.panes.get(pane_id));

        let pane = match pane {
            Some(p) => p,
            None => return Ok(false),
        };

        // Only meaningful for running local PTY panes.
        if !pane.lifecycle.is_active() {
            return Ok(false);
        }

        let pty = match &pane.pty_session {
            Some(s) => s,
            None => return Ok(false), // SSH pane or not yet spawned
        };

        let shell_pid = match pty.shell_pid() {
            Some(pid) => pid,
            None => return Ok(false), // session type does not track shell PID
        };

        let fg_pgid = pty
            .foreground_pgid()
            .map_err(|e| crate::error::SessionError::PtyIo(e.to_string()))?;

        // A non-shell foreground process exists when the foreground PGID differs
        // from the shell's PID (the shell is its own process group leader).
        Ok(fg_pgid != shell_pid as libc::pid_t)
    }

    /// Force-terminate a pane with a synthetic exit code (e2e-testing only).
    ///
    /// Sets `pane.lifecycle = Terminated { exit_code: Some(exit_code), error: None }`
    /// without consulting the real PTY child process. Used by `inject_pane_exit` to
    /// simulate process termination in E2E tests.
    ///
    /// Returns `Err` if the pane is not found.
    #[cfg(feature = "e2e-testing")]
    pub fn set_pane_terminated_with_code(
        &self,
        pane_id: &PaneId,
        exit_code: i32,
    ) -> Result<(), SessionError> {
        let mut inner = self.inner.write();
        let pane = inner
            .tabs
            .values_mut()
            .find_map(|e| e.panes.get_mut(pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        pane.lifecycle = crate::session::lifecycle::PaneLifecycleState::Terminated {
            exit_code: Some(exit_code),
            error: None,
        };
        Ok(())
    }

    /// Returns `true` if the pane identified by `pane_id` is a local PTY session
    /// (not SSH). Returns `false` if the pane is an SSH pane or does not exist.
    pub fn is_local_pane(&self, pane_id: &PaneId) -> bool {
        let inner = self.inner.read();
        inner
            .tabs
            .values()
            .find_map(|e| e.panes.get(pane_id))
            .map(|p| p.ssh_channel.is_none())
            .unwrap_or(false)
    }
}
