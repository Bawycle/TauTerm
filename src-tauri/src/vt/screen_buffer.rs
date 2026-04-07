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

// ---------------------------------------------------------------------------
// Scrollback line
// ---------------------------------------------------------------------------

/// A single line in the scrollback ring.
///
/// `soft_wrapped` is `true` when this line ended because the terminal width was
/// exhausted (the cursor automatically wrapped to the next row) rather than
/// because a hard newline (`\n`) was received.
///
/// This flag is used by the search engine to join consecutive soft-wrapped lines
/// into a single logical string (FS-SB-008, FS-SEARCH-002).
#[derive(Debug, Clone)]
pub struct ScrollbackLine {
    /// Cell content of the line.
    pub cells: Vec<Cell>,
    /// `true` if the line break is a soft wrap; `false` if it is a hard newline.
    pub soft_wrapped: bool,
}

/// A scrollback line returned by `VtProcessor::get_scrollback_line`.
///
/// Carries both the cell content and the `soft_wrapped` flag so that callers
/// (e.g. IPC commands, selection copy) can join soft-wrapped lines correctly
/// without producing spurious newlines (FS-SB-011).
#[derive(Debug, Clone)]
pub struct ScrollbackLineRef {
    /// Cell content of the line (cloned from the ring).
    pub cells: Vec<Cell>,
    /// `true` if the line break was caused by auto-wrap rather than a hard newline.
    pub soft_wrapped: bool,
}

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

/// A single cell in a screen snapshot, serialized to the frontend for rendering.
///
/// # Frontend rendering contracts
///
/// The frontend MUST respect the following rules when interpreting a `SnapshotCell`:
///
/// ## Bold color promotion
/// When `bold == true` and `fg` is `Color::Ansi { index }` with `index` in `[1, 7]`
/// (the 7 non-black standard colors), the frontend MUST resolve the displayed color
/// using `index + 8` (the bright variant). Index 0 (black) is **excluded** from
/// promotion — bold black renders as ordinary black.
///
/// ## Dim (faint)
/// When `dim == true`, the frontend MUST apply `opacity: var(--term-dim-opacity)`
/// (design token value: 0.5) to the foreground color. Dim is independent of `bold`
/// and applies after bold color promotion.
///
/// ## Reverse video
/// When `inverse == true`, the frontend MUST swap the resolved foreground and
/// background colors. The swap operates on the **resolved** values — i.e. after
/// bold color promotion has been applied to `fg`, and after substituting terminal
/// defaults for any `None` color.
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
    /// OSC 8 hyperlink URI for this cell, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperlink: Option<String>,
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
            hyperlink: cell.hyperlink.as_ref().map(|h| h.as_ref().to_owned()),
        }
    }
}

/// Describes a rectangular region of dirty cells to be sent as a screen update.
#[derive(Debug, Clone, Default)]
pub struct DirtyRegion {
    pub rows: std::collections::HashSet<u16>,
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
        self.rows.insert(row);
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
            self.rows.extend(&other.rows);
        }
        self.cursor_moved |= other.cursor_moved;
    }
}

/// The full screen buffer state for one screen (normal or alternate).
pub struct ScreenBuffer {
    pub cols: u16,
    pub rows: u16,
    /// Row-major grid: `rows × cols` cells.
    cells: Vec<Vec<Cell>>,
    /// Scrollback ring (normal screen only; capacity = `scrollback_limit`).
    scrollback: std::collections::VecDeque<ScrollbackLine>,
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
                let evicted = self.cells[top].clone();
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
        buf.scroll_up(0, 2, 1, true, false);
        assert_eq!(buf.scrollback_len(), 1);
    }

    #[test]
    fn scroll_up_partial_region_does_not_add_to_scrollback() {
        let mut buf = ScreenBuffer::new(5, 5, 100);
        buf.scroll_up(1, 3, 1, false, false);
        assert_eq!(buf.scrollback_len(), 0);
    }

    #[test]
    fn scroll_down_one_line_region_does_not_panic() {
        // Regression: scroll_down with top == bottom (1-row region) must not
        // panic. The old guard `top >= bottom` would early-return, preventing
        // the scroll; the corrected guard `top > bottom` allows scroll_down to
        // operate on a single row (clear it), which is the correct VT behaviour
        // for CSI T (SD) targeting a 1-row region.
        let mut buf = ScreenBuffer::new(5, 5, 100);
        // Write a marker on row 2.
        if let Some(cell) = buf.get_mut(2, 0) {
            cell.grapheme = "X".to_string();
        }
        // scroll_down with a 1-row region [2, 2] — must not panic.
        buf.scroll_down(2, 2, 1);
        // The single row in the region is cleared to Cell::default().
        assert_eq!(
            buf.get(2, 0).map(|c| c.grapheme.as_str()),
            Some(&*Cell::default().grapheme)
        );
    }

    #[test]
    fn scrollback_respects_limit() {
        let limit = 3usize;
        let mut buf = ScreenBuffer::new(5, 1, limit);
        for _ in 0..10 {
            buf.scroll_up(0, 0, 1, true, false);
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
    fn cursor_moved_makes_non_empty() {
        let mut d = DirtyRegion::default();
        d.mark_cursor_moved();
        assert!(
            !d.is_empty(),
            "cursor_moved must make DirtyRegion non-empty"
        );
    }

    #[test]
    fn merge_propagates_cursor_moved() {
        let mut a = DirtyRegion::default();
        let mut b = DirtyRegion::default();
        b.mark_cursor_moved();
        a.merge(&b);
        assert!(
            a.cursor_moved,
            "merge must propagate cursor_moved from source"
        );
        assert!(!a.is_empty());
    }

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
