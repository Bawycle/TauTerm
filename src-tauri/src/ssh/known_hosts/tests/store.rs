// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

use super::super::store::{KnownHostLookup, KnownHostsStore};

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
        .lookup_with_system_fallback("remote.server", "ssh-ed25519", &key_bytes, Some(&sys_path))
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
        .lookup_with_system_fallback("some.host", "ssh-ed25519", &[0xAA], Some(&nonexistent_path))
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
    let content =
        format!("# This is a comment\n\nexample.org ssh-ed25519 {key_b64}\n# Another comment\n");

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
