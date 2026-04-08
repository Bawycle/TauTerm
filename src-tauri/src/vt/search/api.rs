// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use crate::vt::screen_buffer::ScrollbackLine;

use super::literal::find_literal_logical;
use super::logical_lines::{build_logical_lines, logical_line_to_text};
use super::matcher::{Matcher, build_matcher};

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
