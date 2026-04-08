// SPDX-License-Identifier: MPL-2.0

use crate::vt::cell::Cell;

// ---------------------------------------------------------------------------
// Scrollback line
// ---------------------------------------------------------------------------

/// A single line in the scrollback ring.
///
/// `soft_wrapped` is `true` when this line ended because the terminal width was
/// exhausted (the cursor automatically wrapped to the next row) rather than
/// because a hard newline (`\n`) was received.
///
/// This flag is used by the search engine to join consecutive soft-wrapped lines
/// into a single logical string (FS-SB-008, FS-SEARCH-002).
#[derive(Debug, Clone)]
pub struct ScrollbackLine {
    /// Cell content of the line.
    pub cells: Vec<Cell>,
    /// `true` if the line break is a soft wrap; `false` if it is a hard newline.
    pub soft_wrapped: bool,
}

/// A scrollback line returned by `VtProcessor::get_scrollback_line`.
///
/// Carries both the cell content and the `soft_wrapped` flag so that callers
/// (e.g. IPC commands, selection copy) can join soft-wrapped lines correctly
/// without producing spurious newlines (FS-SB-011).
#[derive(Debug, Clone)]
pub struct ScrollbackLineRef {
    /// Cell content of the line (cloned from the ring).
    pub cells: Vec<Cell>,
    /// `true` if the line break was caused by auto-wrap rather than a hard newline.
    pub soft_wrapped: bool,
}
