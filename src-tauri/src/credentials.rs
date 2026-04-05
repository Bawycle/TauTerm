// SPDX-License-Identifier: MPL-2.0

//! Credential manager — public API wrapping the platform credential store.
//!
//! `CredentialManager` wraps the PAL `CredentialStore` trait, providing a
//! platform-agnostic interface for storing and retrieving SSH credentials
//! via the OS keychain (Secret Service on Linux).
//!
//! On Linux, the underlying store is `LinuxCredentialStore`, which uses the
//! `secret-service` crate to interface with GNOME Keyring or KWallet via D-Bus.
//! On macOS and Windows, platform-specific stubs are used (out of scope for v1).
//!
//! Credentials are keyed as `tauterm:ssh:{connection_id}:{username}` and stored
//! as UTF-8 byte sequences. The manager never logs credential values.

use crate::error::CredentialError;
use crate::platform::{self, CredentialStore};

/// Key format: `tauterm:ssh:{connection_id}:{username}`
fn credential_key(connection_id: &str, username: &str) -> String {
    format!("tauterm:ssh:{connection_id}:{username}")
}

/// Public credential manager — wraps the platform credential store.
pub struct CredentialManager {
    store: Box<dyn CredentialStore>,
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialManager {
    /// Create a new `CredentialManager` backed by the platform credential store.
    ///
    /// On Linux this is `LinuxCredentialStore` (Secret Service via D-Bus).
    /// Call `is_available()` to check whether the store is operational before use.
    pub fn new() -> Self {
        Self {
            store: platform::create_credential_store(),
        }
    }

    /// Returns `true` if the underlying platform credential store is operational.
    ///
    /// On Linux: probes the D-Bus Secret Service. Returns `false` if the daemon
    /// is not running (e.g., headless server without GNOME Keyring / KWallet).
    pub fn is_available(&self) -> bool {
        self.store.is_available()
    }

    /// Store a password for a connection. Never logs the credential value.
    pub async fn store_password(
        &self,
        connection_id: &str,
        username: &str,
        password: &str,
    ) -> Result<(), CredentialError> {
        let key = credential_key(connection_id, username);
        self.store.store(&key, password.as_bytes())
    }

    /// Retrieve a stored password. Returns `None` if not found.
    pub async fn get_password(
        &self,
        connection_id: &str,
        username: &str,
    ) -> Result<Option<String>, CredentialError> {
        let key = credential_key(connection_id, username);
        let bytes = self.store.get(&key)?;
        match bytes {
            None => Ok(None),
            Some(b) => {
                let s = String::from_utf8(b).map_err(|e| {
                    CredentialError::Io(format!("Stored credential is not valid UTF-8: {e}"))
                })?;
                Ok(Some(s))
            }
        }
    }

    /// Delete stored credentials for a connection.
    pub async fn delete_password(
        &self,
        connection_id: &str,
        username: &str,
    ) -> Result<(), CredentialError> {
        let key = credential_key(connection_id, username);
        self.store.delete(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CredentialError;
    use std::sync::Mutex;

    /// credential_key must produce the expected format.
    #[test]
    fn credential_key_format() {
        let key = credential_key("conn-abc", "alice");
        assert_eq!(key, "tauterm:ssh:conn-abc:alice");
    }

    /// CredentialManager::new() must not panic — platform store construction is
    /// always valid (returns a no-op stub if the platform is not supported).
    #[test]
    fn credential_manager_new_does_not_panic() {
        let _mgr = CredentialManager::new();
    }

    /// is_available() must return a bool without panicking.
    /// On CI / headless environments this may return false — that is valid.
    #[test]
    fn credential_manager_is_available_returns_bool() {
        let mgr = CredentialManager::new();
        let _ = mgr.is_available(); // just must not panic
    }

    // -----------------------------------------------------------------------
    // CRED-MOCK-001: In-memory mock CredentialStore — round-trip tests
    //
    // These tests exercise CredentialManager logic (key formatting, UTF-8
    // decoding) without requiring a live D-Bus Secret Service daemon.
    // -----------------------------------------------------------------------

    /// In-memory credential store for unit tests. Thread-safe via Mutex.
    struct MockCredentialStore {
        data: Mutex<std::collections::HashMap<String, Vec<u8>>>,
    }

    impl MockCredentialStore {
        fn new() -> Self {
            Self {
                data: Mutex::new(std::collections::HashMap::new()),
            }
        }
    }

    impl crate::platform::CredentialStore for MockCredentialStore {
        fn is_available(&self) -> bool {
            true
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

    fn mock_manager() -> CredentialManager {
        CredentialManager {
            store: Box::new(MockCredentialStore::new()),
        }
    }

    /// CRED-MOCK-001: store_password → get_password round-trip returns same value.
    #[tokio::test]
    async fn cred_mock_001_store_and_retrieve_password_round_trip() {
        let mgr = mock_manager();
        mgr.store_password("conn-1", "alice", "s3cr3t!")
            .await
            .expect("store must succeed");

        let result = mgr
            .get_password("conn-1", "alice")
            .await
            .expect("get must succeed");

        assert_eq!(
            result,
            Some("s3cr3t!".to_string()),
            "Retrieved password must match stored"
        );
    }

    /// CRED-MOCK-002: get_password returns None when nothing is stored.
    #[tokio::test]
    async fn cred_mock_002_get_password_returns_none_when_not_stored() {
        let mgr = mock_manager();
        let result = mgr
            .get_password("conn-nonexistent", "bob")
            .await
            .expect("get must succeed (not an error)");
        assert!(result.is_none(), "Must return None for unknown credential");
    }

    /// CRED-MOCK-003: delete_password removes the credential.
    #[tokio::test]
    async fn cred_mock_003_delete_removes_stored_password() {
        let mgr = mock_manager();
        mgr.store_password("conn-2", "carol", "pass123")
            .await
            .expect("store must succeed");

        mgr.delete_password("conn-2", "carol")
            .await
            .expect("delete must succeed");

        let result = mgr
            .get_password("conn-2", "carol")
            .await
            .expect("get must succeed");
        assert!(result.is_none(), "Password must be gone after delete");
    }

    /// CRED-MOCK-004: credential_key isolation — two different users on the same
    /// connection must not share the same stored credential.
    #[tokio::test]
    async fn cred_mock_004_different_users_have_different_keys() {
        let mgr = mock_manager();
        mgr.store_password("conn-3", "alice", "alice_pass")
            .await
            .expect("store alice");
        mgr.store_password("conn-3", "bob", "bob_pass")
            .await
            .expect("store bob");

        let alice = mgr.get_password("conn-3", "alice").await.unwrap();
        let bob = mgr.get_password("conn-3", "bob").await.unwrap();

        assert_eq!(alice, Some("alice_pass".to_string()));
        assert_eq!(bob, Some("bob_pass".to_string()));
    }

    /// CRED-MOCK-005: overwriting a credential replaces the old value.
    #[tokio::test]
    async fn cred_mock_005_overwrite_updates_stored_password() {
        let mgr = mock_manager();
        mgr.store_password("conn-4", "dave", "old_pass")
            .await
            .unwrap();
        mgr.store_password("conn-4", "dave", "new_pass")
            .await
            .unwrap();

        let result = mgr.get_password("conn-4", "dave").await.unwrap();
        assert_eq!(
            result,
            Some("new_pass".to_string()),
            "Overwrite must replace old value"
        );
    }
}
