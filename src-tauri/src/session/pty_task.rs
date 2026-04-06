// SPDX-License-Identifier: MPL-2.0

//! PTY read task — one Tokio task per pane.
//!
//! Reads bytes from the PTY master (via a blocking reader from `portable-pty`),
//! feeds them to `VtProcessor`, coalesces dirty regions, and emits `screen-update`
//! events to the frontend via `AppHandle` (§6.2 of ARCHITECTURE.md).
//!
//! The reader (`Box<dyn Read + Send>`) is a synchronous blocking reader.
//! We run it on `tauri::async_runtime::spawn_blocking` to avoid blocking Tokio worker threads.
//! Using Tauri's async runtime (rather than `tokio::task` directly) ensures the runtime
//! is available even when called from Tauri's `setup()` hook.
//!
//! Back-pressure: all available bytes are processed before emitting a single
//! event. Rate limiting is a future improvement (§6.5).

use std::io::Read;
use std::sync::{Arc, Mutex};

use parking_lot::RwLock;
use tauri::AppHandle;

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
use crate::vt::{DirtyRegion, VtProcessor, modes::MouseEncoding, modes::MouseReportingMode};

/// Handle to a running PTY read task.
/// Dropping this handle signals the task to stop.
pub struct PtyTaskHandle {
    pub(crate) abort: tokio::task::AbortHandle,
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
/// `reader` — a synchronous `Read` source from `portable-pty`'s `try_clone_reader()`,
/// wrapped in `Arc<Mutex<...>>` so it can be passed to `spawn_blocking`.
///
/// Returns a `PtyTaskHandle` that aborts the task on drop.
pub fn spawn_pty_read_task(
    pane_id: PaneId,
    vt: Arc<RwLock<VtProcessor>>,
    app: AppHandle,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    registry: Arc<SessionRegistry>,
) -> PtyTaskHandle {
    let task = tauri::async_runtime::spawn_blocking(move || {
        let mut buf = vec![0u8; 4096];
        loop {
            // Read from PTY master — blocking call.
            let n = {
                let mut rdr = match reader.lock() {
                    Ok(g) => g,
                    Err(_) => {
                        tracing::error!("PTY reader mutex poisoned on pane {pane_id}");
                        break;
                    }
                };
                match rdr.read(&mut buf) {
                    Ok(0) => {
                        tracing::debug!("PTY EOF on pane {pane_id}");
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        tracing::error!("PTY read error on pane {pane_id}: {e}");
                        break;
                    }
                }
            };

            // Process bytes through the VT processor.
            let (dirty, mode_changed, new_title, new_cursor_shape, bell, osc52) = {
                let mut proc = vt.write();
                let dirty = proc.process(&buf[..n]);
                let changed = proc.mode_changed;
                if changed {
                    proc.mode_changed = false;
                }
                let title = proc.take_title_changed();
                let cursor_shape = proc.take_cursor_shape_changed();
                let bell = proc.take_bell_pending();
                let osc52 = proc.take_osc52_write();
                (dirty, changed, title, cursor_shape, bell, osc52)
            };

            if mode_changed {
                let event = build_mode_state_event(&pane_id, &vt);
                emit_mode_state_changed(&app, event);
            }

            if let Some(shape) = new_cursor_shape {
                emit_cursor_style_changed(
                    &app,
                    CursorStyleChangedEvent {
                        pane_id: pane_id.clone(),
                        shape,
                    },
                );
            }

            if bell {
                emit_bell_triggered(
                    &app,
                    BellTriggeredEvent {
                        pane_id: pane_id.clone(),
                    },
                );
            }

            // FS-VT-075: forward OSC 52 clipboard write to the frontend.
            if let Some(data) = osc52 {
                emit_osc52_write_requested(
                    &app,
                    Osc52WriteRequestedEvent {
                        pane_id: pane_id.clone(),
                        data,
                    },
                );
            }

            if let Some(title) = new_title
                && let Some(tab_state) = registry.update_pane_title(&pane_id, title)
            {
                emit_session_state_changed(
                    &app,
                    SessionStateChangedEvent {
                        change_type: SessionChangeType::PaneMetadataChanged,
                        tab: Some(tab_state),
                        active_tab_id: None,
                        closed_tab_id: None,
                    },
                );
            }

            if !dirty.is_empty() {
                // FS-NOTIF-001: if this pane is not the active pane, emit a background-output notification.
                if !registry.is_active_pane(&pane_id)
                    && let Some((_, tab_state)) = registry.get_tab_state_for_pane(&pane_id)
                {
                    emit_notification_changed(
                        &app,
                        NotificationChangedEvent {
                            tab_id: tab_state.id,
                            pane_id: pane_id.clone(),
                            notification: Some(PaneNotificationDto::BackgroundOutput),
                        },
                    );
                }

                let event = build_screen_update_event(&pane_id, &vt, &dirty);
                emit_screen_update(&app, event);
            }
        }

        // FS-NOTIF-002: PTY process exited — emit notification with exit code.
        // The blocking Read task does not have access to the child's wait() result;
        // the exit code is read from the pane's lifecycle state in the registry if it
        // has already been updated to `Terminated`, otherwise -1 is used as a sentinel
        // meaning "process exited with unknown code".
        let exit_code = registry.get_pane_exit_code(&pane_id).unwrap_or(-1);
        if let Some((_, tab_state)) = registry.get_tab_state_for_pane(&pane_id) {
            emit_notification_changed(
                &app,
                NotificationChangedEvent {
                    tab_id: tab_state.id,
                    pane_id: pane_id.clone(),
                    notification: Some(PaneNotificationDto::ProcessExited { exit_code }),
                },
            );
        }
    });

    PtyTaskHandle {
        abort: task.inner().abort_handle(),
    }
}

/// Build a `ModeStateChangedEvent` from the current mode state.
pub(crate) fn build_mode_state_event(
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
) -> ModeStateChangedEvent {
    let proc = vt.read();
    let modes = proc.mode_state();
    let mouse_reporting = match modes.mouse_reporting {
        MouseReportingMode::None => "none",
        MouseReportingMode::X10 => "x10",
        MouseReportingMode::Normal => "normal",
        MouseReportingMode::ButtonEvent => "buttonEvent",
        MouseReportingMode::AnyEvent => "anyEvent",
    };
    let mouse_encoding = match modes.mouse_encoding {
        MouseEncoding::X10 => "x10",
        MouseEncoding::Sgr => "sgr",
        MouseEncoding::Urxvt => "urxvt",
    };
    ModeStateChangedEvent {
        pane_id: pane_id.clone(),
        decckm: modes.decckm,
        deckpam: modes.deckpam,
        mouse_reporting: mouse_reporting.to_string(),
        mouse_encoding: mouse_encoding.to_string(),
        focus_events: modes.focus_events,
        bracketed_paste: modes.bracketed_paste,
    }
}

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
        blink: false, // cursor_blink not yet tracked in VtProcessor snapshot
    };

    let scrollback_lines = snapshot.scrollback_lines;

    ScreenUpdateEvent {
        pane_id: pane_id.clone(),
        cells,
        cursor,
        scrollback_lines,
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
