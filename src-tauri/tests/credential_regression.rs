// SPDX-License-Identifier: MPL-2.0

//! Credential security regression tests — no live daemon required.
//!
//! These tests cover SEC-CRED-001 through SEC-CRED-006 without requiring a
//! live Secret Service daemon (Podman) or a real SSH server. They run under
//! `cargo nextest run` in any standard CI environment.
//!
//! MUST pass before any merge touching:
//!   - `src-tauri/src/credentials.rs`
//!   - `src-tauri/src/ssh/manager.rs`
//!   - `src-tauri/src/commands/ssh_prompt_cmds.rs`

use tau_term_lib::error::CredentialError;
use tau_term_lib::platform::CredentialStore;

// ---------------------------------------------------------------------------
// SEC-CRED-001: No password field in SshConnectionConfig serialization
// ---------------------------------------------------------------------------

/// SEC-CRED-001: Passwords MUST NOT appear in `SshConnectionConfig` serialized form.
///
/// Credentials go to the OS keychain via `CredentialManager`. The connection
/// config only carries connection metadata (host, port, username, identity file
/// path). Verifies that the serialized JSON contains no "password" key, ensuring
/// that the struct design itself enforces the separation (FS-CRED-001).
#[test]
fn cred_reg_001_ssh_connection_config_has_no_password_in_serde() {
    use tau_term_lib::session::ids::ConnectionId;
    use tau_term_lib::ssh::SshConnectionConfig;

    let config = SshConnectionConfig {
        id: ConnectionId::new(),
        label: "test-server".to_string(),
        host: "192.0.2.1".to_string(),
        port: 22,
        username: "alice".to_string(),
        identity_file: None,
        allow_osc52_write: false,
        keepalive_interval_secs: None,
        keepalive_max_failures: None,
    };

    let json =
        serde_json::to_string(&config).expect("SshConnectionConfig serialization must not fail");

    assert!(
        !json.contains("password"),
        "SshConnectionConfig must not serialize a password field; got: {json}"
    );
    assert!(
        !json.contains("Password"),
        "SshConnectionConfig must not serialize a Password field (case variant); got: {json}"
    );
    assert!(
        !json.contains("secret"),
        "SshConnectionConfig must not serialize a secret field; got: {json}"
    );
}

// ---------------------------------------------------------------------------
// SEC-CRED-002: Credentials implements ZeroizeOnDrop (compile-time assertion)
// ---------------------------------------------------------------------------

/// SEC-CRED-002: `Credentials` MUST implement `ZeroizeOnDrop` so password bytes
/// are overwritten in memory on drop (FS-CRED-003).
///
/// This is a compile-time check: if `ZeroizeOnDrop` is ever removed from
/// `Credentials`, this function will fail to compile, catching the regression
/// before any test even runs.
#[test]
fn cred_reg_002_credentials_implements_zeroize_on_drop() {
    fn assert_zeroize_on_drop<T: zeroize::ZeroizeOnDrop>() {}
    // If ZeroizeOnDrop is removed from Credentials, this line fails to compile.
    assert_zeroize_on_drop::<tau_term_lib::ssh::Credentials>();
}

// ---------------------------------------------------------------------------
// SEC-CRED-003: Credentials Debug impl redacts the password value
// ---------------------------------------------------------------------------

/// SEC-CRED-003: The `Debug` impl of `Credentials` MUST NOT expose the password
/// value in formatted output (FS-CRED-004).
///
/// The struct uses a manual `Debug` impl. This test verifies that the plaintext
/// password is never leaked into log output via the `{:?}` formatter.
#[test]
fn cred_reg_003_credentials_debug_redacts_password() {
    use tau_term_lib::ssh::Credentials;

    let creds = Credentials {
        username: "alice".to_string(),
        password: Some("s3cr3t_hunter2".to_string()),
        private_key_path: None,
        save_in_keychain: false,
    };

    let debug_output = format!("{creds:?}");

    assert!(
        !debug_output.contains("s3cr3t_hunter2"),
        "Debug output must not contain the plaintext password; got: {debug_output}"
    );
    // Verify the redaction marker is present, confirming the field is shown as redacted
    // rather than silently omitted (which would make the output misleading).
    assert!(
        debug_output.contains("redacted"),
        "Debug output must include a <redacted> marker for the password field; got: {debug_output}"
    );
}

// ---------------------------------------------------------------------------
// SEC-CRED-004: SshConnectionConfig stores identity by path only, not key content
// ---------------------------------------------------------------------------

/// SEC-CRED-004: Private key content MUST NOT be stored in `SshConnectionConfig`
/// (FS-CRED-002). Only the path to the identity file is stored.
///
/// Verifies that deserializing a JSON payload containing raw key material either
/// fails or silently drops the field — and that re-serialization never emits
/// private key content.
#[test]
fn cred_reg_004_ssh_connection_config_stores_identity_path_only() {
    use tau_term_lib::ssh::SshConnectionConfig;

    // Attempt to deserialize a payload carrying raw private key material.
    let json_with_private_key = r#"{"privateKey": "-----BEGIN RSA PRIVATE KEY-----\nMIIE..."}"#;
    let result = serde_json::from_str::<SshConnectionConfig>(json_with_private_key);

    match result {
        Ok(config) => {
            // Deserialization succeeded (extra fields ignored) — verify the config
            // does not carry any raw private key content when re-serialized.
            let re_serialized =
                serde_json::to_string(&config).expect("re-serialization must not fail");
            assert!(
                !re_serialized.contains("privateKey"),
                "Re-serialized config must not contain a privateKey field; got: {re_serialized}"
            );
            assert!(
                !re_serialized.contains("BEGIN RSA"),
                "Re-serialized config must not contain private key material; got: {re_serialized}"
            );
            assert!(
                !re_serialized.contains("BEGIN OPENSSH"),
                "Re-serialized config must not contain OpenSSH private key material; got: {re_serialized}"
            );
        }
        Err(_) => {
            // Strict parsing rejected the unknown field — this is the stricter,
            // equally acceptable outcome.
        }
    }
}

// ---------------------------------------------------------------------------
// SEC-CRED-005: Unavailable store returns Err, no disk fallback
// ---------------------------------------------------------------------------

/// In-memory stub that always reports the credential store as unavailable.
///
/// Used by `cred_reg_005` to verify that `CredentialManager` propagates
/// `CredentialError::Unavailable` without falling back to writing credentials
/// to disk.
struct AlwaysUnavailableStore;

impl CredentialStore for AlwaysUnavailableStore {
    fn is_available(&self) -> bool {
        false
    }

    fn store(&self, _key: &str, _secret: &[u8]) -> Result<(), CredentialError> {
        Err(CredentialError::Unavailable(
            "test: no keychain available".to_string(),
        ))
    }

    fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, CredentialError> {
        // Unavailable store returns no data.
        Ok(None)
    }

    fn delete(&self, _key: &str) -> Result<(), CredentialError> {
        Ok(())
    }
}

/// SEC-CRED-005: When the credential store is unavailable, `store_password()`
/// MUST return `Err(CredentialError::Unavailable)` and MUST NOT fall back to
/// writing credentials to disk (FS-CRED-005).
///
/// Uses `CredentialManager::new_with_store()` (test-only constructor) to inject
/// `AlwaysUnavailableStore` without requiring a live daemon.
#[tokio::test]
async fn cred_reg_005_unavailable_store_returns_err_no_disk_fallback() {
    use tau_term_lib::credentials::CredentialManager;

    let manager = CredentialManager::new_with_store(Box::new(AlwaysUnavailableStore));

    let result = manager
        .store_password("conn-id-test", "alice", "hunter2")
        .await;

    assert!(
        matches!(result, Err(CredentialError::Unavailable(_))),
        "store_password must return Unavailable when the store is unavailable; got: {result:?}"
    );

    // Verify no fallback credential files were created anywhere on disk.
    assert!(
        !std::path::Path::new("credentials.json").exists(),
        "CredentialManager must not create a credentials.json fallback file in the working directory"
    );
    // Check the XDG config path with the home directory properly expanded.
    if let Ok(home) = std::env::var("HOME") {
        let xdg_fallback = std::path::Path::new(&home)
            .join(".config")
            .join("tauterm")
            .join("credentials.json");
        assert!(
            !xdg_fallback.exists(),
            "CredentialManager must not create a plaintext fallback at {xdg_fallback:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// SEC-CRED-006: KnownHostsStore lookup returns Mismatch on key change
// ---------------------------------------------------------------------------

/// SEC-CRED-006: A `known_hosts` file entry for a host with a different key
/// than the one offered MUST cause `KnownHostsStore::lookup()` to return
/// `KnownHostLookup::Mismatch`, never `Trusted` (FS-SSH-011).
///
/// This test uses `KnownHostsStore` directly with a temp file, requiring no
/// live SSH server. It also verifies that `KnownHostLookup` has a `Mismatch`
/// variant as a compile-time structural regression guard.
#[test]
fn cred_reg_006_known_hosts_mismatch_returns_error() {
    use std::io::Write as _;
    use tau_term_lib::ssh::known_hosts::{KnownHostLookup, KnownHostsStore};

    // --- Compile-time structural guard ---
    // If KnownHostLookup::Mismatch is ever removed or renamed, this function
    // will fail to compile, catching the regression before any assertion runs.
    fn assert_mismatch_variant_exists(lookup: KnownHostLookup) -> bool {
        matches!(
            lookup,
            KnownHostLookup::Mismatch {
                stored: _,
                offered_key_type: _,
                offered_key_bytes: _,
            }
        )
    }

    // --- Behavioural test ---
    // Build a known_hosts file with a stored ed25519 key for "testhost.local".
    let stored_key: Vec<u8> = vec![0xAA; 32]; // 32-byte "stored" key
    let offered_key: Vec<u8> = vec![0xBB; 32]; // different 32-byte "offered" key

    use base64::Engine as _;
    let key_b64 = base64::engine::general_purpose::STANDARD.encode(&stored_key);
    let known_hosts_content = format!("testhost.local ssh-ed25519 {key_b64}\n");

    // Write the known_hosts content to a temp file.
    let mut tmp =
        tempfile::NamedTempFile::new().expect("failed to create temp file for known_hosts");
    tmp.write_all(known_hosts_content.as_bytes())
        .expect("failed to write known_hosts content");

    let store = KnownHostsStore::new(tmp.path().to_path_buf());

    // Lookup with a mismatched key must return Mismatch.
    let result = store
        .lookup("testhost.local", "ssh-ed25519", &offered_key)
        .expect("lookup must not return an I/O error");

    assert!(
        assert_mismatch_variant_exists(result),
        "lookup() must return KnownHostLookup::Mismatch when the stored key differs from the offered key"
    );

    // Lookup with the correct key must return Trusted.
    let trusted_result = store
        .lookup("testhost.local", "ssh-ed25519", &stored_key)
        .expect("lookup must not return an I/O error for the correct key");

    assert!(
        matches!(trusted_result, KnownHostLookup::Trusted(_)),
        "lookup() must return KnownHostLookup::Trusted when the stored key matches the offered key"
    );

    // Lookup for an unknown host must return Unknown.
    let unknown_result = store
        .lookup("unknown.host", "ssh-ed25519", &stored_key)
        .expect("lookup must not return an I/O error for an unknown host");

    assert!(
        matches!(unknown_result, KnownHostLookup::Unknown),
        "lookup() must return KnownHostLookup::Unknown for a host not in the known_hosts file"
    );
}
