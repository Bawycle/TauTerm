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
    /// Ignored sequence.
    Ignore,
}

/// Parse an OSC command byte sequence into an `OscAction`.
/// `params` is the raw content of the OSC sequence (everything between ESC ] and ST/BEL).
pub fn parse_osc(params: &[u8]) -> OscAction {
    // OSC format: Ps ; Pt (numeric command ; text payload).
    let content = match std::str::from_utf8(params) {
        Ok(s) => s,
        Err(_) => return OscAction::Ignore,
    };

    let (cmd_str, rest) = match content.find(';') {
        Some(pos) => (&content[..pos], &content[pos + 1..]),
        None => (content, ""),
    };

    let cmd: u32 = match cmd_str.parse() {
        Ok(n) => n,
        Err(_) => return OscAction::Ignore,
    };

    match cmd {
        // OSC 0 / 1 / 2 — window/icon title.
        0 | 1 | 2 => {
            // Strip C0/C1 control characters and truncate to 256 chars (§8.1).
            let title: String = rest
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
        // OSC 8 — hyperlink: ESC ] 8 ; params ; uri ST
        // Format: `8 ; id=<id> ; <uri>` or `8 ; ; ` (end hyperlink).
        8 => {
            // Split into `id_params` and `uri`.
            let (id_params, uri_str) = match rest.find(';') {
                Some(pos) => (&rest[..pos], &rest[pos + 1..]),
                None => return OscAction::Ignore,
            };
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
        52 => {
            // Format: `52 ; <target> ; <base64-encoded data>`
            // Read is permanently rejected (§8.2).
            let (target, data_b64) = match rest.find(';') {
                Some(pos) => (&rest[..pos], &rest[pos + 1..]),
                None => return OscAction::Ignore,
            };
            // "?" means clipboard read — permanently rejected.
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
        _ => OscAction::Ignore,
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
    if input.len() % 4 != 0 {
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
