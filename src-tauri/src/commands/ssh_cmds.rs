// SPDX-License-Identifier: MPL-2.0

//! SSH session management Tauri commands.
//!
//! Commands: open_ssh_connection, close_ssh_connection, reconnect_ssh.

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::{AppHandle, State};

use crate::error::TauTermError;
use crate::preferences::PreferencesStore;
use crate::session::SessionRegistry;
use crate::session::ids::{ConnectionId, PaneId};
use crate::ssh::SshManager;

#[tauri::command]
pub async fn open_ssh_connection(
    pane_id: PaneId,
    connection_id: ConnectionId,
    ssh_manager: State<'_, Arc<SshManager>>,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
    registry: State<'_, Arc<SessionRegistry>>,
    app: AppHandle,
) -> Result<(), TauTermError> {
    let config = {
        let store = prefs.read();
        let preferences = store.get();
        preferences
            .connections
            .into_iter()
            .find(|c| c.id == connection_id)
            .ok_or_else(|| {
                TauTermError::new(
                    "CONNECTION_NOT_FOUND",
                    "The specified SSH connection was not found.",
                )
            })?
    };

    // Retrieve the pane's VtProcessor and dimensions for the SSH read task.
    let vt = registry.get_pane_vt(&pane_id).map_err(TauTermError::from)?;
    let (cols, rows) = registry
        .get_pane_dims(&pane_id)
        .map_err(TauTermError::from)?;

    // Path validation is performed inside SshManager::open_connection (FINDING-004).
    ssh_manager
        .open_connection(pane_id, &config, None, app, vt, cols, rows)
        .await
        .map_err(TauTermError::from)
}

#[tauri::command]
pub async fn close_ssh_connection(
    pane_id: PaneId,
    ssh_manager: State<'_, Arc<SshManager>>,
) -> Result<(), TauTermError> {
    ssh_manager
        .close_connection(pane_id)
        .await
        .map_err(TauTermError::from)
}

#[tauri::command]
pub async fn reconnect_ssh(
    pane_id: PaneId,
    ssh_manager: State<'_, Arc<SshManager>>,
    registry: State<'_, Arc<SessionRegistry>>,
    app: AppHandle,
) -> Result<(), TauTermError> {
    let vt = registry.get_pane_vt(&pane_id).map_err(TauTermError::from)?;
    let (cols, rows) = registry
        .get_pane_dims(&pane_id)
        .map_err(TauTermError::from)?;

    ssh_manager
        .reconnect(pane_id, app, vt, cols, rows)
        .await
        .map_err(TauTermError::from)
}
