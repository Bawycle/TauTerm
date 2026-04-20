// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use crate::vt::osc::{OscAction, parse_osc};
use crate::vt::processor::{MAX_CWD_BYTES, MAX_TITLE_CHARS, VtProcessor};

/// Maximum number of entries in the title stack (OSC 22/23).
/// Matches xterm's default. Excess pushes are silently ignored.
pub(super) const TITLE_STACK_MAX: usize = 16;

/// Maximum total byte length of an OSC sequence payload (all fields combined,
/// including separator bytes). Sequences exceeding this limit are silently
/// dropped to prevent memory exhaustion (FS-SEC-005 / GAP-009).
///
/// Note: `vte` 0.15 with the `std` feature (the default) uses an unbounded
/// `Vec<u8>` for the OSC raw buffer and imposes **no** size limit before
/// dispatching to `osc_dispatch`. The 1024-byte constant in the `vte` source
/// only applies when `no_std` is active. This guard is therefore the sole
/// enforcement point for the 4 096-byte limit required by FS-SEC-005.
const OSC_PAYLOAD_MAX: usize = 4096;

pub(super) fn handle_osc(p: &mut VtProcessor, params: &[&[u8]]) {
    // Guard: silently ignore oversized OSC sequences to prevent DoS (FS-SEC-005).
    // total_len accounts for each field plus one byte per separator.
    let total_len: usize = params.iter().map(|p| p.len() + 1).sum::<usize>();
    if total_len > OSC_PAYLOAD_MAX {
        return;
    }
    match parse_osc(params) {
        OscAction::SetTitle(title) => {
            // ADR-0028 §VT input caps: defense-in-depth bound on accumulated
            // title length. `parse_osc` strips C0/C1 control chars **before**
            // returning here, so by the time the title reaches this site the
            // strip is already applied — the truncation below cannot consume
            // budget on stripped-out bytes (defense against C0-bourrage).
            // Order invariant: STRIP (upstream in `parse_osc`) → TRUNCATE (here).
            // If a future refactor removes the strip from `parse_osc`, the
            // truncate below would still bound memory but no longer enforce
            // the security property — keep the strip upstream.
            let bounded = if title.chars().count() > MAX_TITLE_CHARS {
                let orig_len = title.chars().count();
                let truncated: String = title.chars().take(MAX_TITLE_CHARS).collect();
                tracing::warn!(
                    orig_chars = orig_len,
                    cap = MAX_TITLE_CHARS,
                    "VT title truncated to cap (possible DoS amplification attempt)"
                );
                truncated
            } else {
                title
            };
            if p.title != bounded {
                p.title = bounded;
                p.title_changed = true;
            }
        }
        OscAction::PushTitle => {
            if p.title_stack.len() < TITLE_STACK_MAX {
                p.title_stack.push(p.title.clone());
            }
            // Excess pushes beyond TITLE_STACK_MAX are silently ignored (DoS prevention).
        }
        OscAction::PopTitle => {
            if let Some(t) = p.title_stack.pop()
                && p.title != t
            {
                p.title = t;
                p.title_changed = true;
            }
        }
        OscAction::SetHyperlink { uri, id } => {
            // FS-VT-070–073: store the active hyperlink URI/ID in the processor.
            // Subsequent printable characters will inherit this URI until it is cleared.
            match uri {
                None => {
                    // OSC 8 ;; — end hyperlink.
                    p.current_hyperlink = None;
                    p.current_hyperlink_id = None;
                }
                Some(uri_str) => {
                    let new_id: Option<Arc<str>> = id.map(|s| Arc::from(s.as_str()));
                    // FS-VT-072: if same ID as current hyperlink, reuse the existing
                    // Arc to keep identity stable across multi-line continuations.
                    let reuse = matches!(
                        (&p.current_hyperlink_id, &new_id),
                        (Some(existing), Some(new)) if existing == new
                    );
                    if reuse {
                        // Same ID → URI should be the same; keep existing Arc.
                    } else {
                        p.current_hyperlink = Some(Arc::from(uri_str.as_str()));
                        p.current_hyperlink_id = new_id;
                    }
                }
            }
        }
        OscAction::ClipboardWrite(text) => {
            // FS-VT-075 / SEC-OSC-002: forward only when the policy allows it.
            if p.allow_osc52_write {
                p.pending_osc52_write = Some(text);
            }
        }
        OscAction::SetCwd(path) => {
            // ADR-0028 §VT input caps: drop oversized OSC 7 updates outright
            // rather than truncate them — a truncated cwd would point to a
            // wrong directory, which is worse than no update at all.
            if path.len() > MAX_CWD_BYTES {
                tracing::warn!(
                    bytes = path.len(),
                    cap = MAX_CWD_BYTES,
                    "VT OSC 7 cwd update dropped: payload exceeds cap"
                );
                return;
            }
            if p.cwd.as_deref() != Some(path.as_str()) {
                p.cwd = Some(path);
                p.cwd_changed = true;
            }
        }
        OscAction::Ignore => {}
    }
}
