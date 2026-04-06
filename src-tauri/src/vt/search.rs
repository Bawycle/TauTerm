// SPDX-License-Identifier: MPL-2.0

//! Scrollback search — iterate scrollback lines, join soft-wrap boundaries,
//! return `SearchMatch` positions.

use serde::{Deserialize, Serialize};

use crate::vt::cell::Cell;
use crate::vt::screen_buffer::ScrollbackLine;

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

// ---------------------------------------------------------------------------
// Soft-wrap group building
// ---------------------------------------------------------------------------

/// A logical search unit: one or more consecutive scrollback rows joined by soft wraps.
struct LogicalLine<'a> {
    /// Row index (in the original scrollback) of the first row in this group.
    first_row: usize,
    /// All constituent rows, in order.
    rows: Vec<&'a Vec<Cell>>,
}

/// Group consecutive scrollback lines into logical lines, joining across soft-wrap
/// boundaries (FS-SB-008 / FS-SEARCH-002).
///
/// A row with `soft_wrapped == true` continues into the next row. The last row of
/// a group has `soft_wrapped == false` (or is the final row in the scrollback).
fn build_logical_lines<'a>(
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
fn logical_line_to_text(rows: &[&Vec<Cell>]) -> (String, Vec<(usize, u16)>) {
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

// ---------------------------------------------------------------------------
// Match emitters
// ---------------------------------------------------------------------------

/// Find all literal matches in a logical line and push `SearchMatch` entries.
fn find_literal_logical(
    haystack: &str,
    needle: &str,
    case_sensitive: bool,
    positions: &[(usize, u16)],
    first_row: usize,
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

        let (row_off, col_start) = positions.get(char_start).copied().unwrap_or((0, 0));
        let col_end = positions
            .get(char_end)
            .map(|&(_, c)| c)
            .unwrap_or_else(|| col_start + needle.chars().count() as u16);

        out.push(SearchMatch {
            scrollback_row: first_row + row_off,
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
/// Consecutive soft-wrapped lines are joined into a single logical line for
/// matching purposes (FS-SB-008 / FS-SEARCH-002). A word split across a
/// soft-wrap boundary is therefore found as a single match.
///
/// Returns all matches in row-major order (oldest row first, left-to-right
/// within a row). Returns an empty Vec if the query is empty or regex
/// compilation fails.
pub fn search_scrollback<'a>(
    scrollback_lines: impl Iterator<Item = &'a ScrollbackLine>,
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

    let groups = build_logical_lines(scrollback_lines);
    let mut results = Vec::new();

    for group in &groups {
        let (text, positions) = logical_line_to_text(&group.rows);
        if text.is_empty() {
            continue;
        }

        match &matcher {
            Matcher::Literal {
                needle,
                case_sensitive,
            } => {
                find_literal_logical(
                    &text,
                    needle,
                    *case_sensitive,
                    &positions,
                    group.first_row,
                    &mut results,
                );
            }
            Matcher::Regex(re) => {
                for m in re.find_iter(&text) {
                    let char_start = text[..m.start()].chars().count();
                    let char_end = text[..m.end()].chars().count();
                    let (row_off, col_start) = positions.get(char_start).copied().unwrap_or((0, 0));
                    let col_end = positions
                        .get(char_end)
                        .map(|&(_, c)| c)
                        .unwrap_or(col_start + (char_end - char_start) as u16);
                    results.push(SearchMatch {
                        scrollback_row: group.first_row + row_off,
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
            hyperlink: None,
        }
    }

    /// Wrap a `Vec<Cell>` into a hard-newline `ScrollbackLine`.
    fn hard(cells: Vec<Cell>) -> ScrollbackLine {
        ScrollbackLine {
            cells,
            soft_wrapped: false,
        }
    }

    /// Wrap a `Vec<Cell>` into a soft-wrapped `ScrollbackLine`.
    fn soft(cells: Vec<Cell>) -> ScrollbackLine {
        ScrollbackLine {
            cells,
            soft_wrapped: true,
        }
    }

    // -----------------------------------------------------------------------
    // FS-SEARCH-001 — literal case-insensitive search returns matches
    // -----------------------------------------------------------------------

    #[test]
    fn fs_search_001_literal_case_insensitive_finds_match() {
        let lines = vec![hard(make_row("Hello World", 20))];
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
        let lines = vec![hard(make_row("error: file not found", 30))];
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
        let lines = vec![hard(make_row("test", 10))];
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
        let lines = vec![hard(make_row("hello", 10))];
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
        let lines = vec![hard(make_row("aabaa", 10))];
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
        let lines = vec![hard(make_row("Hello", 10))];
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
        let lines = vec![hard(make_row("foo.*bar  ", 10))];
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
        let lines = vec![hard(make_row("func(arg) rest", 20))];
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
        let lines = vec![soft(make_row("hello", 5)), hard(make_row("world", 5))];

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
        let lines = vec![
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
        let lines = vec![soft(make_row("aab", 3)), hard(make_row("bcc", 3))];

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

    // -----------------------------------------------------------------------
    // TEST-VT-022 — Search on alternate screen returns zero results
    //
    // The alternate screen has no scrollback (FS-SB-004). `VtProcessor::search`
    // searches `normal.scrollback_iter()` — when the alternate screen is active
    // that iterator still reflects the *normal* scrollback. The spec requires
    // that a search issued while the alternate screen is active returns 0 results
    // because the alternate buffer does not participate in scrollback search.
    // -----------------------------------------------------------------------

    /// TEST-VT-022: search while alternate screen is active returns 0 results.
    ///
    /// We inject content through the normal screen (so it ends up in scrollback),
    /// then switch to the alternate screen and verify search returns nothing.
    #[test]
    fn test_vt_022_search_on_alternate_screen_returns_zero_results() {
        use crate::vt::processor::VtProcessor;

        let mut vt = VtProcessor::new(80, 24, 1000);

        // Write some text to the normal screen and scroll it into the scrollback
        // by filling more lines than the visible rows (25 > 24).
        let mut data = b"needle\r\n".to_vec();
        for _ in 0..24 {
            data.extend_from_slice(b"other\r\n");
        }
        vt.process(&data);

        let query = SearchQuery {
            text: "needle".to_string(),
            case_sensitive: true,
            regex: false,
        };
        let normal_results = vt.search(&query);
        assert!(
            !normal_results.is_empty(),
            "TEST-VT-022 prerequisite: 'needle' must be found in normal scrollback"
        );

        // Switch to alternate screen (CSI ? 1049 h).
        vt.process(b"\x1b[?1049h");

        // Search while alternate screen is active must return 0 results.
        let alt_results = vt.search(&query);
        assert_eq!(
            alt_results.len(),
            0,
            "TEST-VT-022: search while alternate screen is active must return 0 results (FS-SB-004)"
        );

        // Restore normal screen (no state leak between tests).
        vt.process(b"\x1b[?1049l");
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
        let lines = vec![hard(make_row("hello", 5)), hard(make_row("world", 5))];

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
}
