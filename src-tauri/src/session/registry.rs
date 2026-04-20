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
use crate::platform::PtyBackend;
use crate::preferences::PreferencesStore;
use crate::session::{
    ids::{PaneId, TabId},
    pane::PaneSession,
    tab::TabState,
};

mod layout;
mod pane_ops;
mod pane_prefs;
mod pane_state;
mod pty_helpers;
mod shell;
mod tab_ops;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// PTY minimum dimensions (FS-PTY)
// ---------------------------------------------------------------------------

/// Minimum number of columns accepted by `resize_pane`.
///
/// Any value below this is clamped up before being forwarded to the PTY and
/// `VtProcessor`. Protects against degenerate grids that would break VT
/// parsing and PTY scrollback assumptions.
pub const MIN_COLS: u16 = 20;

/// Minimum number of rows accepted by `resize_pane`.
pub const MIN_ROWS: u16 = 5;

/// Clamp `(cols, rows)` to the minimum terminal dimensions enforced by
/// [`SessionRegistry::resize_pane`]. Extracted as a free function so that
/// the clamping logic can be unit-tested independently of the PTY backend.
#[inline]
pub(crate) fn clamp_pane_dimensions(cols: u16, rows: u16) -> (u16, u16) {
    (cols.max(MIN_COLS), rows.max(MIN_ROWS))
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for creating a new tab.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateTabConfig {
    /// Optional initial label. `None` = use process title via OSC.
    pub label: Option<String>,
    /// Initial terminal dimensions.
    pub cols: u16,
    pub rows: u16,
    /// Cell pixel dimensions for `TIOCSWINSZ` / SSH `pty-req`.
    /// Defaults to 0 (unknown) — applications that use pixel-perfect rendering
    /// (e.g. Sixel, ReGIS) will report incorrect cell sizes until the frontend
    /// sends accurate values.
    #[serde(default)]
    pub pixel_width: u16,
    #[serde(default)]
    pub pixel_height: u16,
    /// Optional shell executable path. `None` = use `$SHELL` or fall back to `/bin/sh`.
    /// Must be an absolute path to an executable file.
    #[serde(default)]
    pub shell: Option<String>,
    /// Whether to launch a login shell (first tab) or non-login interactive shell
    /// (subsequent tabs and panes). Default: false.
    #[serde(default)]
    pub login: bool,
    /// If set, inherit the working directory from this pane (with `/proc` fallback
    /// when OSC 7 is unavailable). FS-VT-064.
    #[serde(default)]
    pub source_pane_id: Option<PaneId>,
}

/// Scroll position state returned by `scroll_pane`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScrollPositionState {
    #[specta(type = f64)]
    pub offset: i64,
    #[specta(type = f64)]
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
    /// Injected foreground process names — present only in e2e-testing builds.
    /// Keyed by PaneId; checked before the real `tcgetpgrp` path in
    /// `has_foreground_process` so E2E tests can simulate a busy pane.
    #[cfg(feature = "e2e-testing")]
    pub injected_foreground: dashmap::DashMap<PaneId, String>,
}

struct RegistryInner {
    tabs: HashMap<TabId, TabEntry>,
    /// Reverse index: PaneId → TabId for O(1) pane-to-tab lookups.
    pane_to_tab: HashMap<PaneId, TabId>,
    active_tab_id: Option<TabId>,
    next_order: u32,
}

impl RegistryInner {
    fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            pane_to_tab: HashMap::new(),
            active_tab_id: None,
            next_order: 0,
        }
    }

    /// O(1) lookup: find which tab contains a pane.
    fn tab_id_for_pane(&self, pane_id: &PaneId) -> Result<TabId, SessionError> {
        self.pane_to_tab
            .get(pane_id)
            .cloned()
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))
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
            #[cfg(feature = "e2e-testing")]
            injected_foreground: dashmap::DashMap::new(),
        })
    }

    /// Record a frame-ack timestamp for a pane (P-HT-6).
    ///
    /// Uses a read-lock only — the `AtomicU64::store` is lock-free.
    /// Silently ignored if the pane no longer exists (race with close).
    pub fn record_frame_ack(&self, pane_id: &PaneId) {
        let inner = self.inner.read();
        if let Some(tab_id) = inner.pane_to_tab.get(pane_id)
            && let Some(entry) = inner.tabs.get(tab_id)
            && let Some(pane) = entry.panes.get(pane_id)
        {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            pane.last_frame_ack_ms
                .store(ts, std::sync::atomic::Ordering::Relaxed);
        }
    }
}
