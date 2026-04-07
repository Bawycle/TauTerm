// SPDX-License-Identifier: MPL-2.0

//! Pane-level operations: split, close, input, scroll, resize, set-active.

use crate::error::SessionError;
use crate::platform::PtySession;
use crate::session::{
    ids::PaneId,
    lifecycle::PaneLifecycleState,
    pane::PaneSession,
    pty_task::spawn_pty_read_task,
    tab::{SplitDirection, TabState},
};

use super::{
    ScrollPositionState, SessionRegistry, clamp_pane_dimensions,
    layout::{remove_pane_from_tree, replace_leaf_with_split},
    pty_helpers::{get_reader_handle, get_writer_handle},
    shell::resolve_shell_path,
};

impl SessionRegistry {
    /// Split the pane identified by `pane_id` in the given direction.
    /// Returns the updated `TabState`.
    pub fn split_pane(
        &self,
        pane_id: PaneId,
        direction: SplitDirection,
    ) -> Result<TabState, SessionError> {
        // Resolve shell and env for the new pane.
        let shell_path = resolve_shell_path(None)?;

        let mut inner = self.inner.write();

        // Find which tab contains this pane.
        let tab_id = inner
            .tabs
            .iter()
            .find(|(_, e)| e.panes.contains_key(&pane_id))
            .map(|(id, _)| id.clone())
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;

        let entry = inner
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| SessionError::TabNotFound(tab_id.to_string()))?;

        // Determine dimensions from the existing pane's VtProcessor.
        let (cols, rows) = {
            let pane = entry
                .panes
                .get(&pane_id)
                .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
            let vt = pane.vt.read();
            let snap = vt.get_snapshot();
            (snap.cols / 2, snap.rows)
        };

        let cols_str = cols.to_string();
        let rows_str = rows.to_string();
        let term_program_version = env!("CARGO_PKG_VERSION");
        let display = std::env::var("DISPLAY").ok();
        let wayland = std::env::var("WAYLAND_DISPLAY").ok();
        let dbus = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        let mut env: Vec<(&str, &str)> = vec![
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LINES", &rows_str),
            ("COLUMNS", &cols_str),
            ("TERM_PROGRAM", "TauTerm"),
            ("TERM_PROGRAM_VERSION", term_program_version),
        ];
        if let Some(ref v) = display {
            env.push(("DISPLAY", v.as_str()));
        }
        if let Some(ref v) = wayland {
            env.push(("WAYLAND_DISPLAY", v.as_str()));
        }
        if let Some(ref v) = dbus {
            env.push(("DBUS_SESSION_BUS_ADDRESS", v.as_str()));
        }

        // Drop the write lock before calling into pty_backend (avoid holding the lock
        // during a potentially slow spawn).
        drop(inner);

        let pty_box: Box<dyn PtySession> = self
            .pty_backend
            .open_session(cols, rows, &shell_path, &[], &env)
            .map_err(|e| SessionError::PtySpawn(e.to_string()))?;

        // Re-acquire write lock to insert the new pane.
        let mut inner = self.inner.write();
        let entry = inner
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| SessionError::TabNotFound(tab_id.to_string()))?;

        let new_pane_id = PaneId::new();
        // Read scrollback limit from preferences (FS-SB-002).
        // Prefs lock is not held across the pty_backend call above.
        let scrollback_lines = self.prefs.read().get().terminal.scrollback_lines;
        let mut new_pane = PaneSession::new(new_pane_id.clone(), cols, rows, scrollback_lines);
        new_pane.lifecycle = PaneLifecycleState::Running;

        let reader_handle = get_reader_handle(&*pty_box);
        let writer_handle = get_writer_handle(&*pty_box);

        // Extract the injectable sender BEFORE pty_box is moved into new_pane.pty_session.
        // See ADR-0015-implementation-notes.md §5.3 (split_pane variant).
        #[cfg(feature = "e2e-testing")]
        let injectable_tx = pty_box.injectable_sender();

        if let Some(reader) = reader_handle
            && let Some(registry) = self.self_ref.upgrade()
        {
            let task = spawn_pty_read_task(
                new_pane_id.clone(),
                new_pane.vt.clone(),
                self.app.clone(),
                reader,
                writer_handle,
                registry,
            );
            new_pane.pty_task = Some(task);
        }
        new_pane.pty_session = Some(pty_box);

        // Register the injectable sender under the real new_pane_id.
        #[cfg(feature = "e2e-testing")]
        if let Some(tx) = injectable_tx {
            self.injectable_registry.register(new_pane_id.clone(), tx);
        }

        let new_pane_state = new_pane.to_state();
        entry.panes.insert(new_pane_id.clone(), new_pane);

        // Rebuild the layout tree, replacing the target leaf with a split node.
        let existing_state = {
            let pane = entry
                .panes
                .get(&pane_id)
                .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
            pane.to_state()
        };

        let new_layout = replace_leaf_with_split(
            entry.state.layout.clone(),
            &pane_id,
            new_pane_id.clone(),
            new_pane_state,
            existing_state,
            direction,
        );

        entry.state.layout = new_layout;
        entry.state.active_pane_id = new_pane_id;

        Ok(entry.state.clone())
    }

    /// Close a pane. Returns the updated `TabState` if the tab still exists,
    /// or `None` if the last pane was closed (tab removed).
    pub fn close_pane(&self, pane_id: PaneId) -> Result<Option<TabState>, SessionError> {
        let mut inner = self.inner.write();

        let tab_id = inner
            .tabs
            .iter()
            .find(|(_, e)| e.panes.contains_key(&pane_id))
            .map(|(id, _)| id.clone())
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;

        let entry = inner
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| SessionError::TabNotFound(tab_id.to_string()))?;
        // Dropping the PaneSession also drops the PtyTaskHandle, which aborts the read task.
        entry.panes.remove(&pane_id);

        // Deregister the injectable sender so its drop triggers EOF in the read task.
        // Must happen after panes.remove() (which drops PaneSession and PtyTaskHandle)
        // so the read task is not racing against a live sender reference.
        // See ADR-0015-implementation-notes.md §5.4 and §10.3.
        #[cfg(feature = "e2e-testing")]
        self.injectable_registry.remove(&pane_id);

        if entry.panes.is_empty() {
            // Last pane — remove the tab.
            inner.tabs.remove(&tab_id);
            if inner.active_tab_id.as_ref() == Some(&tab_id) {
                inner.active_tab_id = inner.tabs.keys().next().cloned();
            }
            return Ok(None);
        }

        // Rebuild layout tree, collapsing the removed pane's sibling.
        let new_layout = remove_pane_from_tree(entry.state.layout.clone(), &pane_id);
        entry.state.layout = new_layout;

        // Update active pane if the closed pane was active.
        if entry.state.active_pane_id == pane_id
            && let Some(first_id) = entry.panes.keys().next()
        {
            entry.state.active_pane_id = first_id.clone();
        }

        Ok(Some(entry.state.clone()))
    }

    /// Write input bytes to the pane's PTY.
    ///
    /// Returns `((), did_reset_scroll)` where `did_reset_scroll` is `true` when
    /// the pane was scrolled up and has been snapped back to the live view.
    /// The command handler is responsible for emitting `scroll-position-changed`
    /// in that case, because `SessionRegistry` does not hold an `AppHandle`.
    pub fn send_input(&self, pane_id: PaneId, data: Vec<u8>) -> Result<bool, SessionError> {
        let mut inner = self.inner.write();
        let tab = inner
            .tabs
            .values_mut()
            .find(|e| e.panes.contains_key(&pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let pane = tab
            .panes
            .get_mut(&pane_id)
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;

        pane.write_input(&data)?;

        // Snap back to live view on any PTY input (scroll-freeze policy).
        let did_reset_scroll = pane.scroll_offset > 0;
        if did_reset_scroll {
            pane.scroll_offset = 0;
        }
        Ok(did_reset_scroll)
    }

    /// Set the pane's scrollback viewport to `new_offset` lines from the bottom.
    ///
    /// `new_offset` is an absolute offset: 0 = live view, increasing values = scrolled
    /// further up into the scrollback buffer. The value is clamped to `[0, scrollback_len]`.
    /// On the alternate screen (which has no scrollback) the offset is always forced to 0.
    pub fn scroll_pane(
        &self,
        pane_id: PaneId,
        new_offset: i64,
    ) -> Result<ScrollPositionState, SessionError> {
        let mut inner = self.inner.write();
        let tab = inner
            .tabs
            .values_mut()
            .find(|e| e.panes.contains_key(&pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let pane = tab
            .panes
            .get_mut(&pane_id)
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;

        let is_alt = pane.vt.read().is_alt_screen_active();
        let n = pane.vt.read().scrollback_len();
        let clamped = if is_alt {
            0
        } else {
            new_offset.clamp(0, n as i64)
        };
        pane.scroll_offset = clamped;

        Ok(ScrollPositionState {
            offset: clamped,
            scrollback_lines: n,
        })
    }

    /// Resize the pane's PTY and VtProcessor grid.
    pub fn resize_pane(
        &self,
        pane_id: PaneId,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<(), SessionError> {
        let mut inner = self.inner.write();
        let tab = inner
            .tabs
            .values_mut()
            .find(|e| e.panes.contains_key(&pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let pane = tab
            .panes
            .get_mut(&pane_id)
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let (cols, rows) = clamp_pane_dimensions(cols, rows);
        pane.resize(cols, rows, pixel_width, pixel_height)
    }

    /// Set the active pane (and its containing tab) in the registry.
    ///
    /// Called by the `set_active_pane` command handler, which then emits
    /// `session-state-changed` with `ActivePaneChanged` (ARCHITECTURE.md §4.2).
    pub fn set_active_pane(&self, pane_id: PaneId) -> Result<TabState, SessionError> {
        let mut inner = self.inner.write();

        // Find the tab that contains this pane.
        let tab_id = inner
            .tabs
            .iter()
            .find(|(_, e)| e.panes.contains_key(&pane_id))
            .map(|(id, _)| id.clone())
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;

        // Update the registry's active_tab_id.
        inner.active_tab_id = Some(tab_id.clone());

        // Update the tab's active_pane_id.
        let entry = inner
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| SessionError::TabNotFound(tab_id.to_string()))?;
        entry.state.active_pane_id = pane_id;

        Ok(entry.state.clone())
    }
}
