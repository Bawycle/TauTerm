// SPDX-License-Identifier: MPL-2.0

//! SSH session management Tauri commands.
//!
//! Commands: open_ssh_connection, close_ssh_connection, reconnect_ssh.

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::{AppHandle, State};

use crate::credentials::CredentialManager;
use crate::error::TauTermError;
use crate::preferences::PreferencesStore;
use crate::session::SessionRegistry;
use crate::session::ids::{ConnectionId, PaneId};
use crate::ssh::{Credentials, SshManager};

/// Inner implementation — shared by both the production and e2e-testing variants.
async fn open_ssh_connection_impl(
    pane_id: PaneId,
    connection_id: ConnectionId,
    ssh_manager: &Arc<SshManager>,
    prefs: &Arc<RwLock<PreferencesStore>>,
    registry: &Arc<SessionRegistry>,
    credential_manager: &Arc<CredentialManager>,
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

    // Look up stored password from OS keychain (FS-CRED-001, FS-CRED-005).
    // Never log credential values.
    let credentials = if credential_manager.is_available() {
        match credential_manager
            .get_password(&config.id.to_string(), &config.username)
            .await
        {
            Ok(Some(password)) => Some(Credentials {
                username: config.username.to_string(),
                password: Some(password),
                private_key_path: config.identity_file.clone(),
                save_in_keychain: false,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("Keychain lookup failed for {}: {e}", config.host);
                None
            }
        }
    } else {
        None
    };

    // Retrieve the pane's VtProcessor and dimensions for the SSH read task.
    let vt = registry.get_pane_vt(&pane_id).map_err(TauTermError::from)?;
    let (cols, rows) = registry
        .get_pane_dims(&pane_id)
        .map_err(TauTermError::from)?;

    // Apply per-connection OSC 52 write policy — overrides the global preference
    // for this SSH pane and protects it from future propagate_osc52_allow calls (arch §8.2).
    registry.apply_pane_osc52_override(&pane_id, config.allow_osc52_write);

    // Path validation is performed inside SshManager::open_connection (FINDING-004).
    ssh_manager
        .open_connection(
            pane_id,
            &config,
            credentials,
            app,
            vt,
            cols,
            rows,
            Arc::clone(registry),
        )
        .await
        .map_err(TauTermError::from)
}

/// Production variant — registered when `e2e-testing` feature is NOT active.
#[cfg(not(feature = "e2e-testing"))]
#[tauri::command]
pub async fn open_ssh_connection(
    pane_id: PaneId,
    connection_id: ConnectionId,
    ssh_manager: State<'_, Arc<SshManager>>,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
    registry: State<'_, Arc<SessionRegistry>>,
    credential_manager: State<'_, Arc<CredentialManager>>,
    app: AppHandle,
) -> Result<(), TauTermError> {
    open_ssh_connection_impl(
        pane_id,
        connection_id,
        &ssh_manager,
        &prefs,
        &registry,
        &credential_manager,
        app,
    )
    .await
}

/// E2E testing variant — registered when `e2e-testing` feature IS active.
///
/// Checks the `SshFailureRegistry` before proceeding; if the pane was armed
/// with `inject_ssh_failure`, returns a synthetic error immediately so E2E
/// tests can assert the rollback path without needing a real SSH server.
#[cfg(feature = "e2e-testing")]
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn open_ssh_connection(
    pane_id: PaneId,
    connection_id: ConnectionId,
    ssh_manager: State<'_, Arc<SshManager>>,
    prefs: State<'_, Arc<RwLock<PreferencesStore>>>,
    registry: State<'_, Arc<SessionRegistry>>,
    credential_manager: State<'_, Arc<CredentialManager>>,
    app: AppHandle,
    failure_registry: State<'_, Arc<crate::commands::testing::SshFailureRegistry>>,
) -> Result<(), TauTermError> {
    if failure_registry.consume() {
        return Err(TauTermError::new(
            "E2E_INJECTED_FAILURE",
            "Synthetic SSH failure injected by E2E test.",
        ));
    }
    open_ssh_connection_impl(
        pane_id,
        connection_id,
        &ssh_manager,
        &prefs,
        &registry,
        &credential_manager,
        app,
    )
    .await
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
        .reconnect(pane_id, app, vt, cols, rows, Arc::clone(&*registry))
        .await
        .map_err(TauTermError::from)
}
