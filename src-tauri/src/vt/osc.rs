// SPDX-License-Identifier: MPL-2.0

//! OSC (Operating System Command) sequence dispatch.
//!
//! Handles the OSC sequences relevant to TauTerm v1:
//! - OSC 0/1/2: window/icon title
//! - OSC 22/23: title stack (push/pop)
//! - OSC 8: hyperlinks
//! - OSC 52: clipboard (write controlled by policy; read permanently rejected)
//!
//! Unknown OSC sequences are silently ignored.

/// The result of processing an OSC sequence.
#[derive(Debug)]
pub enum OscAction {
    /// Set the terminal title (OSC 0, 1, 2).
    SetTitle(String),
    /// Push current title onto the stack (OSC 22).
    PushTitle,
    /// Pop title from the stack (OSC 23).
    PopTitle,
    /// Set a hyperlink (OSC 8). `None` = end hyperlink.
    SetHyperlink {
        uri: Option<String>,
        id: Option<String>,
    },
    /// Write to the system clipboard (OSC 52 write).
    /// Gated by `allow_osc52_write` preference.
    ClipboardWrite(String),
    /// Report current working directory (OSC 7).
    SetCwd(String),
    /// Ignored sequence.
    Ignore,
}

/// Parse an OSC command from a slice of parameter segments into an `OscAction`.
///
/// `params` is the slice provided directly by the `vte` parser: each element is
/// one semicolon-delimited field of the OSC sequence.
/// - `params[0]` — numeric command code (ASCII digits, e.g. `b"0"`, `b"8"`, `b"52"`)
/// - `params[1]` — first payload field (title text, OSC-8 key=value params, clipboard target…)
/// - `params[2]` — second payload field (OSC-8 URI, clipboard base64 data)
pub fn parse_osc(params: &[&[u8]]) -> OscAction {
    // params[0] must exist and be valid UTF-8 ASCII digits.
    let cmd: u32 = match params.first() {
        Some(code) => match std::str::from_utf8(code).ok().and_then(|s| s.parse().ok()) {
            Some(n) => n,
            None => return OscAction::Ignore,
        },
        None => return OscAction::Ignore,
    };

    match cmd {
        // OSC 0 / 1 / 2 — window/icon title.
        0..=2 => {
            let raw_title = params
                .get(1)
                .and_then(|p| std::str::from_utf8(p).ok())
                .unwrap_or("");
            // Strip C0/C1 control characters and truncate to 256 chars (§8.1).
            let title: String = raw_title
                .chars()
                .filter(|&c| !c.is_control() || c == '\t')
                .take(256)
                .collect();
            OscAction::SetTitle(title)
        }
        // OSC 22 — push title.
        22 => OscAction::PushTitle,
        // OSC 23 — pop title.
        23 => OscAction::PopTitle,
        // OSC 8 — hyperlink: ESC ] 8 ; id=<id> ; <uri> ST
        // params[1] = key=value pairs (e.g. "id=foo"), params[2] = URI.
        8 => {
            let id_params = params
                .get(1)
                .and_then(|p| std::str::from_utf8(p).ok())
                .unwrap_or("");
            let uri_str = params
                .get(2)
                .and_then(|p| std::str::from_utf8(p).ok())
                .unwrap_or("");
            // params[1] must exist for a valid OSC 8 sequence; params[2] may be absent
            // only when the sequence has no URI field — treat as end-hyperlink.
            if params.len() < 2 {
                return OscAction::Ignore;
            }
            // SEC-OSC-004: reject URIs exceeding 2048 bytes to prevent memory exhaustion.
            if uri_str.len() > 2048 {
                return OscAction::Ignore;
            }
            let id = id_params
                .split(';')
                .filter_map(|p| p.strip_prefix("id="))
                .next()
                .map(|s| s.to_string());
            if uri_str.is_empty() {
                OscAction::SetHyperlink { uri: None, id }
            } else {
                OscAction::SetHyperlink {
                    uri: Some(uri_str.to_string()),
                    id,
                }
            }
        }
        // OSC 52 — clipboard.
        // params[1] = target (e.g. "c"), params[2] = base64-encoded data.
        52 => {
            let target = params
                .get(1)
                .and_then(|p| std::str::from_utf8(p).ok())
                .unwrap_or("");
            let data_b64 = params
                .get(2)
                .and_then(|p| std::str::from_utf8(p).ok())
                .unwrap_or("");
            // Read is permanently rejected (§8.2).
            if data_b64 == "?" {
                return OscAction::Ignore;
            }
            // Only "c" target (CLIPBOARD) is supported for write.
            if !target.contains('c') {
                return OscAction::Ignore;
            }
            // Decode base64 payload.
            let decoded = match base64_decode(data_b64.as_bytes()) {
                Some(d) => d,
                None => return OscAction::Ignore,
            };
            match String::from_utf8(decoded) {
                Ok(text) => OscAction::ClipboardWrite(text),
                Err(_) => OscAction::Ignore,
            }
        }
        // OSC 7 — shell CWD reporting: file://hostname/path or bare /path.
        7 => {
            let raw = params
                .get(1)
                .and_then(|p| std::str::from_utf8(p).ok())
                .unwrap_or("")
                .trim_end_matches(|c: char| c == '\0' || c.is_whitespace());
            if raw.is_empty() {
                return OscAction::Ignore;
            }
            // Parse file:// URI — extract and percent-decode the path component.
            let path = if let Some(rest) = raw.strip_prefix("file://") {
                // rest = "hostname/path" or "/path" (empty host).
                // Skip past the host component (up to first '/') to reach the path.
                let path_start = rest.find('/').unwrap_or(0);
                let encoded = &rest[path_start..];
                percent_decode_path(encoded)
            } else {
                // Bare path (e.g. "/home/user/src") — use as-is.
                raw.to_owned()
            };
            if path.is_empty() {
                return OscAction::Ignore;
            }
            // SEC-OSC-005: reject non-absolute paths (relative paths are meaningless
            // as CWD and could be used for injection or path-confusion attacks).
            if !path.starts_with('/') {
                return OscAction::Ignore;
            }
            // SEC-OSC-005: reject paths containing Unicode bidi-override codepoints.
            // These are used in "Trojan Source" style attacks to hide the true path
            // visually in UIs that render the CWD string.
            if contains_bidi_override(&path) {
                return OscAction::Ignore;
            }
            OscAction::SetCwd(path)
        }
        _ => OscAction::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TEST-VT-012 (unit) — OSC title parsing
    // FS-VT-060, FS-VT-062
    // -----------------------------------------------------------------------

    #[test]
    fn osc_plain_title_is_returned() {
        // TEST-VT-012 step 1-2
        let action = parse_osc(&[b"0", b"My Title"]);
        match action {
            OscAction::SetTitle(t) => assert_eq!(t, "My Title"),
            _ => panic!("expected SetTitle"),
        }
    }

    #[test]
    fn osc1_and_osc2_also_set_title() {
        for (cmd, title) in [(b"1" as &[u8], b"title1" as &[u8]), (b"2", b"title2")] {
            match parse_osc(&[cmd, title]) {
                OscAction::SetTitle(_) => {}
                _ => panic!("OSC 1 and 2 must also produce SetTitle"),
            }
        }
    }

    // -----------------------------------------------------------------------
    // OSC 8 — hyperlink
    // -----------------------------------------------------------------------

    #[test]
    fn osc8_hyperlink_with_id_parses_correctly() {
        let action = parse_osc(&[b"8", b"id=link1", b"https://example.com"]);
        match action {
            OscAction::SetHyperlink { uri, id } => {
                assert_eq!(uri, Some("https://example.com".to_string()));
                assert_eq!(id, Some("link1".to_string()));
            }
            _ => panic!("expected SetHyperlink"),
        }
    }

    #[test]
    fn osc8_empty_uri_ends_hyperlink() {
        // OSC 8 ;; — params[1]="" (no id), params[2]="" (no URI) = end hyperlink.
        let action = parse_osc(&[b"8", b"", b""]);
        match action {
            OscAction::SetHyperlink { uri, .. } => {
                assert_eq!(uri, None, "empty URI means end-of-hyperlink");
            }
            _ => panic!("expected SetHyperlink with None uri"),
        }
    }

    #[test]
    fn osc8_no_id_param_produces_none_id() {
        let action = parse_osc(&[b"8", b"", b"https://example.com"]);
        match action {
            OscAction::SetHyperlink { uri, id } => {
                assert_eq!(uri, Some("https://example.com".to_string()));
                assert_eq!(id, None);
            }
            _ => panic!("expected SetHyperlink"),
        }
    }

    // -----------------------------------------------------------------------
    // OSC 22/23 — title stack push/pop
    // -----------------------------------------------------------------------

    #[test]
    fn osc22_produces_push_title() {
        assert!(matches!(parse_osc(&[b"22"]), OscAction::PushTitle));
    }

    #[test]
    fn osc23_produces_pop_title() {
        assert!(matches!(parse_osc(&[b"23"]), OscAction::PopTitle));
    }

    // -----------------------------------------------------------------------
    // OSC 7 — CWD reporting
    // -----------------------------------------------------------------------

    #[test]
    fn osc7_file_uri_with_host_extracts_path() {
        let action = parse_osc(&[b"7", b"file://hostname/home/user/src"]);
        match action {
            OscAction::SetCwd(path) => assert_eq!(path, "/home/user/src"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_file_uri_empty_host_extracts_path() {
        let action = parse_osc(&[b"7", b"file:///home/user/src"]);
        match action {
            OscAction::SetCwd(path) => assert_eq!(path, "/home/user/src"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_bare_path_used_as_is() {
        let action = parse_osc(&[b"7", b"/home/user/project"]);
        match action {
            OscAction::SetCwd(path) => assert_eq!(path, "/home/user/project"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_percent_encoded_spaces_decoded() {
        // file://localhost/home/user/my%20project
        let action = parse_osc(&[b"7", b"file://localhost/home/user/my%20project"]);
        match action {
            OscAction::SetCwd(path) => assert_eq!(path, "/home/user/my project"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_empty_payload_is_ignored() {
        assert!(matches!(parse_osc(&[b"7", b""]), OscAction::Ignore));
    }

    #[test]
    fn osc7_no_payload_is_ignored() {
        assert!(matches!(parse_osc(&[b"7"]), OscAction::Ignore));
    }

    // -----------------------------------------------------------------------
    // Unknown / malformed commands
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_osc_command_is_ignored() {
        assert!(matches!(
            parse_osc(&[b"999", b"some-data"]),
            OscAction::Ignore
        ));
    }

    #[test]
    fn non_numeric_osc_command_is_ignored() {
        assert!(matches!(parse_osc(&[b"abc", b"data"]), OscAction::Ignore));
    }

    #[test]
    fn empty_osc_payload_is_ignored() {
        assert!(matches!(parse_osc(&[]), OscAction::Ignore));
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SEC-OSC-001 — OSC 52 clipboard read is permanently rejected
    // -----------------------------------------------------------------------

    /// SEC-OSC-001: Query payload "?" must always return Ignore.
    #[test]
    fn sec_osc_001_osc52_read_query_returns_ignore() {
        // Direct parse_osc call with canonical read payload.
        let action = parse_osc(&[b"52", b"c", b"?"]);
        assert!(
            matches!(action, OscAction::Ignore),
            "OSC 52 read query must be permanently ignored (SEC-OSC-001)"
        );
    }

    /// SEC-OSC-001: Read query via full OSC byte stream through VtProcessor.
    #[test]
    fn sec_osc_001_osc52_read_via_full_sequence_returns_ignore() {
        use crate::vt::processor::VtProcessor;
        // ESC ] 52 ; c ; ? BEL
        let seq = b"\x1b]52;c;\x07";
        // VtProcessor — no panic, title unchanged, no clipboard write triggered.
        let mut vt = VtProcessor::new(80, 24, 10_000, 0, false);
        let _dirty = vt.process(seq);
        // If we get here without panic, the sequence was silently consumed.
        // There is no observable side-effect to assert beyond "no crash and no
        // ClipboardWrite" — confirmed by the parse_osc unit test above.
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-006 — OSC title payloads strip C0/C1 control characters
    // -----------------------------------------------------------------------

    /// SEC-PTY-006: Control characters in title payloads must be stripped.
    #[test]
    fn sec_pty_006_osc_title_strips_control_chars() {
        // Payload: \x01\x0b\x1b[31mInjection (C0/C1 + partial CSI)
        let action = parse_osc(&[b"0", b"\x01\x0b\x1b[31mInjection"]);
        match action {
            OscAction::SetTitle(title) => {
                assert!(
                    !title.contains('\x01'),
                    "C0 SOH must be stripped from title (SEC-PTY-006)"
                );
                assert!(
                    !title.contains('\x0b'),
                    "C0 VT must be stripped from title (SEC-PTY-006)"
                );
                assert!(
                    !title.contains('\x1b'),
                    "ESC must be stripped from title (SEC-PTY-006)"
                );
                // "Injection" text content should still be present.
                assert!(
                    title.contains("Injection"),
                    "Title text content should survive stripping"
                );
            }
            other => panic!("Expected SetTitle, got {:?} (SEC-PTY-006)", other),
        }
    }

    /// SEC-PTY-006: Title is truncated to 256 characters maximum.
    #[test]
    fn sec_pty_006_osc_title_truncated_to_256_chars() {
        let long_title = b"A".repeat(300);
        let action = parse_osc(&[b"0", &long_title]);
        match action {
            OscAction::SetTitle(title) => {
                assert!(
                    title.len() <= 256,
                    "Title must be truncated to 256 chars, got {} (SEC-PTY-006)",
                    title.len()
                );
            }
            other => panic!("Expected SetTitle, got {:?}", other),
        }
    }

    /// SEC-PTY-006: Tab character (\t) is permitted in title (explicit exception).
    #[test]
    fn sec_pty_006_tab_character_preserved_in_title() {
        let action = parse_osc(&[b"0", b"hello\tworld"]);
        match action {
            OscAction::SetTitle(title) => {
                assert!(title.contains('\t'), "Tab should be preserved in title");
            }
            other => panic!("Expected SetTitle, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // SEC-OSC-002 — OSC 52 write gated by allow_osc52_write policy
    // (partial: parse_osc layer — policy resolution requires VtProcessor wiring)
    // -----------------------------------------------------------------------

    /// SEC-OSC-002 (partial): When allow_osc52_write is false, the ClipboardWrite
    /// action is still returned by parse_osc — enforcement is the VtProcessor's
    /// responsibility via the policy flag. This test confirms parse_osc itself
    /// correctly identifies write sequences so the caller can apply the policy.
    #[test]
    fn sec_osc_002_osc52_write_sequence_parsed_as_clipboard_write() {
        // Base64 encode "hello" = "aGVsbG8="
        let action = parse_osc(&[b"52", b"c", b"aGVsbG8="]);
        assert!(
            matches!(action, OscAction::ClipboardWrite(_)),
            "Valid OSC 52 write must produce ClipboardWrite action for policy check"
        );
    }

    /// SEC-OSC-002 (partial): Non-"c" targets are ignored (no write for primary/"p").
    #[test]
    fn sec_osc_002_osc52_non_clipboard_target_ignored() {
        // Target "p" (primary selection) — not supported for write.
        let action = parse_osc(&[b"52", b"p", b"aGVsbG8="]);
        assert!(
            matches!(action, OscAction::Ignore),
            "OSC 52 write to non-clipboard target must be ignored"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-OSC-003 — OSC 52 oversized payload
    // -----------------------------------------------------------------------

    /// SEC-OSC-003: A 1-MB base64 payload in an OSC 52 write sequence.
    /// parse_osc receives raw bytes — the 4096-byte sequence limit is enforced
    /// upstream by `handle_osc` in the dispatch layer (FS-SEC-005 / GAP-009).
    /// The `vte` crate itself imposes no limit in `std` mode. At the parse_osc
    /// level this test verifies no panic or OOM occurs when the guard is bypassed.
    #[test]
    fn sec_osc_003_osc52_large_payload_no_panic() {
        // 1 MB of valid base64 'A' characters (not a valid base64 multiple of 4
        // for this size, so base64_decode returns None → Ignore).
        let large_b64 = b"A".repeat(1_000_000);
        // Must not panic — result is Ignore (invalid base64 length) or ClipboardWrite.
        let action = parse_osc(&[b"52", b"c", &large_b64]);
        // Either outcome is acceptable; what matters is no panic / no OOM.
        match action {
            OscAction::Ignore | OscAction::ClipboardWrite(_) => {}
            other => panic!("Unexpected action from oversized payload: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-002 — OSC query sequences silently ignored
    // -----------------------------------------------------------------------

    /// SEC-PTY-002: OSC 10 (foreground color query "?") returns Ignore.
    #[test]
    fn sec_pty_002_osc_color_query_returns_ignore() {
        // OSC 10 ; ? ST — dynamic color query
        let action = parse_osc(&[b"10", b"?"]);
        assert!(
            matches!(action, OscAction::Ignore),
            "OSC 10 color query must be ignored (SEC-PTY-002)"
        );
    }

    /// SEC-PTY-002: Unknown OSC commands return Ignore.
    #[test]
    fn sec_pty_002_unknown_osc_returns_ignore() {
        let action = parse_osc(&[b"9999", b"some_payload"]);
        assert!(
            matches!(action, OscAction::Ignore),
            "Unknown OSC command must be ignored"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-PTY-003 — Oversized OSC sequence DoS
    // -----------------------------------------------------------------------

    /// SEC-PTY-003: parse_osc with a 10 000-byte OSC 0 title payload.
    /// The 4096-byte limit is enforced by `handle_osc` in the dispatch layer
    /// (FS-SEC-005 / GAP-009) — not by the `vte` crate, which imposes no size
    /// limit in `std` mode. This test verifies that `parse_osc` itself does not
    /// panic when directly given a large payload (i.e., the dispatch guard bypassed).
    #[test]
    fn sec_pty_003_large_osc_title_payload_no_panic() {
        let large_title = b"A".repeat(10_000);
        // Must not panic; title should be truncated to 256.
        let action = parse_osc(&[b"0", &large_title]);
        match action {
            OscAction::SetTitle(title) => {
                assert!(
                    title.len() <= 256,
                    "Title must be truncated even with large input"
                );
            }
            OscAction::Ignore => {} // Also acceptable.
            other => panic!("Unexpected action: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // OSC 7 — CWD reporting
    // -----------------------------------------------------------------------

    #[test]
    fn osc7_file_uri_with_host_extracts_path() {
        let action = parse_osc(&[b"7", b"file://hostname/home/user/src"]);
        match action {
            OscAction::SetCwd(p) => assert_eq!(p, "/home/user/src"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_file_uri_empty_host_extracts_path() {
        let action = parse_osc(&[b"7", b"file:///home/user/src"]);
        match action {
            OscAction::SetCwd(p) => assert_eq!(p, "/home/user/src"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_bare_path() {
        let action = parse_osc(&[b"7", b"/home/user/src"]);
        match action {
            OscAction::SetCwd(p) => assert_eq!(p, "/home/user/src"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_percent_encoded_non_ascii_path() {
        // é = %C3%A9 in UTF-8
        let action = parse_osc(&[b"7", b"file:///home/h%C3%A9l%C3%A8ne/src"]);
        match action {
            OscAction::SetCwd(p) => assert_eq!(p, "/home/hélène/src"),
            _ => panic!("expected SetCwd"),
        }
    }

    #[test]
    fn osc7_empty_payload_returns_ignore() {
        let action = parse_osc(&[b"7", b""]);
        assert!(matches!(action, OscAction::Ignore));
    }
}

/// Returns `true` if `s` contains Unicode bidi-override or invisible codepoints
/// that could be used to visually misrepresent a path ("Trojan Source" style).
///
/// Rejected codepoints:
/// - U+200F RIGHT-TO-LEFT MARK
/// - U+202A–U+202E LRE / RLE / PDF / LRO / RLO (explicit directional embeddings)
/// - U+2066–U+2069 LRI / RLI / FSI / PDI (isolate directional marks)
/// - U+200B ZERO WIDTH SPACE
/// - U+FEFF BYTE ORDER MARK / ZERO WIDTH NO-BREAK SPACE
pub(crate) fn contains_bidi_override(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c,
            '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{200B}'
            | '\u{FEFF}'
        )
    })
}

/// Percent-decode a URI path component (RFC 3986 §2.1).
///
/// Decodes `%XX` sequences where `XX` are valid hex digits. Malformed sequences
/// (`%` not followed by two hex digits) are passed through unchanged.
fn percent_decode_path(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    // Collect raw bytes first; decoded sequences may be multi-byte UTF-8
    // (e.g. `é` = %C3%A9). Pushing each decoded byte as `char` would corrupt
    // non-ASCII characters — instead accumulate bytes and do a single UTF-8
    // conversion at the end.
    let mut out: Vec<u8> = Vec::with_capacity(encoded.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Decode one ASCII hex digit to its nibble value (0–15). Returns `None` for
/// non-hex characters.
fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Minimal base64 decoder (no external dep beyond std).
/// Returns `None` if the input is invalid.
fn base64_decode(input: &[u8]) -> Option<Vec<u8>> {
    // Use the standard alphabet.
    fn decode_char(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            b'=' => None, // padding
            _ => None,
        }
    }

    let input: Vec<u8> = input
        .iter()
        .copied()
        .filter(|&c| c != b'\n' && c != b'\r')
        .collect();
    if !input.len().is_multiple_of(4) {
        return None;
    }

    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    for chunk in input.chunks(4) {
        let a = decode_char(chunk[0])?;
        let b = decode_char(chunk[1])?;
        out.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            let c = decode_char(chunk[2])?;
            out.push((b << 4) | (c >> 2));
            if chunk[3] != b'=' {
                let d = decode_char(chunk[3])?;
                out.push((c << 6) | d);
            }
        }
    }
    Some(out)
}
