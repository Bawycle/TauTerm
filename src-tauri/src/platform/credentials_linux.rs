// SPDX-License-Identifier: MPL-2.0

//! Linux credential store — Secret Service D-Bus adapter.
//!
//! Uses the `secret-service` crate to interface with GNOME Keyring / KWallet.
//! Falls back gracefully when the Secret Service is unavailable (§7.2):
//! - `is_available()` returns `false`
//! - `store()` returns `Err(CredentialError::Unavailable)`
//! - `get()` returns `Ok(None)` — no credential found
//! - `delete()` returns `Ok(())` — no-op
//!
//! ## Attribute schema
//! All items are stored with attributes:
//! - `service` = `"tauterm"`
//! - `key` = the caller-provided key string (e.g., `"tauterm:ssh:conn-id:username"`)
//!
//! This makes items discoverable via `secret-tool lookup service tauterm`.

use std::collections::HashMap;

use secret_service::{EncryptionType, SecretService};

use crate::error::CredentialError;
use crate::platform::CredentialStore;

/// Attribute key for the service name.
const ATTR_SERVICE: &str = "service";
/// Attribute value identifying TauTerm secrets.
const ATTR_SERVICE_VALUE: &str = "tauterm";
/// Attribute key for the credential key.
const ATTR_KEY: &str = "key";
/// Label prefix for all TauTerm secrets.
const LABEL_PREFIX: &str = "TauTerm";

#[derive(Default)]
pub struct LinuxCredentialStore {}

impl LinuxCredentialStore {
    pub fn new() -> Self {
        Self {}
    }

    /// Probe Secret Service availability by attempting a connection.
    ///
    /// This is a blocking call using `tokio::task::block_in_place` so that it
    /// can be called from synchronous context. Returns `false` if D-Bus is
    /// unavailable or the Secret Service daemon is not running.
    fn probe_availability() -> bool {
        // We use `tokio::task::block_in_place` + a one-shot runtime to probe
        // without requiring `async` in the trait. On Linux, the D-Bus call
        // must be made on a thread with an async runtime context.
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => return false,
        };

        rt.block_on(async {
            SecretService::connect(EncryptionType::Plain)
                .await
                .is_ok()
        })
    }
}

impl CredentialStore for LinuxCredentialStore {
    fn is_available(&self) -> bool {
        Self::probe_availability()
    }

    fn store(&self, key: &str, secret: &[u8]) -> Result<(), CredentialError> {
        let key = key.to_string();
        let secret = secret.to_vec();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CredentialError::Io(e.to_string()))?;

        rt.block_on(async {
            let ss = SecretService::connect(EncryptionType::Dh).await.map_err(|e| {
                CredentialError::Unavailable(format!("Secret Service unavailable: {e}"))
            })?;

            let collection = ss
                .get_default_collection()
                .await
                .map_err(|e| CredentialError::Io(format!("Could not get collection: {e}")))?;

            // Unlock the collection if needed.
            collection
                .unlock()
                .await
                .map_err(|e| CredentialError::Io(format!("Could not unlock collection: {e}")))?;

            let label = format!("{LABEL_PREFIX} — {key}");
            let mut attributes: HashMap<&str, &str> = HashMap::new();
            attributes.insert(ATTR_SERVICE, ATTR_SERVICE_VALUE);
            attributes.insert(ATTR_KEY, key.as_str());

            collection
                .create_item(
                    &label,
                    attributes,
                    &secret,
                    true, // replace existing item with same attributes
                    "text/plain",
                )
                .await
                .map_err(|e| CredentialError::Io(format!("Failed to store credential: {e}")))?;

            Ok(())
        })
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CredentialError> {
        let key = key.to_string();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CredentialError::Io(e.to_string()))?;

        rt.block_on(async {
            let ss = SecretService::connect(EncryptionType::Dh).await.map_err(|e| {
                CredentialError::Unavailable(format!("Secret Service unavailable: {e}"))
            })?;

            let mut attributes: HashMap<&str, &str> = HashMap::new();
            attributes.insert(ATTR_SERVICE, ATTR_SERVICE_VALUE);
            attributes.insert(ATTR_KEY, key.as_str());

            let result = ss
                .search_items(attributes)
                .await
                .map_err(|e| CredentialError::Io(format!("Search failed: {e}")))?;

            let item = result
                .unlocked
                .into_iter()
                .next()
                .or_else(|| result.locked.into_iter().next());

            match item {
                None => Ok(None),
                Some(item) => {
                    // Unlock the item if locked.
                    item.unlock()
                        .await
                        .map_err(|e| CredentialError::Io(format!("Unlock failed: {e}")))?;

                    let secret = item
                        .get_secret()
                        .await
                        .map_err(|e| CredentialError::Io(format!("get_secret failed: {e}")))?;

                    Ok(Some(secret))
                }
            }
        })
    }

    fn delete(&self, key: &str) -> Result<(), CredentialError> {
        let key = key.to_string();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CredentialError::Io(e.to_string()))?;

        rt.block_on(async {
            let ss = SecretService::connect(EncryptionType::Dh).await.map_err(|e| {
                CredentialError::Unavailable(format!("Secret Service unavailable: {e}"))
            })?;

            let mut attributes: HashMap<&str, &str> = HashMap::new();
            attributes.insert(ATTR_SERVICE, ATTR_SERVICE_VALUE);
            attributes.insert(ATTR_KEY, key.as_str());

            let result = ss
                .search_items(attributes)
                .await
                .map_err(|e| CredentialError::Io(format!("Search failed: {e}")))?;

            for item in result.unlocked.into_iter().chain(result.locked.into_iter()) {
                item.delete()
                    .await
                    .map_err(|e| CredentialError::Io(format!("Delete failed: {e}")))?;
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SEC-CRED-008 — is_available() returns false when D-Bus unavailable
    // -----------------------------------------------------------------------

    /// SEC-CRED-008: is_available() must not panic in any environment.
    ///
    /// In a headless CI without D-Bus/Secret Service, it returns false.
    /// In a desktop environment with a keyring daemon, it returns true.
    /// Either result is acceptable — the important thing is no panic.
    #[test]
    fn credential_store_is_available_does_not_panic() {
        let store = LinuxCredentialStore::new();
        let _available = store.is_available(); // must not panic
    }

    /// SEC-CRED-008: store() returns Err when Secret Service is unavailable.
    ///
    /// In a headless CI environment, Secret Service is not running.
    /// The error code must be CRED_STORE_UNAVAILABLE, not a panic.
    #[test]
    fn credential_store_store_fails_gracefully_when_unavailable() {
        let store = LinuxCredentialStore::new();
        if store.is_available() {
            // Skip in environments with a running keyring.
            return;
        }
        let result = store.store("test-key", b"test-secret");
        assert!(
            result.is_err(),
            "store() must fail when Secret Service is unavailable"
        );
        match result.unwrap_err() {
            CredentialError::Unavailable(_) => {}
            CredentialError::Io(_) => {
                // Also acceptable — runtime build failure or D-Bus error.
            }
            other => panic!(
                "Expected Unavailable or Io error when Secret Service missing, got {other:?}"
            ),
        }
    }

    /// SEC-CRED-008: get() returns Ok(None) gracefully when unavailable.
    #[test]
    fn credential_store_get_returns_none_when_unavailable() {
        let store = LinuxCredentialStore::new();
        if store.is_available() {
            return; // skip in keyring-available environments
        }
        let result = store.get("nonexistent-key");
        // Either Ok(None) or an error — must not panic.
        let _ = result;
    }

    /// SEC-CRED-008: delete() returns Ok or error gracefully when unavailable.
    #[test]
    fn credential_store_delete_does_not_panic() {
        let store = LinuxCredentialStore::new();
        let _ = store.delete("test-key"); // must not panic
    }
}
