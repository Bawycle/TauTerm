// SPDX-License-Identifier: MPL-2.0

use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use parking_lot::RwLock;
use tauri::AppHandle;
use tokio::sync::mpsc::Sender;
use tokio::time::Duration;

use crate::events::{
    emit_notification_changed,
    types::{NotificationChangedEvent, PaneNotificationDto},
};
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::vt::VtProcessor;

use crate::vt::DirtyRegion;

use super::emitter::emit_all_pending;
use super::{ProcessOutput, PtyTaskHandle};

/// Minimum debounce window — floor for adaptive scaling and idle decay.
pub(crate) const DEBOUNCE_MIN: Duration = Duration::from_millis(12);

/// Maximum debounce window — cap to avoid perceptible input latency.
pub(crate) const DEBOUNCE_MAX: Duration = Duration::from_millis(100);

/// Multiplier applied to the measured emit duration to compute the next
/// debounce interval. A value slightly above 1.0 gives the frontend a
/// comfortable margin to process the event before the next one arrives.
const DEBOUNCE_SCALE: f64 = 1.2;

/// Decay factor applied on idle ticks (no pending output). Exponentially
/// shrinks the debounce interval back toward `DEBOUNCE_MIN` when the PTY
/// is quiet, ensuring low latency for interactive use after a burst.
const DEBOUNCE_DECAY: f64 = 0.5;

/// Ack age above which debounce is escalated (Stage 1).
pub(crate) const ACK_STALE_THRESHOLD_MS: u64 = 200;

/// Debounce interval during stale-ack mode.
pub(crate) const ACK_STALE_DEBOUNCE: Duration = Duration::from_millis(250);

/// Ack age above which dirty updates are dropped (Stage 2).
pub(crate) const ACK_DROP_THRESHOLD_MS: u64 = 1000;

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute the next debounce interval from the measured emit duration.
///
/// The result is `emit_duration * DEBOUNCE_SCALE`, clamped to
/// `[DEBOUNCE_MIN, DEBOUNCE_MAX]`.
pub(crate) fn next_debounce(emit_duration: Duration) -> Duration {
    emit_duration
        .mul_f64(DEBOUNCE_SCALE)
        .clamp(DEBOUNCE_MIN, DEBOUNCE_MAX)
}

// ---------------------------------------------------------------------------
// spawn_pty_read_task
// ---------------------------------------------------------------------------

/// Spawn the two-task PTY pipeline for a pane.
///
/// `writer` is the PTY master writer used to send DSR/DA/CPR responses back to
/// the shell. It is `None` for sessions that do not support writing back (e.g.
/// injectable E2E sessions), in which case responses are silently discarded.
///
/// Returns a `PtyTaskHandle` that aborts both tasks on drop.
pub fn spawn_pty_read_task(
    pane_id: PaneId,
    vt: Arc<RwLock<VtProcessor>>,
    app: AppHandle,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    registry: Arc<SessionRegistry>,
    last_frame_ack_ms: Arc<AtomicU64>,
) -> PtyTaskHandle {
    // Bounded channel: back-pressure to PTY kernel when Task 2 is slow.
    // INVARIANT: the VtProcessor write-lock is always released before blocking_send,
    // so no deadlock is possible between Task 1 and the IPC layer.
    let (tx, rx) = tokio::sync::mpsc::channel::<ProcessOutput>(256);

    // ------------------------------------------------------------------
    // Task 1 — blocking reader
    // ------------------------------------------------------------------
    let pane_id_r = pane_id.clone();
    let vt_r = vt.clone();
    let tx_r: Sender<ProcessOutput> = tx;
    // Clone the writer Arc so it can be moved into the spawn_blocking closure.
    // `None` for sessions that do not support writing back (e.g. injectable).
    let writer_r = writer;

    let read_task = tauri::async_runtime::spawn_blocking(move || {
        let mut buf = vec![0u8; 4096];

        loop {
            // Read from PTY master — blocking call. Lock is held only for the
            // duration of the read so the write-lock on `vt` can proceed after.
            let n = {
                let mut rdr = match reader.lock() {
                    Ok(g) => g,
                    Err(_) => {
                        tracing::error!("PTY reader mutex poisoned on pane {pane_id_r}");
                        break;
                    }
                };
                match rdr.read(&mut buf) {
                    Ok(0) => {
                        tracing::debug!("PTY EOF on pane {pane_id_r}");
                        break;
                    }
                    Ok(n) => n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                        // EINTR — interrupted by signal delivery, retry immediately.
                        continue;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // EAGAIN/EWOULDBLOCK — should not occur on a blocking PTY fd, but
                        // defensive: yield to avoid busy-spinning and retry.
                        std::thread::yield_now();
                        continue;
                    }
                    Err(e) => {
                        // EIO (process exited, master fd dead) or other fatal read error.
                        // Treat as end-of-session — the emit task will fire ProcessExited.
                        tracing::debug!("PTY read ended on pane {pane_id_r}: {e}");
                        break;
                    }
                }
            }; // read lock released here

            // Process bytes through the VT processor. The write-lock is held
            // only for the duration of processing and side-effect extraction —
            // not across the channel send or the response writes.
            let (output, responses) = {
                let mut proc = vt_r.write();
                let dirty = proc.process(&buf[..n]);
                let mode_changed = proc.mode_changed;
                if mode_changed {
                    proc.mode_changed = false;
                }
                let new_title = proc.take_title_changed();
                let new_cursor_shape = proc.take_cursor_shape_changed();
                let bell = proc.take_bell_pending();
                let osc52 = proc.take_osc52_write();
                let new_cwd = proc.take_cwd_changed();
                // Extract DSR/DA/CPR responses while still holding the write-lock.
                // They are written to the PTY master AFTER the lock is released
                // to prevent a deadlock when the shell echoes back immediately.
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
            }; // write-lock released here

            // Write DSR/DA/CPR responses back to the PTY master.
            // The write-lock on `vt` is no longer held here, so there is no
            // risk of deadlocking when the shell sends back a reply that would
            // trigger a new `vt.write()` acquisition in this same task.
            if !responses.is_empty()
                && let Some(ref w) = writer_r
            {
                match w.lock() {
                    Ok(mut writer) => {
                        for resp in &responses {
                            if let Err(e) = writer.write_all(resp) {
                                tracing::warn!(
                                    "Failed to write VT response to PTY master on pane \
                                     {pane_id_r}: {e}"
                                );
                            }
                        }
                    }
                    Err(_) => {
                        tracing::error!("PTY writer mutex poisoned on pane {pane_id_r}");
                    }
                }
            }

            // Forward to Task 2. If Task 2 has exited (channel closed), stop.
            if tx_r.blocking_send(output).is_err() {
                tracing::debug!("PTY emit task closed, stopping reader for pane {pane_id_r}");
                break;
            }
        }

        // Channel sender is dropped here, signalling EOF to Task 2.
        tracing::debug!("PTY reader task finished for pane {pane_id_r}");
    });

    // ------------------------------------------------------------------
    // Task 2 — async coalescer / emitter
    // ------------------------------------------------------------------
    let pane_id_e = pane_id.clone();
    let vt_e = vt.clone();
    let app_e = app.clone();
    let registry_e = registry.clone();
    let ack_ms_e = last_frame_ack_ms;
    let mut rx_e = rx;

    let emit_task = tauri::async_runtime::spawn(async move {
        let mut pending = ProcessOutput::default();
        let mut current_debounce = DEBOUNCE_MIN;
        let mut was_in_drop_mode = false;
        let sleep_fut = tokio::time::sleep(current_debounce);
        tokio::pin!(sleep_fut);

        loop {
            tokio::select! {
                // Receive a chunk from Task 1.
                msg = rx_e.recv() => {
                    match msg {
                        Some(output) => {
                            let flush_now = output.needs_immediate_flush;
                            pending.merge(output);
                            if flush_now {
                                // CPR/DSR response was sent — bypass debounce to update cursor
                                // state promptly. Tools like vim/neovim/fzf use CPR to sync
                                // their rendering and will stall until this event arrives.
                                // Drain any concurrently buffered output to avoid splitting the
                                // update across two events.
                                while let Ok(more) = rx_e.try_recv() {
                                    pending.merge(more);
                                }
                                if !pending.is_empty() {
                                    let emit_duration = emit_all_pending(
                                        &app_e,
                                        &pane_id_e,
                                        &vt_e,
                                        &registry_e,
                                        &mut pending,
                                    );
                                    current_debounce = next_debounce(emit_duration);
                                } else {
                                    // Nothing to emit, but clear the flag to avoid stale hints.
                                    pending.needs_immediate_flush = false;
                                }
                                // Re-arm sleep from now with updated period.
                                sleep_fut.as_mut().reset(tokio::time::Instant::now() + current_debounce);
                            }
                        }
                        None => {
                            // Channel closed — Task 1 finished (EOF or error).
                            // Flush any remaining pending output before exiting.
                            if !pending.is_empty() {
                                let _ = emit_all_pending(
                                    &app_e,
                                    &pane_id_e,
                                    &vt_e,
                                    &registry_e,
                                    &mut pending,
                                );
                            }
                            break;
                        }
                    }
                }

                // Adaptive debounce timer — flush accumulated output.
                _ = &mut sleep_fut => {
                    // Drain any output buffered during this tick before emitting.
                    // Prevents splitting application redraw bursts (e.g. CSI 2J + redraw)
                    // across two separate screen-update events. try_recv() is non-blocking
                    // and returns Err immediately when the channel is empty.
                    while let Ok(output) = rx_e.try_recv() {
                        pending.merge(output);
                    }

                    // P-HT-6: frame-ack backpressure.
                    let ack_age_ms = now_ms().saturating_sub(ack_ms_e.load(Ordering::Relaxed));
                    let in_drop_mode = ack_age_ms > ACK_DROP_THRESHOLD_MS;
                    let in_stale_mode = ack_age_ms > ACK_STALE_THRESHOLD_MS;

                    if !pending.is_empty() {
                        if in_drop_mode {
                            // Stage 2: suppress dirty cell updates + cursor_moved.
                            // Non-visual events preserved: mode_changed, new_cursor_shape,
                            // bell, osc52, new_title, new_cwd.
                            pending.dirty = DirtyRegion::default();
                            pending.needs_immediate_flush = false;
                        } else if was_in_drop_mode {
                            // Exiting drop mode: frontend grid is stale. Force full redraw.
                            pending.dirty.is_full_redraw = true;
                        }

                        if !pending.is_empty() {
                            let emit_duration = emit_all_pending(
                                &app_e,
                                &pane_id_e,
                                &vt_e,
                                &registry_e,
                                &mut pending,
                            );
                            current_debounce = if in_stale_mode {
                                ACK_STALE_DEBOUNCE
                            } else {
                                next_debounce(emit_duration)
                            };
                        } else {
                            // All content was dirty-only and was dropped.
                            pending = ProcessOutput::default();
                        }
                    } else {
                        // Idle tick: exponential decay toward minimum.
                        // No stale escalation on idle ticks.
                        current_debounce = DEBOUNCE_MIN.max(current_debounce.mul_f64(DEBOUNCE_DECAY));
                    }

                    was_in_drop_mode = in_drop_mode;

                    // Always re-arm after timer fires.
                    sleep_fut.as_mut().reset(tokio::time::Instant::now() + current_debounce);
                }
            }
        }

        // FS-NOTIF-002: PTY process exited — transition to Terminated and get exit info.
        // mark_pane_terminated() calls wait() on the child to recover the exit
        // code and sets pane.lifecycle = Terminated before we emit the notification.
        let (exit_code, signal_name) =
            tokio::task::block_in_place(|| registry_e.mark_pane_terminated(&pane_id_e));
        if let Some((_, tab_state)) = registry_e.get_tab_state_for_pane(&pane_id_e) {
            emit_notification_changed(
                &app_e,
                NotificationChangedEvent {
                    tab_id: tab_state.id,
                    pane_id: pane_id_e.clone(),
                    notification: Some(PaneNotificationDto::ProcessExited {
                        exit_code,
                        signal_name,
                    }),
                },
            );
        }

        tracing::debug!("PTY emit task finished for pane {pane_id_e}");
    });

    PtyTaskHandle::new(
        read_task.inner().abort_handle(),
        emit_task.inner().abort_handle(),
    )
}
