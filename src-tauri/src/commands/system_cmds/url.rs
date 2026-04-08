// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use tauri::State;

use crate::error::TauTermError;
use crate::session::{SessionRegistry, ids::PaneId};

/// Open a URL in the system browser. Scheme is validated (§8.1 / FS-VT-073).
///
/// `pane_id` — the pane from which the link was activated. When provided, the
/// `file://` scheme is allowed only if that pane is a local PTY session. When
/// absent (e.g. called without a pane context), `file://` is always rejected.
#[tauri::command]
pub async fn open_url(
    url: String,
    pane_id: Option<String>,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), TauTermError> {
    let is_local = pane_id
        .as_deref()
        .map(|id| registry.is_local_pane(&PaneId(id.to_string())))
        .unwrap_or(false);
    validate_url_scheme(&url, is_local)?;
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| {
        TauTermError::with_detail(
            "OPEN_URL_FAILED",
            "Failed to open URL in browser.",
            e.to_string(),
        )
    })
}

/// Validate that a URL scheme is whitelisted (§8.1 / FS-VT-073).
///
/// `is_local_pty` — when `true`, the `file://` scheme is additionally allowed
/// (local PTY sessions may activate `file://` hyperlinks). When `false` (SSH
/// sessions or unknown context), `file://` is rejected as an information-
/// disclosure risk (SEC-PATH-004).
pub(super) fn validate_url_scheme(url: &str, is_local_pty: bool) -> Result<(), TauTermError> {
    const ALWAYS_ALLOWED: &[&str] = &["http", "https", "mailto", "ssh"];
    const MAX_URL_LEN: usize = 2048;

    if url.len() > MAX_URL_LEN {
        return Err(TauTermError::new(
            "INVALID_URL",
            "URL exceeds maximum allowed length.",
        ));
    }

    // Check for C0/C1 control characters.
    if url
        .chars()
        .any(|c| (c as u32) < 0x20 || (0x80..=0x9F).contains(&(c as u32)))
    {
        return Err(TauTermError::new(
            "INVALID_URL",
            "URL contains invalid control characters.",
        ));
    }

    let scheme = url
        .split_once(':')
        .map(|(s, _)| s)
        .unwrap_or("")
        .to_lowercase();

    if ALWAYS_ALLOWED.contains(&scheme.as_str()) {
        return Ok(());
    }

    // `file://` is only permitted for local PTY sessions (FS-VT-073).
    if scheme == "file" && is_local_pty {
        return Ok(());
    }

    Err(TauTermError::new(
        "INVALID_URL_SCHEME",
        "The URL scheme is not permitted.",
    ))
}
