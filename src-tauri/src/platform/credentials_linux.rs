// SPDX-License-Identifier: MPL-2.0

//! Linux credential store — Secret Service D-Bus adapter.
//!
//! Uses the `secret-service` crate to interface with GNOME Keyring / KWallet.
//! Falls back gracefully when the Secret Service is unavailable (§7.2).

use crate::error::CredentialError;
use crate::platform::CredentialStore;

#[derive(Default)]
pub struct LinuxCredentialStore {}

impl LinuxCredentialStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl CredentialStore for LinuxCredentialStore {
    fn is_available(&self) -> bool {
        // TODO: probe Secret Service D-Bus availability.
        false
    }

    fn store(&self, _key: &str, _secret: &[u8]) -> Result<(), CredentialError> {
        // TODO: implement via secret-service crate.
        Err(CredentialError::Unavailable(
            "Secret Service not yet implemented.".to_string(),
        ))
    }

    fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, CredentialError> {
        // TODO: implement via secret-service crate.
        Ok(None)
    }

    fn delete(&self, _key: &str) -> Result<(), CredentialError> {
        // TODO: implement via secret-service crate.
        Ok(())
    }
}
