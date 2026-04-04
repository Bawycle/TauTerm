// SPDX-License-Identifier: MPL-2.0

//! SSH session management Tauri commands.
//!
//! Commands: open_ssh_connection, close_ssh_connection, reconnect_ssh.

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::State;

use crate::error::TauTermError;
use crate::platform::validation::validate_ssh_identity_path;
use crate::preferences::PreferencesStore;
use crate::session::ids::{ConnectionId, PaneId};
use crate::ssh::SshManager;

#[tauri::command]
pub async fn open_ssh_connection(
    pane_id: PaneId,
    connection_id: ConnectionId,
    ssh_manager: State<'_, Arc<SshManager>>,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
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

    // Validate identity file path before passing to SshManager (FINDING-004).
    if let Some(ref identity_file) = config.identity_file {
        validate_ssh_identity_path(identity_file)?;
    }

    ssh_manager
        .open_connection(pane_id, &config, None)
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
) -> Result<(), TauTermError> {
    ssh_manager
        .reconnect(pane_id)
        .await
        .map_err(TauTermError::from)
}
