// SPDX-License-Identifier: MPL-2.0

//! Pane session — owns a PTY task handle and a `VtProcessor`.
//!
//! Each pane corresponds to one terminal session (local PTY or SSH channel).
//! The `VtProcessor` is wrapped in `Arc<RwLock<...>>` so the PTY read task
//! can hold a reference independently of the registry's lock (§6.2 of ARCHITECTURE.md).

use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::session::{ids::PaneId, lifecycle::PaneLifecycleState};
use crate::ssh::SshLifecycleState;
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
}

impl PaneSession {
    pub fn new(id: PaneId, cols: u16, rows: u16) -> Self {
        Self {
            vt: Arc::new(RwLock::new(VtProcessor::new(cols, rows))),
            lifecycle: PaneLifecycleState::Spawning,
            title: None,
            ssh_state: None,
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
        }
    }
}
