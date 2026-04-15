// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::State;

use crate::error::TauTermError;
use crate::preferences::PreferencesStore;

#[tauri::command]
#[specta::specta]
pub async fn mark_context_menu_used(
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
) -> Result<(), TauTermError> {
    prefs.read().mark_context_menu_used().map_err(|e| {
        TauTermError::with_detail(
            "PREFERENCES_ERROR",
            "Failed to persist context menu flag.",
            e.to_string(),
        )
    })
}
