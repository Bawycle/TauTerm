// SPDX-License-Identifier: MPL-2.0

//! PTY read pipeline: blocking reader (Task 1) + shared async coalescer
//! (`session/output::run`, formerly Task 2) + PTY-specific termination.

use std::io::{Read, Write};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use parking_lot::RwLock;
use tauri::AppHandle;
use tokio::sync::mpsc::Sender;

use crate::events::{
    emit_notification_changed,
    types::{NotificationChangedEvent, PaneNotificationDto},
};
use crate::session::ids::PaneId;
use crate::session::output::{Coalescer, CoalescerConfig, CoalescerContext, ProcessOutput, run};
use crate::session::registry::SessionRegistry;
use crate::vt::VtProcessor;

use super::PtyTaskHandle;

// ---------------------------------------------------------------------------
// spawn_pty_read_task
// ---------------------------------------------------------------------------

/// Spawn the PTY pipeline for a pane: blocking reader (Task 1) + shared
/// async coalescer + PTY-specific termination block.
///
/// `writer` is the PTY master writer used to send DSR/DA/CPR responses back to
/// the shell. It is `None` for sessions that do not support writing back (e.g.
/// injectable E2E sessions), in which case responses are silently discarded.
///
/// Returns a [`PtyTaskHandle`] that aborts both tasks on drop.
pub fn spawn_pty_read_task(
    pane_id: PaneId,
    vt: Arc<RwLock<VtProcessor>>,
    app: AppHandle,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    registry: Arc<SessionRegistry>,
    last_frame_ack_ms: Arc<AtomicU64>,
) -> PtyTaskHandle {
    // Bounded channel: back-pressure to PTY kernel when the coalescer is slow.
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

            // Forward to the coalescer. If the coalescer has exited
            // (channel closed), stop.
            if tx_r.blocking_send(output).is_err() {
                tracing::debug!("PTY emit task closed, stopping reader for pane {pane_id_r}");
                break;
            }
        }

        // Channel sender is dropped here, signalling EOF to the coalescer.
        tracing::debug!("PTY reader task finished for pane {pane_id_r}");
    });

    // ------------------------------------------------------------------
    // Coalescer — shared async task (formerly Task 2)
    // ------------------------------------------------------------------
    //
    // CRITICAL: spawned via `tauri::async_runtime::spawn` (NOT
    // `tokio::spawn`) to preserve the scheduling/ordering invariants
    // relied upon by TEST-ACK-018, TEST-ACK-019, all TEST-ADPT-*, and
    // DEL-ASYNC-PTY-009. See ADR-0028 Decisions §2 and Risk #13.
    let pane_id_e = pane_id.clone();
    let app_e = app.clone();
    let registry_e = registry.clone();

    let config = CoalescerConfig::pty_default();
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

        // PTY-specific termination — runs AFTER the coalescer has flushed any
        // remaining pending output and returned. Caller-managed termination
        // model per ADR-0028 Decisions §3-4.
        //
        // FS-NOTIF-002: PTY process exited — transition to Terminated and get
        // exit info. mark_pane_terminated() calls wait() on the child to
        // recover the exit code and sets pane.lifecycle = Terminated before we
        // emit the notification.
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
