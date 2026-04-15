// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use parking_lot::RwLock;

use crate::events::types::{ModeStateChangedEvent, ScreenUpdateEvent};
use crate::session::ids::PaneId;
use crate::vt::{DirtyRegion, VtProcessor};

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
// cell_color_to_dto
// ---------------------------------------------------------------------------

/// Convert a `vt::cell::Color` to the IPC `ColorDto`.
///
/// Extracted as a free function so `build_scrolled_viewport_event` can reuse
/// it without duplicating the match expression.
pub(crate) fn cell_color_to_dto(c: crate::vt::cell::Color) -> crate::events::types::ColorDto {
    use crate::events::types::ColorDto;
    match c {
        crate::vt::cell::Color::Ansi { index } => ColorDto::Ansi { index },
        crate::vt::cell::Color::Ansi256 { index } => ColorDto::Ansi256 { index },
        crate::vt::cell::Color::Rgb { r, g, b } => ColorDto::Rgb { r, g, b },
    }
}

// ---------------------------------------------------------------------------
// build_screen_update_event
// ---------------------------------------------------------------------------

/// Build a `ScreenUpdateEvent` from the dirty region returned by `VtProcessor::process()`.
///
/// - Full redraw (`dirty.is_full_redraw`): calls `get_snapshot()` and sends all cells.
/// - Partial update: accesses dirty rows directly via `active_buf_ref().get_row()` —
///   no full snapshot clone. Only the rows listed in `dirty.rows` are included.
///
/// INVARIANT: the read-lock on `vt` is released before this function returns.
/// Callers must not hold the write-lock when calling this function.
///
/// `pub(crate)` so `session::ssh_task` can reuse it without duplication.
/// `#[doc(hidden)]` exposes this function to benchmarks without making it part
/// of the public API.
#[doc(hidden)]
pub fn build_screen_update_event(
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
    dirty: &DirtyRegion,
) -> ScreenUpdateEvent {
    use crate::events::types::{CellUpdate, CursorState};

    let proc = vt.read();

    if dirty.is_full_redraw {
        // Full redraw: snapshot clone is unavoidable — every cell must be sent.
        let snapshot = proc.get_snapshot();
        let cols = snapshot.cols as usize;

        let cells: Vec<CellUpdate> = snapshot
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
                    attrs: snapshot_cell_to_attrs_dto(cell, &cell_color_to_dto),
                    hyperlink: cell.hyperlink.clone(),
                }
            })
            .collect();

        let cursor = CursorState {
            row: snapshot.cursor_row,
            col: snapshot.cursor_col,
            visible: snapshot.cursor_visible,
            shape: snapshot.cursor_shape,
            blink: proc.cursor_blink,
        };

        ScreenUpdateEvent {
            pane_id: pane_id.clone(),
            cells,
            cursor,
            scrollback_lines: snapshot.scrollback_lines,
            is_full_redraw: true,
            cols: snapshot.cols,
            rows: snapshot.rows,
            scroll_offset: 0,
        }
    } else {
        // Partial update: access dirty rows directly — no full snapshot clone.
        let meta = proc.get_screen_meta();
        let buf = proc.active_buf_ref();

        let dirty_count = dirty.rows.iter().count();
        let mut cells: Vec<CellUpdate> = Vec::with_capacity(dirty_count * meta.cols as usize);

        for row in dirty.rows.iter() {
            if let Some(row_cells) = buf.get_row(row) {
                for (col, cell) in row_cells.iter().enumerate() {
                    cells.push(CellUpdate {
                        row,
                        col: col as u16,
                        content: cell.grapheme.to_string(),
                        width: cell.width,
                        attrs: cell_attrs_to_dto(&cell.attrs),
                        hyperlink: cell.hyperlink.as_ref().map(|h| h.as_ref().to_owned()),
                    });
                }
            }
        }

        let cursor = CursorState {
            row: meta.cursor_row,
            col: meta.cursor_col,
            visible: meta.cursor_visible,
            shape: meta.cursor_shape,
            blink: meta.cursor_blink,
        };

        ScreenUpdateEvent {
            pane_id: pane_id.clone(),
            cells,
            cursor,
            scrollback_lines: meta.scrollback_lines,
            is_full_redraw: false,
            cols: meta.cols,
            rows: meta.rows,
            scroll_offset: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// build_scrolled_viewport_event
// ---------------------------------------------------------------------------

/// Build a `ScreenUpdateEvent` that represents a scrolled viewport compositing
/// scrollback lines and live screen rows.
///
/// ## Composite viewport math
///
/// Let `k = scroll_offset`, `N = scrollback_len()`, `R = rows`.
/// For viewport row `i`:
///   - `content_pos = N as i64 - k + i as i64`
///   - `content_pos < N as i64`  → scrollback line at index `content_pos`
///   - `content_pos >= N as i64` → live screen row `(content_pos - N as i64) as usize`
///
/// `k` is clamped to `[0, N]` so `content_pos >= 0` always holds.
///
/// Always sets `is_full_redraw: true` and `scroll_offset: k`.
/// Cursor is hidden (`visible: false`) whenever `k > 0`.
/// `#[doc(hidden)]` exposes this function to benchmarks without making it part
/// of the public API.
#[doc(hidden)]
pub fn build_scrolled_viewport_event(
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
    scroll_offset: i64,
) -> ScreenUpdateEvent {
    use crate::events::types::{CellAttrsDto, CellUpdate, CursorState};
    use crate::vt::screen_buffer::SnapshotCell;

    let proc = vt.read();
    let snapshot = proc.get_snapshot();
    let cols = snapshot.cols as usize;
    let rows = snapshot.rows as usize;
    let n = proc.scrollback_len();

    // Clamp k: alt screen has no scrollback, always 0.
    let k = if proc.is_alt_screen_active() {
        0
    } else {
        scroll_offset.clamp(0, n as i64)
    };

    // Build a blank default attrs value for padding cells.
    let blank_attrs = CellAttrsDto {
        fg: None,
        bg: None,
        bold: None,
        dim: None,
        italic: None,
        underline: None,
        blink: None,
        inverse: None,
        hidden: None,
        strikethrough: None,
        underline_color: None,
    };

    let mut cells: Vec<CellUpdate> = Vec::with_capacity(rows * cols);

    for i in 0..rows {
        let content_pos = n as i64 - k + i as i64;
        if content_pos < n as i64 {
            // Source: scrollback line at index `content_pos`.
            let sb_idx = content_pos as usize;
            let line_cells: Vec<CellUpdate> = if let Some(sb_line) =
                proc.get_scrollback_line(sb_idx)
            {
                (0..cols)
                    .map(|col_idx| {
                        if col_idx < sb_line.cells.len() {
                            let snap_cell = SnapshotCell::from(&sb_line.cells[col_idx]);
                            CellUpdate {
                                row: i as u16,
                                col: col_idx as u16,
                                content: snap_cell.content.clone(),
                                width: snap_cell.width,
                                attrs: snapshot_cell_to_attrs_dto(&snap_cell, &cell_color_to_dto),
                                hyperlink: snap_cell.hyperlink.clone(),
                            }
                        } else {
                            CellUpdate {
                                row: i as u16,
                                col: col_idx as u16,
                                content: String::from(" "),
                                width: 1,
                                attrs: blank_attrs.clone(),
                                hyperlink: None,
                            }
                        }
                    })
                    .collect()
            } else {
                // Scrollback line not found — pad entire row.
                (0..cols)
                    .map(|col_idx| CellUpdate {
                        row: i as u16,
                        col: col_idx as u16,
                        content: String::from(" "),
                        width: 1,
                        attrs: blank_attrs.clone(),
                        hyperlink: None,
                    })
                    .collect()
            };
            cells.extend(line_cells);
        } else {
            // Source: live screen row.
            let live_row = (content_pos - n as i64) as usize;
            let row_start = live_row * cols;
            let row_end = (row_start + cols).min(snapshot.cells.len());
            for (col_offset, cell) in snapshot.cells[row_start..row_end].iter().enumerate() {
                cells.push(CellUpdate {
                    row: i as u16,
                    col: col_offset as u16,
                    content: cell.content.clone(),
                    width: cell.width,
                    attrs: snapshot_cell_to_attrs_dto(cell, &cell_color_to_dto),
                    hyperlink: cell.hyperlink.clone(),
                });
            }
            // Pad if row_end was clamped (shouldn't normally happen but guards underflow).
            let produced = row_end.saturating_sub(row_start);
            for col_offset in produced..cols {
                cells.push(CellUpdate {
                    row: i as u16,
                    col: col_offset as u16,
                    content: String::from(" "),
                    width: 1,
                    attrs: blank_attrs.clone(),
                    hyperlink: None,
                });
            }
        }
    }

    let cursor = CursorState {
        row: snapshot.cursor_row,
        col: snapshot.cursor_col,
        // Hide cursor whenever we are scrolled away from the live view.
        visible: k == 0 && snapshot.cursor_visible,
        shape: snapshot.cursor_shape,
        blink: proc.cursor_blink,
    };

    ScreenUpdateEvent {
        pane_id: pane_id.clone(),
        cells,
        cursor,
        scrollback_lines: n,
        is_full_redraw: true,
        cols: snapshot.cols,
        rows: snapshot.rows,
        scroll_offset: k,
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn snapshot_cell_to_attrs_dto(
    cell: &crate::vt::screen_buffer::SnapshotCell,
    color_to_dto: &impl Fn(crate::vt::cell::Color) -> crate::events::types::ColorDto,
) -> crate::events::types::CellAttrsDto {
    use crate::events::types::CellAttrsDto;
    CellAttrsDto {
        fg: cell.fg.map(color_to_dto),
        bg: cell.bg.map(color_to_dto),
        bold: if cell.bold { Some(true) } else { None },
        dim: if cell.dim { Some(true) } else { None },
        italic: if cell.italic { Some(true) } else { None },
        underline: if cell.underline != 0 {
            Some(cell.underline)
        } else {
            None
        },
        blink: if cell.blink { Some(true) } else { None },
        inverse: if cell.inverse { Some(true) } else { None },
        hidden: if cell.hidden { Some(true) } else { None },
        strikethrough: if cell.strikethrough { Some(true) } else { None },
        underline_color: cell.underline_color.map(color_to_dto),
    }
}

/// Build a `CellAttrsDto` directly from a `Cell`'s `CellAttrs`, bypassing the
/// `SnapshotCell` intermediary. Used by the partial-update path in
/// `build_screen_update_event` to avoid a full snapshot clone.
fn cell_attrs_to_dto(attrs: &crate::vt::cell::CellAttrs) -> crate::events::types::CellAttrsDto {
    use crate::events::types::CellAttrsDto;
    CellAttrsDto {
        fg: attrs.fg.map(cell_color_to_dto),
        bg: attrs.bg.map(cell_color_to_dto),
        bold: if attrs.bold { Some(true) } else { None },
        dim: if attrs.dim { Some(true) } else { None },
        italic: if attrs.italic { Some(true) } else { None },
        underline: if attrs.underline != 0 {
            Some(attrs.underline)
        } else {
            None
        },
        blink: if attrs.blink { Some(true) } else { None },
        inverse: if attrs.inverse { Some(true) } else { None },
        hidden: if attrs.hidden { Some(true) } else { None },
        strikethrough: if attrs.strikethrough {
            Some(true)
        } else {
            None
        },
        underline_color: attrs.underline_color.map(cell_color_to_dto),
    }
}
