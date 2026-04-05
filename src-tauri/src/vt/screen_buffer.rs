// SPDX-License-Identifier: MPL-2.0

//! Terminal screen buffer: cell grid, scrollback ring, dirty tracking, resize.
//!
//! `ScreenBuffer` maintains:
//! - The visible cell grid (normal or alternate screen)
//! - A scrollback ring for the normal screen (lines scrolled off the top)
//! - Dirty region tracking for efficient screen-update event generation
//! - Resize logic (reflow not required for v1: truncate/pad on resize)
//!
//! Scrollback policy (§3.2, FS-VT-053, FS-SB-004):
//! Only lines scrolled off the top of a full-screen scroll region enter the ring.
//! Lines evicted by a partial DECSTBM scroll region are discarded (not stored).
//! The alternate screen buffer never contributes to scrollback.

use serde::{Deserialize, Serialize};

use crate::vt::cell::Cell;

/// The maximum number of scrollback lines (configurable via preferences in the
/// full implementation; this constant is the hard upper bound).
pub const MAX_SCROLLBACK_LINES: usize = 100_000;

/// A snapshot of the visible screen content, used for `get_pane_screen_snapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenSnapshot {
    pub cols: u16,
    pub rows: u16,
    /// Row-major flat array: rows × cols cells.
    pub cells: Vec<SnapshotCell>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_visible: bool,
    pub cursor_shape: u8,
    pub scrollback_lines: usize,
    pub scroll_offset: i64,
}

/// A single cell in a snapshot — serializable to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotCell {
    pub content: String,
    pub width: u8,
    // Attributes encoded as per CellAttrsDto (re-encoded at snapshot time).
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: u8,
    pub blink: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<crate::vt::cell::Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<crate::vt::cell::Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_color: Option<crate::vt::cell::Color>,
}

impl From<&Cell> for SnapshotCell {
    fn from(cell: &Cell) -> Self {
        Self {
            content: cell.grapheme.clone(),
            width: cell.width,
            bold: cell.attrs.bold,
            dim: cell.attrs.dim,
            italic: cell.attrs.italic,
            underline: cell.attrs.underline,
            blink: cell.attrs.blink,
            inverse: cell.attrs.inverse,
            hidden: cell.attrs.hidden,
            strikethrough: cell.attrs.strikethrough,
            fg: cell.attrs.fg,
            bg: cell.attrs.bg,
            underline_color: cell.attrs.underline_color,
        }
    }
}

/// Describes a rectangular region of dirty cells to be sent as a screen update.
#[derive(Debug, Clone, Default)]
pub struct DirtyRegion {
    pub rows: std::collections::HashSet<u16>,
    pub is_full_redraw: bool,
}

impl DirtyRegion {
    pub fn is_empty(&self) -> bool {
        !self.is_full_redraw && self.rows.is_empty()
    }

    pub fn mark_row(&mut self, row: u16) {
        self.rows.insert(row);
    }

    pub fn mark_full_redraw(&mut self) {
        self.is_full_redraw = true;
        self.rows.clear();
    }

    pub fn merge(&mut self, other: &DirtyRegion) {
        if other.is_full_redraw {
            self.mark_full_redraw();
        } else {
            self.rows.extend(&other.rows);
        }
    }
}

/// The full screen buffer state for one screen (normal or alternate).
pub struct ScreenBuffer {
    pub cols: u16,
    pub rows: u16,
    /// Row-major grid: `rows × cols` cells.
    cells: Vec<Vec<Cell>>,
    /// Scrollback ring (normal screen only; capacity = `scrollback_limit`).
    scrollback: std::collections::VecDeque<Vec<Cell>>,
    pub scrollback_limit: usize,
    /// Pending dirty region since last `take_dirty`.
    dirty: DirtyRegion,
}

impl ScreenBuffer {
    pub fn new(cols: u16, rows: u16, scrollback_limit: usize) -> Self {
        let cells = (0..rows)
            .map(|_| (0..cols).map(|_| Cell::default()).collect())
            .collect();
        Self {
            cols,
            rows,
            cells,
            scrollback: std::collections::VecDeque::new(),
            scrollback_limit,
            dirty: DirtyRegion::default(),
        }
    }

    /// Get a reference to a cell (bounds-checked).
    pub fn get(&self, row: u16, col: u16) -> Option<&Cell> {
        self.cells
            .get(row as usize)
            .and_then(|r| r.get(col as usize))
    }

    /// Get a mutable reference to a cell (bounds-checked).
    pub fn get_mut(&mut self, row: u16, col: u16) -> Option<&mut Cell> {
        let row = row as usize;
        let col = col as usize;
        if row < self.cells.len() && col < self.cells[row].len() {
            self.dirty.mark_row(row as u16);
            Some(&mut self.cells[row][col])
        } else {
            None
        }
    }

    /// Get an entire row.
    pub fn get_row(&self, row: u16) -> Option<&Vec<Cell>> {
        self.cells.get(row as usize)
    }

    /// Erase a range of cells on a row (replace with default Cell).
    pub fn erase_cells(&mut self, row: u16, col_start: u16, col_end: u16) {
        if let Some(r) = self.cells.get_mut(row as usize) {
            let end = (col_end as usize).min(r.len());
            for cell in r.iter_mut().take(end).skip(col_start as usize) {
                *cell = Cell::default();
            }
            self.dirty.mark_row(row);
        }
    }

    /// Erase lines in a range (replace with blank lines).
    pub fn erase_lines(&mut self, row_start: u16, row_end: u16) {
        for row in row_start..row_end.min(self.rows) {
            if let Some(r) = self.cells.get_mut(row as usize) {
                r.iter_mut().for_each(|c| *c = Cell::default());
                self.dirty.mark_row(row);
            }
        }
    }

    /// Scroll up by `count` lines within `[top, bottom]` (DECSTBM-bounded).
    /// Lines scrolled off the top of a full-screen region enter scrollback.
    /// Lines scrolled off a partial region are discarded (FS-SB-004).
    pub fn scroll_up(&mut self, top: u16, bottom: u16, count: u16, is_full_screen: bool) {
        let count = count as usize;
        let top = top as usize;
        let bottom = bottom as usize;

        if top >= bottom || top >= self.cells.len() {
            return;
        }

        let bottom = bottom.min(self.cells.len() - 1);

        for _ in 0..count {
            if is_full_screen && !self.cells.is_empty() {
                let evicted = self.cells[top].clone();
                if self.scrollback.len() >= self.scrollback_limit {
                    self.scrollback.pop_front();
                }
                self.scrollback.push_back(evicted);
            }

            for row in top..bottom {
                self.cells.swap(row, row + 1);
                self.dirty.mark_row(row as u16);
            }
            // Clear the newly exposed bottom line.
            if let Some(r) = self.cells.get_mut(bottom) {
                r.iter_mut().for_each(|c| *c = Cell::default());
            }
            self.dirty.mark_row(bottom as u16);
        }
    }

    /// Scroll down by `count` lines within `[top, bottom]`.
    pub fn scroll_down(&mut self, top: u16, bottom: u16, count: u16) {
        if self.cells.is_empty() || top >= bottom {
            return;
        }
        let count = count as usize;
        let top = top as usize;
        let bottom = (bottom as usize).min(self.cells.len() - 1);

        for _ in 0..count {
            for row in (top + 1..=bottom).rev() {
                self.cells.swap(row, row - 1);
                self.dirty.mark_row(row as u16);
            }
            // Clear the newly exposed top line.
            if let Some(r) = self.cells.get_mut(top) {
                r.iter_mut().for_each(|c| *c = Cell::default());
            }
            self.dirty.mark_row(top as u16);
        }
    }

    /// Resize the buffer. Truncates or pads rows and columns.
    /// Does not perform scrollback reflow in v1.
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        let old_rows = self.rows as usize;
        let new_rows_usize = new_rows as usize;
        let new_cols_usize = new_cols as usize;

        // Resize each existing row.
        for row in &mut self.cells {
            row.resize(new_cols_usize, Cell::default());
        }

        // Add or remove rows.
        self.cells.resize_with(new_rows_usize, || {
            (0..new_cols_usize).map(|_| Cell::default()).collect()
        });

        self.cols = new_cols;
        self.rows = new_rows;

        let _ = old_rows;
        self.dirty.mark_full_redraw();
    }

    /// Take the pending dirty region, leaving an empty one.
    pub fn take_dirty(&mut self) -> DirtyRegion {
        std::mem::take(&mut self.dirty)
    }

    /// Get a scrollback line by 0-based index from the oldest line.
    pub fn get_scrollback_line(&self, index: usize) -> Option<&Vec<Cell>> {
        self.scrollback.get(index)
    }

    /// Number of lines in scrollback.
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Build a full screen snapshot.
    pub fn snapshot(
        &self,
        cursor_row: u16,
        cursor_col: u16,
        cursor_visible: bool,
        cursor_shape: u8,
        scroll_offset: i64,
    ) -> ScreenSnapshot {
        let cells = self
            .cells
            .iter()
            .flat_map(|row| row.iter().map(SnapshotCell::from))
            .collect();
        ScreenSnapshot {
            cols: self.cols,
            rows: self.rows,
            cells,
            cursor_row,
            cursor_col,
            cursor_visible,
            cursor_shape,
            scrollback_lines: self.scrollback.len(),
            scroll_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Construction ---

    #[test]
    fn new_buffer_has_correct_dimensions() {
        let buf = ScreenBuffer::new(80, 24, 1000);
        assert_eq!(buf.cols, 80);
        assert_eq!(buf.rows, 24);
        assert_eq!(buf.scrollback_limit, 1000);
    }

    #[test]
    fn new_buffer_cells_are_default() {
        let buf = ScreenBuffer::new(10, 5, 100);
        for row in 0..5u16 {
            for col in 0..10u16 {
                assert_eq!(buf.get(row, col), Some(&Cell::default()));
            }
        }
    }

    #[test]
    fn new_buffer_has_empty_scrollback() {
        let buf = ScreenBuffer::new(80, 24, 1000);
        assert_eq!(buf.scrollback_len(), 0);
    }

    // --- Cell access ---

    #[test]
    fn get_out_of_bounds_returns_none() {
        let buf = ScreenBuffer::new(5, 5, 100);
        assert!(buf.get(5, 0).is_none());
        assert!(buf.get(0, 5).is_none());
        assert!(buf.get(10, 10).is_none());
    }

    #[test]
    fn get_mut_marks_row_dirty() {
        let mut buf = ScreenBuffer::new(5, 5, 100);
        // Take initial clean dirty region.
        let _ = buf.take_dirty();
        let _ = buf.get_mut(2, 3);
        let dirty = buf.take_dirty();
        assert!(dirty.rows.contains(&2));
    }

    // --- Erase operations ---

    #[test]
    fn erase_cells_replaces_with_default() {
        let mut buf = ScreenBuffer::new(10, 5, 100);
        if let Some(cell) = buf.get_mut(0, 3) {
            cell.grapheme = "X".to_string();
        }
        buf.erase_cells(0, 0, 10);
        assert_eq!(buf.get(0, 3), Some(&Cell::default()));
    }

    #[test]
    fn erase_lines_replaces_with_default_rows() {
        let mut buf = ScreenBuffer::new(5, 5, 100);
        if let Some(cell) = buf.get_mut(1, 0) {
            cell.grapheme = "Y".to_string();
        }
        buf.erase_lines(1, 2);
        assert_eq!(buf.get(1, 0), Some(&Cell::default()));
    }

    // --- Scroll up / scrollback ---

    #[test]
    fn scroll_up_full_screen_adds_to_scrollback() {
        let mut buf = ScreenBuffer::new(5, 3, 100);
        // Write something on row 0 so we can identify it in scrollback.
        if let Some(cell) = buf.get_mut(0, 0) {
            cell.grapheme = "A".to_string();
        }
        buf.scroll_up(0, 2, 1, true);
        assert_eq!(buf.scrollback_len(), 1);
    }

    #[test]
    fn scroll_up_partial_region_does_not_add_to_scrollback() {
        let mut buf = ScreenBuffer::new(5, 5, 100);
        buf.scroll_up(1, 3, 1, false);
        assert_eq!(buf.scrollback_len(), 0);
    }

    #[test]
    fn scrollback_respects_limit() {
        let limit = 3usize;
        let mut buf = ScreenBuffer::new(5, 1, limit);
        for _ in 0..10 {
            buf.scroll_up(0, 0, 1, true);
        }
        assert!(buf.scrollback_len() <= limit);
    }

    // --- Resize ---

    #[test]
    fn resize_updates_dimensions() {
        let mut buf = ScreenBuffer::new(80, 24, 1000);
        buf.resize(120, 40);
        assert_eq!(buf.cols, 120);
        assert_eq!(buf.rows, 40);
    }

    #[test]
    fn resize_triggers_full_redraw() {
        let mut buf = ScreenBuffer::new(80, 24, 1000);
        let _ = buf.take_dirty();
        buf.resize(100, 30);
        let dirty = buf.take_dirty();
        assert!(dirty.is_full_redraw);
    }

    // --- Dirty region ---

    #[test]
    fn take_dirty_clears_region() {
        let mut buf = ScreenBuffer::new(5, 5, 100);
        let _ = buf.get_mut(0, 0);
        let dirty = buf.take_dirty();
        assert!(!dirty.is_empty());
        let after = buf.take_dirty();
        assert!(after.is_empty());
    }

    // --- DirtyRegion helpers ---

    #[test]
    fn dirty_region_mark_full_redraw_overrides_rows() {
        let mut region = DirtyRegion::default();
        region.mark_row(3);
        region.mark_full_redraw();
        assert!(region.is_full_redraw);
        assert!(region.rows.is_empty());
    }

    #[test]
    fn dirty_region_merge_propagates_full_redraw() {
        let mut a = DirtyRegion::default();
        a.mark_row(1);
        let mut b = DirtyRegion::default();
        b.mark_full_redraw();
        a.merge(&b);
        assert!(a.is_full_redraw);
    }

    // --- Snapshot ---

    #[test]
    fn snapshot_cell_count_equals_cols_times_rows() {
        let buf = ScreenBuffer::new(80, 24, 1000);
        let snap = buf.snapshot(0, 0, true, 0, 0);
        assert_eq!(snap.cells.len(), 80 * 24);
        assert_eq!(snap.cols, 80);
        assert_eq!(snap.rows, 24);
    }
}
