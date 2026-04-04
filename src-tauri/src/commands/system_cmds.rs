// SPDX-License-Identifier: MPL-2.0

//! System-level Tauri commands.
//!
//! Commands: copy_to_clipboard, get_clipboard, open_url,
//!           mark_context_menu_used, get_session_state.

use std::sync::Arc;

use tauri::State;

use crate::error::TauTermError;
use crate::session::{SessionRegistry, SessionState};

/// Maximum clipboard text size: 16 MiB.
///
/// Protects against clipboard-based DoS where a process in a pane triggers
/// a large clipboard write via OSC 52 or direct IPC call.
const MAX_CLIPBOARD_LEN: usize = 16 * 1024 * 1024;

#[tauri::command]
pub async fn get_session_state(
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<SessionState, TauTermError> {
    Ok(registry.get_state_snapshot())
}

#[tauri::command]
pub async fn copy_to_clipboard(text: String) -> Result<(), TauTermError> {
    if text.len() > MAX_CLIPBOARD_LEN {
        return Err(TauTermError::new(
            "CLIPBOARD_TOO_LARGE",
            "Clipboard text exceeds maximum allowed size.",
        ));
    }
    tokio::task::spawn_blocking(move || {
        let mut cb = arboard::Clipboard::new().map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_UNAVAILABLE",
                "Could not access the system clipboard.",
                e.to_string(),
            )
        })?;
        cb.set_text(&text).map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_WRITE_FAILED",
                "Failed to write to clipboard.",
                e.to_string(),
            )
        })
    })
    .await
    .map_err(|e| TauTermError::with_detail("INTERNAL_ERROR", "Clipboard task failed.", e.to_string()))?
}

#[tauri::command]
pub async fn get_clipboard() -> Result<String, TauTermError> {
    tokio::task::spawn_blocking(|| {
        let mut cb = arboard::Clipboard::new().map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_UNAVAILABLE",
                "Could not access the system clipboard.",
                e.to_string(),
            )
        })?;
        cb.get_text().map_err(|e| {
            TauTermError::with_detail(
                "CLIPBOARD_READ_FAILED",
                "Failed to read from clipboard.",
                e.to_string(),
            )
        })
    })
    .await
    .map_err(|e| TauTermError::with_detail("INTERNAL_ERROR", "Clipboard task failed.", e.to_string()))?
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

#[cfg(test)]
mod security_tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SEC-PATH-003 — URL scheme allowlist enforced (javascript:, data:, blob:)
    // -----------------------------------------------------------------------

    /// SEC-PATH-003: javascript: scheme must be rejected.
    #[test]
    fn sec_path_003_javascript_scheme_rejected() {
        let result = validate_url_scheme("javascript:alert(1)");
        assert!(
            result.is_err(),
            "javascript: scheme must be rejected (SEC-PATH-003)"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code, "INVALID_URL_SCHEME",
            "Error code must be INVALID_URL_SCHEME"
        );
    }

    /// SEC-PATH-003: data: scheme must be rejected.
    #[test]
    fn sec_path_003_data_scheme_rejected() {
        let result = validate_url_scheme("data:text/html,<script>alert(1)</script>");
        assert!(
            result.is_err(),
            "data: scheme must be rejected (SEC-PATH-003)"
        );
    }

    /// SEC-PATH-003: blob: scheme must be rejected.
    #[test]
    fn sec_path_003_blob_scheme_rejected() {
        let result = validate_url_scheme("blob:http://example.com/uuid");
        assert!(
            result.is_err(),
            "blob: scheme must be rejected (SEC-PATH-003)"
        );
    }

    /// SEC-PATH-003: vbscript: scheme must be rejected.
    #[test]
    fn sec_path_003_vbscript_scheme_rejected() {
        let result = validate_url_scheme("vbscript:msgbox(1)");
        assert!(
            result.is_err(),
            "vbscript: scheme must be rejected (SEC-PATH-003)"
        );
    }

    /// SEC-PATH-003: Unknown custom scheme must be rejected.
    #[test]
    fn sec_path_003_custom_scheme_rejected() {
        let result = validate_url_scheme("foobar:something");
        assert!(
            result.is_err(),
            "Unknown custom scheme must be rejected (SEC-PATH-003)"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-PATH-004 — file:// scheme rejected
    // -----------------------------------------------------------------------

    /// SEC-PATH-004: file:// URIs must be rejected (information disclosure risk).
    #[test]
    fn sec_path_004_file_scheme_rejected() {
        let result = validate_url_scheme("file:///etc/passwd");
        assert!(
            result.is_err(),
            "file:// scheme must be rejected (SEC-PATH-004)"
        );
    }

    /// SEC-PATH-004: file:// with traversal must also be rejected.
    #[test]
    fn sec_path_004_file_scheme_with_traversal_rejected() {
        let result = validate_url_scheme("file:///../../etc/shadow");
        assert!(result.is_err(), "file:// with traversal must be rejected");
    }

    // -----------------------------------------------------------------------
    // Allowed schemes pass validation
    // -----------------------------------------------------------------------

    /// Allowed scheme: https
    #[test]
    fn sec_path_003_https_scheme_allowed() {
        let result = validate_url_scheme("https://example.com");
        assert!(result.is_ok(), "https: scheme must be allowed");
    }

    /// Allowed scheme: http
    #[test]
    fn sec_path_003_http_scheme_allowed() {
        let result = validate_url_scheme("http://example.com");
        assert!(result.is_ok(), "http: scheme must be allowed");
    }

    /// Allowed scheme: mailto
    #[test]
    fn sec_path_003_mailto_scheme_allowed() {
        let result = validate_url_scheme("mailto:user@example.com");
        assert!(result.is_ok(), "mailto: scheme must be allowed");
    }

    /// Allowed scheme: ssh
    #[test]
    fn sec_path_003_ssh_scheme_allowed() {
        let result = validate_url_scheme("ssh://user@host");
        assert!(result.is_ok(), "ssh: scheme must be allowed");
    }

    // -----------------------------------------------------------------------
    // URL length limit
    // -----------------------------------------------------------------------

    /// URL exceeding 2048 characters is rejected.
    #[test]
    fn sec_url_length_limit_enforced() {
        let long_url = format!("https://example.com/{}", "a".repeat(2049));
        let result = validate_url_scheme(&long_url);
        assert!(
            result.is_err(),
            "URLs longer than 2048 chars must be rejected"
        );
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_URL", "Error code must be INVALID_URL");
    }

    /// URL of exactly 2048 characters is accepted.
    #[test]
    fn sec_url_at_length_limit_accepted() {
        // Build a URL of exactly 2048 bytes.
        // Base: "https://x.com/" = 15 chars.
        // Pad: 2048 - 15 = 2033 chars of 'a'.
        let base = "https://x.com/";
        let path = "a".repeat(2048 - base.len());
        let url = format!("{}{}", base, path);
        assert_eq!(
            url.len(),
            2048,
            "Test construction error: URL must be exactly 2048 chars"
        );
        let result = validate_url_scheme(&url);
        assert!(result.is_ok(), "URL of exactly 2048 chars must be accepted");
    }

    // -----------------------------------------------------------------------
    // Control character injection in URL
    // -----------------------------------------------------------------------

    /// URL containing C0 control characters must be rejected.
    #[test]
    fn sec_url_control_chars_rejected() {
        let result = validate_url_scheme("https://ex\x01ample.com");
        assert!(
            result.is_err(),
            "URL with C0 control chars must be rejected"
        );
    }

    /// URL containing C1 control characters must be rejected.
    #[test]
    fn sec_url_c1_control_chars_rejected() {
        let result = validate_url_scheme("https://ex\u{0080}ample.com");
        assert!(
            result.is_err(),
            "URL with C1 control chars must be rejected"
        );
    }

    // -----------------------------------------------------------------------
    // SEC-IPC-005 — Language enum rejects unknown variants (schema enforcement)
    // -----------------------------------------------------------------------

    /// SEC-IPC-005: Unknown language string "de" must fail serde deserialization.
    #[test]
    fn sec_ipc_005_unknown_language_variant_rejected_by_serde() {
        use crate::preferences::schema::Language;
        let result: Result<Language, _> = serde_json::from_str("\"de\"");
        assert!(
            result.is_err(),
            "Unknown language variant 'de' must fail deserialization (SEC-IPC-005)"
        );
    }

    /// SEC-IPC-005: SQL injection payload as language value must fail deserialization.
    #[test]
    fn sec_ipc_005_language_injection_payload_rejected() {
        use crate::preferences::schema::Language;
        let result: Result<Language, _> =
            serde_json::from_str("\"en'; DROP TABLE preferences; --\"");
        assert!(
            result.is_err(),
            "SQL injection payload as language must be rejected (SEC-IPC-005)"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-IPC-CLIP-001 — copy_to_clipboard rejects oversized payloads
    // -----------------------------------------------------------------------

    /// TEST-IPC-CLIP-001: copy_to_clipboard with text exceeding MAX_CLIPBOARD_LEN
    /// must return CLIPBOARD_TOO_LARGE without touching arboard.
    #[tokio::test]
    async fn ipc_clip_001_copy_to_clipboard_rejects_oversized_payload() {
        let oversized = "x".repeat(MAX_CLIPBOARD_LEN + 1);
        let result = copy_to_clipboard(oversized).await;
        assert!(
            result.is_err(),
            "Oversized clipboard payload must be rejected (TEST-IPC-CLIP-001)"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code, "CLIPBOARD_TOO_LARGE",
            "Error code must be CLIPBOARD_TOO_LARGE"
        );
    }

    /// TEST-IPC-CLIP-002: copy_to_clipboard with empty text must be accepted
    /// (validation layer only — actual clipboard write is environment-dependent).
    #[tokio::test]
    async fn ipc_clip_002_copy_to_clipboard_accepts_empty_string() {
        // Empty string passes validation; arboard may fail in headless CI —
        // we only assert the validation layer does not reject it.
        let result = copy_to_clipboard(String::new()).await;
        // In a headless CI without X11/Wayland, arboard returns an error —
        // that is acceptable. We assert it is not a CLIPBOARD_TOO_LARGE error.
        if let Err(ref err) = result {
            assert_ne!(
                err.code, "CLIPBOARD_TOO_LARGE",
                "Empty string must not be rejected as too large (TEST-IPC-CLIP-002)"
            );
        }
    }

    /// TEST-IPC-CLIP-003: copy_to_clipboard at exactly MAX_CLIPBOARD_LEN must pass validation.
    #[tokio::test]
    async fn ipc_clip_003_copy_to_clipboard_accepts_at_limit() {
        let at_limit = "x".repeat(MAX_CLIPBOARD_LEN);
        let result = copy_to_clipboard(at_limit).await;
        if let Err(ref err) = result {
            assert_ne!(
                err.code, "CLIPBOARD_TOO_LARGE",
                "Text at exactly MAX_CLIPBOARD_LEN must pass size validation (TEST-IPC-CLIP-003)"
            );
        }
    }

    // -----------------------------------------------------------------------
    // TEST-IPC-CLIP-004 — get_clipboard returns Ok or a non-CLIPBOARD_TOO_LARGE error
    // -----------------------------------------------------------------------

    /// TEST-IPC-CLIP-004: get_clipboard must not panic. In headless CI it may
    /// return an error, but it must never return a CLIPBOARD_TOO_LARGE code
    /// (which applies only to writes).
    #[tokio::test]
    async fn ipc_clip_004_get_clipboard_does_not_panic() {
        let result = get_clipboard().await;
        if let Err(ref err) = result {
            assert_ne!(
                err.code, "CLIPBOARD_TOO_LARGE",
                "get_clipboard must never return CLIPBOARD_TOO_LARGE (read-only operation)"
            );
        }
        // If Ok, any string is acceptable.
    }
}
