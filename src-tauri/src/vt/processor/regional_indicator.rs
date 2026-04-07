// SPDX-License-Identifier: MPL-2.0

//! Regional Indicator pair-detection helpers for `VtProcessor`.
//!
//! Contains `handle_regional_indicator()`, `flush_pending_ri_narrow()`, and
//! `confirm_ri_narrow()`.

use compact_str::CompactString;

use crate::vt::cell::Cell;

use super::VtProcessor;

impl VtProcessor {
    /// Process a Regional Indicator codepoint.
    ///
    /// - If no RI is pending: write provisionally as 2-cell wide char and store in
    ///   `pending_ri`.
    /// - If an RI is pending on the *same row*: confirm as a flag pair — rewrite
    ///   both RIs into the same 2 cells (grapheme = base + second RI), clear pending.
    /// - If the pending RI is on a *different row*: confirm the previous as narrow
    ///   (1-cell) and start a fresh provisional for the new RI.
    pub(super) fn handle_regional_indicator(&mut self, c: char) {
        let current_row = self.cursor_row();

        if let Some((prev_ch, prev_col, prev_row)) = self.pending_ri.take() {
            if prev_row == current_row {
                // Second RI on the same row → confirmed flag pair.
                // The first RI was written provisionally at (prev_row, prev_col) as width=2.
                // We now update the grapheme of that cell to include both codepoints and
                // leave the cursor at prev_col + 2 (the phantom cell stays in place).
                let prev_attrs = self
                    .active_buf_ref()
                    .get(prev_row, prev_col)
                    .map(|cell| cell.attrs)
                    .unwrap_or_default();
                let prev_hyperlink = self
                    .active_buf_ref()
                    .get(prev_row, prev_col)
                    .and_then(|cell| cell.hyperlink.clone());
                let mut flag_grapheme = CompactString::const_new("");
                flag_grapheme.push(prev_ch);
                flag_grapheme.push(c);
                // Rewrite the base cell with the full flag grapheme; phantom stays.
                if let Some(cell) = self.active_buf_mut().get_mut(prev_row, prev_col) {
                    cell.grapheme = flag_grapheme;
                    cell.attrs = prev_attrs;
                    cell.width = 2;
                    cell.hyperlink = prev_hyperlink;
                }
                // Cursor is already at prev_col + 2 from the provisional write.
                return;
            } else {
                // Different row: confirm the previous RI as narrow (1-cell).
                self.confirm_ri_narrow(prev_ch, prev_col, prev_row);
            }
        }

        // Write the new RI provisionally as a 2-cell wide char and store pending.
        let (write_row, write_col) = self.apply_wrap_pending();
        let attrs = self.current_attrs;
        let hyperlink = self.current_hyperlink.clone();
        let cols = self.cols;
        if let Some(cell) = self.active_buf_mut().get_mut(write_row, write_col) {
            cell.grapheme = compact_str::format_compact!("{c}");
            cell.attrs = attrs;
            cell.width = 2;
            cell.hyperlink = hyperlink;
        }
        if write_col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(write_row, write_col + 1)
        {
            *cell = Cell::phantom();
        }
        let new_col = write_col + 2;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols.saturating_sub(1);
        } else {
            self.active_cursor_mut().col = new_col;
        }
        self.pending_ri = Some((c, write_col, write_row));
    }

    /// Confirm a pending RI as narrow (width=1) — called when followed by non-RI.
    pub(super) fn flush_pending_ri_narrow(&mut self) {
        if let Some((ch, col, row)) = self.pending_ri.take() {
            self.confirm_ri_narrow(ch, col, row);
        }
    }

    /// Rewrite the RI cell at `(row, col)` as width=1 and clear the phantom.
    pub(super) fn confirm_ri_narrow(&mut self, ch: char, col: u16, row: u16) {
        let attrs = self
            .active_buf_ref()
            .get(row, col)
            .map(|c| c.attrs)
            .unwrap_or_default();
        let hyperlink = self
            .active_buf_ref()
            .get(row, col)
            .and_then(|c| c.hyperlink.clone());
        let cols = self.cols;
        // Rewrite the RI cell as narrow (width=1).
        if let Some(cell) = self.active_buf_mut().get_mut(row, col) {
            cell.grapheme = compact_str::format_compact!("{ch}");
            cell.attrs = attrs;
            cell.width = 1;
            cell.hyperlink = hyperlink;
        }
        // Clear the former phantom cell.
        if col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(row, col + 1)
            && cell.is_phantom()
        {
            *cell = Cell::default();
        }
        // Move the cursor to just after the narrow RI cell.
        self.active_cursor_mut().row = row;
        let new_col = col + 1;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols.saturating_sub(1);
        } else {
            self.active_cursor_mut().col = new_col;
        }
    }
}
