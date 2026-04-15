// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use tauri::State;

use crate::error::TauTermError;
use crate::session::{SessionRegistry, SessionState};

#[tauri::command]
#[specta::specta]
pub async fn get_session_state(
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<SessionState, TauTermError> {
    Ok(registry.get_state_snapshot())
}
