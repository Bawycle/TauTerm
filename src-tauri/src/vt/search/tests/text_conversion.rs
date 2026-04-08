// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

use super::super::text_conversion::cells_to_text;
use super::helpers::make_cell;

#[test]
fn cells_to_text_skips_phantom_cells() {
    // Simulate a wide char 'W' at col 0, phantom at col 1, 'x' at col 2.
    // Only 3 cells so there are no trailing default spaces.
    let mut row = vec![Cell::phantom(); 3];
    row[0] = make_cell("W");
    row[0].width = 2;
    row[1] = Cell::phantom();
    row[2] = make_cell("x");
    let (text, char_to_col) = cells_to_text(&row);
    assert_eq!(text, "Wx");
    // char index 0 = 'W' → col 0
    assert_eq!(char_to_col[0], 0);
    // char index 1 = 'x' → col 2 (phantom at col 1 is skipped)
    assert_eq!(char_to_col[1], 2);
}
