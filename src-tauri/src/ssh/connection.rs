// SPDX-License-Identifier: MPL-2.0

//! SSH connection state machine.
//!
//! Models the lifecycle of a single SSH session (§5.2 of ARCHITECTURE.md):
//! Connecting → Authenticating → Connected ↔ Disconnected | Closed.
//!
//! The full implementation requires `russh` integration.
//! This stub holds the state machine and placeholder fields.

use crate::session::ids::PaneId;
use crate::ssh::{SshConnectionConfig, SshLifecycleState};

/// An active or pending SSH connection for one pane.
pub struct SshConnection {
    pub pane_id: PaneId,
    pub config: SshConnectionConfig,
    state: parking_lot::Mutex<SshLifecycleState>,
}

impl SshConnection {
    pub fn new(pane_id: PaneId, config: SshConnectionConfig) -> Self {
        Self {
            pane_id,
            config,
            state: parking_lot::Mutex::new(SshLifecycleState::Connecting),
        }
    }

    /// Get the current lifecycle state.
    pub fn state(&self) -> SshLifecycleState {
        self.state.lock().clone()
    }

    /// Transition to a new state.
    pub fn set_state(&self, new_state: SshLifecycleState) {
        *self.state.lock() = new_state;
    }
}
