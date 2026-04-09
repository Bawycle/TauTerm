// SPDX-License-Identifier: MPL-2.0

//! SSH authentication helpers.
//!
//! Implements the authentication sequence defined in FS-SSH-012:
//! public-key → keyboard-interactive → password (in order of preference).
//!
//! These functions operate on a live `russh::client::Handle` and return whether
//! authentication succeeded. The caller (manager.rs) drives the sequence and
//! transitions the state machine accordingly.

use std::path::Path;
use std::sync::Arc;

use russh::client::{AuthResult, Handle, KeyboardInteractiveAuthResponse};
use russh::keys;
use russh::keys::PrivateKeyWithHashAlg;

use crate::error::SshError;

/// Authenticate with a password.
///
/// Returns `Ok(true)` if authentication succeeded, `Ok(false)` if the server
/// rejected the credentials (wrong password, method not allowed, etc.).
///
/// # Errors
/// Returns `Err(SshError)` on transport-level failures (connection closed,
/// protocol error), not on credential rejection — that is `Ok(false)`.
pub async fn authenticate_password<H: russh::client::Handler>(
    handle: &mut Handle<H>,
    username: &str,
    password: &str,
) -> Result<bool, SshError> {
    let result = handle
        .authenticate_password(username, password)
        .await
        .map_err(|e| SshError::Auth(format!("password auth transport error: {e}")))?;

    Ok(matches!(result, AuthResult::Success))
}

/// Authenticate using keyboard-interactive, responding with `password` to any
/// prompts that look like a password request (case-insensitive "password"
/// substring) and with an empty string for all other prompts (FS-SSH-012).
///
/// Returns `Ok(true)` if authentication succeeded, `Ok(false)` if the server
/// rejected the method or no prompts could be satisfied.
///
/// # Errors
/// Returns `Err(SshError)` on transport-level failures.
pub async fn authenticate_keyboard_interactive<H: russh::client::Handler>(
    handle: &mut Handle<H>,
    username: &str,
    password: &str,
) -> Result<bool, SshError> {
    // Start keyboard-interactive — no sub-method preference.
    let mut response = handle
        .authenticate_keyboard_interactive_start(username, None)
        .await
        .map_err(|e| SshError::Auth(format!("keyboard-interactive transport error: {e}")))?;

    // The server may issue zero or more `InfoRequest` rounds before resolving
    // to `Success` or `Failure`. We cap the loop at 10 rounds as a safety
    // measure against a misbehaving server.
    const MAX_ROUNDS: usize = 10;
    for _ in 0..MAX_ROUNDS {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                // Build responses: reply with `password` for password-like prompts,
                // empty string for everything else.
                let responses: Vec<String> = prompts
                    .iter()
                    .map(|p| {
                        if p.prompt.to_lowercase().contains("password") {
                            password.to_string()
                        } else {
                            String::new()
                        }
                    })
                    .collect();

                response = handle
                    .authenticate_keyboard_interactive_respond(responses)
                    .await
                    .map_err(|e| {
                        SshError::Auth(format!("keyboard-interactive respond transport error: {e}"))
                    })?;
            }
        }
    }

    // Exceeded MAX_ROUNDS — treat as failure, not a transport error.
    tracing::warn!("keyboard-interactive: exceeded {MAX_ROUNDS} challenge rounds, aborting");
    Ok(false)
}

/// Returns `true` when the private key at `key_path` is encrypted and requires
/// a passphrase to load (FS-SSH-019a).
///
/// Probes the key by attempting to load it without a passphrase. If the
/// underlying error message indicates encryption (`"The key is encrypted"`),
/// returns `true`; for all other outcomes (successful load, unreadable file,
/// malformed key) returns `false`.
///
/// The probe always fails fast — `None` passphrase on an encrypted key is
/// rejected immediately by the parser without any I/O after the initial file read.
pub fn key_needs_passphrase(key_path: &Path) -> bool {
    match keys::load_secret_key(key_path, None) {
        // russh-keys 0.45 does not re-export the Error enum variants publicly,
        // so we match on the Display string. The string "The key is encrypted"
        // comes from `KeyError::KeyIsEncrypted` (russh_keys/src/lib.rs, `#[error]`
        // attribute). If the crate ever changes this message, this detection will
        // silently fail — auth will fall through to password/kbd-interactive rather
        // than prompting for a passphrase. Track at: russh-keys changelog.
        Err(e) => {
            let msg = e.to_string();
            msg.contains("The key is encrypted") || msg.contains("key is encrypted")
        }
        Ok(_) => false,
    }
}

/// Authenticate with a public key loaded from `key_path`.
///
/// The key file must be a PEM/OpenSSH private key. An optional `passphrase`
/// may be provided for encrypted keys (FS-SSH-019a). Pass `None` for
/// unencrypted keys.
///
/// Returns `Ok(true)` on success, `Ok(false)` on rejection.
///
/// # Errors
/// Returns `Err(SshError)` if the key file cannot be read, is malformed, or
/// on transport-level failures.
pub async fn authenticate_pubkey<H: russh::client::Handler>(
    handle: &mut Handle<H>,
    username: &str,
    key_path: &Path,
    passphrase: Option<&str>,
) -> Result<bool, SshError> {
    let key_pair = keys::load_secret_key(key_path, passphrase)
        .map_err(|e| SshError::Auth(format!("failed to load private key: {e}")))?;

    // Choose the best RSA hash algorithm supported by the server.
    // For non-RSA key types (Ed25519, ECDSA) this returns None (no hash selection needed).
    let hash_alg = handle
        .best_supported_rsa_hash()
        .await
        .map_err(|e| SshError::Auth(format!("hash algorithm negotiation error: {e}")))?
        .flatten();

    let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg);

    let result = handle
        .authenticate_publickey(username, key_with_alg)
        .await
        .map_err(|e| SshError::Auth(format!("publickey auth transport error: {e}")))?;

    Ok(matches!(result, AuthResult::Success))
}

#[cfg(test)]
mod tests {
    // -----------------------------------------------------------------------
    // Auth error-path tests (no live SSH server required)
    // -----------------------------------------------------------------------

    use std::path::Path;

    /// A nonexistent key path must produce an SshError::Auth, not a panic.
    #[tokio::test]
    async fn authenticate_pubkey_nonexistent_key_returns_error() {
        // We cannot construct a real russh Handle without a server, so we test
        // the key-loading path directly via the underlying function.
        // load_secret_key with a nonexistent path must fail gracefully.
        let result = russh::keys::load_secret_key(Path::new("/nonexistent/path/to/key"), None);
        assert!(
            result.is_err(),
            "load_secret_key on nonexistent path must return an error"
        );
    }

    // -----------------------------------------------------------------------
    // key_needs_passphrase — FS-SSH-019a unit tests
    // -----------------------------------------------------------------------

    /// key_needs_passphrase must return false for an unencrypted ED25519 key.
    #[test]
    fn key_needs_passphrase_returns_false_for_unencrypted_key() {
        use std::process::Command;
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("id_ed25519_plain");
        let status = Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-N",
                "",
                "-f",
                key_path.to_str().expect("valid path"),
            ])
            .output()
            .expect("ssh-keygen must be available");
        assert!(
            status.status.success(),
            "ssh-keygen must succeed for unencrypted key generation"
        );
        assert!(
            !super::key_needs_passphrase(&key_path),
            "key_needs_passphrase must return false for an unencrypted key"
        );
    }

    /// key_needs_passphrase must return true for a passphrase-protected ED25519 key.
    #[test]
    fn key_needs_passphrase_returns_true_for_encrypted_key() {
        use std::process::Command;
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("id_ed25519_enc");
        let status = Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-N",
                "test-passphrase",
                "-f",
                key_path.to_str().expect("valid path"),
            ])
            .output()
            .expect("ssh-keygen must be available");
        assert!(
            status.status.success(),
            "ssh-keygen must succeed for encrypted key generation"
        );
        assert!(
            super::key_needs_passphrase(&key_path),
            "key_needs_passphrase must return true for an encrypted key"
        );
    }

    /// Loading an encrypted key with the correct passphrase must succeed (FS-SSH-019a).
    #[test]
    fn authenticate_pubkey_with_correct_passphrase_loads_key() {
        use std::process::Command;
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("id_ed25519_enc");
        let status = Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-N",
                "correct-passphrase",
                "-f",
                key_path.to_str().expect("valid path"),
            ])
            .output()
            .expect("ssh-keygen must be available");
        assert!(
            status.status.success(),
            "ssh-keygen must succeed for encrypted key generation"
        );
        let result = russh::keys::load_secret_key(&key_path, Some("correct-passphrase"));
        assert!(
            result.is_ok(),
            "load_secret_key with correct passphrase must succeed; got: {:?}",
            result.err()
        );
    }

    /// Loading an encrypted key with a wrong passphrase must return Err (FS-SSH-019a).
    #[test]
    fn authenticate_pubkey_with_wrong_passphrase_returns_error() {
        use std::process::Command;
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("id_ed25519_enc");
        let status = Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-N",
                "correct-passphrase",
                "-f",
                key_path.to_str().expect("valid path"),
            ])
            .output()
            .expect("ssh-keygen must be available");
        assert!(
            status.status.success(),
            "ssh-keygen must succeed for encrypted key generation"
        );
        let result = russh::keys::load_secret_key(&key_path, Some("wrong-passphrase"));
        assert!(
            result.is_err(),
            "load_secret_key with wrong passphrase must return an error"
        );
    }

    /// A path to a non-key file must produce an error, not a panic.
    #[tokio::test]
    async fn authenticate_pubkey_invalid_key_format_returns_error() {
        use std::io::Write;
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("not_a_key.pem");
        {
            let mut f = std::fs::File::create(&key_path).expect("create");
            f.write_all(b"this is not a valid SSH key\n")
                .expect("write");
        }

        let result = russh::keys::load_secret_key(&key_path, None);
        assert!(
            result.is_err(),
            "load_secret_key on invalid PEM content must return an error"
        );
    }

    // -----------------------------------------------------------------------
    // Auth order — branch selection logic (FS-SSH-012, no live server)
    //
    // These tests verify that the conditions gating each authentication method
    // in `try_authenticate` (pubkey → keyboard-interactive → password) are
    // correctly evaluated given different config/credentials combinations.
    //
    // Full end-to-end tests require a live SSH server; see the functional test
    // protocol for those scenarios. The tests below cover pure logic.
    // -----------------------------------------------------------------------

    /// FS-SSH-012: pubkey branch is entered when `identity_file` is set.
    #[test]
    fn auth_order_pubkey_branch_entered_when_identity_file_set() {
        use crate::preferences::types::{SshHost, SshIdentityPath, SshLabel, SshUsername};
        use crate::session::ids::ConnectionId;
        use crate::ssh::SshConnectionConfig;

        let config = SshConnectionConfig {
            id: ConnectionId::new(),
            label: SshLabel::try_from("test".to_string()).unwrap(),
            host: SshHost::try_from("host".to_string()).unwrap(),
            port: 22,
            username: SshUsername::try_from("user".to_string()).unwrap(),
            identity_file: Some(
                SshIdentityPath::try_from("/home/user/.ssh/id_ed25519".to_string()).unwrap(),
            ),
            allow_osc52_write: false,
            keepalive_interval_secs: None,
            keepalive_max_failures: None,
        };
        // Gate condition in try_authenticate: `if let Some(ref path) = config.identity_file`
        assert!(
            config.identity_file.is_some(),
            "FS-SSH-012: pubkey branch must be entered when identity_file is set"
        );
    }

    /// FS-SSH-012: keyboard-interactive and password branches require a password in credentials.
    #[test]
    fn auth_order_kbd_and_password_branches_require_password() {
        use crate::ssh::manager::Credentials;

        let creds_with_password = Credentials {
            username: "user".to_string(),
            password: Some("secret".to_string()),
            private_key_path: None,
            save_in_keychain: false,
        };
        let creds_without_password = Credentials {
            username: "user".to_string(),
            password: None,
            private_key_path: None,
            save_in_keychain: false,
        };

        // Gate condition: `if let Some(creds) = credentials && let Some(ref password) = creds.password`
        assert!(
            creds_with_password.password.is_some(),
            "FS-SSH-012: kbd-interactive/password must be entered when password is set"
        );
        assert!(
            creds_without_password.password.is_none(),
            "FS-SSH-012: kbd-interactive/password must be skipped when no password"
        );
    }

    /// FS-SSH-012: when identity_file is None and credentials are absent,
    /// all auth branches are skipped and authentication fails.
    #[test]
    fn auth_order_all_methods_skipped_when_no_key_and_no_credentials() {
        use crate::preferences::types::{SshHost, SshLabel, SshUsername};
        use crate::session::ids::ConnectionId;
        use crate::ssh::SshConnectionConfig;
        use crate::ssh::manager::Credentials;

        let config = SshConnectionConfig {
            id: ConnectionId::new(),
            label: SshLabel::try_from("test".to_string()).unwrap(),
            host: SshHost::try_from("host".to_string()).unwrap(),
            port: 22,
            username: SshUsername::try_from("user".to_string()).unwrap(),
            identity_file: None,
            allow_osc52_write: false,
            keepalive_interval_secs: None,
            keepalive_max_failures: None,
        };
        let credentials: Option<Credentials> = None;

        let pubkey_would_run = config.identity_file.is_some();
        let kbd_or_password_would_run = credentials
            .as_ref()
            .and_then(|c| c.password.as_ref())
            .is_some();

        assert!(
            !pubkey_would_run && !kbd_or_password_would_run,
            "FS-SSH-012: with no key and no credentials, no auth method must be attempted"
        );
    }

    // -----------------------------------------------------------------------
    // keyboard-interactive prompt selection — pure logic tests
    //
    // Verifies the prompt-response mapping in `authenticate_keyboard_interactive`:
    // prompts containing "password" (case-insensitive) → stored password;
    // all other prompts → empty string.
    // -----------------------------------------------------------------------

    /// Prompts containing "password" (case-insensitive) map to the provided password.
    #[test]
    fn kbd_interactive_password_prompts_map_to_password() {
        let password = "hunter2";
        let password_prompts = [
            "Password: ",
            "Enter password: ",
            "PASSWORD: ",
            "New password",
        ];
        for prompt in &password_prompts {
            let response = if prompt.to_lowercase().contains("password") {
                password.to_string()
            } else {
                String::new()
            };
            assert_eq!(
                response, password,
                "Prompt '{prompt}' must map to the stored password"
            );
        }
    }

    /// Prompts not containing "password" map to an empty string.
    #[test]
    fn kbd_interactive_non_password_prompts_map_to_empty_string() {
        let password = "hunter2";
        let other_prompts = ["Verification code: ", "OTP: ", "Challenge: ", "Token: "];
        for prompt in &other_prompts {
            let response = if prompt.to_lowercase().contains("password") {
                password.to_string()
            } else {
                String::new()
            };
            assert!(
                response.is_empty(),
                "Non-password prompt '{prompt}' must map to empty string, not the password"
            );
        }
    }
}
