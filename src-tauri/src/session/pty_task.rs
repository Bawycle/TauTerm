// SPDX-License-Identifier: MPL-2.0

//! PTY read task — one pair of Tokio tasks per pane.
//!
//! ## Architecture: two-task design
//!
//! The previous single-task design ran `rdr.read()` and the debounce check in
//! the same loop, which meant that if the PTY was silent after a burst, the
//! blocking `read()` would hold the thread indefinitely and the accumulated
//! dirty region would never be flushed until the next keystroke.
//!
//! The two-task design separates concerns:
//!
//! **Task 1 — reader (spawn_blocking)**
//! Reads raw bytes from the PTY, feeds them to `VtProcessor`, and sends the
//! resulting `ProcessOutput` through a bounded channel (capacity 256) to Task 2.
//! When the PTY reaches EOF the task exits naturally, closing the channel.
//!
//! **Task 2 — coalescer/emitter (async)**
//! Receives `ProcessOutput` values, coalesces them via `DirtyRegion::merge`,
//! and emits `screen-update` (and other) events on a timer-driven debounce
//! interval. Because Task 2 uses `tokio::select!`, the timer fires even when
//! the reader is blocked — the last batch is always flushed (§6.5).
//!
//! Back-pressure: dirty regions are coalesced over `SCREEN_UPDATE_DEBOUNCE`
//! before emitting a single `screen-update` event. This prevents flooding the
//! frontend when high-volume apps (`yes`, `seq`) write faster than the
//! frontend can consume events.

mod emitter;
mod event_builders;
mod reader;

pub(crate) use event_builders::{
    build_mode_state_event, build_screen_update_event, build_scrolled_viewport_event,
};
pub use reader::spawn_pty_read_task;

use crate::vt::DirtyRegion;

// ---------------------------------------------------------------------------
// ProcessOutput — data produced by processing one PTY chunk
// ---------------------------------------------------------------------------

/// Output produced by processing one chunk of PTY bytes in Task 1.
///
/// Task 2 coalesces multiple `ProcessOutput` values via `merge()` before
/// emitting events to the frontend.
#[derive(Default)]
pub(crate) struct ProcessOutput {
    pub dirty: DirtyRegion,
    pub mode_changed: bool,
    pub new_title: Option<String>,
    pub new_cursor_shape: Option<u8>,
    pub bell: bool,
    pub osc52: Option<String>,
}

impl ProcessOutput {
    /// Merge another output into `self`.
    ///
    /// - `dirty`: union (never loses dirty rows; full-redraw propagates).
    /// - `mode_changed`: OR (any mode change is preserved).
    /// - Scalar fields (`new_title`, `new_cursor_shape`, `osc52`): last-wins.
    /// - `bell`: OR (any bell in the window is preserved).
    fn merge(&mut self, other: ProcessOutput) {
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
    }

    fn is_empty(&self) -> bool {
        self.dirty.is_empty()
            && !self.mode_changed
            && self.new_title.is_none()
            && self.new_cursor_shape.is_none()
            && !self.bell
            && self.osc52.is_none()
    }
}

// ---------------------------------------------------------------------------
// PtyTaskHandle
// ---------------------------------------------------------------------------

/// Handle to the running PTY read/emit task pair.
///
/// Dropping this handle aborts both tasks. `abort()` does the same explicitly.
pub struct PtyTaskHandle {
    read_abort: tokio::task::AbortHandle,
    emit_abort: tokio::task::AbortHandle,
}

impl PtyTaskHandle {
    /// Wrap two `AbortHandle`s into a `PtyTaskHandle`.
    pub fn new(read_abort: tokio::task::AbortHandle, emit_abort: tokio::task::AbortHandle) -> Self {
        Self {
            read_abort,
            emit_abort,
        }
    }

    /// Construct from a single abort handle (used in tests that create a
    /// synthetic handle without a real emit task).
    pub fn from_abort_handle(abort: tokio::task::AbortHandle) -> Self {
        // In test contexts there is no emit task, so we reuse the same handle
        // for both slots. Aborting twice is harmless.
        Self {
            read_abort: abort.clone(),
            emit_abort: abort,
        }
    }

    /// Abort both tasks.
    pub fn abort(&self) {
        self.read_abort.abort();
        self.emit_abort.abort();
    }
}

impl Drop for PtyTaskHandle {
    fn drop(&mut self) {
        self.read_abort.abort();
        self.emit_abort.abort();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::RwLock;

    use super::build_screen_update_event;
    use crate::events::types::{CursorState, ScreenUpdateEvent};
    use crate::session::ids::PaneId;
    use crate::vt::{DirtyRegion, VtProcessor};

    fn make_vt(cols: u16, rows: u16) -> Arc<RwLock<VtProcessor>> {
        Arc::new(RwLock::new(VtProcessor::new(cols, rows, 1_000)))
    }

    fn full_redraw_dirty() -> DirtyRegion {
        DirtyRegion {
            rows: Default::default(),
            is_full_redraw: true,
            cursor_moved: false,
        }
    }

    // -----------------------------------------------------------------------
    // build_screen_update_event_includes_snapshot_dims
    // -----------------------------------------------------------------------

    /// Verifies that `build_screen_update_event` populates `cols` and `rows`
    /// from the current `ScreenSnapshot` dimensions.
    #[test]
    fn build_screen_update_event_includes_snapshot_dims() {
        let vt = make_vt(100, 30);
        let pane_id = PaneId(String::from("test-pane-1"));
        let dirty = full_redraw_dirty();

        let event = build_screen_update_event(&pane_id, &vt, &dirty);

        assert_eq!(event.cols, 100);
        assert_eq!(event.rows, 30);
    }

    // -----------------------------------------------------------------------
    // build_screen_update_event_after_resize_reflects_new_dims
    // -----------------------------------------------------------------------

    /// Verifies that after a `VtProcessor::resize`, the next
    /// `build_screen_update_event` reports the updated dimensions.
    #[test]
    fn build_screen_update_event_after_resize_reflects_new_dims() {
        let vt = make_vt(80, 24);
        vt.write().resize(120, 40);

        let pane_id = PaneId(String::from("test-pane-2"));
        let dirty = full_redraw_dirty();

        let event = build_screen_update_event(&pane_id, &vt, &dirty);

        assert_eq!(event.cols, 120);
        assert_eq!(event.rows, 40);
    }

    // -----------------------------------------------------------------------
    // screen_update_event_serde_roundtrip
    // -----------------------------------------------------------------------

    /// Verifies that `cols` and `rows` survive a JSON serialization/deserialization
    /// round-trip (guards against accidental `#[serde(skip)]` or rename regressions).
    #[test]
    fn screen_update_event_serde_roundtrip() {
        let event = ScreenUpdateEvent {
            pane_id: PaneId(String::from("test-pane-3")),
            cells: vec![],
            cursor: CursorState {
                row: 0,
                col: 0,
                visible: true,
                shape: 0,
                blink: false,
            },
            scrollback_lines: 0,
            is_full_redraw: false,
            cols: 80,
            rows: 24,
            scroll_offset: 0,
        };

        let json = serde_json::to_string(&event).expect("serialization failed");
        let decoded: ScreenUpdateEvent =
            serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(decoded.cols, 80);
        assert_eq!(decoded.rows, 24);

        // Confirm camelCase keys are present in the raw JSON.
        assert!(
            json.contains("\"cols\":80"),
            "expected \"cols\":80 in {json}"
        );
        assert!(
            json.contains("\"rows\":24"),
            "expected \"rows\":24 in {json}"
        );
    }

    // -----------------------------------------------------------------------
    // screen_update_event_serde_roundtrip_non_square
    // -----------------------------------------------------------------------

    /// Verifies round-trip with non-standard dimensions (wide terminal).
    #[test]
    fn screen_update_event_serde_roundtrip_non_square() {
        let event = ScreenUpdateEvent {
            pane_id: PaneId(String::from("test-pane-4")),
            cells: vec![],
            cursor: CursorState {
                row: 0,
                col: 0,
                visible: true,
                shape: 0,
                blink: false,
            },
            scrollback_lines: 0,
            is_full_redraw: true,
            cols: 220,
            rows: 50,
            scroll_offset: 0,
        };

        let json = serde_json::to_string(&event).expect("serialization failed");
        let decoded: ScreenUpdateEvent =
            serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(decoded.cols, 220);
        assert_eq!(decoded.rows, 50);
        assert!(decoded.is_full_redraw);
    }

    // -----------------------------------------------------------------------
    // Scrolled viewport tests (TEST-SB-VIEWPORT-*)
    // -----------------------------------------------------------------------

    fn make_vt_with_scrollback(
        cols: u16,
        rows: u16,
        scrollback: usize,
    ) -> Arc<RwLock<VtProcessor>> {
        Arc::new(RwLock::new(VtProcessor::new(cols, rows, scrollback)))
    }

    /// Push lines into the VT. Each call writes `line\r\n` so the terminal
    /// scrolls the oldest line into the scrollback buffer once rows are full.
    fn push_lines(vt: &mut VtProcessor, lines: &[&str]) {
        for line in lines {
            vt.process(line.as_bytes());
            vt.process(b"\r\n");
        }
    }

    // -----------------------------------------------------------------------
    // TEST-SB-VIEWPORT-001
    // -----------------------------------------------------------------------

    /// Composite viewport with k < rows: rows 0..(k) come from scrollback,
    /// rows k..rows come from the live screen.
    #[test]
    fn viewport_k_less_than_rows_composites_sb_and_live() {
        use super::build_scrolled_viewport_event;

        // 3-row terminal: pushing 4 lines puts LINE1 into scrollback.
        let vt = make_vt_with_scrollback(10, 3, 100);
        {
            let mut proc = vt.write();
            push_lines(&mut proc, &["LINE1", "LINE2", "LINE3", "LINE4"]);
        }

        let pane_id = PaneId(String::from("sb-viewport-001"));
        // k=2: rows 0-1 from scrollback, row 2 from live screen.
        let event = build_scrolled_viewport_event(&pane_id, &vt, 2);

        assert!(event.is_full_redraw, "must be full redraw");
        assert_eq!(event.scroll_offset, 2);
        assert_eq!(event.rows, 3);

        // Row 0 should be the scrollback line whose content starts with 'L' (LINE1).
        let row0_content: String = event
            .cells
            .iter()
            .filter(|c| c.row == 0)
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .concat();
        assert!(
            row0_content.trim_end().starts_with('L'),
            "row 0 should come from scrollback, got: {row0_content:?}"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-SB-VIEWPORT-002
    // -----------------------------------------------------------------------

    /// Composite viewport with k == rows: all rows come from scrollback.
    #[test]
    fn viewport_k_equals_rows_all_from_scrollback() {
        use super::build_scrolled_viewport_event;

        // 3-row terminal, push 6 lines → 3 in scrollback.
        let vt = make_vt_with_scrollback(10, 3, 100);
        {
            let mut proc = vt.write();
            push_lines(&mut proc, &["SB1", "SB2", "SB3", "SB4", "SB5", "SB6"]);
        }

        let pane_id = PaneId(String::from("sb-viewport-002"));
        let n = vt.read().scrollback_len();
        assert!(n >= 3, "need at least 3 scrollback lines, got {n}");

        // k == rows: all 3 viewport rows map to scrollback lines.
        let event = build_scrolled_viewport_event(&pane_id, &vt, 3);

        assert!(event.is_full_redraw);
        assert_eq!(event.scroll_offset, 3);

        // Every row should have content sourced from scrollback (non-blank).
        for row in 0..3u16 {
            let row_content: String = event
                .cells
                .iter()
                .filter(|c| c.row == row)
                .map(|c| c.content.as_str())
                .collect::<Vec<_>>()
                .concat();
            assert!(
                !row_content.trim().is_empty(),
                "row {row} should have scrollback content, got: {row_content:?}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // TEST-SB-VIEWPORT-003
    // -----------------------------------------------------------------------

    /// k=0 produces a live-view event with cursor state intact.
    #[test]
    fn viewport_k_zero_produces_live_view_event() {
        use super::build_scrolled_viewport_event;

        let vt = make_vt_with_scrollback(10, 3, 100);
        {
            let mut proc = vt.write();
            push_lines(&mut proc, &["LINE1", "LINE2", "LINE3", "LINE4"]);
        }

        let pane_id = PaneId(String::from("sb-viewport-003"));
        let event = build_scrolled_viewport_event(&pane_id, &vt, 0);

        assert_eq!(event.scroll_offset, 0, "k=0 must produce scroll_offset=0");
        // Cursor visibility: k==0 so cursor.visible mirrors the live VT state.
        let snap = vt.read();
        let live_visible = snap.get_snapshot().cursor_visible;
        drop(snap);
        assert_eq!(
            event.cursor.visible, live_visible,
            "cursor.visible must match live VT when k=0"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-SB-VIEWPORT-004
    // -----------------------------------------------------------------------

    /// Cursor is hidden whenever k > 0.
    #[test]
    fn viewport_cursor_hidden_when_k_gt_zero() {
        use super::build_scrolled_viewport_event;

        let vt = make_vt_with_scrollback(10, 3, 100);
        {
            let mut proc = vt.write();
            push_lines(&mut proc, &["LINE1", "LINE2", "LINE3", "LINE4"]);
        }

        let pane_id = PaneId(String::from("sb-viewport-004"));
        let event = build_scrolled_viewport_event(&pane_id, &vt, 1);

        assert!(
            !event.cursor.visible,
            "cursor must be hidden when scroll_offset > 0"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-SB-VIEWPORT-005
    // -----------------------------------------------------------------------

    /// `scroll_offset` survives a JSON serde round-trip with camelCase key.
    #[test]
    fn screen_update_event_scroll_offset_serde_roundtrip() {
        let event = ScreenUpdateEvent {
            pane_id: PaneId(String::from("sb-viewport-005")),
            cells: vec![],
            cursor: crate::events::types::CursorState {
                row: 0,
                col: 0,
                visible: false,
                shape: 0,
                blink: false,
            },
            scrollback_lines: 10,
            is_full_redraw: true,
            cols: 80,
            rows: 24,
            scroll_offset: 42,
        };

        let json = serde_json::to_string(&event).expect("serialization failed");
        let decoded: ScreenUpdateEvent =
            serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(decoded.scroll_offset, 42);
        assert!(
            json.contains("\"scrollOffset\":42"),
            "expected \"scrollOffset\":42 in {json}"
        );
    }
}
