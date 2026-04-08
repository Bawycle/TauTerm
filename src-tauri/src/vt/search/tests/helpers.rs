// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::{Cell, CellAttrs};
use crate::vt::screen_buffer::ScrollbackLine;

pub(super) fn make_row(text: &str, cols: usize) -> Vec<Cell> {
    let mut row = vec![Cell::default(); cols];
    for (i, ch) in text.chars().enumerate() {
        if i < cols {
            row[i].grapheme = compact_str::format_compact!("{ch}");
        }
    }
    row
}

pub(super) fn make_cell(g: &str) -> Cell {
    Cell {
        grapheme: g.into(),
        attrs: CellAttrs::default(),
        width: 1,
        hyperlink: None,
    }
}

/// Wrap a `Vec<Cell>` into a hard-newline `ScrollbackLine`.
pub(super) fn hard(cells: Vec<Cell>) -> ScrollbackLine {
    ScrollbackLine {
        cells,
        soft_wrapped: false,
    }
}

/// Wrap a `Vec<Cell>` into a soft-wrapped `ScrollbackLine`.
pub(super) fn soft(cells: Vec<Cell>) -> ScrollbackLine {
    ScrollbackLine {
        cells,
        soft_wrapped: true,
    }
}
