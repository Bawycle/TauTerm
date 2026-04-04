// SPDX-License-Identifier: MPL-2.0

//! Windows credential store stub — not supported in v1.

use crate::error::CredentialError;
use crate::platform::CredentialStore;

#[derive(Default)]
pub struct WindowsCredentialStore {}

impl WindowsCredentialStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl CredentialStore for WindowsCredentialStore {
    fn is_available(&self) -> bool {
        false
    }

    fn store(&self, _key: &str, _secret: &[u8]) -> Result<(), CredentialError> {
        unimplemented!("Windows credential store not supported in v1")
    }

    fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, CredentialError> {
        unimplemented!("Windows credential store not supported in v1")
    }

    fn delete(&self, _key: &str) -> Result<(), CredentialError> {
        unimplemented!("Windows credential store not supported in v1")
    }
}
