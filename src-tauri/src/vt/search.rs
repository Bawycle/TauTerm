// SPDX-License-Identifier: MPL-2.0

//! Scrollback search — iterate scrollback lines, skip soft-wrap boundaries,
//! return `SearchMatch` positions.

use serde::{Deserialize, Serialize};

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
