// SPDX-License-Identifier: MPL-2.0

//! Session management Tauri commands.
//!
//! Commands: create_tab, close_tab, rename_tab, reorder_tab,
//!           split_pane, close_pane, set_active_pane.

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::error::{SshError, TauTermError};
use crate::events::{SessionStateChangedEvent, emit_session_state_changed};
use crate::session::{
    SessionRegistry,
    ids::{PaneId, TabId},
    registry::CreateTabConfig,
    tab::{SplitDirection, TabState},
};
use crate::ssh::SshManager;

#[tauri::command]
#[specta::specta]
pub async fn create_tab(
    config: CreateTabConfig,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<TabState, TauTermError> {
    registry.create_tab(config).map_err(TauTermError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn close_tab(
    tab_id: TabId,
    registry: State<'_, Arc<SessionRegistry>>,
    ssh_manager: State<'_, Arc<SshManager>>,
    app: AppHandle,
) -> Result<(), TauTermError> {
    // Close any SSH connections for panes in this tab before removing the tab
    // from the registry. Best-effort: PaneNotFound is expected for PTY panes.
    for pane_id in registry.get_tab_pane_ids(&tab_id) {
        if let Err(e) = ssh_manager.close_connection(pane_id).await
            && !matches!(e, SshError::PaneNotFound(_))
        {
            tracing::warn!("close_tab: SSH close_connection failed: {e}");
        }
    }

    let new_active_tab_id = registry
        .close_tab(tab_id.clone())
        .map_err(TauTermError::from)?;

    emit_session_state_changed(
        &app,
        SessionStateChangedEvent::TabClosed {
            closed_tab_id: tab_id,
            active_tab_id: new_active_tab_id,
        },
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
pub async fn set_pane_label(
    pane_id: PaneId,
    label: Option<String>,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<TabState, TauTermError> {
    registry
        .rename_pane(pane_id, label)
        .map_err(TauTermError::from)
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
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
#[specta::specta]
pub async fn close_pane(
    pane_id: PaneId,
    registry: State<'_, Arc<SessionRegistry>>,
    ssh_manager: State<'_, Arc<SshManager>>,
) -> Result<Option<TabState>, TauTermError> {
    // Close any SSH connection for this pane before removing it. Best-effort.
    if let Err(e) = ssh_manager.close_connection(pane_id.clone()).await
        && !matches!(e, SshError::PaneNotFound(_))
    {
        tracing::warn!("close_pane: SSH close_connection failed: {e}");
    }
    registry.close_pane(pane_id).map_err(TauTermError::from)
}

#[tauri::command]
#[specta::specta]
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
        SessionStateChangedEvent::ActiveTabChanged {
            tab: tab_state,
            active_tab_id: tab_id,
        },
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
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
        SessionStateChangedEvent::ActivePaneChanged { tab: tab_state },
    );

    Ok(())
}

/// FS-PTY-008: Detect whether a non-shell foreground process is active in a pane.
///
/// Returns `true` when the PTY foreground process group (from `tcgetpgrp`) differs
/// from the shell's PID — indicating that the user has run a command that has not
/// yet returned to the shell prompt.
///
/// Returns `false` when:
/// - The pane does not exist.
/// - The pane is not in `Running` state (already terminated, closing, etc.).
/// - The pane is an SSH pane (no local PTY master fd).
/// - The shell PID is unknown (session type does not track it).
///
/// The `tcgetpgrp` syscall is performed inside the registry, delegated through
/// `PtySession::foreground_pgid()` in the platform layer — no `unsafe` here.
#[tauri::command]
#[specta::specta]
pub async fn has_foreground_process(
    pane_id: PaneId,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<bool, TauTermError> {
    registry
        .has_foreground_process(&pane_id)
        .map_err(TauTermError::from)
}
