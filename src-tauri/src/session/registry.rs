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

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::error::SessionError;
use crate::session::{
    ids::{PaneId, TabId},
    pane::PaneSession,
    tab::{PaneNode, SessionState, SplitDirection, TabState},
};
use crate::vt::screen_buffer::ScreenSnapshot;

/// Configuration for creating a new tab.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTabConfig {
    /// Optional initial label. `None` = use process title via OSC.
    pub label: Option<String>,
    /// Initial terminal dimensions.
    pub cols: u16,
    pub rows: u16,
}

/// Scroll position state returned by `scroll_pane`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollPositionState {
    pub offset: i64,
    pub scrollback_lines: usize,
}

/// Internal per-tab data.
struct TabEntry {
    state: TabState,
    panes: HashMap<PaneId, PaneSession>,
}

/// The session registry — thread-safe, injected as Tauri state.
pub struct SessionRegistry {
    inner: RwLock<RegistryInner>,
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
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(RegistryInner::new()),
        })
    }

    /// Create a new tab with a single pane.
    pub fn create_tab(&self, config: CreateTabConfig) -> Result<TabState, SessionError> {
        let mut inner = self.inner.write();

        let tab_id = TabId::new();
        let pane_id = PaneId::new();
        let order = inner.next_order;
        inner.next_order += 1;

        let pane = PaneSession::new(pane_id.clone(), config.cols, config.rows);
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
        let mut inner = self.inner.write();

        // Find which tab contains this pane.
        let tab_id = inner
            .tabs
            .iter()
            .find(|(_, e)| e.panes.contains_key(&pane_id))
            .map(|(id, _)| id.clone())
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;

        let entry = inner.tabs.get_mut(&tab_id).unwrap();

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

        let new_pane_id = PaneId::new();
        let new_pane = PaneSession::new(new_pane_id.clone(), cols, rows);
        let new_pane_state = new_pane.to_state();
        entry.panes.insert(new_pane_id.clone(), new_pane);

        // Rebuild the layout tree, replacing the target leaf with a split node.
        let existing_state = {
            let pane = entry.panes.get(&pane_id).unwrap();
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

        let entry = inner.tabs.get_mut(&tab_id).unwrap();
        entry.panes.remove(&pane_id);

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
    pub fn send_input(&self, pane_id: PaneId, data: Vec<u8>) -> Result<(), SessionError> {
        // TODO: write `data` to the pane's PTY master fd.
        // Gated on the platform PTY abstraction being wired up.
        let _ = data;
        let inner = self.inner.read();
        let tab = inner
            .tabs
            .values()
            .find(|e| e.panes.contains_key(&pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let _ = tab;
        Ok(())
    }

    /// Scroll the pane's scrollback by `offset` lines (negative = up).
    pub fn scroll_pane(
        &self,
        pane_id: PaneId,
        offset: i64,
    ) -> Result<ScrollPositionState, SessionError> {
        let inner = self.inner.read();
        let tab = inner
            .tabs
            .values()
            .find(|e| e.panes.contains_key(&pane_id))
            .ok_or_else(|| SessionError::PaneNotFound(pane_id.to_string()))?;
        let pane = tab.panes.get(&pane_id).unwrap();
        let vt = pane.vt.read();
        let snap = vt.get_snapshot();
        let scrollback_lines = snap.scrollback_lines;
        // TODO: maintain scroll offset state per pane.
        Ok(ScrollPositionState {
            offset,
            scrollback_lines,
        })
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
