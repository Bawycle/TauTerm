// SPDX-License-Identifier: MPL-2.0

//! PTY pane lifecycle state machine.
//!
//! Models the states a pane can be in (§5.1 of ARCHITECTURE.md):
//! Spawning → Running → Terminated | Closing → Closed.

use serde::{Deserialize, Serialize};

/// The lifecycle state of a local PTY pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum PaneLifecycleState {
    /// PTY is being allocated and the child process is being forked.
    Spawning,
    /// PTY I/O is active; input and output are flowing.
    Running,
    /// Child process exited. Pane is visible with exit code.
    /// User may restart (→ Spawning) or close (→ Closed).
    Terminated {
        /// Process exit code. `None` if the exit code could not be determined.
        exit_code: Option<i32>,
        /// Optional human-readable error description.
        error: Option<String>,
    },
    /// User requested close; SIGHUP was sent to the process group.
    /// Waiting for the process to exit.
    Closing,
    /// PTY is fully closed; all resources have been released.
    Closed,
}

impl PaneLifecycleState {
    /// Returns `true` if I/O operations are allowed in this state.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Returns `true` if the pane has been fully torn down.
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }
}
