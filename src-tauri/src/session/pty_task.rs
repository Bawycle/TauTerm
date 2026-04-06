// SPDX-License-Identifier: MPL-2.0

//! PTY read task — one pair of Tokio tasks per pane.
//!
//! ## Architecture: two-task design
//!
//! The previous single-task design ran `rdr.read()` and the debounce check in
//! the same loop, which meant that if the PTY was silent after a burst, the
//! blocking `read()` would hold the thread indefinitely and the accumulated
//! dirty region would never be flushed until the next keystroke.
//!
//! The two-task design separates concerns:
//!
//! **Task 1 — reader (spawn_blocking)**
//! Reads raw bytes from the PTY, feeds them to `VtProcessor`, and sends the
//! resulting `ProcessOutput` through an unbounded channel to Task 2.
//! When the PTY reaches EOF the task exits naturally, closing the channel.
//!
//! **Task 2 — coalescer/emitter (async)**
//! Receives `ProcessOutput` values, coalesces them via `DirtyRegion::merge`,
//! and emits `screen-update` (and other) events on a timer-driven debounce
//! interval. Because Task 2 uses `tokio::select!`, the timer fires even when
//! the reader is blocked — the last batch is always flushed (§6.5).
//!
//! Back-pressure: dirty regions are coalesced over `SCREEN_UPDATE_DEBOUNCE`
//! before emitting a single `screen-update` event. This prevents flooding the
//! frontend when high-volume apps (`yes`, `seq`) write faster than the
//! frontend can consume events.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use parking_lot::RwLock;
use tauri::AppHandle;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Duration;

use crate::events::{
    emit_bell_triggered, emit_cursor_style_changed, emit_mode_state_changed,
    emit_notification_changed, emit_osc52_write_requested, emit_screen_update,
    emit_session_state_changed,
    types::{
        BellTriggeredEvent, CursorStyleChangedEvent, ModeStateChangedEvent,
        NotificationChangedEvent, Osc52WriteRequestedEvent, PaneNotificationDto, ScreenUpdateEvent,
        SessionChangeType, SessionStateChangedEvent,
    },
};
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::vt::{DirtyRegion, VtProcessor};

/// Debounce window for coalescing `screen-update` events.
///
/// After processing PTY bytes, Task 2 waits up to this duration before
/// emitting, coalescing further reads into a single event. This prevents
/// flooding the frontend when high-volume apps write faster than the frontend
/// can consume events (§6.5).
const SCREEN_UPDATE_DEBOUNCE: Duration = Duration::from_millis(12);

// ---------------------------------------------------------------------------
// ProcessOutput — data produced by processing one PTY chunk
// ---------------------------------------------------------------------------

/// Output produced by processing one chunk of PTY bytes in Task 1.
///
/// Task 2 coalesces multiple `ProcessOutput` values via `merge()` before
/// emitting events to the frontend.
#[derive(Default)]
pub(crate) struct ProcessOutput {
    pub dirty: DirtyRegion,
    pub mode_changed: bool,
    pub new_title: Option<String>,
    pub new_cursor_shape: Option<u8>,
    pub bell: bool,
    pub osc52: Option<String>,
}

impl ProcessOutput {
    /// Merge another output into `self`.
    ///
    /// - `dirty`: union (never loses dirty rows; full-redraw propagates).
    /// - `mode_changed`: OR (any mode change is preserved).
    /// - Scalar fields (`new_title`, `new_cursor_shape`, `osc52`): last-wins.
    /// - `bell`: OR (any bell in the window is preserved).
    fn merge(&mut self, other: ProcessOutput) {
        self.dirty.merge(&other.dirty);
        self.mode_changed |= other.mode_changed;
        if other.new_title.is_some() {
            self.new_title = other.new_title;
        }
        if other.new_cursor_shape.is_some() {
            self.new_cursor_shape = other.new_cursor_shape;
        }
        self.bell |= other.bell;
        if other.osc52.is_some() {
            self.osc52 = other.osc52;
        }
    }

    fn is_empty(&self) -> bool {
        self.dirty.is_empty()
            && !self.mode_changed
            && self.new_title.is_none()
            && self.new_cursor_shape.is_none()
            && !self.bell
            && self.osc52.is_none()
    }
}

// ---------------------------------------------------------------------------
// PtyTaskHandle
// ---------------------------------------------------------------------------

/// Handle to the running PTY read/emit task pair.
///
/// Dropping this handle aborts both tasks. `abort()` does the same explicitly.
pub struct PtyTaskHandle {
    read_abort: tokio::task::AbortHandle,
    emit_abort: tokio::task::AbortHandle,
}

impl PtyTaskHandle {
    /// Wrap two `AbortHandle`s into a `PtyTaskHandle`.
    pub fn new(read_abort: tokio::task::AbortHandle, emit_abort: tokio::task::AbortHandle) -> Self {
        Self {
            read_abort,
            emit_abort,
        }
    }

    /// Construct from a single abort handle (used in tests that create a
    /// synthetic handle without a real emit task).
    pub fn from_abort_handle(abort: tokio::task::AbortHandle) -> Self {
        // In test contexts there is no emit task, so we reuse the same handle
        // for both slots. Aborting twice is harmless.
        Self {
            read_abort: abort.clone(),
            emit_abort: abort,
        }
    }

    /// Abort both tasks.
    pub fn abort(&self) {
        self.read_abort.abort();
        self.emit_abort.abort();
    }
}

impl Drop for PtyTaskHandle {
    fn drop(&mut self) {
        self.read_abort.abort();
        self.emit_abort.abort();
    }
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
) -> PtyTaskHandle {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ProcessOutput>();

    // ------------------------------------------------------------------
    // Task 1 — blocking reader
    // ------------------------------------------------------------------
    let pane_id_r = pane_id.clone();
    let vt_r = vt.clone();
    let tx_r: UnboundedSender<ProcessOutput> = tx;
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
                    Err(e) => {
                        tracing::error!("PTY read error on pane {pane_id_r}: {e}");
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
                    },
                    responses,
                )
            }; // write-lock released here

            // Write DSR/DA/CPR responses back to the PTY master.
            // The write-lock on `vt` is no longer held here, so there is no
            // risk of deadlocking when the shell sends back a reply that would
            // trigger a new `vt.write()` acquisition in this same task.
            if !responses.is_empty() && let Some(ref w) = writer_r {
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
            if tx_r.send(output).is_err() {
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
    let mut rx_e = rx;

    let emit_task = tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(SCREEN_UPDATE_DEBOUNCE);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut pending = ProcessOutput::default();

        loop {
            tokio::select! {
                // Receive a chunk from Task 1.
                msg = rx_e.recv() => {
                    match msg {
                        Some(output) => {
                            pending.merge(output);
                        }
                        None => {
                            // Channel closed — Task 1 finished (EOF or error).
                            // Flush any remaining pending output before exiting.
                            if !pending.is_empty() {
                                emit_all_pending(
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

                // Debounce timer — flush accumulated output.
                _ = interval.tick() => {
                    if !pending.is_empty() {
                        emit_all_pending(
                            &app_e,
                            &pane_id_e,
                            &vt_e,
                            &registry_e,
                            &mut pending,
                        );
                    }
                }
            }
        }

        // FS-NOTIF-002: PTY process exited — emit notification with exit code.
        let exit_code = registry_e.get_pane_exit_code(&pane_id_e).unwrap_or(-1);
        if let Some((_, tab_state)) = registry_e.get_tab_state_for_pane(&pane_id_e) {
            emit_notification_changed(
                &app_e,
                NotificationChangedEvent {
                    tab_id: tab_state.id,
                    pane_id: pane_id_e.clone(),
                    notification: Some(PaneNotificationDto::ProcessExited { exit_code }),
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

// ---------------------------------------------------------------------------
// emit_all_pending — flush one coalesced window to the frontend
// ---------------------------------------------------------------------------

/// Emit all pending events accumulated in one debounce window.
///
/// Resets `pending` to `ProcessOutput::default()` on return.
fn emit_all_pending(
    app: &AppHandle,
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
    registry: &Arc<SessionRegistry>,
    pending: &mut ProcessOutput,
) {
    // FS-NOTIF-001: background-output notification.
    if !pending.dirty.is_empty()
        && !registry.is_active_pane(pane_id)
        && let Some((_, tab_state)) = registry.get_tab_state_for_pane(pane_id)
    {
        emit_notification_changed(
            app,
            NotificationChangedEvent {
                tab_id: tab_state.id,
                pane_id: pane_id.clone(),
                notification: Some(PaneNotificationDto::BackgroundOutput),
            },
        );
    }

    if !pending.dirty.is_empty() {
        let event = build_screen_update_event(pane_id, vt, &pending.dirty);
        emit_screen_update(app, event);
    }

    if pending.mode_changed {
        let event = build_mode_state_event(pane_id, vt);
        emit_mode_state_changed(app, event);
    }

    if let Some(shape) = pending.new_cursor_shape {
        emit_cursor_style_changed(
            app,
            CursorStyleChangedEvent {
                pane_id: pane_id.clone(),
                shape,
            },
        );
    }

    if pending.bell {
        emit_bell_triggered(
            app,
            BellTriggeredEvent {
                pane_id: pane_id.clone(),
            },
        );
    }

    if let Some(data) = pending.osc52.take() {
        emit_osc52_write_requested(
            app,
            Osc52WriteRequestedEvent {
                pane_id: pane_id.clone(),
                data,
            },
        );
    }

    if let Some(title) = pending.new_title.take()
        && let Some(tab_state) = registry.update_pane_title(pane_id, title)
    {
        emit_session_state_changed(
            app,
            SessionStateChangedEvent {
                change_type: SessionChangeType::PaneMetadataChanged,
                tab: Some(tab_state),
                active_tab_id: None,
                closed_tab_id: None,
            },
        );
    }

    *pending = ProcessOutput::default();
}

// ---------------------------------------------------------------------------
// build_mode_state_event
// ---------------------------------------------------------------------------

/// Build a `ModeStateChangedEvent` from the current mode state.
pub(crate) fn build_mode_state_event(
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
) -> ModeStateChangedEvent {
    let proc = vt.read();
    let modes = proc.mode_state();
    ModeStateChangedEvent {
        pane_id: pane_id.clone(),
        decckm: modes.decckm,
        deckpam: modes.deckpam,
        mouse_reporting: modes.mouse_reporting,
        mouse_encoding: modes.mouse_encoding,
        focus_events: modes.focus_events,
        bracketed_paste: modes.bracketed_paste,
    }
}

// ---------------------------------------------------------------------------
// build_screen_update_event
// ---------------------------------------------------------------------------

/// Build a `ScreenUpdateEvent` from the dirty region returned by `VtProcessor::process()`.
///
/// Takes a snapshot and extracts cells for each dirty row.
/// `pub(crate)` so `session::ssh_task` can reuse it without duplication.
pub(crate) fn build_screen_update_event(
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
    dirty: &DirtyRegion,
) -> ScreenUpdateEvent {
    use crate::events::types::{CellUpdate, ColorDto, CursorState};
    use crate::vt::cell::Color;

    let proc = vt.read();
    let snapshot = proc.get_snapshot();
    let cols = snapshot.cols as usize;

    // Convert a `vt::cell::Color` to the IPC `ColorDto`.
    let color_to_dto = |c: Color| -> ColorDto {
        match c {
            Color::Ansi { index } => ColorDto::Ansi { index },
            Color::Ansi256 { index } => ColorDto::Ansi256 { index },
            Color::Rgb { r, g, b } => ColorDto::Rgb { r, g, b },
        }
    };

    let cells: Vec<CellUpdate> = if dirty.is_full_redraw {
        // Full redraw: send all cells.
        snapshot
            .cells
            .iter()
            .enumerate()
            .map(|(idx, cell)| {
                let row = (idx / cols) as u16;
                let col = (idx % cols) as u16;
                CellUpdate {
                    row,
                    col,
                    content: cell.content.clone(),
                    width: cell.width,
                    attrs: snapshot_cell_to_attrs_dto(cell, &color_to_dto),
                    hyperlink: cell.hyperlink.clone(),
                }
            })
            .collect()
    } else {
        // Partial update: only dirty rows.
        let mut updates = Vec::new();
        for &row in &dirty.rows {
            let row_start = row as usize * cols;
            let row_end = (row_start + cols).min(snapshot.cells.len());
            for (col_offset, cell) in snapshot.cells[row_start..row_end].iter().enumerate() {
                updates.push(CellUpdate {
                    row,
                    col: col_offset as u16,
                    content: cell.content.clone(),
                    width: cell.width,
                    attrs: snapshot_cell_to_attrs_dto(cell, &color_to_dto),
                    hyperlink: cell.hyperlink.clone(),
                });
            }
        }
        updates
    };

    let cursor = CursorState {
        row: snapshot.cursor_row,
        col: snapshot.cursor_col,
        visible: snapshot.cursor_visible,
        shape: snapshot.cursor_shape,
        blink: proc.cursor_blink,
    };

    let scrollback_lines = snapshot.scrollback_lines;

    ScreenUpdateEvent {
        pane_id: pane_id.clone(),
        cells,
        cursor,
        scrollback_lines,
        is_full_redraw: dirty.is_full_redraw,
    }
}

fn snapshot_cell_to_attrs_dto(
    cell: &crate::vt::screen_buffer::SnapshotCell,
    color_to_dto: &impl Fn(crate::vt::cell::Color) -> crate::events::types::ColorDto,
) -> crate::events::types::CellAttrsDto {
    use crate::events::types::CellAttrsDto;
    CellAttrsDto {
        fg: cell.fg.map(color_to_dto),
        bg: cell.bg.map(color_to_dto),
        bold: cell.bold,
        dim: cell.dim,
        italic: cell.italic,
        underline: cell.underline,
        blink: cell.blink,
        inverse: cell.inverse,
        hidden: cell.hidden,
        strikethrough: cell.strikethrough,
        underline_color: cell.underline_color.map(color_to_dto),
    }
}
