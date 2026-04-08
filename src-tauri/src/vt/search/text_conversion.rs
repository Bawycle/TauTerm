// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

/// Convert a row of cells to a plain string and a mapping from char index → col.
///
/// Phantom cells (continuations of wide characters) are skipped in the text
/// but the col mapping still points to the wide character's starting column.
pub(super) fn cells_to_text(row: &[Cell]) -> (String, Vec<u16>) {
    let mut text = String::with_capacity(row.len());
    // char_to_col[i] = column index of the i-th char in `text`.
    let mut char_to_col: Vec<u16> = Vec::with_capacity(row.len());

    for (col, cell) in row.iter().enumerate() {
        if cell.grapheme.is_empty() || cell.is_phantom() {
            // Phantom cell — skip (part of a wide character already output).
            continue;
        }
        let start_char_idx = text.chars().count();
        text.push_str(&cell.grapheme);
        let end_char_idx = text.chars().count();
        for _ in start_char_idx..end_char_idx {
            char_to_col.push(col as u16);
        }
    }

    // Strip trailing whitespace — terminal rows are padded with spaces to
    // column width, which would otherwise produce spurious search misses.
    let trimmed_len = text.trim_end().chars().count();
    text.truncate(text.trim_end().len());
    char_to_col.truncate(trimmed_len);

    (text, char_to_col)
}
