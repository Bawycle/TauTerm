// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;
use crate::vt::screen_buffer::ScrollbackLine;

use super::text_conversion::cells_to_text;

/// A logical search unit: one or more consecutive scrollback rows joined by soft wraps.
pub(super) struct LogicalLine<'a> {
    /// Row index (in the original scrollback) of the first row in this group.
    pub(super) first_row: usize,
    /// All constituent rows, in order.
    pub(super) rows: Vec<&'a Vec<Cell>>,
}

/// Group consecutive scrollback lines into logical lines, joining across soft-wrap
/// boundaries (FS-SB-008 / FS-SEARCH-002).
///
/// A row with `soft_wrapped == true` continues into the next row. The last row of
/// a group has `soft_wrapped == false` (or is the final row in the scrollback).
pub(super) fn build_logical_lines<'a>(
    scrollback_lines: impl Iterator<Item = &'a ScrollbackLine>,
) -> Vec<LogicalLine<'a>> {
    let mut groups: Vec<LogicalLine<'_>> = Vec::new();
    let mut current: Option<LogicalLine<'_>> = None;

    for (idx, sl) in scrollback_lines.enumerate() {
        match current.as_mut() {
            None => {
                current = Some(LogicalLine {
                    first_row: idx,
                    rows: vec![&sl.cells],
                });
            }
            Some(group) => {
                group.rows.push(&sl.cells);
            }
        }
        // When a line is NOT soft-wrapped, it terminates the current group.
        if !sl.soft_wrapped
            && let Some(group) = current.take()
        {
            groups.push(group);
        }
    }
    // Flush any open group (trailing soft-wrapped lines or last line overall).
    if let Some(group) = current {
        groups.push(group);
    }
    groups
}

/// Convert a logical line (potentially multiple rows) to a flat string with
/// `(row_offset, col)` position metadata for each character.
///
/// `row_offset` is relative to `LogicalLine::first_row`.
pub(super) fn logical_line_to_text(rows: &[&Vec<Cell>]) -> (String, Vec<(usize, u16)>) {
    let mut text = String::new();
    let mut positions: Vec<(usize, u16)> = Vec::new();

    for (row_offset, row) in rows.iter().enumerate() {
        let (row_text, char_to_col) = cells_to_text(row);
        text.push_str(&row_text);
        for col in char_to_col {
            positions.push((row_offset, col));
        }
    }

    (text, positions)
}
