// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

use super::dirty_region::DirtyRegion;
use super::scrollback::ScrollbackLine;

/// The full screen buffer state for one screen (normal or alternate).
pub struct ScreenBuffer {
    pub cols: u16,
    pub rows: u16,
    /// Row-major grid: `rows × cols` cells.
    pub(super) cells: Vec<Vec<Cell>>,
    /// Scrollback ring (normal screen only; capacity = `scrollback_limit`).
    pub(super) scrollback: std::collections::VecDeque<ScrollbackLine>,
    pub scrollback_limit: usize,
    /// Pending dirty region since last `take_dirty`.
    pub(super) dirty: DirtyRegion,
}

impl ScreenBuffer {
    pub fn new(cols: u16, rows: u16, scrollback_limit: usize) -> Self {
        // DirtyRows is a [u64; 4] bitfield supporting up to 256 rows.
        // Enforce this at construction time so dirty-tracking never silently drops rows.
        debug_assert!(
            rows <= 256,
            "ScreenBuffer::new: rows={rows} exceeds DirtyRows capacity (256)"
        );
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
}
