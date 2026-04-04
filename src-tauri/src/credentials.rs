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
}
