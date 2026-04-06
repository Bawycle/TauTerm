// SPDX-License-Identifier: MPL-2.0

//! Deprecated SSH algorithm detection (FS-SSH-014).
//!
//! Detects use of `ssh-rsa` (SHA-1) and `ssh-dss` in the identity file
//! configuration and emits a `ssh-warning` event for each deprecated algorithm
//! found.  The check runs after key loading succeeds, so the path and key type
//! are known.
//!
//! ## russh limitation
//!
//! russh 0.60 does not expose the *negotiated* host-key algorithm from the
//! handshake in its `Handler` trait (there is no `algorithms_negotiated`
//! callback).  The only reliable point at which we know the algorithm is
//! **when the key is loaded** (for the client identity file) or when
//! `check_server_key` is called (for the server host key).
//!
//! Detection strategy:
//! - **Server host key**: `check_server_key` receives the server's `PublicKey`.
//!   Call `check_server_key_algorithm` from there to emit warnings for
//!   `ssh-rsa` (any hash — we flag SHA-1, which is what `ssh-rsa` implies when
//!   used without an explicit `rsa-sha2-*` negotiation) and `ssh-dss`.
//! - **Client identity**: checked after loading the key in `authenticate_pubkey`.
//!   Call `check_identity_key_algorithm` with the loaded key type string.

use tauri::AppHandle;

use crate::events::{SshWarningEvent, emit_ssh_warning};
use crate::session::ids::PaneId;

/// Known-deprecated algorithm identifiers.
const DEPRECATED_ALGORITHMS: &[(&str, &str)] = &[
    (
        "ssh-rsa",
        "ssh-rsa uses SHA-1, which is cryptographically weak. \
         Prefer ssh-ed25519 or ecdsa-sha2-nistp256.",
    ),
    (
        "ssh-dss",
        "ssh-dss (DSA) uses a 1024-bit key and SHA-1, both deprecated. \
         Prefer ssh-ed25519 or ecdsa-sha2-nistp256.",
    ),
];

/// Check a server host-key algorithm string and emit a warning if deprecated.
///
/// Called from `TauTermSshHandler::check_server_key` after reading
/// `server_public_key.algorithm().as_str()`.
///
/// This is a best-effort, non-blocking check — errors are ignored.
pub fn check_server_key_algorithm(app: &AppHandle, pane_id: &PaneId, algorithm: &str) {
    if let Some((_, reason)) = DEPRECATED_ALGORITHMS
        .iter()
        .find(|(alg, _)| *alg == algorithm)
    {
        emit_ssh_warning(
            app,
            SshWarningEvent {
                pane_id: pane_id.clone(),
                algorithm: algorithm.to_string(),
                reason: reason.to_string(),
            },
        );
    }
}

/// Check a client identity key algorithm string and emit a warning if deprecated.
///
/// Called after loading the private key in `authenticate_pubkey` (auth.rs).
pub fn check_identity_key_algorithm(app: &AppHandle, pane_id: &PaneId, algorithm: &str) {
    check_server_key_algorithm(app, pane_id, algorithm);
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // FS-SSH-014 — deprecated algorithm detection
    // -----------------------------------------------------------------------

    #[test]
    fn ssh_rsa_is_deprecated() {
        let hit = DEPRECATED_ALGORITHMS
            .iter()
            .any(|(alg, _)| *alg == "ssh-rsa");
        assert!(
            hit,
            "ssh-rsa must be in the deprecated algorithm list (FS-SSH-014)"
        );
    }

    #[test]
    fn ssh_dss_is_deprecated() {
        let hit = DEPRECATED_ALGORITHMS
            .iter()
            .any(|(alg, _)| *alg == "ssh-dss");
        assert!(
            hit,
            "ssh-dss must be in the deprecated algorithm list (FS-SSH-014)"
        );
    }

    #[test]
    fn ssh_ed25519_is_not_deprecated() {
        let hit = DEPRECATED_ALGORITHMS
            .iter()
            .any(|(alg, _)| *alg == "ssh-ed25519");
        assert!(
            !hit,
            "ssh-ed25519 must NOT be in the deprecated algorithm list"
        );
    }

    #[test]
    fn ecdsa_nistp256_is_not_deprecated() {
        let hit = DEPRECATED_ALGORITHMS
            .iter()
            .any(|(alg, _)| *alg == "ecdsa-sha2-nistp256");
        assert!(
            !hit,
            "ecdsa-sha2-nistp256 must NOT be in the deprecated algorithm list"
        );
    }

    #[test]
    fn deprecated_algorithms_have_non_empty_reasons() {
        for (alg, reason) in DEPRECATED_ALGORITHMS {
            assert!(
                !reason.is_empty(),
                "Deprecated algorithm '{alg}' must have a non-empty reason"
            );
        }
    }
}
