// SPDX-License-Identifier: MPL-2.0

use super::super::api::{SearchQuery, search_scrollback};
use super::helpers::{hard, make_row};

// -----------------------------------------------------------------------
// FS-SEARCH-003 — regex search
// -----------------------------------------------------------------------

#[test]
fn fs_search_003_regex_finds_match() {
    let lines = [hard(make_row("error: file not found", 30))];
    let query = SearchQuery {
        text: r"error:\s+\w+".to_string(),
        case_sensitive: false,
        regex: true,
    };
    let matches = search_scrollback(lines.iter(), &query);
    assert!(!matches.is_empty(), "Regex should find a match");
    assert_eq!(matches[0].col_start, 0);
}

#[test]
fn fs_search_invalid_regex_returns_empty() {
    let lines = [hard(make_row("test", 10))];
    let query = SearchQuery {
        text: "[invalid(regex".to_string(),
        case_sensitive: false,
        regex: true,
    };
    let matches = search_scrollback(lines.iter(), &query);
    assert!(
        matches.is_empty(),
        "Invalid regex must return empty, not panic"
    );
}
