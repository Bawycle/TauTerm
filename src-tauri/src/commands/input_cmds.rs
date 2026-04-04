// SPDX-License-Identifier: MPL-2.0

//! Input and terminal I/O Tauri commands.
//!
//! Commands: send_input, scroll_pane, scroll_to_bottom, search_pane,
//!           get_pane_screen_snapshot, resize_pane.

use std::sync::Arc;

use tauri::State;

use crate::error::TauTermError;
use crate::session::{SessionRegistry, ids::PaneId, registry::ScrollPositionState};
use crate::vt::{SearchMatch, SearchQuery, screen_buffer::ScreenSnapshot};

/// Maximum payload size for a single `send_input` call (64 KiB).
/// Prevents DoS via oversized IPC payloads (FINDING-003 / SEC-IPC-002).
const SEND_INPUT_MAX_BYTES: usize = 65_536;

/// Validate that the input payload does not exceed the size limit.
///
/// Extracted as a pure function so it can be unit-tested without Tauri state
/// (SEC-IPC-006 / FINDING-003).
fn validate_input_size(data: &[u8]) -> Result<(), TauTermError> {
    if data.len() > SEND_INPUT_MAX_BYTES {
        return Err(TauTermError::new(
            "INVALID_INPUT_SIZE",
            "Input payload exceeds maximum allowed size of 64 KiB",
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn send_input(
    pane_id: PaneId,
    data: Vec<u8>,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), TauTermError> {
    validate_input_size(&data)?;
    registry
        .send_input(pane_id, data)
        .map_err(TauTermError::from)
}

#[tauri::command]
pub async fn scroll_pane(
    pane_id: PaneId,
    offset: i64,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<ScrollPositionState, TauTermError> {
    registry
        .scroll_pane(pane_id, offset)
        .map_err(TauTermError::from)
}

#[tauri::command]
pub async fn scroll_to_bottom(
    pane_id: PaneId,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), TauTermError> {
    let _ = registry
        .scroll_pane(pane_id, 0)
        .map_err(TauTermError::from)?;
    Ok(())
}

#[tauri::command]
pub async fn search_pane(
    pane_id: PaneId,
    query: SearchQuery,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<Vec<SearchMatch>, TauTermError> {
    let inner = registry
        .get_pane_snapshot(&pane_id)
        .map_err(TauTermError::from)?;
    // TODO: run search on the VtProcessor directly rather than on the snapshot.
    let _ = (inner, query);
    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_pane_screen_snapshot(
    pane_id: PaneId,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<ScreenSnapshot, TauTermError> {
    registry
        .get_pane_snapshot(&pane_id)
        .map_err(TauTermError::from)
}

/// Resize a pane's PTY. Debounced by the backend (§6.5).
#[tauri::command]
pub async fn resize_pane(
    pane_id: PaneId,
    cols: u16,
    rows: u16,
    pixel_width: u16,
    pixel_height: u16,
    _registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), TauTermError> {
    // TODO: forward to pane's resize debouncer.
    let _ = (pane_id, cols, rows, pixel_width, pixel_height);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // SEC-IPC-006 — send_input rejects oversized payloads (FINDING-003)
    // -----------------------------------------------------------------------

    /// SEC-IPC-006: Payload of exactly the limit must be accepted.
    #[test]
    fn sec_ipc_006_send_input_at_size_limit_accepted() {
        let data = vec![0u8; SEND_INPUT_MAX_BYTES];
        assert!(
            validate_input_size(&data).is_ok(),
            "Payload at exact limit ({} bytes) must be accepted",
            SEND_INPUT_MAX_BYTES
        );
    }

    /// SEC-IPC-006: Payload of 65537 bytes (limit + 1) must be rejected with INVALID_INPUT_SIZE.
    #[test]
    fn sec_ipc_006_send_input_oversized_payload_rejected() {
        let data = vec![0u8; SEND_INPUT_MAX_BYTES + 1];
        let result = validate_input_size(&data);
        assert!(
            result.is_err(),
            "Oversized payload ({} bytes) must be rejected (SEC-IPC-006 / FINDING-003)",
            data.len()
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code, "INVALID_INPUT_SIZE",
            "Error code must be INVALID_INPUT_SIZE. Got: {}",
            err.code
        );
    }

    /// SEC-IPC-006: Empty payload must be accepted.
    #[test]
    fn sec_ipc_006_empty_payload_accepted() {
        let data: Vec<u8> = Vec::new();
        assert!(
            validate_input_size(&data).is_ok(),
            "Empty payload must be accepted"
        );
    }
}
