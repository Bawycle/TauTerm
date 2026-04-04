// SPDX-License-Identifier: MPL-2.0

//! VT module — terminal emulation engine.
//!
//! Re-exports the public API types used by the rest of the backend.
//! Internal sub-modules implement the full VT/ANSI state machine,
//! screen buffer management, SGR parsing, and scrollback search.

pub mod cell;
pub mod charset;
pub mod modes;
pub mod mouse;
pub mod osc;
pub mod processor;
pub mod screen_buffer;
pub mod search;
pub mod sgr;

pub use cell::{Cell, CellAttrs, Color, Hyperlink};
pub use mouse::MouseEvent;
pub use processor::VtProcessor;
pub use screen_buffer::{DirtyRegion, ScreenSnapshot};
pub use search::{SearchMatch, SearchQuery};
