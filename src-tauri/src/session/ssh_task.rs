// SPDX-License-Identifier: MPL-2.0

//! SSH channel read task — one async Tokio task per SSH pane.
//!
//! Mirrors `session/pty_task.rs` but operates on an async `russh::Channel`
//! instead of a blocking PTY reader. The read loop processes `ChannelMsg::Data`
//! messages, feeds bytes to `VtProcessor`, and emits `screen-update` events.
//!
//! Task lifecycle:
//! - Spawned by `SshManager::connect_task` after the PTY channel is opened.
//! - Aborted via `SshTaskHandle::abort()` or on drop.
//! - Terminates naturally when the channel sends `ChannelMsg::Eof` or `Close`.

use std::sync::Arc;

use parking_lot::RwLock;
use russh::ChannelMsg;
use tauri::AppHandle;

use crate::events::{
    SshStateChangedEvent, emit_mode_state_changed, emit_screen_update, emit_session_state_changed,
    emit_ssh_state_changed, types::SessionStateChangedEvent,
};
use crate::session::ids::PaneId;
use crate::session::pty_task::{build_mode_state_event, build_screen_update_event};
use crate::session::registry::SessionRegistry;
use crate::ssh::SshLifecycleState;
use crate::vt::VtProcessor;

/// Handle to a running SSH read task. Dropping this handle aborts the task.
pub struct SshTaskHandle {
    pub(crate) abort: tokio::task::AbortHandle,
}

impl SshTaskHandle {
    /// Wrap an `AbortHandle` in an `SshTaskHandle`.
    pub fn from_abort_handle(abort: tokio::task::AbortHandle) -> Self {
        Self { abort }
    }

    /// Abort the SSH read task.
    pub fn abort(&self) {
        self.abort.abort();
    }
}

impl Drop for SshTaskHandle {
    fn drop(&mut self) {
        self.abort.abort();
    }
}

/// Spawn an async SSH channel read task.
///
/// `channel` — the russh channel, wrapped in `Arc<tokio::sync::Mutex<...>>` so it
/// can be shared with the write path. The read task holds the mutex for the
/// duration of each `wait()` call, which is non-blocking in the async sense.
///
/// Returns an `SshTaskHandle` that aborts the task on drop.
pub fn spawn_ssh_read_task(
    pane_id: PaneId,
    vt: Arc<RwLock<VtProcessor>>,
    app: AppHandle,
    channel: Arc<tokio::sync::Mutex<russh::Channel<russh::client::Msg>>>,
    registry: Arc<SessionRegistry>,
) -> SshTaskHandle {
    let task = tokio::spawn(async move {
        loop {
            let msg = {
                let mut ch = channel.lock().await;
                ch.wait().await
            };

            match msg {
                Some(ChannelMsg::Data { ref data }) => {
                    let bytes: &[u8] = data;
                    let (dirty, mode_changed, new_title) = {
                        let mut proc = vt.write();
                        let dirty = proc.process(bytes);
                        let changed = proc.mode_changed;
                        if changed {
                            proc.mode_changed = false;
                        }
                        let title = proc.take_title_changed();
                        (dirty, changed, title)
                    };
                    if mode_changed {
                        let event = build_mode_state_event(&pane_id, &vt);
                        emit_mode_state_changed(&app, event);
                    }
                    if let Some(title) = new_title
                        && let Some(tab_state) = registry.update_pane_title(&pane_id, title)
                    {
                        emit_session_state_changed(
                            &app,
                            SessionStateChangedEvent::PaneMetadataChanged { tab: tab_state },
                        );
                    }
                    if !dirty.is_empty() {
                        let event = build_screen_update_event(&pane_id, &vt, &dirty);
                        emit_screen_update(&app, event);
                    }
                }
                Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                    // stderr from the remote shell — feed to VT processor so it
                    // appears in the terminal (same as PTY stderr mixing).
                    let bytes: &[u8] = data;
                    let (dirty, mode_changed, new_title) = {
                        let mut proc = vt.write();
                        let dirty = proc.process(bytes);
                        let changed = proc.mode_changed;
                        if changed {
                            proc.mode_changed = false;
                        }
                        let title = proc.take_title_changed();
                        (dirty, changed, title)
                    };
                    if mode_changed {
                        let event = build_mode_state_event(&pane_id, &vt);
                        emit_mode_state_changed(&app, event);
                    }
                    if let Some(title) = new_title
                        && let Some(tab_state) = registry.update_pane_title(&pane_id, title)
                    {
                        emit_session_state_changed(
                            &app,
                            SessionStateChangedEvent::PaneMetadataChanged { tab: tab_state },
                        );
                    }
                    if !dirty.is_empty() {
                        let event = build_screen_update_event(&pane_id, &vt, &dirty);
                        emit_screen_update(&app, event);
                    }
                }
                Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                    tracing::debug!("SSH channel closed for pane {pane_id}");
                    emit_ssh_state_changed(
                        &app,
                        SshStateChangedEvent {
                            pane_id,
                            state: SshLifecycleState::Closed,
                        },
                    );
                    return;
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    tracing::debug!("SSH shell exited with status {exit_status} on pane {pane_id}");
                    // Continue reading until Eof/Close to drain any remaining output.
                }
                Some(_) => {
                    // Other messages (WindowAdjust, etc.) — ignore silently.
                }
            }
        }
    });

    SshTaskHandle {
        abort: task.abort_handle(),
    }
}
