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

#[tauri::command]
pub async fn send_input(
    pane_id: PaneId,
    data: Vec<u8>,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), TauTermError> {
    if data.len() > SEND_INPUT_MAX_BYTES {
        return Err(TauTermError::new(
            "INPUT_TOO_LARGE",
            &format!(
                "Input payload exceeds the maximum allowed size of {} bytes.",
                SEND_INPUT_MAX_BYTES
            ),
        ));
    }
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
