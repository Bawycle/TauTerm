// SPDX-License-Identifier: MPL-2.0

//! Injectable SSH output registry — E2E testing only (`e2e-testing` feature).
//!
//! Provides `SshInjectableRegistry`, which maps `PaneId` →
//! `mpsc::Sender<ProcessOutput>`. The `inject_ssh_output` Tauri command uses
//! this registry to push VT-processed output into the SSH coalescer pipeline
//! for a pane, exactly as the real SSH reader (Task A in `ssh_task.rs`) would.
//!
//! The sender is registered by `create_mock_ssh_pane` (testing command) which
//! spins up a coalescer task (Task B) without a real russh channel, giving
//! E2E tests a fully functional SSH output pipeline without a network
//! connection.
//!
//! See also: `platform/pty_injectable.rs` for the analogous PTY mechanism.

#![cfg(feature = "e2e-testing")]

use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::session::ids::PaneId;
use crate::session::output::ProcessOutput;

/// Pane-to-sender map for SSH E2E injection.
///
/// Managed as Tauri state (`State<Arc<SshInjectableRegistry>>`) in
/// e2e-testing builds. `DashMap` provides interior mutability with
/// `Send + Sync`, matching the `InjectableRegistry` pattern for PTY.
pub struct SshInjectableRegistry {
    senders: DashMap<PaneId, mpsc::Sender<ProcessOutput>>,
}

impl SshInjectableRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            senders: DashMap::new(),
        }
    }

    /// Register the `ProcessOutput` sender for a pane.
    ///
    /// Called by `create_mock_ssh_pane` after spawning the coalescer task.
    pub(crate) fn register(&self, pane_id: PaneId, tx: mpsc::Sender<ProcessOutput>) {
        self.senders.insert(pane_id, tx);
    }

    /// Send a `ProcessOutput` to the pane's coalescer.
    ///
    /// Returns `Err` if the pane ID is not registered or the channel is closed.
    pub(crate) async fn send(&self, pane_id: &PaneId, output: ProcessOutput) -> Result<(), String> {
        match self.senders.get(pane_id) {
            None => Err(format!(
                "SSH pane not found in injectable registry: {pane_id}"
            )),
            Some(tx) => tx
                .send(output)
                .await
                .map_err(|_| format!("SSH injectable channel closed for pane {pane_id}")),
        }
    }

    /// Remove the sender for a closed pane.
    pub(crate) fn remove(&self, pane_id: &PaneId) {
        self.senders.remove(pane_id);
    }
}

impl Default for SshInjectableRegistry {
    fn default() -> Self {
        Self::new()
    }
}
