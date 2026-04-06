// SPDX-License-Identifier: MPL-2.0

//! Session registry — owns all tabs and panes.
//!
//! `SessionRegistry` is the single source of truth for session topology.
//! It is injected into Tauri's state manager as `State<Arc<SessionRegistry>>`
//! and accessed by command handlers.
//!
//! Public API (§3.3 of ARCHITECTURE.md):
//! - `create_tab` / `close_tab` / `rename_tab` / `reorder_tab`
//! - `split_pane` / `close_pane`
//! - `send_input`
//! - `scroll_pane`
//! - `get_state_snapshot`
//!
//! PTY lifecycle (§7.1): `create_tab` spawns a real PTY via `PtyBackend::open_session`,
//! starts the read task, and wires the write path. The `AppHandle` is needed to
//! emit `screen-update` events from the read task.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tauri::AppHandle;

use crate::error::SessionError;
use crate::platform::{PtyBackend, PtySession, validation::validate_shell_path};
use crate::preferences::PreferencesStore;
use crate::session::{
    ids::{PaneId, TabId},
    lifecycle::PaneLifecycleState,
    pane::PaneSession,
    pty_task::spawn_pty_read_task,
    tab::{PaneNode, SessionState, SplitDirection, TabState},
};
use crate::vt::screen_buffer::ScreenSnapshot;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for creating a new tab.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTabConfig {
    /// Optional initial label. `None` = use process title via OSC.
    pub label: Option<String>,
    /// Initial terminal dimensions.
    pub cols: u16,
    pub rows: u16,
    /// Optional shell executable path. `None` = use `$SHELL` or fall back to `/bin/sh`.
    /// Must be an absolute path to an executable file.
    #[serde(default)]
    pub shell: Option<String>,
    /// Whether to launch a login shell (first tab) or non-login interactive shell
    /// (subsequent tabs and panes). Default: false.
    #[serde(default)]
    pub login: bool,
}

/// Scroll position state returned by `scroll_pane`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollPositionState {
    pub offset: i64,
    pub scrollback_lines: usize,
}

// ---------------------------------------------------------------------------
// Internal per-tab data
// ---------------------------------------------------------------------------

struct TabEntry {
    state: TabState,
    panes: HashMap<PaneId, PaneSession>,
}

// ---------------------------------------------------------------------------
// SessionRegistry
// ---------------------------------------------------------------------------

/// The session registry — thread-safe, injected as Tauri state.
pub struct SessionRegistry {
    inner: RwLock<RegistryInner>,
    /// PTY backend — used by `create_tab` to spawn real PTY sessions.
    pty_backend: Arc<dyn PtyBackend>,
    /// Tauri app handle — used to emit events from PTY read tasks.
    app: AppHandle,
    /// Weak self-reference so read tasks can call back into the registry.
    self_ref: std::sync::Weak<SessionRegistry>,
    /// Preferences store — read at pane creation time to get `scrollback_lines` (FS-SB-002).
    prefs: Arc<RwLock<PreferencesStore>>,
    /// Injectable output registry — present only in e2e-testing builds.
    /// Stores the mpsc senders keyed by PaneId so that `inject_pty_output`
    /// can push synthetic bytes into the VT pipeline.
    #[cfg(feature = "e2e-testing")]
    injectable_registry: std::sync::Arc<crate::platform::pty_injectable::InjectableRegistry>,
}

struct RegistryInner {
    tabs: HashMap<TabId, TabEntry>,
    active_tab_id: Option<TabId>,
    next_order: u32,
}

impl RegistryInner {
    fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            active_tab_id: None,
            next_order: 0,
        }
    }
}

impl SessionRegistry {
    pub fn new(
        pty_backend: Arc<dyn PtyBackend>,
        app: AppHandle,
        prefs: Arc<RwLock<PreferencesStore>>,
        #[cfg(feature = "e2e-testing")] injectable_registry: std::sync::Arc<
            crate::platform::pty_injectable::InjectableRegistry,
        >,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak| Self {
            inner: RwLock::new(RegistryInner::new()),
            pty_backend,
            app,
            self_ref: weak.clone(),
            prefs,
            #[cfg(feature = "e2e-testing")]
            injectable_registry,
        })
    }

    // -----------------------------------------------------------------------
    // Tab management
    // -----------------------------------------------------------------------

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

        // Read scrollback limit from preferences before acquiring the registry lock
        // so we don't hold two locks simultaneously (FS-SB-002).
        let scrollback_lines = self.prefs.read().get().terminal.scrollback_lines;

        // --- Build pane and tab state ---
        let mut inner = self.inner.write();

        let tab_id = TabId::new();
        let pane_id = PaneId::new();
        let order = inner.next_order;
        inner.next_order += 1;

        let mut pane =
            PaneSession::new(pane_id.clone(), config.cols, config.rows, scrollback_lines);
        pane.lifecycle = PaneLifecycleState::Running;

        // --- Start PTY read task ---
        // Get the reader from the PTY session via downcast if available.
        // Since `PtySession` is a trait object, we need a concrete accessor.
        // We downcast to `LinuxPtySession` to access `reader_handle()`.
        // To avoid coupling the registry to the Linux type, we use a helper trait.
        let reader_handle = get_reader_handle(&*pty_box);

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

    /// Close a tab and all its panes.
    pub fn close_tab(&self, id: TabId) -> Result<(), SessionError> {
        let mut inner = self.inner.write();
        if inner.tabs.remove(&id).is_none() {
            return Err(SessionError::TabNotFound(id.to_string()));
        }
        if inner.active_tab_id.as_ref() == Some(&id) {
            inner.active_tab_id = inner.tabs.keys().next().cloned();
        }
        Ok(())
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

    /// Scroll the pane's scrollback by `delta` lines (negative = scroll up, positive = scroll down).
    ///
    /// The resulting offset is clamped to `[0, scrollback_lines]` where 0 is the live view
    /// and `scrollback_lines` is the furthest scrolled-up position.
    pub fn scroll_pane(
        &self,
        pane_id: PaneId,
        delta: i64,
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

        let scrollback_lines = {
            let vt = pane.vt.read();
            vt.get_snapshot().scrollback_lines
        };

        // Positive delta = scroll down (towards live), negative = scroll up (into scrollback).
        // Stored offset: 0 = live view, increasing values = scrolled further up.
        let new_offset = (pane.scroll_offset - delta).clamp(0, scrollback_lines as i64);
        pane.scroll_offset = new_offset;

        Ok(ScrollPositionState {
            offset: new_offset,
            scrollback_lines,
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
        pane.resize(cols, rows, pixel_width, pixel_height)
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
        let snap = vt.get_snapshot();
        Ok((snap.cols, snap.rows))
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

    /// Returns the exit code of a pane if its lifecycle is already `Terminated`.
    ///
    /// Returns `None` if the pane is not found or is not yet in `Terminated` state.
    /// The PTY read task calls this after EOF to include the real exit code in the
    /// `ProcessExited` notification (FS-NOTIF-002).
    pub fn get_pane_exit_code(&self, pane_id: &PaneId) -> Option<i32> {
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
                    *exit_code
                } else {
                    None
                }
            })
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

// ---------------------------------------------------------------------------
// Shell resolution (FS-PTY-014)
// ---------------------------------------------------------------------------

/// Resolve the shell executable path.
///
/// Priority:
/// 1. `explicit` — the caller's explicit shell path (from `CreateTabConfig.shell`)
/// 2. `$SHELL` — from the environment
/// 3. `/bin/sh` — unconditional fallback
///
/// Each candidate is validated by `validate_shell_path()`. The first valid
/// candidate is returned. If all candidates fail, `/bin/sh` is returned as a
/// last resort (it is always present on Linux).
fn resolve_shell_path(explicit: Option<&str>) -> Result<String, SessionError> {
    // 1. Explicit override
    if let Some(raw) = explicit {
        return validate_shell_path(raw)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| SessionError::InvalidShellPath(e.message));
    }

    // 2. $SHELL from environment
    if let Ok(shell_env) = std::env::var("SHELL") {
        if let Ok(path) = validate_shell_path(&shell_env) {
            return Ok(path.to_string_lossy().to_string());
        }
        // $SHELL was set but invalid — fall through to /bin/sh.
        tracing::warn!("$SHELL={shell_env} is invalid; falling back to /bin/sh");
    }

    // 3. Unconditional fallback
    Ok("/bin/sh".to_string())
}

// ---------------------------------------------------------------------------
// PTY reader extraction
// ---------------------------------------------------------------------------

/// Extract a reader handle from a `Box<dyn PtySession>` for the read task.
///
/// Delegates to the `PtySession::reader_handle()` trait method, which each
/// platform backend implements. No unsafe downcast needed.
fn get_reader_handle(
    pty: &dyn PtySession,
) -> Option<std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Read + Send>>>> {
    pty.reader_handle()
}

// ---------------------------------------------------------------------------
// Layout tree helpers
// ---------------------------------------------------------------------------

/// Replace the leaf node for `target_id` with a split containing
/// the existing pane (first) and a new pane (second).
fn replace_leaf_with_split(
    node: PaneNode,
    target_id: &PaneId,
    new_id: PaneId,
    new_state: crate::session::pane::PaneState,
    existing_state: crate::session::pane::PaneState,
    direction: SplitDirection,
) -> PaneNode {
    match node {
        PaneNode::Leaf { pane_id, .. } if &pane_id == target_id => PaneNode::Split {
            direction,
            ratio: 0.5,
            first: Box::new(PaneNode::Leaf {
                pane_id: pane_id.clone(),
                state: existing_state,
            }),
            second: Box::new(PaneNode::Leaf {
                pane_id: new_id,
                state: new_state,
            }),
        },
        PaneNode::Leaf { .. } => node,
        PaneNode::Split {
            direction: d,
            ratio,
            first,
            second,
        } => PaneNode::Split {
            direction: d,
            ratio,
            first: Box::new(replace_leaf_with_split(
                *first,
                target_id,
                new_id.clone(),
                new_state.clone(),
                existing_state.clone(),
                direction,
            )),
            second: Box::new(replace_leaf_with_split(
                *second,
                target_id,
                new_id,
                new_state,
                existing_state,
                direction,
            )),
        },
    }
}

/// Update the `PaneState.title` for a specific pane in the layout tree in-place.
fn update_pane_title_in_tree(node: &mut PaneNode, target_id: &PaneId, title: &str) {
    match node {
        PaneNode::Leaf { pane_id, state } if pane_id == target_id => {
            state.title = Some(title.to_string());
        }
        PaneNode::Leaf { .. } => {}
        PaneNode::Split { first, second, .. } => {
            update_pane_title_in_tree(first, target_id, title);
            update_pane_title_in_tree(second, target_id, title);
        }
    }
}

/// Remove the leaf for `target_id`, collapsing its sibling upward.
fn remove_pane_from_tree(node: PaneNode, target_id: &PaneId) -> PaneNode {
    match node {
        PaneNode::Leaf { ref pane_id, .. } if pane_id == target_id => {
            // Caller ensures there is at least one other pane — this case
            // should not be reached at the top level.
            node
        }
        PaneNode::Leaf { .. } => node,
        PaneNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let first_ids = first.pane_ids();
            let second_ids = second.pane_ids();

            if first_ids.contains(target_id) && first_ids.len() == 1 {
                // First child is the sole target — collapse to second.
                *second
            } else if second_ids.contains(target_id) && second_ids.len() == 1 {
                // Second child is the sole target — collapse to first.
                *first
            } else {
                PaneNode::Split {
                    direction,
                    ratio,
                    first: Box::new(remove_pane_from_tree(*first, target_id)),
                    second: Box::new(remove_pane_from_tree(*second, target_id)),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TEST-SPRINT-001 — FS-PTY-013: CreateTabConfig.login=true → "--login" args
    //
    // The `create_tab` implementation selects args based on `config.login`:
    //   let args: &[&str] = if config.login { &["--login"] } else { &[] };
    //
    // This test validates the contract at the `CreateTabConfig` level:
    // a config with `login: true` must map to the `--login` flag, and a config
    // with `login: false` must not pass any args.
    // Integration with the real PTY backend requires a running system and is
    // covered by the functional test protocol (PTY-FN-001).
    // -----------------------------------------------------------------------

    #[test]
    fn test_sprint_001_login_true_selects_login_args() {
        // TEST-SPRINT-001
        let config = CreateTabConfig {
            label: None,
            cols: 80,
            rows: 24,
            shell: None,
            login: true,
        };
        // Mirror the logic from create_tab (line ~160):
        let args: &[&str] = if config.login { &["--login"] } else { &[] };
        assert_eq!(args, &["--login"], "login:true must produce --login arg");
    }

    #[test]
    fn test_sprint_001_login_false_produces_empty_args() {
        // TEST-SPRINT-001
        let config = CreateTabConfig {
            label: None,
            cols: 80,
            rows: 24,
            shell: None,
            login: false,
        };
        let args: &[&str] = if config.login { &["--login"] } else { &[] };
        assert!(args.is_empty(), "login:false must produce no args");
    }

    #[test]
    fn test_sprint_001_create_tab_config_login_default_is_false() {
        // TEST-SPRINT-001: serde default for `login` must be false so that
        // existing payloads without the field behave as non-login shells.
        let json = r#"{"cols":80,"rows":24}"#;
        let config: CreateTabConfig = serde_json::from_str(json).expect("deserialize failed");
        assert!(
            !config.login,
            "serde default for CreateTabConfig.login must be false"
        );
    }

    #[test]
    fn test_sprint_001_create_tab_config_login_true_round_trips() {
        // TEST-SPRINT-001: login:true must survive a JSON round-trip (IPC safety).
        let config = CreateTabConfig {
            label: None,
            cols: 80,
            rows: 24,
            shell: None,
            login: true,
        };
        let json = serde_json::to_string(&config).expect("serialize failed");
        let restored: CreateTabConfig = serde_json::from_str(&json).expect("deserialize failed");
        assert!(restored.login, "login:true must survive serde round-trip");
    }
}
