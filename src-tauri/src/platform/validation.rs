// SPDX-License-Identifier: MPL-2.0

//! Path validation utilities for security-sensitive inputs.
//!
//! Validates SSH identity file paths and shell executable paths before passing
//! them to the OS. Rejects relative paths, traversal attempts, and paths outside
//! permitted directories (§6.1 of ARCHITECTURE.md, FINDING-004).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::error::TauTermError;

// ---------------------------------------------------------------------------
// SSH identity file validation
// ---------------------------------------------------------------------------

/// Validates an SSH identity file path.
///
/// Rules:
/// - Must be absolute.
/// - Must canonicalize (file must exist and be accessible).
/// - Canonical path must be within `~/.ssh/`.
///
/// Returns the canonicalized `PathBuf` on success.
pub fn validate_ssh_identity_path(raw: &str) -> Result<PathBuf, TauTermError> {
    let raw_path = Path::new(raw);

    if !raw_path.is_absolute() {
        return Err(TauTermError::new(
            "INVALID_SSH_IDENTITY_PATH",
            "SSH identity file path must be absolute.",
        ));
    }

    let canonical = raw_path.canonicalize().map_err(|e| {
        TauTermError::with_detail(
            "INVALID_SSH_IDENTITY_PATH",
            "SSH identity file does not exist or is not accessible.",
            e.to_string(),
        )
    })?;

    let ssh_dir = ssh_directory()?;

    if !canonical.starts_with(&ssh_dir) {
        return Err(TauTermError::with_detail(
            "INVALID_SSH_IDENTITY_PATH",
            "SSH identity file must be located within the user's ~/.ssh/ directory.",
            format!(
                "Path '{}' is outside '{}'",
                canonical.display(),
                ssh_dir.display()
            ),
        ));
    }

    Ok(canonical)
}

// ---------------------------------------------------------------------------
// Shell executable validation
// ---------------------------------------------------------------------------

/// Validates a shell executable path.
///
/// Rules:
/// - Must be absolute.
/// - Must canonicalize (file must exist and be accessible).
/// - Must have the executable bit set.
///
/// No whitelist is applied — the terminal emulator must support custom shells
/// (fish, nushell, zsh, etc.).
///
/// Returns the canonicalized `PathBuf` on success.
pub fn validate_shell_path(raw: &str) -> Result<PathBuf, TauTermError> {
    let raw_path = Path::new(raw);

    if !raw_path.is_absolute() {
        return Err(TauTermError::new(
            "INVALID_SHELL_PATH",
            "Shell executable path must be absolute.",
        ));
    }

    let canonical = raw_path.canonicalize().map_err(|e| {
        TauTermError::with_detail(
            "INVALID_SHELL_PATH",
            "Shell executable does not exist or is not accessible.",
            e.to_string(),
        )
    })?;

    let metadata = std::fs::metadata(&canonical).map_err(|e| {
        TauTermError::with_detail(
            "INVALID_SHELL_PATH",
            "Could not read shell executable metadata.",
            e.to_string(),
        )
    })?;

    let mode = metadata.permissions().mode();
    // Check owner, group, or other execute bits (0o111).
    if mode & 0o111 == 0 {
        return Err(TauTermError::with_detail(
            "INVALID_SHELL_PATH",
            "The specified shell is not executable.",
            format!("File mode: {:o}", mode),
        ));
    }

    Ok(canonical)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the canonical `~/.ssh` directory path.
fn ssh_directory() -> Result<PathBuf, TauTermError> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        TauTermError::new("INTERNAL_ERROR", "HOME environment variable is not set.")
    })?;
    Ok(home.join(".ssh"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    // -----------------------------------------------------------------------
    // SSH identity path tests
    // -----------------------------------------------------------------------

    /// Happy path: a real file in ~/.ssh/ that exists.
    #[test]
    fn ssh_identity_path_valid_file_in_ssh_dir() {
        // Use known file: ~/.ssh directory itself isn't a file — create a tmp
        // fixture inside ~/.ssh if it exists, otherwise skip.
        let home = match std::env::var_os("HOME").map(PathBuf::from) {
            Some(h) => h,
            None => return, // No HOME — skip.
        };
        let ssh_dir = home.join(".ssh");
        if !ssh_dir.exists() {
            return; // No ~/.ssh — skip rather than create.
        }

        // Write a temporary key fixture.
        let fixture = ssh_dir.join("tauterm_test_key.tmp");
        std::fs::write(&fixture, b"fake-key").expect("write fixture");

        let result = validate_ssh_identity_path(fixture.to_str().unwrap());
        let _ = std::fs::remove_file(&fixture);

        assert!(
            result.is_ok(),
            "Valid ~/.ssh/ path should be accepted. Error: {:?}",
            result.err()
        );
    }

    /// Relative path must be rejected.
    #[test]
    fn ssh_identity_path_rejects_relative_path() {
        let result = validate_ssh_identity_path("id_rsa");
        assert!(result.is_err(), "Relative path must be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_SSH_IDENTITY_PATH");
    }

    /// Path traversal via relative component must be rejected.
    #[test]
    fn ssh_identity_path_rejects_traversal_even_if_absolute() {
        // A path that looks absolute but tries to escape via traversal.
        // After canonicalize, /etc/shadow would be outside ~/.ssh/.
        let result = validate_ssh_identity_path("/../../../etc/shadow");
        // Either not-absolute check fires (relative check) or canonicalize fails
        // or the canonical path is outside ~/.ssh/ — all produce an error.
        assert!(result.is_err(), "Traversal path must be rejected");
    }

    /// Path outside ~/.ssh/ must be rejected.
    #[test]
    fn ssh_identity_path_rejects_path_outside_ssh_dir() {
        // Use /tmp which is guaranteed to exist and be outside ~/.ssh/.
        // We can't guarantee /tmp/key exists so we test the canonicalize step.
        // The key point: even if the path were valid, /tmp is not ~/.ssh/.
        // We need a file that exists but is outside ~/.ssh/.
        // /etc/hostname is a reliable existing file on Linux.
        let result = validate_ssh_identity_path("/etc/hostname");
        match result {
            Err(e) => assert_eq!(
                e.code, "INVALID_SSH_IDENTITY_PATH",
                "Should fail with INVALID_SSH_IDENTITY_PATH, got: {}",
                e.code
            ),
            Ok(_) => panic!("/etc/hostname is outside ~/.ssh/ and must be rejected"),
        }
    }

    /// Non-existent path must be rejected (canonicalize fails).
    #[test]
    fn ssh_identity_path_rejects_nonexistent_path() {
        let result = validate_ssh_identity_path("/home/nobody/.ssh/does_not_exist_tauterm");
        assert!(result.is_err(), "Non-existent path must be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_SSH_IDENTITY_PATH");
    }

    // -----------------------------------------------------------------------
    // Shell path tests
    // -----------------------------------------------------------------------

    /// Happy path: /bin/sh is always present and executable on Linux.
    #[test]
    fn shell_path_valid_executable() {
        // /bin/sh is guaranteed on Linux targets.
        let result = validate_shell_path("/bin/sh");
        assert!(
            result.is_ok(),
            "Valid shell /bin/sh should be accepted. Error: {:?}",
            result.err()
        );
    }

    /// Reject non-executable file.
    #[test]
    fn shell_path_rejects_non_executable_file() {
        use std::io::Write;

        // Create a temp file without executable bit.
        let tmp = std::env::temp_dir().join("tauterm_nonexec_test.tmp");
        let mut f = std::fs::File::create(&tmp).expect("create tmp");
        f.write_all(b"not a real shell").expect("write tmp");
        // Explicitly remove executable bit.
        let mut perms = f.metadata().expect("metadata").permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&tmp, perms).expect("set perms");
        drop(f);

        let result = validate_shell_path(tmp.to_str().unwrap());
        let _ = std::fs::remove_file(&tmp);
        assert!(result.is_err(), "Non-executable file must be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_SHELL_PATH");
    }

    /// Relative shell path must be rejected.
    #[test]
    fn shell_path_rejects_relative_path() {
        let result = validate_shell_path("bash");
        assert!(result.is_err(), "Relative shell path must be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_SHELL_PATH");
    }

    /// Non-existent shell path must be rejected.
    #[test]
    fn shell_path_rejects_nonexistent_path() {
        let result = validate_shell_path("/usr/local/bin/tauterm_fake_shell_9999");
        assert!(result.is_err(), "Non-existent shell must be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_SHELL_PATH");
    }
}
