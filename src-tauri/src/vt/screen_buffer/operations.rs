// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

use super::buffer::ScreenBuffer;
use super::dirty_region::DirtyRegion;
use super::scrollback::ScrollbackLine;
use super::snapshot::{ScreenSnapshot, SnapshotCell};

impl ScreenBuffer {
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

    /// Insert `count` blank cells at `(row, col)`, shifting existing cells right.
    /// Cells that would extend beyond the line are discarded (no wrap to next row).
    /// Conforms to ECMA-48 §8.3.64 (ICH).
    pub fn insert_cells(&mut self, row: u16, col: u16, count: u16) {
        if let Some(r) = self.cells.get_mut(row as usize) {
            let col = col as usize;
            let count = count as usize;
            let len = r.len();
            if col >= len {
                return; // cursor is beyond the line — no-op
            }
            // Shift cells [col .. len-count) to the right by `count` positions.
            // Cells that overflow the right margin are discarded.
            let shift = count.min(len - col);
            // Move cells rightward by shifting from right to left.
            let src_end = len - shift; // last source index (exclusive)
            for i in (col..src_end).rev() {
                r[i + shift] = r[i].clone();
            }
            // Fill the vacated cells with blank defaults.
            for cell in r[col..col + shift].iter_mut() {
                *cell = Cell::default();
            }
            self.dirty.mark_row(row);
        }
    }

    /// Delete `count` cells at `(row, col)`, shifting remaining cells left.
    /// Cells from the right fill with blanks. Conforms to ECMA-48 §8.3.26 (DCH).
    pub fn delete_cells(&mut self, row: u16, col: u16, count: u16) {
        if let Some(r) = self.cells.get_mut(row as usize) {
            let col = col as usize;
            let count = count as usize;
            let len = r.len();
            if col >= len {
                return;
            }
            let shift = count.min(len - col);
            // Move cells leftward.
            for i in col..len - shift {
                r[i] = r[i + shift].clone();
            }
            // Fill the vacated cells at the right with blanks.
            for cell in r[len - shift..len].iter_mut() {
                *cell = Cell::default();
            }
            self.dirty.mark_row(row);
        }
    }

    /// Scroll up by `count` lines within `[top, bottom]` (DECSTBM-bounded).
    /// Lines scrolled off the top of a full-screen region enter scrollback.
    /// Lines scrolled off a partial region are discarded (FS-SB-004).
    ///
    /// `soft_wrapped` — when `true`, the first evicted line is marked as a soft
    /// wrap (it was pushed out by auto-wrap, not a hard newline). Subsequent
    /// lines in the same `scroll_up` call are always hard-newline lines.
    pub fn scroll_up(
        &mut self,
        top: u16,
        bottom: u16,
        count: u16,
        is_full_screen: bool,
        soft_wrapped: bool,
    ) {
        let count = count as usize;
        let top = top as usize;
        let bottom = bottom as usize;

        // `top > bottom` is an invalid region; `top == bottom` is a single-row
        // region (e.g. a 1-row terminal) which is valid and must still evict the
        // row into scrollback.
        if top > bottom || top >= self.cells.len() {
            return;
        }

        let bottom = bottom.min(self.cells.len() - 1);

        for i in 0..count {
            if is_full_screen && !self.cells.is_empty() {
                let blank = vec![Cell::default(); self.cols as usize];
                let evicted = std::mem::replace(&mut self.cells[top], blank);
                if self.scrollback.len() >= self.scrollback_limit {
                    self.scrollback.pop_front();
                }
                // Only the first evicted line in this scroll_up call may be
                // soft-wrapped — subsequent ones are always hard-newline lines
                // (they correspond to existing screen content, not the cursor row).
                self.scrollback.push_back(ScrollbackLine {
                    cells: evicted,
                    soft_wrapped: soft_wrapped && i == 0,
                });
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
        if self.cells.is_empty() || top > bottom {
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
        debug_assert!(
            new_rows <= 256,
            "ScreenBuffer::resize: new_rows={new_rows} exceeds DirtyRows capacity (256)"
        );
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
    pub fn get_scrollback_line(&self, index: usize) -> Option<&ScrollbackLine> {
        self.scrollback.get(index)
    }

    /// Iterate over all scrollback lines from oldest to newest.
    pub fn scrollback_iter(&self) -> impl Iterator<Item = &ScrollbackLine> {
        self.scrollback.iter()
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
