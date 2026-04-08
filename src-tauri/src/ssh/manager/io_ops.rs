// SPDX-License-Identifier: MPL-2.0

use crate::error::SshError;
use crate::session::ids::PaneId;

use super::{Credentials, SshManager};

impl SshManager {
    /// Send input bytes to the SSH PTY channel for a pane.
    pub async fn send_input(&self, pane_id: &PaneId, data: Vec<u8>) -> Result<(), SshError> {
        let conn = self
            .connections
            .get(pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        conn.send_input(data).await
    }

    /// Resize the SSH PTY channel for a pane.
    pub async fn resize_pane(
        &self,
        pane_id: &PaneId,
        cols: u16,
        rows: u16,
        px_w: u16,
        px_h: u16,
    ) -> Result<(), SshError> {
        let conn = self
            .connections
            .get(pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;
        conn.resize(cols, rows, px_w, px_h).await
    }

    /// Deliver credentials to a pending SSH auth prompt for a pane.
    ///
    /// The connect task parks a oneshot sender in `pending_credentials` while
    /// waiting for the user. This method resolves it.
    pub fn provide_credentials(
        &self,
        pane_id: &PaneId,
        creds: Credentials,
    ) -> Result<(), SshError> {
        let (_, pending) = self
            .pending_credentials
            .remove(pane_id)
            .ok_or_else(|| SshError::NoPendingCredentials(pane_id.to_string()))?;
        // Ignore send error — the connect task may have timed out already.
        let _ = pending.sender.send(creds);
        Ok(())
    }

    /// Close the SSH session for a pane.
    ///
    /// Sends a clean `Disconnect` to the server before dropping the handle,
    /// so the remote end sees a proper close rather than a TCP reset.
    pub async fn close_connection(&self, pane_id: PaneId) -> Result<(), SshError> {
        // Drop the pending credential prompt if any (unblocks the connect task).
        self.pending_credentials.remove(&pane_id);

        let (_, conn) = self
            .connections
            .remove(&pane_id)
            .ok_or_else(|| SshError::PaneNotFound(pane_id.to_string()))?;

        // Abort the read task before touching the handle.
        drop(conn.read_task);

        // Send a clean disconnect to the server.
        let mut guard = conn.handle.lock().await;
        if let Some(handle) = guard.take() {
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "user close", "en")
                .await;
        }

        Ok(())
    }
}
