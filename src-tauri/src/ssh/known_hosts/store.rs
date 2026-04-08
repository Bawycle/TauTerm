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
