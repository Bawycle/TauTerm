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

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Lifecycle state machine — is_active / is_closed predicates
    // -----------------------------------------------------------------------

    #[test]
    fn running_state_is_active() {
        assert!(PaneLifecycleState::Running.is_active());
    }

    #[test]
    fn spawning_state_is_not_active() {
        assert!(!PaneLifecycleState::Spawning.is_active());
    }

    #[test]
    fn terminated_state_is_not_active() {
        assert!(!PaneLifecycleState::Terminated {
            exit_code: Some(0),
            error: None
        }
        .is_active());
    }

    #[test]
    fn closing_state_is_not_active() {
        assert!(!PaneLifecycleState::Closing.is_active());
    }

    #[test]
    fn closed_state_is_closed() {
        assert!(PaneLifecycleState::Closed.is_closed());
    }

    #[test]
    fn running_state_is_not_closed() {
        assert!(!PaneLifecycleState::Running.is_closed());
    }

    // -----------------------------------------------------------------------
    // Serialization — tag-based discriminant (FS-PTY-006 contract)
    // -----------------------------------------------------------------------

    #[test]
    fn running_serializes_with_type_tag() {
        let json = serde_json::to_string(&PaneLifecycleState::Running).expect("serialize failed");
        assert!(json.contains("\"type\":\"running\""), "got: {json}");
    }

    #[test]
    fn terminated_serializes_with_exit_code() {
        let state = PaneLifecycleState::Terminated {
            exit_code: Some(1),
            error: None,
        };
        let json = serde_json::to_string(&state).expect("serialize failed");
        assert!(json.contains("\"exitCode\":1") || json.contains("\"exit_code\":1"), "got: {json}");
    }

    #[test]
    fn terminated_round_trips_through_json() {
        let state = PaneLifecycleState::Terminated {
            exit_code: Some(42),
            error: Some("some error".to_string()),
        };
        let json = serde_json::to_string(&state).expect("serialize failed");
        let restored: PaneLifecycleState = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(state, restored);
    }
}
