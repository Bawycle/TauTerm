// SPDX-License-Identifier: MPL-2.0

use super::dirty_rows::DirtyRows;

/// Describes a rectangular region of dirty cells to be sent as a screen update.
#[derive(Debug, Clone, Default)]
pub struct DirtyRegion {
    pub rows: DirtyRows,
    pub is_full_redraw: bool,
    /// Set when the cursor position or visibility changed without any cell content change.
    /// Ensures that pure cursor-movement sequences (CR, CUP, CUU, etc.) and cursor
    /// visibility toggles (`?25h`/`?25l`) still trigger a `screen-update` event.
    pub cursor_moved: bool,
}

impl DirtyRegion {
    pub fn is_empty(&self) -> bool {
        !self.is_full_redraw && self.rows.is_empty() && !self.cursor_moved
    }

    pub fn mark_row(&mut self, row: u16) {
        self.rows.set(row);
    }

    pub fn mark_full_redraw(&mut self) {
        self.is_full_redraw = true;
        self.rows.clear();
    }

    pub fn mark_cursor_moved(&mut self) {
        self.cursor_moved = true;
    }

    pub fn merge(&mut self, other: &DirtyRegion) {
        if other.is_full_redraw {
            self.mark_full_redraw();
        } else {
            self.rows.merge_from(&other.rows);
        }
        self.cursor_moved |= other.cursor_moved;
    }
}
