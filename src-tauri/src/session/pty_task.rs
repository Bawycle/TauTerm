// SPDX-License-Identifier: MPL-2.0

//! PTY read task — one Tokio task per pane.
//!
//! Reads bytes from the PTY master fd asynchronously, feeds them to
//! `VtProcessor`, coalesces dirty regions, and emits `screen-update` events
//! to the frontend via `AppHandle` (§6.2 of ARCHITECTURE.md).
//!
//! Back-pressure: all available bytes are processed before emitting a single
//! event. Rate limiting is a future improvement (§6.5).

// This module contains the scaffolding for the PTY read task.
// Full implementation requires `portable-pty` integration (platform module).
// The task handle type and spawn function are defined here; the actual
// async loop is wired up in `session/registry.rs` when a pane is created.

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::AppHandle;

use crate::session::ids::PaneId;
use crate::vt::VtProcessor;

/// Handle to a running PTY read task.
/// Dropping this handle signals the task to stop.
pub struct PtyTaskHandle {
    abort: tokio::task::AbortHandle,
}

impl PtyTaskHandle {
    /// Abort the PTY read task.
    pub fn abort(&self) {
        self.abort.abort();
    }
}

impl Drop for PtyTaskHandle {
    fn drop(&mut self) {
        self.abort.abort();
    }
}

/// Spawn a PTY read task.
///
/// `reader` — an `AsyncRead` source (typically the PTY master fd wrapped in
/// `tokio::io::unix::AsyncFd`). The concrete type is provided by the platform
/// module (`platform/pty_linux.rs`).
///
/// Returns a `PtyTaskHandle` that aborts the task on drop.
pub fn spawn_pty_read_task(
    pane_id: PaneId,
    vt: Arc<RwLock<VtProcessor>>,
    app: AppHandle,
    mut reader: impl tokio::io::AsyncRead + Unpin + Send + 'static,
) -> PtyTaskHandle {
    use tokio::io::AsyncReadExt;

    let task = tokio::spawn(async move {
        let mut buf = vec![0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => {
                    tracing::debug!("PTY EOF on pane {pane_id}");
                    break;
                }
                Ok(n) => {
                    let dirty = {
                        let mut proc = vt.write();
                        proc.process(&buf[..n])
                    };
                    if !dirty.is_empty() {
                        // Emit screen-update event.
                        // TODO: build full ScreenUpdateEvent from dirty region + VtProcessor snapshot.
                        let _ = &app;
                        let _ = &dirty;
                    }
                }
                Err(e) => {
                    tracing::error!("PTY read error on pane {pane_id}: {e}");
                    break;
                }
            }
        }
    });

    PtyTaskHandle {
        abort: task.abort_handle(),
    }
}
