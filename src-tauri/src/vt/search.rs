// SPDX-License-Identifier: MPL-2.0

//! Scrollback search — iterate scrollback lines, skip soft-wrap boundaries,
//! return `SearchMatch` positions.

use serde::{Deserialize, Serialize};

use crate::vt::cell::Cell;

/// A search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub text: String,
    pub case_sensitive: bool,
    pub regex: bool,
}

/// A single search match position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    /// Row index in the scrollback (0-based from oldest).
    pub scrollback_row: usize,
    /// Column of the match start.
    pub col_start: u16,
    /// Column of the match end (exclusive).
    pub col_end: u16,
}

// ---------------------------------------------------------------------------
// Internal matcher
// ---------------------------------------------------------------------------

enum Matcher {
    Literal {
        needle: String,
        case_sensitive: bool,
    },
    Regex(regex::Regex),
}

fn build_matcher(query: &SearchQuery) -> Result<Matcher, regex::Error> {
    if query.regex {
        let pattern = if query.case_sensitive {
            query.text.clone()
        } else {
            format!("(?i){}", query.text)
        };
        Ok(Matcher::Regex(regex::Regex::new(&pattern)?))
    } else {
        Ok(Matcher::Literal {
            needle: if query.case_sensitive {
                query.text.clone()
            } else {
                query.text.to_lowercase()
            },
            case_sensitive: query.case_sensitive,
        })
    }
}

// ---------------------------------------------------------------------------
// Cell-to-text conversion
// ---------------------------------------------------------------------------

/// Convert a row of cells to a plain string and a mapping from char index → col.
///
/// Phantom cells (continuations of wide characters) are skipped in the text
/// but the col mapping still points to the wide character's starting column.
fn cells_to_text(row: &[Cell]) -> (String, Vec<u16>) {
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

/// Find all literal matches in `haystack` and append `SearchMatch` to `out`.
fn find_literal(
    haystack: &str,
    needle: &str,
    case_sensitive: bool,
    char_to_col: &[u16],
    row_idx: usize,
    out: &mut Vec<SearchMatch>,
) {
    let search_str: &str;
    let lowered;
    if case_sensitive {
        search_str = haystack;
    } else {
        lowered = haystack.to_lowercase();
        search_str = &lowered;
    }

    let mut start = 0usize;
    while let Some(byte_off) = search_str[start..].find(needle) {
        let abs_byte = start + byte_off;
        let char_start = search_str[..abs_byte].chars().count();
        let char_end = char_start + needle.chars().count();

        let col_start = char_to_col.get(char_start).copied().unwrap_or(0);
        let col_end = char_to_col
            .get(char_end)
            .copied()
            .unwrap_or_else(|| col_start + needle.chars().count() as u16);

        out.push(SearchMatch {
            scrollback_row: row_idx,
            col_start,
            col_end,
        });

        // Advance past this match (by bytes in search_str).
        start = abs_byte + needle.len().max(1);
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Search scrollback lines for matches of `query`.
///
/// Returns all matches in row-major order (oldest row first, left-to-right within a row).
/// Returns an empty Vec if the query is empty, or if regex compilation fails.
pub fn search_scrollback<'a>(
    scrollback_lines: impl Iterator<Item = &'a Vec<Cell>>,
    query: &SearchQuery,
) -> Vec<SearchMatch> {
    if query.text.is_empty() {
        return Vec::new();
    }

    let matcher = match build_matcher(query) {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("search_scrollback: invalid regex: {e}");
            return Vec::new();
        }
    };

    let mut results = Vec::new();

    for (row_idx, row) in scrollback_lines.enumerate() {
        let (text, char_to_col) = cells_to_text(row);
        if text.is_empty() {
            continue;
        }

        match &matcher {
            Matcher::Literal {
                needle,
                case_sensitive,
            } => {
                find_literal(
                    &text,
                    needle,
                    *case_sensitive,
                    &char_to_col,
                    row_idx,
                    &mut results,
                );
            }
            Matcher::Regex(re) => {
                for m in re.find_iter(&text) {
                    let char_start = text[..m.start()].chars().count();
                    let char_end = text[..m.end()].chars().count();
                    let col_start = char_to_col.get(char_start).copied().unwrap_or(0);
                    let col_end = char_to_col
                        .get(char_end)
                        .copied()
                        .unwrap_or(col_start + (char_end - char_start) as u16);
                    results.push(SearchMatch {
                        scrollback_row: row_idx,
                        col_start,
                        col_end,
                    });
                }
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vt::cell::{Cell, CellAttrs};

    fn make_row(text: &str, cols: usize) -> Vec<Cell> {
        let mut row = vec![Cell::default(); cols];
        for (i, ch) in text.chars().enumerate() {
            if i < cols {
                row[i].grapheme = ch.to_string();
            }
        }
        row
    }

    fn make_cell(g: &str) -> Cell {
        Cell {
            grapheme: g.to_string(),
            attrs: CellAttrs::default(),
            width: 1,
        }
    }

    // -----------------------------------------------------------------------
    // FS-SEARCH-001 — literal case-insensitive search returns matches
    // -----------------------------------------------------------------------

    #[test]
    fn fs_search_001_literal_case_insensitive_finds_match() {
        let row = make_row("Hello World", 20);
        let lines = vec![row];
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

    // -----------------------------------------------------------------------
    // FS-SEARCH-003 — regex search
    // -----------------------------------------------------------------------

    #[test]
    fn fs_search_003_regex_finds_match() {
        let row = make_row("error: file not found", 30);
        let lines = vec![row];
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
        let row = make_row("test", 10);
        let lines = vec![row];
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

    #[test]
    fn fs_search_empty_query_returns_empty() {
        let row = make_row("hello", 10);
        let lines = vec![row];
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
        let row = make_row("aabaa", 10);
        let lines = vec![row];
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
        let row = make_row("Hello", 10);
        let lines = vec![row];
        let query = SearchQuery {
            text: "hello".to_string(),
            case_sensitive: true,
            regex: false,
        };
        let matches = search_scrollback(lines.iter(), &query);
        assert!(matches.is_empty(), "Case-sensitive search should not match");
    }

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
        let row = make_row("foo.*bar  ", 10);
        let lines = vec![row];
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
        let row = make_row("func(arg) rest", 20);
        let lines = vec![row];
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
    // SEARCH-SOFT-001: soft-wrapped line — cross-row match
    //
    // When a word spans two consecutive rows (soft-wrapped), search_scrollback
    // in its current row-by-row implementation will NOT find the cross-row match.
    // This test documents the EXPECTED behaviour after soft-wrap support is added.
    //
    // Marked #[ignore] — TDD red phase. Will pass once search_scrollback gains
    // cross-row (soft-wrap) joining.
    // -----------------------------------------------------------------------

    /// SEARCH-SOFT-001: word spanning a soft-wrapped boundary is found as one match.
    ///
    /// cols=5, row0="hello" (soft-wrapped), row1="world"
    /// Query: "helloworld" → one match spanning both rows.
    #[test]
    #[ignore = "cross-row soft-wrap search not yet implemented — TDD red phase (SEARCH-SOFT-001)"]
    fn search_soft_wrap_word_spanning_two_rows_is_found() {
        // Two rows of 5 columns, soft-wrapped (no newline between them)
        let row0 = make_row("hello", 5);
        let row1 = make_row("world", 5);
        let lines = vec![row0, row1];

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
}
