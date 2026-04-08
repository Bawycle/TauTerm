// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use crate::vt::osc::{OscAction, parse_osc};
use crate::vt::processor::VtProcessor;

/// Maximum number of entries in the title stack (OSC 22/23).
/// Matches xterm's default. Excess pushes are silently ignored.
pub(super) const TITLE_STACK_MAX: usize = 16;

pub(super) fn handle_osc(p: &mut VtProcessor, params: &[&[u8]]) {
    // Guard: silently ignore oversized OSC sequences to prevent DoS.
    // total_len accounts for each field plus one byte per separator.
    let total_len: usize = params.iter().map(|p| p.len() + 1).sum::<usize>();
    if total_len > 8192 {
        return;
    }
    match parse_osc(params) {
        OscAction::SetTitle(title) => {
            if p.title != title {
                p.title = title;
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
        OscAction::Ignore => {}
    }
}
