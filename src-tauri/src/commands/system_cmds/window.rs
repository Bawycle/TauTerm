// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::State;

use crate::error::TauTermError;
use crate::events::types::FullscreenState;
use crate::events::{FullscreenStateChangedEvent, emit_fullscreen_state_changed};
use crate::preferences::PreferencesStore;

/// Toggle full-screen mode on the main window (FS-FULL-009).
///
/// Queries the current full-screen state, flips it, persists the preference,
/// and emits a `fullscreen-state-changed` event after a short delay that lets
/// the WM confirm the geometry transition.
///
/// SIGWINCH is intentionally NOT broadcast here — the frontend ResizeObserver
/// will observe the geometry change and call `resize_pane` → `ResizeDebouncer`.
#[tauri::command]
pub async fn toggle_fullscreen(
    window: tauri::Window,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
    app: tauri::AppHandle,
) -> Result<FullscreenState, TauTermError> {
    let currently = window.is_fullscreen().map_err(|e| {
        TauTermError::with_detail(
            "FULLSCREEN_QUERY_FAILED",
            "Could not determine current window state.",
            e.to_string(),
        )
    })?;
    let target = !currently;

    window.set_fullscreen(target).map_err(|e| {
        TauTermError::with_detail(
            "FULLSCREEN_SET_FAILED",
            "Could not change window full-screen state.",
            e.to_string(),
        )
    })?;

    // FS-FULL-009: persist immediately so the state survives restarts.
    prefs.read().set_fullscreen(target).map_err(|e| {
        TauTermError::with_detail(
            "PREFERENCES_ERROR",
            "Failed to persist full-screen preference.",
            e.to_string(),
        )
    })?;

    // Emit the informational event after a brief delay so the WM has time to
    // confirm the geometry transition before the frontend reads window dimensions.
    let app_clone = app.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        emit_fullscreen_state_changed(
            &app_clone,
            FullscreenStateChangedEvent {
                is_fullscreen: target,
            },
        );
    });

    Ok(FullscreenState {
        is_fullscreen: target,
    })
}
