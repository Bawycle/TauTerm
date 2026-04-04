// SPDX-License-Identifier: MPL-2.0

//! System-level Tauri commands.
//!
//! Commands: copy_to_clipboard, get_clipboard, open_url,
//!           mark_context_menu_used, get_session_state.

use std::sync::Arc;

use tauri::State;

use crate::error::TauTermError;
use crate::session::{SessionRegistry, SessionState};

#[tauri::command]
pub async fn get_session_state(
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<SessionState, TauTermError> {
    Ok(registry.get_state_snapshot())
}

#[tauri::command]
pub async fn copy_to_clipboard(text: String) -> Result<(), TauTermError> {
    // TODO: forward to ClipboardBackend.
    let _ = text;
    Ok(())
}

#[tauri::command]
pub async fn get_clipboard() -> Result<String, TauTermError> {
    // TODO: forward to ClipboardBackend.
    Ok(String::new())
}

/// Open a URL in the system browser. Scheme is validated (§8.1).
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), TauTermError> {
    validate_url_scheme(&url)?;
    // TODO: open_url via tauri-plugin-opener.
    let _ = url;
    Ok(())
}

#[tauri::command]
pub async fn mark_context_menu_used() -> Result<(), TauTermError> {
    // TODO: persist "context menu hint shown" flag in preferences.
    Ok(())
}

/// Validate that a URL scheme is whitelisted (§8.1).
fn validate_url_scheme(url: &str) -> Result<(), TauTermError> {
    const ALLOWED_SCHEMES: &[&str] = &["http", "https", "mailto", "ssh"];
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

    if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
        return Err(TauTermError::new(
            "INVALID_URL_SCHEME",
            "The URL scheme is not permitted.",
        ));
    }

    Ok(())
}
