// SPDX-License-Identifier: MPL-2.0

//! `ProcessOutput` — value type produced by processing one chunk of terminal
//! bytes. Shared between the PTY pipeline (`session/pty_task`) and the SSH
//! pipeline (`session/ssh_task`); coalesced by the shared async coalescer
//! (`session/output/coalescer`).

use crate::vt::DirtyRegion;

// ---------------------------------------------------------------------------
// ProcessOutput — data produced by processing one chunk of terminal bytes
// ---------------------------------------------------------------------------

/// Output produced by processing one chunk of terminal bytes (PTY or SSH).
///
/// The async coalescer (see `session/output/coalescer.rs`) merges multiple
/// `ProcessOutput` values via [`ProcessOutput::merge`] before emitting events
/// to the frontend.
#[derive(Default)]
pub(crate) struct ProcessOutput {
    pub dirty: DirtyRegion,
    pub mode_changed: bool,
    pub new_title: Option<String>,
    pub new_cursor_shape: Option<u8>,
    pub bell: bool,
    pub osc52: Option<String>,
    /// New CWD from OSC 7, if changed since last cycle.
    pub new_cwd: Option<String>,
    /// Set when this chunk generated a VT response (CPR, DA, DSR).
    /// The coalescer bypasses the debounce timer and flushes immediately.
    pub needs_immediate_flush: bool,
}

impl ProcessOutput {
    /// Merge another output into `self`.
    ///
    /// - `dirty`: union (never loses dirty rows; full-redraw propagates).
    /// - `mode_changed`: OR (any mode change is preserved).
    /// - Scalar fields (`new_title`, `new_cursor_shape`, `osc52`, `new_cwd`):
    ///   last-wins.
    /// - `bell`: OR (any bell in the window is preserved).
    /// - `needs_immediate_flush`: OR (any pending flush hint is preserved).
    pub(crate) fn merge(&mut self, other: ProcessOutput) {
        self.dirty.merge(&other.dirty);
        self.mode_changed |= other.mode_changed;
        if other.new_title.is_some() {
            self.new_title = other.new_title;
        }
        if other.new_cursor_shape.is_some() {
            self.new_cursor_shape = other.new_cursor_shape;
        }
        self.bell |= other.bell;
        if other.osc52.is_some() {
            self.osc52 = other.osc52;
        }
        if other.new_cwd.is_some() {
            self.new_cwd = other.new_cwd;
        }
        self.needs_immediate_flush |= other.needs_immediate_flush;
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.dirty.is_empty()
            && !self.mode_changed
            && self.new_title.is_none()
            && self.new_cursor_shape.is_none()
            && !self.bell
            && self.osc52.is_none()
            && self.new_cwd.is_none()
    }
}
