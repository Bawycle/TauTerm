// SPDX-License-Identifier: MPL-2.0

//! PTY read task — one Tokio task per pane.
//!
//! Reads bytes from the PTY master (via a blocking reader from `portable-pty`),
//! feeds them to `VtProcessor`, coalesces dirty regions, and emits `screen-update`
//! events to the frontend via `AppHandle` (§6.2 of ARCHITECTURE.md).
//!
//! The reader (`Box<dyn Read + Send>`) is a synchronous blocking reader.
//! We run it on `tokio::task::spawn_blocking` to avoid blocking Tokio worker threads.
//!
//! Back-pressure: all available bytes are processed before emitting a single
//! event. Rate limiting is a future improvement (§6.5).

use std::io::Read;
use std::sync::{Arc, Mutex};

use parking_lot::RwLock;
use tauri::AppHandle;

use crate::events::{emit_screen_update, types::ScreenUpdateEvent};
use crate::session::ids::PaneId;
use crate::vt::{DirtyRegion, VtProcessor};

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
) -> PtyTaskHandle {
    let task = tokio::task::spawn_blocking(move || {
        let mut buf = vec![0u8; 4096];
        loop {
            // Read from PTY master — blocking call.
            let n = {
                let mut rdr = match reader.lock() {
                    Ok(g) => g,
                    Err(_) => {
                        tracing::error!("PTY reader mutex poisoned on pane {pane_id}");
                        return;
                    }
                };
                match rdr.read(&mut buf) {
                    Ok(0) => {
                        tracing::debug!("PTY EOF on pane {pane_id}");
                        return;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        tracing::error!("PTY read error on pane {pane_id}: {e}");
                        return;
                    }
                }
            };

            // Process bytes through the VT processor.
            let dirty = {
                let mut proc = vt.write();
                proc.process(&buf[..n])
            };

            if !dirty.is_empty() {
                let event = build_screen_update_event(&pane_id, &vt, &dirty);
                emit_screen_update(&app, event);
            }
        }
    });

    PtyTaskHandle {
        abort: task.abort_handle(),
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
