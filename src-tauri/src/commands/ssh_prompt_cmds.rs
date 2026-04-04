// SPDX-License-Identifier: MPL-2.0

//! SSH prompt response Tauri commands.
//!
//! Commands: provide_credentials, accept_host_key, reject_host_key,
//!           dismiss_ssh_algorithm_warning.

use crate::error::TauTermError;
use crate::session::ids::PaneId;
use crate::ssh::manager::Credentials;

#[tauri::command]
pub async fn provide_credentials(
    pane_id: PaneId,
    credentials: Credentials,
) -> Result<(), TauTermError> {
    // TODO: forward credentials to the pending SSH auth flow for this pane.
    let _ = (pane_id, credentials);
    Ok(())
}

#[tauri::command]
pub async fn accept_host_key(pane_id: PaneId) -> Result<(), TauTermError> {
    // TODO: record host key in known_hosts and resume connection.
    let _ = pane_id;
    Ok(())
}

#[tauri::command]
pub async fn reject_host_key(pane_id: PaneId) -> Result<(), TauTermError> {
    // TODO: abort the connection for this pane.
    let _ = pane_id;
    Ok(())
}

#[tauri::command]
pub async fn dismiss_ssh_algorithm_warning(pane_id: PaneId) -> Result<(), TauTermError> {
    // TODO: clear the algorithm warning for this pane.
    let _ = pane_id;
    Ok(())
}
