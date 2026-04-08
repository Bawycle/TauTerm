// SPDX-License-Identifier: MPL-2.0

//! Scrollback search — iterate scrollback lines, join soft-wrap boundaries,
//! return `SearchMatch` positions.

mod api;
mod literal;
mod logical_lines;
mod matcher;
mod text_conversion;

#[cfg(test)]
mod tests;

pub use api::{SearchMatch, SearchQuery, search_scrollback};
