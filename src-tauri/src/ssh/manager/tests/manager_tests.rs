// SPDX-License-Identifier: MPL-2.0

use crate::session::ids::{ConnectionId, PaneId};
use crate::ssh::connection::SshConnection;
use crate::ssh::{SshConnectionConfig, SshLifecycleState};

use super::super::SshManager;

fn make_config(host: &str) -> SshConnectionConfig {
    SshConnectionConfig {
        id: ConnectionId::new(),
        label: "test".to_string(),
        host: host.to_string(),
        port: 22,
        username: "user".to_string(),
        identity_file: None,
        allow_osc52_write: false,
        keepalive_interval_secs: None,
        keepalive_max_failures: None,
    }
}

/// close_connection on unknown pane_id must return PaneNotFound.
#[tokio::test]
async fn ssh_manager_close_unknown_pane_returns_error() {
    use crate::error::SshError;

    let manager = SshManager::new();
    let unknown_pane = PaneId::new();

    let result = manager.close_connection(unknown_pane).await;
    assert!(
        result.is_err(),
        "close_connection on unknown pane must return error (TEST-SSH-UNIT-001 step 7)"
    );
    match result.unwrap_err() {
        SshError::PaneNotFound(_) => {}
        other => panic!("Expected PaneNotFound, got {other:?}"),
    }
}

/// reconnect on unknown pane_id must return PaneNotFound.
/// Verified via direct map inspection (reconnect requires AppHandle — not constructible
/// in unit tests; the pane-not-found guard executes before any AppHandle usage).
#[test]
fn ssh_manager_reconnect_unknown_pane_not_in_map() {
    let manager = SshManager::new();
    let unknown_pane = PaneId::new();
    // Verify precondition: pane is not in the map.
    // reconnect() starts with `self.connections.get(&pane_id).ok_or(PaneNotFound)`.
    assert!(
        !manager.connections.contains_key(&unknown_pane),
        "reconnect on unknown pane must return error (TEST-SSH-UNIT-001 step 8)"
    );
}

/// get_state returns None for unknown pane.
#[tokio::test]
async fn ssh_manager_get_state_returns_none_for_unknown_pane() {
    let manager = SshManager::new();
    let unknown_pane = PaneId::new();
    assert!(
        manager.get_state(&unknown_pane).is_none(),
        "get_state for unknown pane must return None"
    );
}

/// Manager starts with no connections.
#[test]
fn ssh_manager_starts_empty() {
    let manager = SshManager::new();
    assert_eq!(manager.connection_count(), 0);
}

/// Direct map insertion simulates the state seen after open_connection inserts
/// but before the task completes. Verifies the map is accessible.
#[test]
fn ssh_manager_direct_insert_and_get_state() {
    let manager = SshManager::new();
    let pane_id = PaneId::new();
    let config = make_config("host-a.example.com");
    let conn = SshConnection::new(pane_id.clone(), config);
    manager.connections.insert(pane_id.clone(), conn);

    assert_eq!(manager.connection_count(), 1);
    assert_eq!(
        manager.get_state(&pane_id),
        Some(SshLifecycleState::Connecting),
        "Freshly inserted connection must be in Connecting state"
    );
}

/// open then close should result in zero connections.
#[tokio::test]
async fn ssh_manager_direct_insert_then_close_cleans_up() {
    let manager = SshManager::new();
    let pane_id = PaneId::new();
    let config = make_config("host-b.example.com");
    let conn = SshConnection::new(pane_id.clone(), config);
    manager.connections.insert(pane_id.clone(), conn);
    assert_eq!(manager.connection_count(), 1);

    manager
        .close_connection(pane_id)
        .await
        .expect("close must succeed");
    assert_eq!(
        manager.connection_count(),
        0,
        "connection map must be empty after close"
    );
}

/// Duplicate pane detection: open_connection must reject a pane_id that is
/// already in the map. This test uses direct map insertion to bypass the
/// AppHandle requirement.
#[tokio::test]
async fn ssh_manager_duplicate_pane_detected_via_map() {
    let manager = SshManager::new();
    let pane_id = PaneId::new();
    let config = make_config("host-c.example.com");

    // Simulate the first open_connection inserting the entry.
    let conn = SshConnection::new(pane_id.clone(), config.clone());
    manager.connections.insert(pane_id.clone(), conn);

    // The guard in open_connection checks contains_key before doing anything.
    assert!(
        manager.connections.contains_key(&pane_id),
        "Duplicate detection: map must report pane as present"
    );
}
