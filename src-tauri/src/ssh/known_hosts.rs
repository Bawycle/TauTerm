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
//! ## Secondary source: `~/.ssh/known_hosts`
//!
//! During TOFU lookup, `KnownHostsStore::lookup_with_system_fallback` also
//! consults `~/.ssh/known_hosts` as a read-only secondary source (FS-SSH-011).
//! A match there is returned as `Trusted` without writing to TauTerm's store.
//! New entries are always written only to `~/.config/tauterm/known_hosts`.
//!
//! ## Security notes (ADR-0007)
//! - File is written with permissions 0600.
//! - `~/.ssh/known_hosts` is read-only — TauTerm never writes to it.
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

    /// Path to the system/user OpenSSH known_hosts: `~/.ssh/known_hosts`.
    ///
    /// Used as a read-only secondary trust source (FS-SSH-011).
    /// Returns `None` if the home directory cannot be determined.
    pub fn system_known_hosts_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".ssh").join("known_hosts"))
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

    /// Look up a host by name and key type.
    ///
    /// Returns `Unknown` if no entry matches `(hostname, offered_key_type)`.
    /// Returns `Trusted` if the stored key bytes match `offered_key_bytes`.
    /// Returns `Mismatch` if the host+key_type is known but the key bytes differ.
    ///
    /// Filtering by `(hostname, key_type)` ensures that entries for other
    /// algorithms (e.g. an `ssh-rsa` key alongside an `ssh-ed25519` key for
    /// the same host) are not incorrectly treated as mismatches or purged.
    pub fn lookup(
        &self,
        hostname: &str,
        offered_key_type: &str,
        offered_key_bytes: &[u8],
    ) -> io::Result<KnownHostLookup> {
        let (entries, _) = self.load()?;

        let stored = entries
            .into_iter()
            .find(|e| e.hostname == hostname && e.key_type == offered_key_type);

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

    /// Look up a host, falling back to `~/.ssh/known_hosts` on `Unknown` (FS-SSH-011).
    ///
    /// Resolution order:
    /// 1. TauTerm store (`~/.config/tauterm/known_hosts`): authoritative for Trusted / Mismatch.
    /// 2. If `Unknown` in TauTerm store: check the system OpenSSH file.
    ///    - Found and matching → return `Trusted` (read-only trust, no write).
    ///    - Found and mismatching → return `Mismatch` (key conflict still reported).
    ///    - Not found → return `Unknown`.
    ///
    /// The system file path can be overridden via `system_path` (used in tests).
    /// Pass `None` to use `system_known_hosts_path()`.
    ///
    /// I/O errors on the system file are logged and ignored (best-effort).
    pub fn lookup_with_system_fallback(
        &self,
        hostname: &str,
        offered_key_type: &str,
        offered_key_bytes: &[u8],
        system_path: Option<&std::path::Path>,
    ) -> io::Result<KnownHostLookup> {
        // Primary lookup in TauTerm's own store.
        let primary = self.lookup(hostname, offered_key_type, offered_key_bytes)?;

        if !matches!(primary, KnownHostLookup::Unknown) {
            return Ok(primary);
        }

        // Primary is Unknown — consult the system OpenSSH file as read-only fallback.
        let sys_path = match system_path
            .map(|p| p.to_path_buf())
            .or_else(Self::system_known_hosts_path)
        {
            Some(p) => p,
            None => return Ok(KnownHostLookup::Unknown),
        };

        let sys_store = KnownHostsStore::new(sys_path);
        match sys_store.lookup(hostname, offered_key_type, offered_key_bytes) {
            Ok(result) => Ok(result),
            Err(e) => {
                // System file I/O errors are non-fatal — log and fall through to Unknown.
                tracing::warn!("system known_hosts read error (ignored): {e}");
                Ok(KnownHostLookup::Unknown)
            }
        }
    }

    /// Add a new host entry to the known-hosts file.
    ///
    /// The file is created with permissions 0600 if it does not exist.
    /// The parent directory is created with 0700 if needed.
    pub fn add_entry(&self, hostname: &str, key_type: &str, key_bytes: &[u8]) -> io::Result<()> {
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

    /// Remove the entry for a given `(hostname, key_type)` from the known-hosts file.
    ///
    /// Used when accepting a new key for a host whose key has changed (Mismatch case):
    /// the old entry is removed before the new one is added.
    ///
    /// Only the entry matching both hostname AND key_type is removed. Entries for
    /// the same host with a different algorithm are preserved (e.g. an `ssh-rsa`
    /// key is not purged when an `ssh-ed25519` key is replaced).
    pub fn remove_entries_for_host(&self, hostname: &str, key_type: &str) -> io::Result<()> {
        let (entries, _) = self.load()?;
        let remaining: Vec<_> = entries
            .into_iter()
            .filter(|e| !(e.hostname == hostname && e.key_type == key_type))
            .collect();

        // Re-write the file with the filtered entries.
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&self.path)?;

        for entry in &remaining {
            let key_b64 = BASE64.encode(&entry.key_bytes);
            writeln!(file, "{} {} {}", entry.hostname, entry.key_type, key_b64)?;
        }

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
        let key_bytes = vec![
            0u8, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57,
        ];
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

    // -----------------------------------------------------------------------
    // Tests for lookup_with_system_fallback (FS-SSH-011 secondary source)
    // -----------------------------------------------------------------------

    /// Host present in system known_hosts (plain format) → Trusted, even if absent
    /// from the TauTerm store.
    #[test]
    fn system_fallback_plain_host_trusted() {
        let key_bytes = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let key_b64 = BASE64.encode(&key_bytes);
        let sys_content = format!("remote.server ssh-ed25519 {key_b64}\n");

        // TauTerm store is empty.
        let tauterm_dir = tempfile::tempdir().expect("tempdir");
        let tauterm_path = tauterm_dir.path().join("known_hosts");
        let store = KnownHostsStore::new(tauterm_path);

        // System file has the entry.
        let sys_dir = tempfile::tempdir().expect("tempdir");
        let sys_path = sys_dir.path().join("known_hosts");
        fs::write(&sys_path, sys_content.as_bytes()).expect("write sys");

        let result = store
            .lookup_with_system_fallback(
                "remote.server",
                "ssh-ed25519",
                &key_bytes,
                Some(&sys_path),
            )
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Trusted(_)),
            "Host in system known_hosts (plain) must be Trusted via fallback (FS-SSH-011)"
        );
    }

    /// Host absent from both stores → Unknown.
    #[test]
    fn system_fallback_absent_both_stores_unknown() {
        let tauterm_dir = tempfile::tempdir().expect("tempdir");
        let tauterm_path = tauterm_dir.path().join("known_hosts");
        let store = KnownHostsStore::new(tauterm_path);

        let sys_dir = tempfile::tempdir().expect("tempdir");
        let sys_path = sys_dir.path().join("known_hosts");
        // System file empty.
        fs::write(&sys_path, b"").expect("write");

        let result = store
            .lookup_with_system_fallback("unknown.host", "ssh-ed25519", &[1, 2, 3], Some(&sys_path))
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Unknown),
            "Host absent from both stores must be Unknown"
        );
    }

    /// TauTerm store is authoritative: if host is Mismatch there, fallback is NOT consulted.
    #[test]
    fn system_fallback_tauterm_mismatch_is_final() {
        let stored_key = vec![0x01, 0x02];
        let offered_key = vec![0xFF, 0xFE]; // different
        let key_b64 = BASE64.encode(&stored_key);
        let content = format!("mismatch.host ssh-ed25519 {key_b64}\n");

        let tauterm_dir = tempfile::tempdir().expect("tempdir");
        let tauterm_path = tauterm_dir.path().join("known_hosts");
        fs::write(&tauterm_path, content.as_bytes()).expect("write");
        let store = KnownHostsStore::new(tauterm_path);

        // System file has the *offered* key (matching).
        let offered_b64 = BASE64.encode(&offered_key);
        let sys_content = format!("mismatch.host ssh-ed25519 {offered_b64}\n");
        let sys_dir = tempfile::tempdir().expect("tempdir");
        let sys_path = sys_dir.path().join("known_hosts");
        fs::write(&sys_path, sys_content.as_bytes()).expect("write sys");

        // TauTerm store says Mismatch → result must be Mismatch regardless of system file.
        let result = store
            .lookup_with_system_fallback(
                "mismatch.host",
                "ssh-ed25519",
                &offered_key,
                Some(&sys_path),
            )
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Mismatch { .. }),
            "TauTerm store Mismatch must not be overridden by system known_hosts"
        );
    }

    /// Hashed entries in system file are skipped; host still resolves Unknown.
    #[test]
    fn system_fallback_hashed_entries_in_system_file_skipped() {
        let tauterm_dir = tempfile::tempdir().expect("tempdir");
        let tauterm_path = tauterm_dir.path().join("known_hosts");
        let store = KnownHostsStore::new(tauterm_path);

        // System file contains only hashed entries — cannot be matched by hostname.
        let sys_content = "|1|abc123=|xyz456= ssh-ed25519 AAAA==\n";
        let sys_dir = tempfile::tempdir().expect("tempdir");
        let sys_path = sys_dir.path().join("known_hosts");
        fs::write(&sys_path, sys_content.as_bytes()).expect("write sys");

        let result = store
            .lookup_with_system_fallback("hashed.host", "ssh-ed25519", &[0x11], Some(&sys_path))
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Unknown),
            "Hashed entries in system file must be skipped → Unknown"
        );
    }

    /// Non-existent system file is silently ignored → Unknown.
    #[test]
    fn system_fallback_missing_system_file_returns_unknown() {
        let tauterm_dir = tempfile::tempdir().expect("tempdir");
        let tauterm_path = tauterm_dir.path().join("known_hosts");
        let store = KnownHostsStore::new(tauterm_path);

        let sys_dir = tempfile::tempdir().expect("tempdir");
        let nonexistent_path = sys_dir.path().join("does_not_exist");

        let result = store
            .lookup_with_system_fallback(
                "some.host",
                "ssh-ed25519",
                &[0xAA],
                Some(&nonexistent_path),
            )
            .expect("lookup must not error on missing system file");

        assert!(
            matches!(result, KnownHostLookup::Unknown),
            "Missing system file must be silently ignored → Unknown"
        );
    }

    /// TauTerm store match takes precedence over system file (Trusted in TauTerm → no fallback).
    #[test]
    fn system_fallback_tauterm_trusted_takes_precedence() {
        let key_bytes = vec![0x01, 0x02, 0x03];
        let key_b64 = BASE64.encode(&key_bytes);
        let content = format!("trusted.host ssh-ed25519 {key_b64}\n");

        let tauterm_dir = tempfile::tempdir().expect("tempdir");
        let tauterm_path = tauterm_dir.path().join("known_hosts");
        fs::write(&tauterm_path, content.as_bytes()).expect("write");
        let store = KnownHostsStore::new(tauterm_path);

        // System file is empty — fallback should never be consulted.
        let sys_dir = tempfile::tempdir().expect("tempdir");
        let sys_path = sys_dir.path().join("known_hosts");
        fs::write(&sys_path, b"").expect("write");

        let result = store
            .lookup_with_system_fallback("trusted.host", "ssh-ed25519", &key_bytes, Some(&sys_path))
            .expect("lookup");

        assert!(
            matches!(result, KnownHostLookup::Trusted(_)),
            "TauTerm store Trusted must be returned without consulting system file"
        );
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
        assert_eq!(
            entries.len(),
            1,
            "Only the non-comment entry should be parsed"
        );
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
