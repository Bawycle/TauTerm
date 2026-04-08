// SPDX-License-Identifier: MPL-2.0

use super::super::api::{SearchQuery, search_scrollback};
use super::helpers::{hard, make_row, soft};

// -----------------------------------------------------------------------
// SEARCH-SOFT-001: soft-wrapped line — cross-row match
//
// When a word spans two consecutive rows connected by a soft wrap,
// search_scrollback must join them and find the cross-row match.
// -----------------------------------------------------------------------

/// SEARCH-SOFT-001: word spanning a soft-wrapped boundary is found as one match.
///
/// cols=5, row0="hello" (soft-wrapped), row1="world"
/// Query: "helloworld" → one match starting on row 0.
#[test]
fn search_soft_wrap_word_spanning_two_rows_is_found() {
    // row0 is soft-wrapped (continues into row1); row1 is a hard newline.
    let lines = [soft(make_row("hello", 5)), hard(make_row("world", 5))];

    let query = SearchQuery {
        text: "helloworld".to_string(),
        case_sensitive: false,
        regex: false,
    };

    let matches = search_scrollback(lines.iter(), &query);
    assert!(
        !matches.is_empty(),
        "Soft-wrap search must find 'helloworld' spanning rows 0 and 1"
    );
    assert_eq!(matches[0].scrollback_row, 0, "Match must start on row 0");
}

// -----------------------------------------------------------------------
// SEARCH-SOFT-002: word spanning three soft-wrapped rows
// -----------------------------------------------------------------------

/// SEARCH-SOFT-002: word spanning three soft-wrapped rows is found.
///
/// cols=4, row0="abcd" (soft), row1="efgh" (soft), row2="ijkl" (hard)
/// Query: "abcdefghijkl" → one match starting on row 0.
#[test]
fn search_soft_wrap_word_spanning_three_rows_is_found() {
    let lines = [
        soft(make_row("abcd", 4)),
        soft(make_row("efgh", 4)),
        hard(make_row("ijkl", 4)),
    ];

    let query = SearchQuery {
        text: "abcdefghijkl".to_string(),
        case_sensitive: true,
        regex: false,
    };

    let matches = search_scrollback(lines.iter(), &query);
    assert!(
        !matches.is_empty(),
        "Soft-wrap search must find 'abcdefghijkl' spanning rows 0, 1, and 2"
    );
    assert_eq!(matches[0].scrollback_row, 0, "Match must start on row 0");
}

// -----------------------------------------------------------------------
// SEARCH-SOFT-003: cross-row match where only the boundary chars span rows
// -----------------------------------------------------------------------

/// SEARCH-SOFT-003: query spanning only 2 chars at the row boundary is found.
///
/// cols=3, row0="aab" (soft), row1="bcc" (hard)
/// Query: "bb" → spans col 2 of row0 and col 0 of row1.
#[test]
fn search_soft_wrap_boundary_chars_found() {
    let lines = [soft(make_row("aab", 3)), hard(make_row("bcc", 3))];

    let query = SearchQuery {
        text: "bb".to_string(),
        case_sensitive: true,
        regex: false,
    };

    let matches = search_scrollback(lines.iter(), &query);
    assert!(
        !matches.is_empty(),
        "Soft-wrap search must find 'bb' spanning the boundary of rows 0 and 1"
    );
}
