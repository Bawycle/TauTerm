// SPDX-License-Identifier: MPL-2.0

//! Terminal screen buffer: cell grid, scrollback ring, dirty tracking, resize.
//!
//! `ScreenBuffer` maintains:
//! - The visible cell grid (normal or alternate screen)
//! - A scrollback ring for the normal screen (lines scrolled off the top)
//! - Dirty region tracking for efficient screen-update event generation
//! - Resize logic (reflow not required for v1: truncate/pad on resize)
//!
//! Scrollback policy (§3.2, FS-VT-053, FS-SB-004):
//! Only lines scrolled off the top of a full-screen scroll region enter the ring.
//! Lines evicted by a partial DECSTBM scroll region are discarded (not stored).
//! The alternate screen buffer never contributes to scrollback.

mod dirty_rows;
mod scrollback;
mod snapshot;
mod dirty_region;
mod buffer;
mod operations;

#[cfg(test)]
mod tests;

pub use buffer::ScreenBuffer;
pub use dirty_region::DirtyRegion;
pub use dirty_rows::DirtyRows;
pub use scrollback::{ScrollbackLine, ScrollbackLineRef};
pub use snapshot::{ScreenSnapshot, SnapshotCell};

/// The maximum number of scrollback lines (configurable via preferences in the
/// full implementation; this constant is the hard upper bound).
pub const MAX_SCROLLBACK_LINES: usize = 100_000;
