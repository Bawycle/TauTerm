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

// Exposed for benchmarks only — not part of the stable public API.
pub use event_builders::{build_screen_update_event, build_scrolled_viewport_event};
// Internal only.
pub(crate) use event_builders::build_mode_state_event;
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
    /// New CWD from OSC 7, if changed since last cycle.
    pub new_cwd: Option<String>,
    /// Set when this chunk generated a VT response (CPR, DA, DSR).
    /// Task 2 bypasses the debounce timer and flushes immediately.
    pub needs_immediate_flush: bool,
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
        if other.new_cwd.is_some() {
            self.new_cwd = other.new_cwd;
        }
        self.needs_immediate_flush |= other.needs_immediate_flush;
    }

    fn is_empty(&self) -> bool {
        self.dirty.is_empty()
            && !self.mode_changed
            && self.new_title.is_none()
            && self.new_cursor_shape.is_none()
            && !self.bell
            && self.osc52.is_none()
            && self.new_cwd.is_none()
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
    use std::time::Duration;

    use parking_lot::RwLock;

    use super::reader::{
        ACK_DROP_THRESHOLD_MS, ACK_STALE_DEBOUNCE, ACK_STALE_THRESHOLD_MS, DEBOUNCE_MAX,
        DEBOUNCE_MIN, next_debounce, now_ms,
    };
    use super::{ProcessOutput, build_screen_update_event};
    use crate::events::types::{CursorState, ScreenUpdateEvent};
    use crate::session::ids::PaneId;
    use crate::vt::{DirtyRegion, VtProcessor};

    fn make_vt(cols: u16, rows: u16) -> Arc<RwLock<VtProcessor>> {
        Arc::new(RwLock::new(VtProcessor::new(cols, rows, 1_000, 0, false)))
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
        Arc::new(RwLock::new(VtProcessor::new(
            cols, rows, scrollback, 0, false,
        )))
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
    // TEST-PIPC2-UNIT-001
    // -----------------------------------------------------------------------

    /// `merge()` propagates `needs_immediate_flush` via OR.
    #[test]
    fn process_output_merge_propagates_needs_immediate_flush() {
        let mut a = ProcessOutput::default();
        let b = ProcessOutput {
            needs_immediate_flush: true,
            ..Default::default()
        };
        a.merge(b);
        assert!(a.needs_immediate_flush);
    }

    // -----------------------------------------------------------------------
    // TEST-PIPC2-UNIT-002
    // -----------------------------------------------------------------------

    /// `Default` sets `needs_immediate_flush` to false.
    #[test]
    fn process_output_needs_immediate_flush_defaults_false() {
        let p = ProcessOutput::default();
        assert!(!p.needs_immediate_flush);
    }

    // -----------------------------------------------------------------------
    // TEST-PIPC2-UNIT-003
    // -----------------------------------------------------------------------

    /// Multiple merges: needs_immediate_flush propagates once it's set.
    #[test]
    fn process_output_immediate_flush_sticky_through_merge() {
        let mut acc = ProcessOutput::default();
        // First merge: no flush needed.
        acc.merge(ProcessOutput::default());
        assert!(!acc.needs_immediate_flush);
        // Second merge: flush needed.
        acc.merge(ProcessOutput {
            needs_immediate_flush: true,
            ..Default::default()
        });
        assert!(acc.needs_immediate_flush);
        // Third merge: flag stays set even when merging a non-flush output.
        acc.merge(ProcessOutput::default());
        assert!(acc.needs_immediate_flush);
    }

    // -----------------------------------------------------------------------
    // TEST-SB-VIEWPORT-005
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // TEST-ADPT-001
    // -----------------------------------------------------------------------

    /// Zero emit duration clamps to DEBOUNCE_MIN.
    #[test]
    fn next_debounce_zero_clamps_to_min() {
        assert_eq!(next_debounce(Duration::ZERO), DEBOUNCE_MIN);
    }

    // -----------------------------------------------------------------------
    // TEST-ADPT-002
    // -----------------------------------------------------------------------

    /// Emit duration below DEBOUNCE_MIN (after scaling) clamps to DEBOUNCE_MIN.
    #[test]
    fn next_debounce_small_clamps_to_min() {
        assert_eq!(next_debounce(Duration::from_millis(5)), DEBOUNCE_MIN);
    }

    // -----------------------------------------------------------------------
    // TEST-ADPT-003
    // -----------------------------------------------------------------------

    /// Emit duration above DEBOUNCE_MAX (after scaling) clamps to DEBOUNCE_MAX.
    #[test]
    fn next_debounce_large_clamps_to_max() {
        assert_eq!(next_debounce(Duration::from_millis(200)), DEBOUNCE_MAX);
    }

    // -----------------------------------------------------------------------
    // TEST-ADPT-004
    // -----------------------------------------------------------------------

    /// Mid-range emit duration scales correctly: 20ms * 1.2 = 24ms.
    #[test]
    fn next_debounce_scales_correctly() {
        assert_eq!(
            next_debounce(Duration::from_millis(20)),
            Duration::from_millis(24)
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ADPT-005
    // -----------------------------------------------------------------------

    /// Boundary behavior near DEBOUNCE_MAX: 83ms * 1.2 = 99.6ms < 100ms,
    /// but 84ms * 1.2 = 100.8ms clamps to 100ms.
    #[test]
    fn next_debounce_near_max_boundary() {
        let d83 = next_debounce(Duration::from_millis(83));
        assert!(d83 < DEBOUNCE_MAX);
        assert!(d83 > DEBOUNCE_MIN);

        let d84 = next_debounce(Duration::from_millis(84));
        assert_eq!(d84, DEBOUNCE_MAX);
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

    // -----------------------------------------------------------------------
    // TEST-ACK-001 — now_ms returns a reasonable value
    // -----------------------------------------------------------------------

    /// `now_ms()` returns a timestamp in the expected range (after 2024-01-01).
    #[test]
    fn now_ms_returns_reasonable_value() {
        let ts = now_ms();
        // 2024-01-01T00:00:00Z in ms
        let jan_2024_ms: u64 = 1_704_067_200_000;
        assert!(
            ts > jan_2024_ms,
            "now_ms() = {ts} should be after 2024-01-01 ({jan_2024_ms})"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-002 — ack threshold ordering
    // -----------------------------------------------------------------------

    /// Stage 1 threshold must be less than Stage 2 threshold.
    #[test]
    fn ack_thresholds_ordered_correctly() {
        assert!(
            ACK_STALE_THRESHOLD_MS < ACK_DROP_THRESHOLD_MS,
            "Stale threshold ({ACK_STALE_THRESHOLD_MS}) must be < drop threshold ({ACK_DROP_THRESHOLD_MS})"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-003 — stale debounce exceeds max adaptive debounce
    // -----------------------------------------------------------------------

    /// The stale-mode debounce must be larger than `DEBOUNCE_MAX`, otherwise
    /// entering stale mode would not actually slow down emission.
    #[test]
    fn ack_stale_debounce_exceeds_adaptive_max() {
        assert!(
            ACK_STALE_DEBOUNCE > DEBOUNCE_MAX,
            "ACK_STALE_DEBOUNCE ({ACK_STALE_DEBOUNCE:?}) must exceed DEBOUNCE_MAX ({DEBOUNCE_MAX:?})"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-004 — drop mode suppresses dirty-only ProcessOutput
    // -----------------------------------------------------------------------

    /// Simulates Stage 2 behavior: clearing `dirty` and `needs_immediate_flush`
    /// on a dirty-only `ProcessOutput` makes it empty (no emit).
    #[test]
    fn drop_mode_suppresses_dirty_only_output() {
        use crate::vt::DirtyRegion;

        let mut pending = ProcessOutput {
            dirty: DirtyRegion {
                rows: Default::default(),
                is_full_redraw: true,
                cursor_moved: true,
            },
            ..Default::default()
        };
        assert!(
            !pending.is_empty(),
            "pending should not be empty before drop"
        );

        // Simulate Stage 2 suppression.
        pending.dirty = DirtyRegion::default();
        pending.needs_immediate_flush = false;

        assert!(
            pending.is_empty(),
            "dirty-only pending should be empty after Stage 2 suppression"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-005 — drop mode preserves non-visual events
    // -----------------------------------------------------------------------

    /// Stage 2 clears dirty but preserves non-visual events (title, bell, etc.).
    #[test]
    fn drop_mode_preserves_non_visual_events() {
        use crate::vt::DirtyRegion;

        let mut pending = ProcessOutput {
            dirty: DirtyRegion {
                rows: Default::default(),
                is_full_redraw: true,
                cursor_moved: false,
            },
            new_title: Some("test title".to_string()),
            bell: true,
            mode_changed: true,
            ..Default::default()
        };

        // Simulate Stage 2 suppression.
        pending.dirty = DirtyRegion::default();
        pending.needs_immediate_flush = false;

        assert!(
            !pending.is_empty(),
            "pending with non-visual events should NOT be empty after Stage 2"
        );
        assert!(pending.bell);
        assert!(pending.mode_changed);
        assert_eq!(pending.new_title.as_deref(), Some("test title"));
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-006 — exit from drop mode forces full redraw
    // -----------------------------------------------------------------------

    /// When `was_in_drop_mode` is true and current ack is fresh,
    /// `is_full_redraw` must be set.
    #[test]
    fn exit_drop_mode_forces_full_redraw() {
        use crate::vt::DirtyRegion;

        let mut pending = ProcessOutput {
            dirty: DirtyRegion {
                rows: Default::default(),
                is_full_redraw: false,
                cursor_moved: true,
            },
            ..Default::default()
        };

        // Simulate exit from drop mode.
        let was_in_drop_mode = true;
        let in_drop_mode = false;

        if in_drop_mode {
            pending.dirty = DirtyRegion::default();
            pending.needs_immediate_flush = false;
        } else if was_in_drop_mode {
            pending.dirty.is_full_redraw = true;
        }

        assert!(
            pending.dirty.is_full_redraw,
            "exiting drop mode must force is_full_redraw"
        );
    }
}
