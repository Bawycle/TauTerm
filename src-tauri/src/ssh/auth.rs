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

use russh::client::{AuthResult, Handle};
use russh::keys::PrivateKeyWithHashAlg;
use russh::keys;

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

/// Authenticate with a public key loaded from `key_path`.
///
/// The key file must be a PEM/OpenSSH private key. Passphrase-protected keys
/// are not supported in v1 — `russh_keys::load_secret_key` is called with
/// `None` passphrase. If the file requires a passphrase, this returns an error.
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
) -> Result<bool, SshError> {
    // Load the private key — no passphrase support in v1.
    let key_pair = keys::load_secret_key(key_path, None)
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
        let result = russh::keys::load_secret_key(
            Path::new("/nonexistent/path/to/key"),
            None,
        );
        assert!(
            result.is_err(),
            "load_secret_key on nonexistent path must return an error"
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
            f.write_all(b"this is not a valid SSH key\n").expect("write");
        }

        let result = russh::keys::load_secret_key(&key_path, None);
        assert!(
            result.is_err(),
            "load_secret_key on invalid PEM content must return an error"
        );
    }
}
