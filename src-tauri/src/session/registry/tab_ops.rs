// SPDX-License-Identifier: MPL-2.0

//! Tab-level operations: create, close, rename, reorder, set-active.

use std::collections::HashMap;

use crate::error::SessionError;
use crate::platform::PtySession;
use crate::session::{
    ids::{PaneId, TabId},
    lifecycle::PaneLifecycleState,
    pane::PaneSession,
    pty_task::spawn_pty_read_task,
    tab::{PaneNode, SessionState, TabState},
};

use super::{
    CreateTabConfig, SessionRegistry, TabEntry,
    layout::update_pane_title_in_tree,
    pty_helpers::{get_reader_handle, get_writer_handle},
    shell::resolve_shell_path,
};

impl SessionRegistry {
    /// Create a new tab with a single pane, spawning a real PTY session.
    ///
    /// Shell resolution order (FS-PTY-014):
    /// 1. `config.shell` if provided and valid
    /// 2. `$SHELL` environment variable if set and valid
    /// 3. `/bin/sh` as final fallback
    pub fn create_tab(&self, config: CreateTabConfig) -> Result<TabState, SessionError> {
        // --- Resolve shell path ---
        let shell_path = resolve_shell_path(config.shell.as_deref())?;

        // --- Build environment (FS-PTY-011, FS-PTY-012) ---
        let cols_str = config.cols.to_string();
        let rows_str = config.rows.to_string();
        let term_program_version = env!("CARGO_PKG_VERSION");
        let mut env: Vec<(&str, &str)> = vec![
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LINES", &rows_str),
            ("COLUMNS", &cols_str),
            ("TERM_PROGRAM", "TauTerm"),
            ("TERM_PROGRAM_VERSION", term_program_version),
        ];

        // Forward display / Wayland / D-Bus session env vars (FS-PTY-012).
        let display = std::env::var("DISPLAY").ok();
        let wayland = std::env::var("WAYLAND_DISPLAY").ok();
        let dbus = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        if let Some(ref v) = display {
            env.push(("DISPLAY", v.as_str()));
        }
        if let Some(ref v) = wayland {
            env.push(("WAYLAND_DISPLAY", v.as_str()));
        }
        if let Some(ref v) = dbus {
            env.push(("DBUS_SESSION_BUS_ADDRESS", v.as_str()));
        }

        // --- Build args (login shell if first tab, FS-PTY-013) ---
        let args: &[&str] = if config.login { &["--login"] } else { &[] };

        // --- Spawn the PTY session via the platform backend ---
        let pty_box: Box<dyn PtySession> = self
            .pty_backend
            .open_session(config.cols, config.rows, &shell_path, args, &env)
            .map_err(|e| SessionError::PtySpawn(e.to_string()))?;

        // Read pane-creation preferences before acquiring the registry lock
        // so we don't hold two locks simultaneously (FS-SB-002).
        let (scrollback_lines, initial_cursor_shape, allow_osc52_write) = {
            let prefs = self.prefs.read().get();
            (
                prefs.terminal.scrollback_lines,
                prefs.appearance.cursor_style.to_decscusr(),
                prefs.terminal.allow_osc52_write,
            )
        };

        // --- Build pane and tab state ---
        let mut inner = self.inner.write();

        let tab_id = TabId::new();
        let pane_id = PaneId::new();
        let order = inner.next_order;
        inner.next_order += 1;

        let mut pane = PaneSession::new(
            pane_id.clone(),
            config.cols,
            config.rows,
            scrollback_lines,
            initial_cursor_shape,
            allow_osc52_write,
        );
        pane.lifecycle = PaneLifecycleState::Running;

        // --- Start PTY read task ---
        // Get the reader from the PTY session via downcast if available.
        // Since `PtySession` is a trait object, we need a concrete accessor.
        // We downcast to `LinuxPtySession` to access `reader_handle()`.
        // To avoid coupling the registry to the Linux type, we use a helper trait.
        let reader_handle = get_reader_handle(&*pty_box);
        let writer_handle = get_writer_handle(&*pty_box);

        // Extract the injectable sender BEFORE pty_box is moved into pane.pty_session.
        // This must happen here so the PaneId is already known (it was generated above)
        // and before the Box<dyn PtySession> value is consumed by the assignment below.
        // See ADR-0015-implementation-notes.md §5.3.
        #[cfg(feature = "e2e-testing")]
        let injectable_tx = pty_box.injectable_sender();

        if let Some(reader) = reader_handle
            && let Some(registry) = self.self_ref.upgrade()
        {
            let task = spawn_pty_read_task(
                pane_id.clone(),
                pane.vt.clone(),
                self.app.clone(),
                reader,
                writer_handle,
                registry,
            );
            pane.pty_task = Some(task);
        }

        pane.pty_session = Some(pty_box);

        // Register the injectable sender under the real PaneId, now that both
        // the PaneId and the sender are available.
        #[cfg(feature = "e2e-testing")]
        if let Some(tx) = injectable_tx {
            self.injectable_registry.register(pane_id.clone(), tx);
        }

        let pane_state = pane.to_state();

        let layout = PaneNode::Leaf {
            pane_id: pane_id.clone(),
            state: pane_state,
        };

        let tab_state = TabState {
            id: tab_id.clone(),
            label: config.label,
            active_pane_id: pane_id.clone(),
            order,
            layout,
        };

        let mut panes = HashMap::new();
        panes.insert(pane_id, pane);

        inner.tabs.insert(
            tab_id.clone(),
            TabEntry {
                state: tab_state.clone(),
                panes,
            },
        );

        if inner.active_tab_id.is_none() {
            inner.active_tab_id = Some(tab_id);
        }

        Ok(tab_state)
    }

    /// Return the pane IDs for a tab without removing anything.
    ///
    /// Used by the `close_tab` command handler to collect pane IDs before
    /// tearing down per-pane resources (e.g. SSH connections).
    pub fn get_tab_pane_ids(&self, tab_id: &TabId) -> Vec<PaneId> {
        let inner = self.inner.read();
        inner
            .tabs
            .get(tab_id)
            .map(|e| e.panes.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Close a tab and all its panes.
    ///
    /// Returns the new `active_tab_id` after closing (i.e. the tab that becomes
    /// active, or `None` if no tabs remain). The caller is responsible for emitting
    /// `session-state-changed` with `TabClosed` using this value.
    pub fn close_tab(&self, id: TabId) -> Result<Option<TabId>, SessionError> {
        let mut inner = self.inner.write();
        if inner.tabs.remove(&id).is_none() {
            return Err(SessionError::TabNotFound(id.to_string()));
        }
        if inner.active_tab_id.as_ref() == Some(&id) {
            inner.active_tab_id = inner.tabs.keys().next().cloned();
        }
        Ok(inner.active_tab_id.clone())
    }

    /// Rename a tab (set or clear the user label).
    pub fn rename_tab(&self, id: TabId, label: Option<String>) -> Result<TabState, SessionError> {
        let mut inner = self.inner.write();
        let entry = inner
            .tabs
            .get_mut(&id)
            .ok_or_else(|| SessionError::TabNotFound(id.to_string()))?;
        entry.state.label = label;
        Ok(entry.state.clone())
    }

    /// Move a tab to a new order position.
    pub fn reorder_tab(&self, id: TabId, new_order: u32) -> Result<(), SessionError> {
        let mut inner = self.inner.write();
        let entry = inner
            .tabs
            .get_mut(&id)
            .ok_or_else(|| SessionError::TabNotFound(id.to_string()))?;
        entry.state.order = new_order;
        Ok(())
    }

    /// Set the active tab in the registry.
    ///
    /// Called by the `set_active_tab` command handler, which then emits
    /// `session-state-changed` with `ActiveTabChanged`.
    pub fn set_active_tab(&self, tab_id: TabId) -> Result<TabState, SessionError> {
        let mut inner = self.inner.write();
        if !inner.tabs.contains_key(&tab_id) {
            return Err(SessionError::TabNotFound(tab_id.to_string()));
        }
        inner.active_tab_id = Some(tab_id.clone());
        Ok(inner.tabs[&tab_id].state.clone())
    }

    /// Update the stored title for a pane (called from PTY/SSH read tasks on OSC title change).
    ///
    /// Returns the updated `TabState` if the pane is found, `None` otherwise.
    pub fn update_pane_title(&self, pane_id: &PaneId, title: String) -> Option<TabState> {
        let mut inner = self.inner.write();
        let (tab_id, entry) = inner
            .tabs
            .iter_mut()
            .find(|(_, e)| e.panes.contains_key(pane_id))
            .map(|(id, e)| (id.clone(), e))?;

        // Update the pane's stored title.
        if let Some(pane) = entry.panes.get_mut(pane_id) {
            pane.title = Some(title.clone());
        }

        // Rebuild the pane's state in the layout tree.
        update_pane_title_in_tree(&mut entry.state.layout, pane_id, &title);

        let _ = tab_id;
        Some(entry.state.clone())
    }

    /// Get a full session state snapshot.
    pub fn get_state_snapshot(&self) -> SessionState {
        let inner = self.inner.read();
        let mut tabs: Vec<TabState> = inner.tabs.values().map(|e| e.state.clone()).collect();
        tabs.sort_by_key(|t| t.order);
        let active_tab_id = inner
            .active_tab_id
            .clone()
            .unwrap_or_else(|| tabs.first().map(|t| t.id.clone()).unwrap_or_default());
        SessionState {
            tabs,
            active_tab_id,
        }
    }
}
