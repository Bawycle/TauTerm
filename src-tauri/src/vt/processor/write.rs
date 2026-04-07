// SPDX-License-Identifier: MPL-2.0

//! Character write helpers for `VtProcessor`.
//!
//! Contains `write_char()`, `apply_wrap_pending()`, and `write_char_at_width()`.

use unicode_width::UnicodeWidthChar;

use crate::vt::cell::Cell;

use super::VtProcessor;

impl VtProcessor {
    /// Write the current character to the active buffer at cursor position, then advance.
    ///
    /// Handles the following special cases before the main write path:
    ///
    /// - **R7 (FS-VT-018)**: Skin-tone modifiers U+1F3FB–U+1F3FF are treated as
    ///   combining marks (width=0): they attach to the preceding cell without advancing
    ///   the cursor.  `unicode_width` would return 2 for these codepoints, so the check
    ///   must occur *before* the width lookup.
    ///
    /// - **R8 (FS-VT-019)**: Regional Indicator (RI) codepoints U+1F1E6–U+1F1FF are
    ///   written provisionally as 2-cell chars.  A *second consecutive* RI on the same
    ///   row confirms a flag pair in those same 2 cells.  A non-RI codepoint confirms
    ///   the previous RI as a 1-cell narrow char (spec: unpaired RI = narrow).
    ///
    /// - **R6 (FS-VT-017)**: Width-1 non-ASCII codepoints that could be emoji are
    ///   buffered in `pending_emoji` until the next codepoint is known.  U+FE0F forces
    ///   width=2 (emoji presentation); U+FE0E keeps width=1 (text presentation).  Any
    ///   other codepoint flushes the buffer at the natural width and is then processed
    ///   normally.  ASCII and inherently-wide (width=2) codepoints are never buffered.
    pub(super) fn write_char(&mut self, c: char) {
        // --- R7: skin-tone modifiers are combining marks (width=0) ------------------
        if matches!(c, '\u{1F3FB}'..='\u{1F3FF}') {
            // A skin-tone modifier is not a second Regional Indicator, so a pending
            // lone RI must be committed as narrow (width=1), not as a confirmed flag.
            self.flush_pending_ri_narrow();
            self.flush_pending_emoji(None);
            let row = self.cursor_row();
            let col = self.cursor_col();
            // Locate the base cell: step back from the cursor, skipping phantom cells
            // (which are the trailing slot of wide characters).  A skin-tone modifier
            // must attach to the *base* cell of the preceding grapheme, not its phantom.
            let target_col = if col > 0 {
                let mut tc = col - 1;
                // If the cell immediately before the cursor is a phantom, walk back
                // one more position to reach the base cell.
                if self
                    .active_buf_ref()
                    .get(row, tc)
                    .is_some_and(|cell| cell.is_phantom())
                    && tc > 0
                {
                    tc -= 1;
                }
                tc
            } else {
                0
            };
            if let Some(cell) = self.active_buf_mut().get_mut(row, target_col) {
                cell.grapheme.push(c);
            }
            return;
        }

        // --- R8: Regional Indicator pair detection ----------------------------------
        if matches!(c, '\u{1F1E6}'..='\u{1F1FF}') {
            self.flush_pending_emoji(None);
            self.handle_regional_indicator(c);
            return;
        }

        // Any non-RI codepoint confirms a pending RI as narrow (unpaired).
        if self.pending_ri.is_some() {
            self.flush_pending_ri_narrow();
        }

        // --- R6: variation selectors ------------------------------------------------
        if c == '\u{FE0F}' {
            // Emoji presentation: upgrade pending base to width=2 (if eligible).
            self.flush_pending_emoji(Some(2));
            return;
        }
        if c == '\u{FE0E}' {
            // Text presentation: flush pending base at width=1.
            self.flush_pending_emoji(Some(1));
            return;
        }

        // Any other codepoint: flush any pending emoji base at its natural width,
        // then proceed to write `c`.
        self.flush_pending_emoji(None);

        // --- Compute width ----------------------------------------------------------
        let char_width = UnicodeWidthChar::width(c).unwrap_or(1) as u8;

        // --- Combining / zero-width characters (width == 0) ------------------------
        // Attach the combining mark to the previous cell (or the current cell when
        // at the start of a line) without advancing the cursor (FS-VT-012/013).
        if char_width == 0 {
            let row = self.cursor_row();
            let col = self.cursor_col();
            let (target_row, target_col) = if col > 0 { (row, col - 1) } else { (row, 0) };
            if let Some(cell) = self.active_buf_mut().get_mut(target_row, target_col) {
                cell.grapheme.push(c);
            }
            return;
        }

        // --- R6 buffering: width-1, potentially-emoji codepoints -------------------
        // Buffer codepoints that are width=1 by unicode_width but *could* have an
        // emoji variation sequence (FE0F / FE0E).  Only codepoints listed in the
        // Unicode `emoji-variation-sequences.txt` data file need to be buffered.
        // ASCII and non-emoji blocks (e.g. box-drawing U+2500–U+257F, Latin extended,
        // currency symbols) are written immediately without buffering.
        if char_width == 1 && super::emoji::is_emoji_vs_eligible(c) {
            // Snapshot the cursor position *after* any pending wrap is applied so
            // that the stored col/row are the actual write position.
            let (write_row, write_col) = self.apply_wrap_pending();
            let attrs = self.current_attrs;
            let hyperlink = self.current_hyperlink.clone();
            self.pending_emoji = Some(super::PendingEmoji {
                ch: c,
                attrs,
                hyperlink,
                col: write_col,
                row: write_row,
            });
            return;
        }

        // --- Normal write path -----------------------------------------------------
        self.write_char_at_width(c, char_width);
    }

    /// Apply the DEC delayed-wrap (if set) and return the resulting (row, col).
    ///
    /// If `wrap_pending` is true *and* DECAWM is enabled, the cursor is moved to
    /// the first column of the next row (or scrolled).  In all cases the final
    /// (row, col) after the potential wrap is returned.  This helper does *not*
    /// write anything to the grid.
    pub(super) fn apply_wrap_pending(&mut self) -> (u16, u16) {
        let row = self.cursor_row();
        let _col = self.cursor_col();
        if self.wrap_pending && self.modes.decawm {
            self.wrap_pending = false;
            let (top, bottom) = self.modes.scroll_region;
            let is_full = top == 0 && bottom == self.rows.saturating_sub(1);
            if row == bottom {
                self.active_buf_mut()
                    .scroll_up(top, bottom, 1, is_full, true);
            } else {
                self.active_cursor_mut().row = (row + 1).min(self.rows.saturating_sub(1));
            }
            self.active_cursor_mut().col = 0;
        }
        (self.cursor_row(), self.cursor_col())
    }

    /// Core write: place `c` at the current cursor position with explicit `width`,
    /// write a phantom cell if `width == 2`, then advance the cursor.
    ///
    /// Applies the DEC delayed-wrap before writing.
    pub(super) fn write_char_at_width(&mut self, c: char, width: u8) {
        let (row, col) = self.apply_wrap_pending();
        let attrs = self.current_attrs;
        let cols = self.cols;
        let hyperlink = self.current_hyperlink.clone();

        if let Some(cell) = self.active_buf_mut().get_mut(row, col) {
            cell.grapheme = c.to_string();
            cell.attrs = attrs;
            cell.width = width;
            cell.hyperlink = hyperlink;
        }

        // Place phantom cell for wide characters (FS-VT-011).
        if width == 2
            && col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(row, col + 1)
        {
            *cell = Cell::phantom();
        }

        // Advance cursor.
        let new_col = col + width as u16;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols - 1;
        } else {
            self.active_cursor_mut().col = new_col;
        }
    }
}
