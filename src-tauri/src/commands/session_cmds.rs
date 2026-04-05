// SPDX-License-Identifier: MPL-2.0

//! Session management Tauri commands.
//!
//! Commands: create_tab, close_tab, rename_tab, reorder_tab,
//!           split_pane, close_pane, set_active_pane.

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::error::TauTermError;
use crate::events::{SessionChangeType, SessionStateChangedEvent, emit_session_state_changed};
use crate::session::{
    SessionRegistry,
    ids::{PaneId, TabId},
    registry::CreateTabConfig,
    tab::{SplitDirection, TabState},
};

#[tauri::command]
pub async fn create_tab(
    config: CreateTabConfig,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<TabState, TauTermError> {
    registry.create_tab(config).map_err(TauTermError::from)
}

#[tauri::command]
pub async fn close_tab(
    tab_id: TabId,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), TauTermError> {
    registry.close_tab(tab_id).map_err(TauTermError::from)
}

#[tauri::command]
pub async fn rename_tab(
    tab_id: TabId,
    label: Option<String>,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<TabState, TauTermError> {
    registry
        .rename_tab(tab_id, label)
        .map_err(TauTermError::from)
}

#[tauri::command]
pub async fn reorder_tab(
    tab_id: TabId,
    new_order: u32,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), TauTermError> {
    registry
        .reorder_tab(tab_id, new_order)
        .map_err(TauTermError::from)
}

#[tauri::command]
pub async fn split_pane(
    pane_id: PaneId,
    direction: SplitDirection,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<TabState, TauTermError> {
    registry
        .split_pane(pane_id, direction)
        .map_err(TauTermError::from)
}

#[tauri::command]
pub async fn close_pane(
    pane_id: PaneId,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<Option<TabState>, TauTermError> {
    registry.close_pane(pane_id).map_err(TauTermError::from)
}

#[tauri::command]
pub async fn set_active_tab(
    tab_id: TabId,
    registry: State<'_, Arc<SessionRegistry>>,
    app: AppHandle,
) -> Result<(), TauTermError> {
    let tab_state = registry
        .set_active_tab(tab_id.clone())
        .map_err(TauTermError::from)?;

    emit_session_state_changed(
        &app,
        SessionStateChangedEvent {
            change_type: SessionChangeType::ActiveTabChanged,
            tab: Some(tab_state),
            active_tab_id: Some(tab_id.to_string()),
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn set_active_pane(
    pane_id: PaneId,
    registry: State<'_, Arc<SessionRegistry>>,
    app: AppHandle,
) -> Result<(), TauTermError> {
    let tab_state = registry
        .set_active_pane(pane_id)
        .map_err(TauTermError::from)?;

    emit_session_state_changed(
        &app,
        SessionStateChangedEvent {
            change_type: SessionChangeType::ActivePaneChanged,
            tab: Some(tab_state),
            active_tab_id: None,
        },
    );

    Ok(())
}
