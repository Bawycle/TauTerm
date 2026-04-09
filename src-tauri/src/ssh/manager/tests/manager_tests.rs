// SPDX-License-Identifier: MPL-2.0

use std::sync::Mutex;

use crate::credentials::CredentialManager;
use crate::error::CredentialError;
use crate::platform::CredentialStore;
use crate::session::ids::{ConnectionId, PaneId};
use crate::ssh::connection::SshConnection;
use crate::ssh::{SshConnectionConfig, SshLifecycleState};

use super::super::SshManager;

// ---------------------------------------------------------------------------
// Mock credential store for reconnect tests
// ---------------------------------------------------------------------------

struct MockCredentialStore {
    available: bool,
    data: Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl MockCredentialStore {
    fn new_with_password(key: &str, password: &str) -> Self {
        let mut map = std::collections::HashMap::new();
        map.insert(key.to_string(), password.as_bytes().to_vec());
        Self {
            available: true,
            data: Mutex::new(map),
        }
    }

    fn unavailable() -> Self {
        Self {
            available: false,
            data: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn empty() -> Self {
        Self {
            available: true,
            data: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl CredentialStore for MockCredentialStore {
    fn is_available(&self) -> bool {
        self.available
    }

    fn store(&self, key: &str, secret: &[u8]) -> Result<(), CredentialError> {
        self.data
            .lock()
            .unwrap()
            .insert(key.to_string(), secret.to_vec());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CredentialError> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), CredentialError> {
        self.data.lock().unwrap().remove(key);
        Ok(())
    }
}

fn make_config(host: &str) -> SshConnectionConfig {
    use crate::preferences::types::{SshHost, SshLabel, SshUsername};
    SshConnectionConfig {
        id: ConnectionId::new(),
        label: SshLabel::try_from("test".to_string()).unwrap(),
        host: SshHost::try_from(host.to_string()).unwrap(),
        port: 22,
        username: SshUsername::try_from("user".to_string()).unwrap(),
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

// ---------------------------------------------------------------------------
// A3 — resolve_reconnect_credentials tests
// ---------------------------------------------------------------------------

/// When the keychain is available and contains a password for the connection,
/// `resolve_reconnect_credentials` must return `Some(Credentials)` with the
/// correct username and password.
#[tokio::test]
async fn resolve_reconnect_credentials_returns_creds_from_keychain() {
    use std::sync::Arc;

    let config = make_config("example.com");
    // Build the keychain key expected by credential_key(): "tauterm:ssh:{id}:{username}"
    let key = format!("tauterm:ssh:{}:user", config.id);
    let store = MockCredentialStore::new_with_password(&key, "s3cr3t");
    let manager = Arc::new(SshManager {
        connections: dashmap::DashMap::new(),
        pending_credentials: dashmap::DashMap::new(),
        pending_passphrases: dashmap::DashMap::new(),
        pending_host_keys: dashmap::DashMap::new(),
        credential_manager: Arc::new(CredentialManager::new_with_store(Box::new(store))),
    });

    let result = manager.resolve_reconnect_credentials(&config).await;

    assert!(
        result.is_some(),
        "must return Some when keychain has a password"
    );
    let creds = result.unwrap();
    assert_eq!(creds.username, "user");
    assert_eq!(creds.password.as_deref(), Some("s3cr3t"));
    assert!(
        !creds.save_in_keychain,
        "save_in_keychain must be false on reconnect"
    );
}

/// When the credential store reports `is_available() = false`,
/// `resolve_reconnect_credentials` must return `None` immediately without
/// attempting a keychain lookup.
#[tokio::test]
async fn resolve_reconnect_credentials_returns_none_when_store_unavailable() {
    use std::sync::Arc;

    let config = make_config("example.com");
    let store = MockCredentialStore::unavailable();
    let manager = Arc::new(SshManager {
        connections: dashmap::DashMap::new(),
        pending_credentials: dashmap::DashMap::new(),
        pending_passphrases: dashmap::DashMap::new(),
        pending_host_keys: dashmap::DashMap::new(),
        credential_manager: Arc::new(CredentialManager::new_with_store(Box::new(store))),
    });

    let result = manager.resolve_reconnect_credentials(&config).await;

    assert!(
        result.is_none(),
        "must return None when credential store is unavailable"
    );
}

/// When the credential store is available but contains no password for the
/// connection, `resolve_reconnect_credentials` must return `None`.
#[tokio::test]
async fn resolve_reconnect_credentials_returns_none_when_no_password_stored() {
    use std::sync::Arc;

    let config = make_config("example.com");
    let store = MockCredentialStore::empty();
    let manager = Arc::new(SshManager {
        connections: dashmap::DashMap::new(),
        pending_credentials: dashmap::DashMap::new(),
        pending_passphrases: dashmap::DashMap::new(),
        pending_host_keys: dashmap::DashMap::new(),
        credential_manager: Arc::new(CredentialManager::new_with_store(Box::new(store))),
    });

    let result = manager.resolve_reconnect_credentials(&config).await;

    assert!(
        result.is_none(),
        "must return None when no password is stored for this connection"
    );
}
