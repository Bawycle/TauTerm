// SPDX-License-Identifier: MPL-2.0

//! Validated newtypes for text fields at the IPC boundary.
//!
//! Each type wraps a `String` and enforces constraints at deserialization time
//! via `#[serde(try_from = "String")]`. This implements "Parse Don't Validate":
//! a value of type `SshHost` is always a valid host, by construction.
//!
//! All `TryFrom<String>` implementations return `Err(String)` with a human-readable
//! message. No regex dependency — validation is done with standard library primitives.

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Macro: generate PartialEq<str> / PartialEq<String> / AsRef<str> for a newtype
// ---------------------------------------------------------------------------

/// Generates ergonomic comparison impls for a String-backed newtype so that
/// `assert_eq!(value, "literal")` and `assert_eq!(value, some_string)` work
/// without requiring explicit dereferencing.
macro_rules! impl_str_eq {
    ($T:ty) => {
        impl PartialEq<str> for $T {
            fn eq(&self, other: &str) -> bool {
                self.0.as_str() == other
            }
        }

        impl PartialEq<&str> for $T {
            fn eq(&self, other: &&str) -> bool {
                self.0.as_str() == *other
            }
        }

        impl PartialEq<String> for $T {
            fn eq(&self, other: &String) -> bool {
                &self.0 == other
            }
        }

        impl AsRef<str> for $T {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn check_no_control_chars(s: &str, field: &str) -> Result<(), String> {
    if s.chars().any(|c| c.is_control()) {
        return Err(format!("{field} must not contain control characters"));
    }
    Ok(())
}

fn check_max_len(s: &str, max: usize, field: &str) -> Result<(), String> {
    if s.len() > max {
        return Err(format!(
            "{field} must not exceed {max} bytes (got {})",
            s.len()
        ));
    }
    Ok(())
}

/// Validate a string as a DNS hostname or IP address (IPv4, bare IPv6, or
/// bracketed IPv6 `[::1]`).
///
/// Does not require any regex dependency — uses `str::parse::<IpAddr>` and
/// per-label ASCII checks per RFC 1035 §2.3.4.
fn validate_hostname_or_ip(s: &str) -> Result<(), String> {
    // IPv4 or bare IPv6 (e.g. "::1", "192.168.1.1").
    if s.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    // Bracketed IPv6 (e.g. "[::1]").
    if let Some(inner) = s.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        if inner.parse::<std::net::Ipv6Addr>().is_ok() {
            return Ok(());
        }
        return Err(format!(
            "bracketed value '{inner}' is not a valid IPv6 address"
        ));
    }
    // DNS hostname validation.
    if s.is_empty() {
        return Err("hostname must not be empty".to_string());
    }
    for label in s.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(format!(
                "DNS label '{label}' is invalid (empty or exceeds 63 characters)"
            ));
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(format!(
                "DNS label '{label}' contains invalid characters (only alphanumeric and hyphen allowed)"
            ));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(format!(
                "DNS label '{label}' must not start or end with a hyphen"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SshHost
// ---------------------------------------------------------------------------

/// A validated SSH hostname or IP address.
///
/// Constraints: max 253 bytes, no control characters, valid hostname/IP format.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(transparent)]
pub struct SshHost(String);

impl TryFrom<String> for SshHost {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_no_control_chars(&s, "SshHost")?;
        check_max_len(&s, 253, "SshHost")?;
        validate_hostname_or_ip(&s)?;
        Ok(Self(s))
    }
}

impl<'de> Deserialize<'de> for SshHost {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl Deref for SshHost {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SshHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_str_eq!(SshHost);

// ---------------------------------------------------------------------------
// SshLabel
// ---------------------------------------------------------------------------

/// A validated SSH connection label (non-empty display name).
///
/// Constraints: max 256 bytes, no control characters, non-empty.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(transparent)]
pub struct SshLabel(String);

impl TryFrom<String> for SshLabel {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_no_control_chars(&s, "SshLabel")?;
        check_max_len(&s, 256, "SshLabel")?;
        if s.is_empty() {
            return Err("SshLabel must not be empty".to_string());
        }
        Ok(Self(s))
    }
}

impl<'de> Deserialize<'de> for SshLabel {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl Deref for SshLabel {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SshLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_str_eq!(SshLabel);

// ---------------------------------------------------------------------------
// SshUsername
// ---------------------------------------------------------------------------

/// A validated SSH username (non-empty).
///
/// Constraints: max 255 bytes, no control characters, non-empty.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(transparent)]
pub struct SshUsername(String);

impl TryFrom<String> for SshUsername {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_no_control_chars(&s, "SshUsername")?;
        check_max_len(&s, 255, "SshUsername")?;
        if s.is_empty() {
            return Err("SshUsername must not be empty".to_string());
        }
        Ok(Self(s))
    }
}

impl<'de> Deserialize<'de> for SshUsername {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl Deref for SshUsername {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SshUsername {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_str_eq!(SshUsername);

// ---------------------------------------------------------------------------
// FontFamily
// ---------------------------------------------------------------------------

/// A validated font family name (may be empty).
///
/// Constraints: max 256 bytes, no control characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(transparent)]
pub struct FontFamily(String);

impl TryFrom<String> for FontFamily {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_no_control_chars(&s, "FontFamily")?;
        check_max_len(&s, 256, "FontFamily")?;
        Ok(Self(s))
    }
}

impl<'de> Deserialize<'de> for FontFamily {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl Deref for FontFamily {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_str_eq!(FontFamily);

impl FontFamily {
    /// Default font family: "monospace" (a known-valid literal).
    pub fn monospace() -> Self {
        Self::try_from("monospace".to_string())
            .expect("'monospace' is a known-valid FontFamily literal")
    }
}

// ---------------------------------------------------------------------------
// ThemeName
// ---------------------------------------------------------------------------

/// A validated theme name (non-empty).
///
/// Constraints: max 128 bytes, no control characters, non-empty.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(transparent)]
pub struct ThemeName(String);

impl TryFrom<String> for ThemeName {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_no_control_chars(&s, "ThemeName")?;
        check_max_len(&s, 128, "ThemeName")?;
        if s.is_empty() {
            return Err("ThemeName must not be empty".to_string());
        }
        Ok(Self(s))
    }
}

impl<'de> Deserialize<'de> for ThemeName {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl Deref for ThemeName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThemeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_str_eq!(ThemeName);

impl ThemeName {
    /// Default theme name: "umbra" (a known-valid literal).
    pub fn umbra() -> Self {
        Self::try_from("umbra".to_string()).expect("'umbra' is a known-valid ThemeName literal")
    }
}

// ---------------------------------------------------------------------------
// WordDelimiters
// ---------------------------------------------------------------------------

/// Validated word delimiter characters (may be empty).
///
/// Constraints: max 128 bytes, no control characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(transparent)]
pub struct WordDelimiters(String);

impl TryFrom<String> for WordDelimiters {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_no_control_chars(&s, "WordDelimiters")?;
        check_max_len(&s, 128, "WordDelimiters")?;
        Ok(Self(s))
    }
}

impl<'de> Deserialize<'de> for WordDelimiters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl Deref for WordDelimiters {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WordDelimiters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_str_eq!(WordDelimiters);

// ---------------------------------------------------------------------------
// SshIdentityPath
// ---------------------------------------------------------------------------

/// A structurally validated SSH identity file path.
///
/// Enforces at serde/construction time: absolute path (starts with `/`),
/// no `..` components, no control characters, max 4096 bytes.
///
/// Does NOT check file existence or `~/.ssh/` boundary — those are runtime
/// checks in `lifecycle.rs::open_connection_inner` via
/// `platform::validation::validate_ssh_identity_path`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(transparent)]
pub struct SshIdentityPath(String);

impl SshIdentityPath {
    pub fn as_path(&self) -> &std::path::Path {
        std::path::Path::new(&self.0)
    }
}

impl TryFrom<String> for SshIdentityPath {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_max_len(&s, 4096, "identity_file")?;
        check_no_control_chars(&s, "identity_file")?;
        if !s.starts_with('/') {
            return Err("identity_file path must be absolute (must start with '/')".to_string());
        }
        if std::path::Path::new(&s)
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(
                "identity_file path must not contain '..' components (path traversal)".to_string(),
            );
        }
        Ok(SshIdentityPath(s))
    }
}

impl<'de> Deserialize<'de> for SshIdentityPath {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl std::ops::Deref for SshIdentityPath {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SshIdentityPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_str_eq!(SshIdentityPath);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SshHost
    // -----------------------------------------------------------------------

    #[test]
    fn ssh_host_valid_hostname_accepted() {
        assert!(SshHost::try_from("example.com".to_string()).is_ok());
    }

    #[test]
    fn ssh_host_valid_ipv4_accepted() {
        assert!(SshHost::try_from("192.168.1.1".to_string()).is_ok());
    }

    #[test]
    fn ssh_host_valid_ipv6_bare_accepted() {
        assert!(SshHost::try_from("::1".to_string()).is_ok());
    }

    #[test]
    fn ssh_host_valid_ipv6_bracketed_accepted() {
        assert!(SshHost::try_from("[::1]".to_string()).is_ok());
    }

    #[test]
    fn ssh_host_empty_rejected() {
        assert!(SshHost::try_from(String::new()).is_err());
    }

    #[test]
    fn ssh_host_control_char_rejected() {
        assert!(SshHost::try_from("exa\x00mple.com".to_string()).is_err());
    }

    #[test]
    fn ssh_host_exceeds_max_len_rejected() {
        // 254 chars — one over DNS max (253).
        let long = "a".repeat(254);
        assert!(SshHost::try_from(long).is_err());
    }

    #[test]
    fn ssh_host_invalid_label_rejected() {
        // Underscore is not allowed in a DNS label.
        assert!(SshHost::try_from("exa_mple.com".to_string()).is_err());
    }

    #[test]
    fn ssh_host_label_starts_with_hyphen_rejected() {
        assert!(SshHost::try_from("-example.com".to_string()).is_err());
    }

    #[test]
    fn ssh_host_label_exceeds_63_chars_rejected() {
        // Label of 64 chars — one over the RFC 1035 limit of 63.
        let long_label = "a".repeat(64);
        let host = format!("{long_label}.com");
        assert!(SshHost::try_from(host).is_err());
    }

    // -----------------------------------------------------------------------
    // SshUsername
    // -----------------------------------------------------------------------

    #[test]
    fn ssh_username_empty_rejected() {
        assert!(SshUsername::try_from(String::new()).is_err());
    }

    #[test]
    fn ssh_username_control_char_rejected() {
        assert!(SshUsername::try_from("user\x01name".to_string()).is_err());
    }

    #[test]
    fn ssh_username_max_len_accepted() {
        let s = "u".repeat(255);
        assert!(SshUsername::try_from(s).is_ok());
    }

    #[test]
    fn ssh_username_exceeds_max_len_rejected() {
        let s = "u".repeat(256);
        assert!(SshUsername::try_from(s).is_err());
    }

    // -----------------------------------------------------------------------
    // FontFamily
    // -----------------------------------------------------------------------

    #[test]
    fn font_family_empty_accepted() {
        assert!(FontFamily::try_from(String::new()).is_ok());
    }

    #[test]
    fn font_family_control_char_rejected() {
        assert!(FontFamily::try_from("Mono\x07space".to_string()).is_err());
    }

    #[test]
    fn font_family_exceeds_max_len_rejected() {
        let s = "a".repeat(257);
        assert!(FontFamily::try_from(s).is_err());
    }

    // -----------------------------------------------------------------------
    // ThemeName
    // -----------------------------------------------------------------------

    #[test]
    fn theme_name_empty_rejected() {
        assert!(ThemeName::try_from(String::new()).is_err());
    }

    #[test]
    fn theme_name_control_char_rejected() {
        assert!(ThemeName::try_from("um\x1bra".to_string()).is_err());
    }

    // -----------------------------------------------------------------------
    // WordDelimiters
    // -----------------------------------------------------------------------

    #[test]
    fn word_delimiters_empty_accepted() {
        assert!(WordDelimiters::try_from(String::new()).is_ok());
    }

    #[test]
    fn word_delimiters_control_char_rejected() {
        // \t is a control character — must be rejected.
        assert!(WordDelimiters::try_from(" \t|".to_string()).is_err());
    }

    // -----------------------------------------------------------------------
    // Serde round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn ssh_host_serde_round_trip() {
        let original = SshHost::try_from("example.com".to_string()).unwrap();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: SshHost = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn ssh_host_serde_rejects_control_char() {
        let json = "\"exa\x00mple.com\"";
        let result = serde_json::from_str::<SshHost>(json);
        assert!(
            result.is_err(),
            "serde must reject control chars in SshHost"
        );
    }

    #[test]
    fn ssh_username_serde_rejects_empty_string() {
        let json = "\"\"";
        let result = serde_json::from_str::<SshUsername>(json);
        assert!(
            result.is_err(),
            "serde must reject empty string for SshUsername"
        );
    }

    // -----------------------------------------------------------------------
    // SshIdentityPath
    // -----------------------------------------------------------------------

    #[test]
    fn ssh_identity_path_valid_accepted() {
        assert!(SshIdentityPath::try_from("/home/user/.ssh/id_ed25519".to_string()).is_ok());
    }

    #[test]
    fn ssh_identity_path_relative_rejected() {
        assert!(SshIdentityPath::try_from("relative/path".to_string()).is_err());
    }

    #[test]
    fn ssh_identity_path_traversal_rejected() {
        assert!(SshIdentityPath::try_from("/home/user/../.ssh/id_rsa".to_string()).is_err());
    }

    #[test]
    fn ssh_identity_path_control_char_rejected() {
        assert!(SshIdentityPath::try_from("/home/user/.ssh/id_\x00rsa".to_string()).is_err());
    }

    #[test]
    fn ssh_identity_path_exceeds_max_len_rejected() {
        let s = format!("/{}", "a".repeat(4096));
        assert!(SshIdentityPath::try_from(s).is_err());
    }

    #[test]
    fn ssh_identity_path_empty_rejected() {
        // Empty string is not absolute (does not start with '/').
        assert!(SshIdentityPath::try_from(String::new()).is_err());
    }

    #[test]
    fn ssh_identity_path_serde_round_trip() {
        let original = SshIdentityPath::try_from("/home/user/.ssh/id_ed25519".to_string()).unwrap();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: SshIdentityPath = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn ssh_identity_path_serde_rejects_traversal() {
        // serde must reject a path containing ".." at deserialization time.
        let json = "\"/home/user/../.ssh/id_rsa\"";
        let result = serde_json::from_str::<SshIdentityPath>(json);
        assert!(
            result.is_err(),
            "serde must reject '..' components in SshIdentityPath"
        );
    }
}
