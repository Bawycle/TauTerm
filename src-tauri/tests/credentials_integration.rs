// SPDX-License-Identifier: MPL-2.0

//! Integration tests for LinuxCredentialStore — require an active SecretService daemon.
//! Run with: cargo nextest run --test credentials_integration
//! In CI: use the Podman environment defined in Containerfile.keyring-test

#[cfg(target_os = "linux")]
mod linux {
    use tau_term_lib::platform::credentials_linux::LinuxCredentialStore;
    use tau_term_lib::platform::CredentialStore;

    // -----------------------------------------------------------------------
    // RAII cleanup guard — guarantees keyring entries are removed even on
    // assertion failures (the Drop impl runs before the test exits).
    // -----------------------------------------------------------------------

    struct Cleanup<'a> {
        store: &'a LinuxCredentialStore,
        key: String,
    }

    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            // Best-effort — ignore errors on cleanup.
            let _ = self.store.delete(&self.key);
        }
    }

    // -----------------------------------------------------------------------
    // SEC-CRED-INT-001: store → get → delete → get round-trip
    // -----------------------------------------------------------------------

    /// Full round-trip: store a credential, retrieve it, delete it, confirm it
    /// is gone.  Skips gracefully when the SecretService daemon is unavailable.
    #[test]
    fn secret_service_roundtrip_store_get_delete() {
        let store = LinuxCredentialStore::new();
        if !store.is_available() {
            eprintln!("SKIP: Secret Service unavailable");
            return;
        }

        const KEY: &str = "tauterm:integration-test:roundtrip";
        const SECRET: &[u8] = b"integration-test-secret";

        store.store(KEY, SECRET).expect("store() must succeed");

        // Cleanup runs in Drop — even if assertions below panic.
        let _cleanup = Cleanup {
            store: &store,
            key: KEY.to_string(),
        };

        let retrieved = store.get(KEY).expect("get() must succeed");
        assert_eq!(
            retrieved,
            Some(SECRET.to_vec()),
            "Retrieved secret must match the stored value"
        );

        store.delete(KEY).expect("delete() must succeed");

        let after_delete = store.get(KEY).expect("get() after delete must succeed");
        assert_eq!(
            after_delete,
            None,
            "get() after delete must return None"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-CRED-INT-002: get on a key that was never stored returns None
    // -----------------------------------------------------------------------

    /// Querying a key that does not exist must return Ok(None), not an error.
    #[test]
    fn secret_service_get_nonexistent_returns_none() {
        let store = LinuxCredentialStore::new();
        if !store.is_available() {
            eprintln!("SKIP: Secret Service unavailable");
            return;
        }

        let result = store
            .get("tauterm:integration-test:nonexistent-key")
            .expect("get() on absent key must not error");

        assert!(
            result.is_none(),
            "get() on a key that was never stored must return None"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-CRED-INT-003: second store() overwrites the first value
    // -----------------------------------------------------------------------

    /// Storing a different secret under the same key must replace the previous
    /// value — not accumulate or return the old one.
    #[test]
    fn secret_service_overwrite_replaces_value() {
        let store = LinuxCredentialStore::new();
        if !store.is_available() {
            eprintln!("SKIP: Secret Service unavailable");
            return;
        }

        const KEY: &str = "tauterm:integration-test:overwrite";

        store
            .store(KEY, b"first")
            .expect("first store() must succeed");

        // Cleanup is registered immediately after the first successful store
        // so that even if the second store() or the assertion fails, the key
        // is removed from the keyring.
        let _cleanup = Cleanup {
            store: &store,
            key: KEY.to_string(),
        };

        store
            .store(KEY, b"second")
            .expect("second store() must succeed");

        let retrieved = store.get(KEY).expect("get() must succeed");
        assert_eq!(
            retrieved,
            Some(b"second".to_vec()),
            "get() after overwrite must return the latest value"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-CRED-INT-004: delete on a key that was never stored is a no-op
    // -----------------------------------------------------------------------

    /// Deleting a non-existent key must return Ok(()) — no error.
    #[test]
    fn secret_service_delete_nonexistent_is_noop() {
        let store = LinuxCredentialStore::new();
        if !store.is_available() {
            eprintln!("SKIP: Secret Service unavailable");
            return;
        }

        store
            .delete("tauterm:integration-test:never-stored")
            .expect("delete() on absent key must return Ok(())");
    }

    // -----------------------------------------------------------------------
    // SEC-CRED-INT-005: binary secret (null bytes + non-UTF-8 bytes) round-trip
    // -----------------------------------------------------------------------

    /// Binary secrets that contain null bytes or values outside the UTF-8 range
    /// must survive a store/get cycle bit-for-bit intact.
    #[test]
    fn secret_service_binary_secret_roundtrip() {
        let store = LinuxCredentialStore::new();
        if !store.is_available() {
            eprintln!("SKIP: Secret Service unavailable");
            return;
        }

        const KEY: &str = "tauterm:integration-test:binary";
        const BINARY_SECRET: &[u8] = b"binary\x00data\xff\xfe";

        store
            .store(KEY, BINARY_SECRET)
            .expect("store() of binary secret must succeed");

        let _cleanup = Cleanup {
            store: &store,
            key: KEY.to_string(),
        };

        let retrieved = store.get(KEY).expect("get() of binary secret must succeed");
        assert_eq!(
            retrieved,
            Some(BINARY_SECRET.to_vec()),
            "Retrieved binary secret must be identical to the stored bytes"
        );
    }
}
