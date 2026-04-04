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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ids::{ConnectionId, PaneId};

    fn make_config() -> SshConnectionConfig {
        SshConnectionConfig {
            id: ConnectionId::new(),
            label: "test-server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            identity_file: None,
            allow_osc52_write: false,
        }
    }

    // -----------------------------------------------------------------------
    // TEST-SSH-007 (partial) — SSH connection state machine transitions
    // FS-SSH-020
    // -----------------------------------------------------------------------

    #[test]
    fn new_connection_starts_in_connecting_state() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        assert_eq!(conn.state(), SshLifecycleState::Connecting);
    }

    #[test]
    fn set_state_transitions_to_authenticating() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Authenticating);
        assert_eq!(conn.state(), SshLifecycleState::Authenticating);
    }

    #[test]
    fn set_state_transitions_to_connected() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Connected);
        assert_eq!(conn.state(), SshLifecycleState::Connected);
    }

    #[test]
    fn set_state_transitions_to_disconnected() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Connected);
        conn.set_state(SshLifecycleState::Disconnected);
        assert_eq!(conn.state(), SshLifecycleState::Disconnected);
    }

    #[test]
    fn set_state_transitions_to_closed() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        conn.set_state(SshLifecycleState::Closed);
        assert_eq!(conn.state(), SshLifecycleState::Closed);
    }

    #[test]
    fn full_lifecycle_sequence_transitions_correctly() {
        let conn = SshConnection::new(PaneId::new(), make_config());
        // Connecting → Authenticating → Connected → Disconnected → Closed
        assert_eq!(conn.state(), SshLifecycleState::Connecting);
        conn.set_state(SshLifecycleState::Authenticating);
        assert_eq!(conn.state(), SshLifecycleState::Authenticating);
        conn.set_state(SshLifecycleState::Connected);
        assert_eq!(conn.state(), SshLifecycleState::Connected);
        conn.set_state(SshLifecycleState::Disconnected);
        assert_eq!(conn.state(), SshLifecycleState::Disconnected);
        conn.set_state(SshLifecycleState::Closed);
        assert_eq!(conn.state(), SshLifecycleState::Closed);
    }

    #[test]
    fn ssh_lifecycle_state_serializes_with_type_tag() {
        let json = serde_json::to_string(&SshLifecycleState::Connected).expect("serialize failed");
        assert!(json.contains("\"type\":\"connected\""), "got: {json}");
    }
}
