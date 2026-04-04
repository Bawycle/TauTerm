// SPDX-License-Identifier: MPL-2.0

//! Credential manager — public API wrapping the platform credential store.
//!
//! `CredentialManager` wraps the PAL `CredentialStore` trait, providing a
//! platform-agnostic interface for storing and retrieving SSH credentials
//! via the OS keychain (Secret Service on Linux).
//!
//! Credentials are stored as `SecVec<u8>` (zeroed on drop) and cleared
//! immediately after the authentication handshake (FS-CRED-003, FS-CRED-004).

// Stub — full implementation requires platform module wiring.

use crate::error::CredentialError;

/// Key format: `tauterm:ssh:{connection_id}:{username}`
fn credential_key(connection_id: &str, username: &str) -> String {
    format!("tauterm:ssh:{connection_id}:{username}")
}

/// Public credential manager — wraps the platform store.
pub struct CredentialManager {
    // TODO: store: Box<dyn crate::platform::CredentialStore>,
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Store a password for a connection. Never logs the credential value.
    pub async fn store_password(
        &self,
        connection_id: &str,
        username: &str,
        _password: &str,
    ) -> Result<(), CredentialError> {
        let _key = credential_key(connection_id, username);
        // TODO: call platform credential store.
        Ok(())
    }

    /// Retrieve a stored password. Returns `None` if not found.
    pub async fn get_password(
        &self,
        connection_id: &str,
        username: &str,
    ) -> Result<Option<String>, CredentialError> {
        let _key = credential_key(connection_id, username);
        // TODO: call platform credential store.
        Ok(None)
    }

    /// Delete stored credentials for a connection.
    pub async fn delete_password(
        &self,
        connection_id: &str,
        username: &str,
    ) -> Result<(), CredentialError> {
        let _key = credential_key(connection_id, username);
        // TODO: call platform credential store.
        Ok(())
    }
}
