// SPDX-License-Identifier: MPL-2.0

//! SSH channel read pipeline: async reader (Task A) feeding the shared
//! coalescer (Task B = `session::output::run`) plus an SSH-specific
//! termination block.
//!
//! Mirrors `session/pty_task/reader.rs` but operates on an async
//! `russh::Channel` instead of a blocking PTY reader. The reader processes
//! `ChannelMsg::Data` and `ChannelMsg::ExtendedData`, feeds bytes to
//! `VtProcessor`, writes coalesced DSR/CPR/DA responses back to the SSH
//! channel, and forwards the resulting [`ProcessOutput`] through a bounded
//! `mpsc` channel (capacity 256) to the shared coalescer task.
//!
//! ## Architecture (post-ADR-0028 Commit 3)
//!
//! - **Task A (reader)** — `tokio::spawn`ed async loop. Owns the SSH channel
//!   write side. For each chunk, calls [`extract_process_output`] to drain
//!   every VT side-effect in a single write-lock window, writes any pending
//!   VT responses (CPR/DSR/DA) back through `channel.lock().await; ch.data()`
//!   AFTER the VT lock has been released, then forwards the chunk to Task B
//!   via the bounded `mpsc::Sender<ProcessOutput>`. On `Eof`/`Close`/`None`
//!   the sender is dropped, signalling EOF to the coalescer.
//!
//! - **Task B (coalescer)** — `tauri::async_runtime::spawn`ed (NOT
//!   `tokio::spawn`) async loop. Runs [`crate::session::output::run`], which
//!   coalesces `ProcessOutput` chunks on an adaptive debounce interval and
//!   honours the frame-ack two-stage backpressure machinery from ADR-0027.
//!   When `run` returns (sender dropped → EOF), this task performs the
//!   SSH-specific termination block: mute `pane.ssh_state = Closed` →
//!   emit `SshLifecycleState::Closed` event → emit `ProcessExited`
//!   notification (parity with PTY for FS-NOTIF-002).
//!
//! ## Lock-ordering invariant
//!
//! The VT write-lock is acquired and released entirely inside
//! [`extract_process_output`] — by code shape, the guard cannot escape the
//! helper. Task A therefore has no way to hold the VT lock across the
//! subsequent `channel.lock().await`. This prevents the lock-inversion
//! between the VT write-lock and the SSH channel mutex documented in
//! ADR-0028 §Security (Risk #1).
//!
//! ## DSR/CPR response coalescing
//!
//! VT responses (CPR/DSR/DA replies) generated during VT processing are
//! collected from the helper and merged into a single `Vec<u8>`, then
//! written via one `ch.data(merged_bytes).await` call AFTER the VT lock has
//! been released. This both reduces SSH channel write syscalls and avoids
//! any lock-inversion risk between the VT write-lock and the SSH channel
//! mutex (ADR-0028 Decisions §10).
//!
//! ## Termination semantics (ADR-0028 Decisions §4)
//!
//! Caller-managed: the coalescer (`session::output::run`) performs no
//! source-specific cleanup. SSH-specific termination runs in Task B AFTER
//! the coalescer returns:
//!
//! 1. `pane.ssh_state` is mutated to `Closed` via
//!    `SessionRegistry::set_ssh_state` (state mutation BEFORE events so
//!    inspectors observe the new state).
//! 2. `ssh-state-changed` event is emitted with `SshLifecycleState::Closed`.
//! 3. `notification-changed` is emitted with
//!    `PaneNotificationDto::ProcessExited { exit_code: None, signal_name: None }`.
//!    Parity with PTY (FS-NOTIF-002). SSH has no `Child`, so `exit_code` is
//!    `None`. `ChannelMsg::ExitStatus` is logged (Task A) but not propagated —
//!    future improvement if needed.
//!
//! Order: state mutation FIRST, then events; Closed BEFORE ProcessExited.
//!
//! ## Task lifecycle
//!
//! - Spawned by `SshManager::connect_task` after the PTY channel is opened.
//! - Aborted via `SshTaskHandle::abort()` or on drop (aborts BOTH tasks).
//! - Terminates naturally when the channel sends `ChannelMsg::Eof` or `Close`.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use parking_lot::RwLock;
use russh::ChannelMsg;
use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::events::{
    SshStateChangedEvent, emit_notification_changed, emit_ssh_state_changed,
    types::{NotificationChangedEvent, PaneNotificationDto},
};
use crate::session::ids::PaneId;
use crate::session::output::{Coalescer, CoalescerConfig, CoalescerContext, ProcessOutput, run};
use crate::session::registry::SessionRegistry;
use crate::ssh::SshLifecycleState;
use crate::vt::VtProcessor;

#[cfg(test)]
mod tests;

/// Handle to the running SSH read/emit task pair.
///
/// Dropping this handle aborts both tasks. `abort()` does the same explicitly.
/// The two-handle layout mirrors `PtyTaskHandle` post-ADR-0028 Commit 3.
pub struct SshTaskHandle {
    read_abort: tokio::task::AbortHandle,
    emit_abort: tokio::task::AbortHandle,
}

impl SshTaskHandle {
    /// Wrap two `AbortHandle`s into an `SshTaskHandle`.
    ///
    /// Used in production by [`spawn_ssh_read_task`] and in tests that
    /// construct synthetic abort handles (TEST-ASYNC-SSH-001/002).
    pub fn new(read_abort: tokio::task::AbortHandle, emit_abort: tokio::task::AbortHandle) -> Self {
        Self {
            read_abort,
            emit_abort,
        }
    }

    /// Abort both the reader task and the coalescer task.
    pub fn abort(&self) {
        self.read_abort.abort();
        self.emit_abort.abort();
    }
}

impl Drop for SshTaskHandle {
    fn drop(&mut self) {
        self.read_abort.abort();
        self.emit_abort.abort();
    }
}

/// Process one chunk of bytes through the VT processor and extract every
/// side-effect (`ProcessOutput` + pending VT responses) in a single
/// write-lock window.
///
/// ## Lock-ordering invariant (load-bearing)
///
/// The VT `RwLock` write-guard is acquired at the top of this function and
/// dropped at the closing brace — by code shape, the guard cannot escape
/// the helper. Callers therefore have no way to hold the VT lock across
/// any subsequent `channel.lock().await` call. This is the key invariant
/// that prevents the VT/SSH-channel lock-inversion documented in
/// ADR-0028 §Security (Risk #1). Do not reshape this helper to return
/// the guard or accept a callback that runs while the guard is alive.
///
/// `bytes` is processed once; the function returns:
/// - the coalesced [`ProcessOutput`] (every `take_*()` extraction merged),
/// - the list of VT responses (CPR/DSR/DA) generated during processing —
///   the caller is responsible for merging and writing them back to the SSH
///   channel **after** this helper returns (lock-ordering rule above).
pub(crate) fn extract_process_output(
    vt: &Arc<RwLock<VtProcessor>>,
    bytes: &[u8],
) -> (ProcessOutput, Vec<Vec<u8>>) {
    let mut proc = vt.write();
    let dirty = proc.process(bytes);
    let mode_changed = proc.mode_changed;
    if mode_changed {
        proc.mode_changed = false;
    }
    let new_title = proc.take_title_changed();
    let new_cursor_shape = proc.take_cursor_shape_changed();
    let bell = proc.take_bell_pending();
    let osc52 = proc.take_osc52_write();
    let new_cwd = proc.take_cwd_changed();
    let responses = proc.take_responses();
    (
        ProcessOutput {
            dirty,
            mode_changed,
            new_title,
            new_cursor_shape,
            bell,
            osc52,
            new_cwd,
            needs_immediate_flush: !responses.is_empty(),
        },
        responses,
    )
    // VT write-lock guard `proc` drops here.
}

/// Spawn the SSH pipeline for a pane: async reader (Task A) + shared
/// coalescer (Task B) + SSH-specific termination block.
///
/// `channel` — the russh channel, wrapped in `Arc<tokio::sync::Mutex<...>>` so it
/// can be shared with the write path. The reader holds the mutex for the
/// duration of each `wait()` call, which is non-blocking in the async sense.
///
/// `last_frame_ack_ms` — per-pane atomic clock cloned via
/// `SessionRegistry::get_pane_frame_ack_clock`. The coalescer reads this on
/// every tick to drive frame-ack two-stage backpressure (ADR-0027). Updated
/// by the `frame_ack` IPC command on the frontend side.
///
/// Returns an [`SshTaskHandle`] that aborts BOTH tasks on drop or `abort()`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ssh_read_task(
    pane_id: PaneId,
    vt: Arc<RwLock<VtProcessor>>,
    app: AppHandle,
    channel: Arc<tokio::sync::Mutex<russh::Channel<russh::client::Msg>>>,
    registry: Arc<SessionRegistry>,
    last_frame_ack_ms: Arc<AtomicU64>,
) -> SshTaskHandle {
    // Bounded channel: backpressure to the SSH read loop when the coalescer is
    // slow. INVARIANT: the VT write-lock is always released before `tx.send`,
    // so no deadlock is possible between Task A and the coalescer.
    let (tx, rx) = mpsc::channel::<ProcessOutput>(256);

    // ------------------------------------------------------------------
    // Task A — async reader
    // ------------------------------------------------------------------
    //
    // SECURITY (no SSH-bytes logging): per `src-tauri/CLAUDE.md`, never log
    // raw SSH chunk contents at any tracing level — they may contain user
    // passwords typed into prompts.
    let pane_id_r = pane_id.clone();
    let vt_r = vt.clone();
    let channel_r = Arc::clone(&channel);

    let read_task = tokio::spawn(async move {
        loop {
            let msg = {
                let mut ch = channel_r.lock().await;
                ch.wait().await
            };

            match msg {
                Some(ChannelMsg::Data { ref data })
                | Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                    // Single VT write-lock window: process bytes + drain every
                    // side-effect. The helper releases the VT lock at its
                    // closing brace BEFORE we touch the SSH channel mutex
                    // below — see `extract_process_output` doc-comment.
                    let (output, responses) = extract_process_output(&vt_r, data);

                    // Coalesce DSR/CPR/DA responses into a single contiguous
                    // write. `pending_responses` is bounded upstream
                    // (ADR-0028 Decisions §9 — Commit 4 for the actual cap),
                    // so the merged buffer cannot grow without bound.
                    //
                    // SAFETY (lock ordering): the VT write-lock from
                    // `extract_process_output` is already released by the
                    // time we reach this `channel_r.lock().await`. Inverting
                    // the order would introduce the deadlock documented in
                    // ADR-0028 Risk #1.
                    if !responses.is_empty() {
                        let merged: Vec<u8> = responses.into_iter().flatten().collect();
                        let ch = channel_r.lock().await;
                        if let Err(e) = ch.data(merged.as_slice()).await {
                            tracing::warn!(
                                "Failed to write VT responses on SSH pane {pane_id_r}: {e}"
                            );
                        }
                    }

                    // Forward to the coalescer. If the coalescer has exited
                    // (channel closed), stop the reader.
                    if tx.send(output).await.is_err() {
                        tracing::debug!(
                            "SSH coalescer task closed, stopping reader for pane {pane_id_r}"
                        );
                        return;
                    }
                }
                Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                    tracing::debug!("SSH channel closed for pane {pane_id_r}");
                    // Drop the sender by returning. The coalescer task observes
                    // `recv() = None`, flushes any remaining pending output,
                    // and runs the SSH-specific termination block.
                    return;
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    tracing::debug!(
                        "SSH shell exited with status {exit_status} on pane {pane_id_r}"
                    );
                    // Continue reading until Eof/Close to drain any remaining output.
                    // Exit status is not propagated (no PTY-style Child to wait on);
                    // the ProcessExited notification emitted in Task B uses
                    // exit_code = None — see ADR-0028 Decisions §4.
                }
                Some(_) => {
                    // Other messages (WindowAdjust, etc.) — ignore silently.
                }
            }
        }
        // `tx` (the only Sender clone) drops here, signalling EOF to the
        // coalescer task in Task B.
    });

    // ------------------------------------------------------------------
    // Task B — shared coalescer + SSH-specific termination block
    // ------------------------------------------------------------------
    //
    // CRITICAL: spawned via `tauri::async_runtime::spawn` (NOT `tokio::spawn`)
    // to preserve the scheduling/ordering invariants relied upon by
    // TEST-ACK-018, TEST-ACK-019, all TEST-ADPT-*, and DEL-ASYNC-PTY-009.
    // See ADR-0028 Decisions §2 and Risk #13.
    let pane_id_e = pane_id.clone();
    let app_e = app.clone();
    let registry_e = registry.clone();

    let config = CoalescerConfig::ssh_default();
    let coalescer = Coalescer::new(&config);
    let ctx = CoalescerContext {
        app,
        pane_id,
        vt,
        registry,
        last_frame_ack_ms,
        config,
    };

    let emit_task = tauri::async_runtime::spawn(async move {
        run(coalescer, ctx, rx).await;

        // SSH-specific termination — runs AFTER the coalescer has flushed any
        // remaining pending output and returned. Caller-managed termination
        // model per ADR-0028 Decisions §4.
        //
        // Order: state mutation FIRST, then events; Closed BEFORE ProcessExited.
        // This guarantees that any frontend handler observing the event also
        // sees `pane.ssh_state = Closed` if it queries the registry.
        registry_e.set_ssh_state(&pane_id_e, SshLifecycleState::Closed);
        emit_ssh_state_changed(
            &app_e,
            SshStateChangedEvent {
                pane_id: pane_id_e.clone(),
                state: SshLifecycleState::Closed,
            },
        );
        // FS-NOTIF-002 parity with PTY: emit a ProcessExited notification on
        // SSH disconnect. SSH has no `Child` — `exit_code` and `signal_name`
        // are both `None`. `ChannelMsg::ExitStatus` is logged in Task A but
        // not propagated (future improvement).
        if let Some((_, tab_state)) = registry_e.get_tab_state_for_pane(&pane_id_e) {
            emit_notification_changed(
                &app_e,
                NotificationChangedEvent {
                    tab_id: tab_state.id,
                    pane_id: pane_id_e.clone(),
                    notification: Some(PaneNotificationDto::ProcessExited {
                        exit_code: None,
                        signal_name: None,
                    }),
                },
            );
        }

        tracing::debug!("SSH emit task finished for pane {pane_id_e}");
    });

    SshTaskHandle::new(read_task.abort_handle(), emit_task.inner().abort_handle())
}
