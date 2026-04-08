// SPDX-License-Identifier: MPL-2.0

use super::super::api::{SearchQuery, search_scrollback};
use super::helpers::{hard, make_row};

// -----------------------------------------------------------------------
// FS-SEARCH-001 — literal case-insensitive search returns matches
// -----------------------------------------------------------------------

#[test]
fn fs_search_001_literal_case_insensitive_finds_match() {
    let lines = [hard(make_row("Hello World", 20))];
    let query = SearchQuery {
        text: "hello".to_string(),
        case_sensitive: false,
        regex: false,
    };
    let matches = search_scrollback(lines.iter(), &query);
    assert_eq!(matches.len(), 1, "Should find one match");
    assert_eq!(matches[0].scrollback_row, 0);
    assert_eq!(matches[0].col_start, 0);
    assert_eq!(matches[0].col_end, 5);
}

#[test]
fn fs_search_empty_query_returns_empty() {
    let lines = [hard(make_row("hello", 10))];
    let query = SearchQuery {
        text: String::new(),
        case_sensitive: false,
        regex: false,
    };
    let matches = search_scrollback(lines.iter(), &query);
    assert!(matches.is_empty());
}

#[test]
fn fs_search_multiple_matches_on_same_row() {
    let lines = [hard(make_row("aabaa", 10))];
    let query = SearchQuery {
        text: "a".to_string(),
        case_sensitive: true,
        regex: false,
    };
    let matches = search_scrollback(lines.iter(), &query);
    assert_eq!(matches.len(), 4, "Should find 4 'a' matches");
}

#[test]
fn fs_search_case_sensitive_no_match() {
    let lines = [hard(make_row("Hello", 10))];
    let query = SearchQuery {
        text: "hello".to_string(),
        case_sensitive: true,
        regex: false,
    };
    let matches = search_scrollback(lines.iter(), &query);
    assert!(matches.is_empty(), "Case-sensitive search should not match");
}

// -----------------------------------------------------------------------
// SEARCH-LITERAL-001: regex metacharacters are safe as literal text
//
// When query.regex = false, metacharacters like .* must not be interpreted
// as regex — they must be matched literally.
// -----------------------------------------------------------------------

/// SEARCH-LITERAL-001: '.*' treated as literal two-char sequence, not wildcard.
#[test]
fn search_literal_regex_special_chars_treated_as_literal() {
    // Row contains the literal string "foo.*bar"
    let lines = [hard(make_row("foo.*bar  ", 10))];
    let query = SearchQuery {
        text: "foo.*bar".to_string(),
        case_sensitive: true,
        regex: false, // literal mode — must NOT treat .* as regex
    };
    let matches = search_scrollback(lines.iter(), &query);
    // If .* were treated as regex, it would either over-match or panic.
    // With literal search, there must be exactly one match.
    assert_eq!(
        matches.len(),
        1,
        "Literal search for 'foo.*bar' must find the exact string, not treat .* as regex"
    );
    assert_eq!(matches[0].col_start, 0);
    assert_eq!(matches[0].col_end, 8); // "foo.*bar" is 8 chars
}

/// SEARCH-LITERAL-002: regex metacharacter '(' as literal does not panic.
#[test]
fn search_literal_open_paren_does_not_panic() {
    let lines = [hard(make_row("func(arg) rest", 20))];
    let query = SearchQuery {
        text: "func(arg)".to_string(),
        case_sensitive: true,
        regex: false,
    };
    // Must not panic and must find the literal match
    let matches = search_scrollback(lines.iter(), &query);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].col_start, 0);
}

// -----------------------------------------------------------------------
// SEARCH-HARD-001: hard-newline rows are NOT joined
//
// Verifies that two hard-newline rows are not concatenated, so a word
// that would only appear if they were joined is NOT found.
// -----------------------------------------------------------------------

/// SEARCH-HARD-001: two hard-newline rows are treated as separate logical lines.
#[test]
fn search_hard_newline_rows_are_not_joined() {
    // "hello" ends with a hard newline, so "world" is on a new logical line.
    let lines = [hard(make_row("hello", 5)), hard(make_row("world", 5))];

    let query = SearchQuery {
        text: "helloworld".to_string(),
        case_sensitive: false,
        regex: false,
    };

    // The query spans both rows — but since they're hard-newline separated,
    // no match should be found.
    let matches = search_scrollback(lines.iter(), &query);
    assert!(
        matches.is_empty(),
        "Hard-newline rows must NOT be joined: 'helloworld' must not match"
    );
}
