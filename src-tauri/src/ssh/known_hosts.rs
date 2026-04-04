// SPDX-License-Identifier: MPL-2.0

//! TauTerm known-hosts file management.
//!
//! Maintains `~/.config/tauterm/known_hosts` in OpenSSH-compatible format.
//! Implements TOFU (Trust On First Use) host key verification (FS-SSH-011).
//!
//! ## File format
//!
//! One entry per line:
//! ```text
//! hostname key-type base64-key [comment]
//! ```
//!
//! Lines beginning with `#` are comments. Empty lines are skipped.
//! Hashed hostname entries (`|1|...`) are silently skipped with a count
//! (they cannot be imported because the plaintext hostname is not recoverable).
//!
//! ## Security notes (ADR-0007)
//! - File is written with permissions 0600.
//! - TauTerm does NOT read from `~/.ssh/known_hosts` automatically.
//! - The Preferences UI offers an explicit "Import from OpenSSH" action.
//! - A mismatch between stored and offered key always returns `HostKeyMismatch`
//!   — there is no "override" path in this module.

use std::fs;
use std::io::{self, BufRead, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

/// An entry in the known-hosts file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownHostEntry {
    /// Hostname or IP address as stored in the file.
    pub hostname: String,
    /// SSH key type identifier, e.g. `ssh-ed25519`, `ssh-rsa`, `ecdsa-sha2-nistp256`.
    pub key_type: String,
    /// Raw public key bytes (decoded from base64).
    pub key_bytes: Vec<u8>,
}

/// Result of a known-hosts lookup.
#[derive(Debug)]
pub enum KnownHostLookup {
    /// Host was not found — first connection (TOFU: prompt the user).
    Unknown,
    /// Host was found and the key matches — trusted.
    Trusted(KnownHostEntry),
    /// Host was found but the key does not match — potential MITM.
    Mismatch {
        stored: KnownHostEntry,
        offered_key_type: String,
        offered_key_bytes: Vec<u8>,
    },
}

/// TauTerm known-hosts store.
pub struct KnownHostsStore {
    path: PathBuf,
}

impl KnownHostsStore {
    /// Create a store pointing to the given file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Default path: `~/.config/tauterm/known_hosts`.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("tauterm").join("known_hosts"))
    }

    /// Load all entries from the known-hosts file.
    ///
    /// Returns a tuple `(entries, skipped_hashed_count)`.
    /// Lines with hashed hostnames (`|1|...`) are silently skipped.
    pub fn load(&self) -> io::Result<(Vec<KnownHostEntry>, usize)> {
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok((Vec::new(), 0));
            }
            Err(e) => return Err(e),
        };

        let mut entries = Vec::new();
        let mut skipped_hashed = 0usize;

        for (line_num, line_result) in io::BufReader::new(file).lines().enumerate() {
            let line = line_result?;
            let trimmed = line.trim();

            // Skip empty lines and comments.
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Skip hashed hostname entries (|1|salt|hash format).
            if trimmed.starts_with('|') {
                skipped_hashed += 1;
                continue;
            }

            // Parse: hostname key-type base64-key [optional-comment]
            let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
            if parts.len() < 3 {
                tracing::warn!(
                    "known_hosts line {}: malformed entry (expected 3+ fields), skipping",
                    line_num + 1
                );
                continue;
            }

            let hostname = parts[0].to_string();
            let key_type = parts[1].to_string();
            // The third field may be "base64key [comment]" — take only the key part.
            let key_b64 = parts[2].split_whitespace().next().unwrap_or("");

            let key_bytes = match BASE64.decode(key_b64) {
                Ok(b) => b,
                Err(_) => {
                    tracing::warn!(
                        "known_hosts line {}: base64 decode failed, skipping",
                        line_num + 1
                    );
                    continue;
                }
            };

            entries.push(KnownHostEntry {
                hostname,
                key_type,
                key_bytes,
            });
        }

        Ok((entries, skipped_hashed))
    }

    /// Look up a host by name.
    ///
    /// Returns `Unknown` if the host has never been seen.
    /// Returns `Trusted` if the stored key matches `offered_key_bytes`.
    /// Returns `Mismatch` if the host is known but the key differs.
    pub fn lookup(
        &self,
        hostname: &str,
        offered_key_type: &str,
        offered_key_bytes: &[u8],
    ) -> io::Result<KnownHostLookup> {
        let (entries, _) = self.load()?;

        let stored = entries.into_iter().find(|e| e.hostname == hostname);

        match stored {
            None => Ok(KnownHostLookup::Unknown),
            Some(entry) if entry.key_bytes == offered_key_bytes => {
                Ok(KnownHostLookup::Trusted(entry))
            }
            Some(entry) => Ok(KnownHostLookup::Mismatch {
                stored: entry,
                offered_key_type: offered_key_type.to_string(),
                offered_key_bytes: offered_key_bytes.to_vec(),
            }),
        }
    }

    /// Add a new host entry to the known-hosts file.
    ///
    /// The file is created with permissions 0600 if it does not exist.
    /// The parent directory is created with 0700 if needed.
    pub fn add_entry(
        &self,
        hostname: &str,
        key_type: &str,
        key_bytes: &[u8],
    ) -> io::Result<()> {
        // Ensure the parent directory exists with restrictive permissions.
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
            // Restrict directory to owner only.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).ok();
            }
        }

        let key_b64 = BASE64.encode(key_bytes);
        let line = format!("{hostname} {key_type} {key_b64}\n");

        // Append to file with 0600 permissions (created if absent).
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(&self.path)?;

        file.write_all(line.as_bytes())?;
        Ok(())
    }

    /// Import entries from a source file (e.g., `~/.ssh/known_hosts`).
    ///
    /// Returns `(imported_count, skipped_hashed_count)`.
    /// Hashed entries are silently skipped (ADR-0007).
    pub fn import_from(&self, source_path: &std::path::Path) -> io::Result<(usize, usize)> {
        let source_store = KnownHostsStore::new(source_path.to_path_buf());
        let (source_entries, skipped_hashed) = source_store.load()?;

        // Load existing entries to avoid duplicates.
        let (existing, _) = self.load()?;
        let existing_hosts: std::collections::HashSet<&str> =
            existing.iter().map(|e| e.hostname.as_str()).collect();

        let mut imported = 0usize;
        for entry in &source_entries {
            if !existing_hosts.contains(entry.hostname.as_str()) {
                self.add_entry(&entry.hostname, &entry.key_type, &entry.key_bytes)?;
                imported += 1;
            }
        }

        Ok((imported, skipped_hashed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_known_hosts(content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("known_hosts");
        let mut f = fs::File::create(&path).expect("create");
        f.write_all(content.as_bytes()).expect("write");
        (dir, path)
    }

    // -----------------------------------------------------------------------
    // TEST-SSH-UNIT-002 — known_hosts parsing and TOFU
    // -----------------------------------------------------------------------

    /// Parse an ED25519 known_hosts entry.
    #[test]
    fn known_hosts_parse_ed25519_entry() {
        let key_bytes = vec![0u8, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57];
        let key_b64 = BASE64.encode(&key_bytes);
        let content = format!("example.com ssh-ed25519 {key_b64}\n");

        let (_dir, path) = write_temp_known_hosts(&content);
        let store = KnownHostsStore::new(path);

        let (entries, skipped) = store.load().expect("load");
        assert_eq!(entries.len(), 1, "Should parse exactly one entry");
        assert_eq!(skipped, 0, "No hashed entries expected");
        assert_eq!(entries[0].hostname, "example.com");
        assert_eq!(entries[0].key_type, "ssh-ed25519");
        assert_eq!(entries[0].key_bytes, key_bytes);
    }

    /// Lookup returns Unknown for a host not in the file.
    #[test]
    fn known_hosts_lookup_unknown_host() {
        let (_dir, path) = write_temp_known_hosts("");
        let store = KnownHostsStore::new(path);

        let result = store
            .lookup("new.host.example.com", "ssh-ed25519", &[1, 2, 3])
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Unknown),
            "Unknown host must return KnownHostLookup::Unknown (TOFU first connection)"
        );
    }

    /// Lookup returns Trusted when key matches.
    #[test]
    fn known_hosts_lookup_trusted_matching_key() {
        let key_bytes = vec![1u8, 2, 3, 4, 5];
        let key_b64 = BASE64.encode(&key_bytes);
        let content = format!("trusted.host ssh-ed25519 {key_b64}\n");

        let (_dir, path) = write_temp_known_hosts(&content);
        let store = KnownHostsStore::new(path);

        let result = store
            .lookup("trusted.host", "ssh-ed25519", &key_bytes)
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Trusted(_)),
            "Matching key must return KnownHostLookup::Trusted"
        );
    }

    /// Lookup returns Mismatch when key differs — potential MITM (SEC-CRED-006).
    #[test]
    fn known_hosts_lookup_mismatch_different_key() {
        let stored_key = vec![1u8, 2, 3, 4, 5];
        let offered_key = vec![9u8, 8, 7, 6, 5]; // different
        let key_b64 = BASE64.encode(&stored_key);
        let content = format!("mitm.host ssh-ed25519 {key_b64}\n");

        let (_dir, path) = write_temp_known_hosts(&content);
        let store = KnownHostsStore::new(path);

        let result = store
            .lookup("mitm.host", "ssh-ed25519", &offered_key)
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Mismatch { .. }),
            "Key mismatch must return KnownHostLookup::Mismatch (SEC-CRED-006 / MITM detection)"
        );
    }

    /// Add a new entry, then look it up.
    #[test]
    fn known_hosts_add_and_lookup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("known_hosts");
        let store = KnownHostsStore::new(path);

        let key_bytes = vec![42u8; 32];
        store
            .add_entry("new.host.example.com", "ssh-ed25519", &key_bytes)
            .expect("add");

        let result = store
            .lookup("new.host.example.com", "ssh-ed25519", &key_bytes)
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Trusted(_)),
            "Added entry must be found as Trusted on lookup"
        );
    }

    /// Hashed hostname entries are skipped with a count.
    #[test]
    fn known_hosts_skips_hashed_entries() {
        let content = concat!(
            "|1|abc123|xyz456 ssh-ed25519 AAAA\n",
            "|1|def789|uvw012 ssh-rsa BBBB\n",
            "plainhost.example ssh-ed25519 CCCC\n"
        );

        let (_dir, path) = write_temp_known_hosts(content);
        let store = KnownHostsStore::new(path);
        let (entries, skipped) = store.load().expect("load");

        assert_eq!(skipped, 2, "Two hashed entries should be counted");
        // The plain host entry is malformed (CCCC is not valid base64 for our purposes)
        // but since we're testing skipping hashed entries, we only assert the skip count.
        let _ = entries; // parsed entries may be empty due to base64 decode failure on CCCC
    }

    /// Comments and empty lines are skipped.
    #[test]
    fn known_hosts_skips_comments_and_empty_lines() {
        let key_bytes = vec![1u8; 4];
        let key_b64 = BASE64.encode(&key_bytes);
        let content = format!(
            "# This is a comment\n\nexample.org ssh-ed25519 {key_b64}\n# Another comment\n"
        );

        let (_dir, path) = write_temp_known_hosts(&content);
        let store = KnownHostsStore::new(path);

        let (entries, skipped) = store.load().expect("load");
        assert_eq!(entries.len(), 1, "Only the non-comment entry should be parsed");
        assert_eq!(skipped, 0);
    }

    /// Non-existent file returns empty entries without error.
    #[test]
    fn known_hosts_nonexistent_file_returns_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("does_not_exist");
        let store = KnownHostsStore::new(path);

        let (entries, skipped) = store.load().expect("load of nonexistent must not error");
        assert!(entries.is_empty());
        assert_eq!(skipped, 0);
    }
}
