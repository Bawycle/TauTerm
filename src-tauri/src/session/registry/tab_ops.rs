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
    layout::{update_pane_cwd_in_tree, update_pane_title_in_tree},
    pty_helpers::{get_reader_handle, get_writer_handle, validated_working_dir},
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

        // --- Resolve working directory from source pane (FS-VT-064) ---
        // OSC 7 CWD takes priority; falls back to /proc/<fg_pid>/cwd.
        let working_dir = if let Some(ref source_id) = config.source_pane_id {
            let inner = self.inner.read();
            let cwd = inner
                .tabs
                .values()
                .flat_map(|e| e.panes.get(source_id))
                .next()
                .and_then(|pane| {
                    pane.cwd.clone().or_else(|| {
                        pane.pty_session
                            .as_ref()
                            .and_then(|pty| pty.foreground_process_cwd())
                    })
                });
            drop(inner);
            validated_working_dir(cwd.as_deref())
        } else {
            None
        };

        // --- Spawn the PTY session via the platform backend ---
        let pty_box: Box<dyn PtySession> = self
            .pty_backend
            .open_session(
                config.cols,
                config.rows,
                config.pixel_width,
                config.pixel_height,
                &shell_path,
                args,
                &env,
                working_dir.as_deref(),
            )
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

    /// Resolve the effective display title for a pane using the priority chain:
    ///
    /// 1. `pane.title` (OSC 0/2) — set by the shell/app
    /// 2. CWD basename from OSC 7 (`pane.cwd`)
    /// 3. Foreground process name from `/proc/{pgid}/comm`
    ///
    /// Note: the user-defined tab label (`TabState.label`) is handled by the
    /// frontend and overrides all of the above at display time. It is NOT folded
    /// into this resolution to preserve the separation between user labels and
    /// auto-detected titles.
    ///
    /// Returns `None` when no title can be determined.
    fn resolve_effective_title(pane: &crate::session::pane::PaneSession) -> Option<String> {
        // Priority 1: OSC 0/2 title set by the shell or running application.
        if let Some(ref t) = pane.title
            && !t.is_empty()
        {
            return Some(t.clone());
        }
        // Priority 2: CWD basename from OSC 7.
        if let Some(ref cwd) = pane.cwd
            && let Some(name) = std::path::Path::new(cwd)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_owned())
            && !name.is_empty()
        {
            return Some(name);
        }
        // Priority 3: foreground process name via /proc/{pgid}/comm.
        if let Some(ref pty) = pane.pty_session
            && let Some(name) = pty.foreground_process_name()
            && !name.is_empty()
        {
            return Some(name);
        }
        None
    }

    /// Update the stored CWD for a pane (called from PTY read task on OSC 7 change).
    ///
    /// Applies the title resolution chain after updating the CWD. Returns the updated
    /// `TabState` only when the effective display title actually changed — `None` if
    /// the pane is not found or if the title is unchanged (e.g. because an OSC 0/2
    /// title takes priority over the CWD basename). This prevents spurious frontend
    /// events on every shell prompt.
    pub fn update_pane_cwd(&self, pane_id: &PaneId, cwd: String) -> Option<TabState> {
        let mut inner = self.inner.write();
        let (_tab_id, entry) = inner
            .tabs
            .iter_mut()
            .find(|(_, e)| e.panes.contains_key(pane_id))
            .map(|(id, e)| (id.clone(), e))?;
        if let Some(pane) = entry.panes.get_mut(pane_id) {
            let old_title = Self::resolve_effective_title(pane);
            pane.cwd = Some(cwd.clone());
            let new_title = Self::resolve_effective_title(pane);
            // Always keep the layout tree's PaneState.cwd in sync.
            update_pane_cwd_in_tree(&mut entry.state.layout, pane_id, &cwd);
            if new_title != old_title {
                if let Some(ref t) = new_title {
                    update_pane_title_in_tree(&mut entry.state.layout, pane_id, t);
                }
                return Some(entry.state.clone());
            }
            // Even when the title didn't change, emit the updated TabState
            // so the frontend receives the new CWD (used by status bar, tab
            // creation with source_pane_id, etc.).
            return Some(entry.state.clone());
        }
        None
    }

    /// Update the stored title for a pane (called from PTY/SSH read tasks on OSC title change).
    ///
    /// Applies the title resolution chain (OSC title → CWD basename → process name)
    /// so the layout tree always shows the best available title.
    ///
    /// Returns `Some(TabState)` only when the effective display title actually changed —
    /// `None` if the pane is not found or if the title is unchanged (prevents spurious
    /// frontend events when the shell resends the same OSC title on every prompt).
    pub fn update_pane_title(&self, pane_id: &PaneId, title: String) -> Option<TabState> {
        let mut inner = self.inner.write();
        let (_tab_id, entry) = inner
            .tabs
            .iter_mut()
            .find(|(_, e)| e.panes.contains_key(pane_id))
            .map(|(id, e)| (id.clone(), e))?;

        // Capture the effective title before updating.
        let old_title = entry
            .panes
            .get(pane_id)
            .and_then(Self::resolve_effective_title);

        // Update the pane's stored OSC title.
        if let Some(pane) = entry.panes.get_mut(pane_id) {
            pane.title = Some(title.clone());
        }

        // Resolve the new effective display title through the priority chain.
        let new_title = entry
            .panes
            .get(pane_id)
            .and_then(Self::resolve_effective_title);

        // If the effective title is unchanged, skip the tree update and suppress the event.
        if new_title == old_title {
            return None;
        }

        let display_title = new_title.unwrap_or(title);
        update_pane_title_in_tree(&mut entry.state.layout, pane_id, &display_title);

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
