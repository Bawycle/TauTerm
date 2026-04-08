// SPDX-License-Identifier: MPL-2.0

use super::api::SearchMatch;

/// Find all literal matches in a logical line and push `SearchMatch` entries.
pub(super) fn find_literal_logical(
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
