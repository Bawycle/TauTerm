// SPDX-License-Identifier: MPL-2.0

//! Tab session — holds the pane layout tree and tab metadata.
//!
//! A tab contains one or more panes arranged in a split tree (`PaneNode`).
//! The tree structure (§4.5.1 of ARCHITECTURE.md) is maintained here and
//! returned to the frontend as `TabState` on all topology mutations.

use serde::{Deserialize, Serialize};

use crate::session::{ids::PaneId, ids::TabId, pane::PaneState};

/// Split direction for `split_pane`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Arborescent pane layout node (§4.5.1 of ARCHITECTURE.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PaneNode {
    /// A leaf node containing a single pane.
    Leaf {
        #[serde(rename = "paneId")]
        pane_id: PaneId,
        state: PaneState,
    },
    /// An interior split node containing two children.
    Split {
        direction: SplitDirection,
        /// Size ratio of the first child (0.0–1.0).
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// Collect all pane IDs present in this subtree.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            Self::Leaf { pane_id, .. } => vec![pane_id.clone()],
            Self::Split { first, second, .. } => {
                let mut ids = first.pane_ids();
                ids.extend(second.pane_ids());
                ids
            }
        }
    }

    /// Find the leaf node for a given pane ID.
    pub fn find_pane(&self, id: &PaneId) -> Option<&PaneNode> {
        match self {
            Self::Leaf { pane_id, .. } if pane_id == id => Some(self),
            Self::Leaf { .. } => None,
            Self::Split { first, second, .. } => {
                first.find_pane(id).or_else(|| second.find_pane(id))
            }
        }
    }
}

/// Serializable tab state — the canonical type returned by IPC commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
    pub id: TabId,
    /// User-defined label. `None` = auto-title from active pane's OSC title.
    pub label: Option<String>,
    pub active_pane_id: PaneId,
    /// Display order (lower = further left in the tab bar).
    pub order: u32,
    /// Pane layout tree.
    pub layout: PaneNode,
}

/// Full session state snapshot (returned by `get_session_state`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub tabs: Vec<TabState>,
    pub active_tab_id: TabId,
}
