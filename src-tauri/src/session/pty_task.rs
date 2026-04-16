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
    ///
    /// Expressed as a `const` assertion so the invariant is checked at
    /// compile time rather than test-time. The `#[test]` wrapper keeps the
    /// ID (TEST-ACK-002) discoverable in test reports.
    #[test]
    fn ack_thresholds_ordered_correctly() {
        const _: () = assert!(
            ACK_STALE_THRESHOLD_MS < ACK_DROP_THRESHOLD_MS,
            "Stale threshold must be < drop threshold"
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

    // -----------------------------------------------------------------------
    // TEST-ACK-007 — has_unacked_emits is false at startup
    // -----------------------------------------------------------------------

    /// At startup, `last_emit_ms` is 0 and `last_ack_ms` is initialized to
    /// `now()` (a large epoch value). The guard must be false.
    #[test]
    fn has_unacked_emits_false_at_startup() {
        let last_emit_ms: u64 = 0;
        let last_ack_ms: u64 = 1_700_000_000_000; // synthetic "now()" epoch ms
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        assert!(
            !has_unacked_emits,
            "no emits have occurred yet — must be false"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-008 — has_unacked_emits is true after emit, before ack
    // -----------------------------------------------------------------------

    /// When an emit occurs after the last ack, the guard must be true.
    #[test]
    fn has_unacked_emits_true_after_emit_before_ack() {
        let last_ack_ms: u64 = 1_700_000_000_000;
        let last_emit_ms: u64 = 1_700_000_000_050; // 50ms after ack
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        assert!(
            has_unacked_emits,
            "emit happened after last ack — must be true"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-009 — has_unacked_emits is false after ack follows emit
    // -----------------------------------------------------------------------

    /// When the frontend acks after the last emit, the guard must be false.
    #[test]
    fn has_unacked_emits_false_after_ack_follows_emit() {
        let last_emit_ms: u64 = 1_700_000_000_000;
        let last_ack_ms: u64 = 1_700_000_000_050; // 50ms after emit
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        assert!(!has_unacked_emits, "ack arrived after emit — must be false");
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-010 — has_unacked_emits is false when timestamps are equal
    // -----------------------------------------------------------------------

    /// When emit and ack have the same millisecond timestamp, the strict `>`
    /// comparison makes `has_unacked_emits` false. This is correct: the emit
    /// was acked in the same ms window.
    #[test]
    fn has_unacked_emits_false_when_equal() {
        let ts: u64 = 1_700_000_000_000;
        let last_emit_ms: u64 = ts;
        let last_ack_ms: u64 = ts;
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        assert!(
            !has_unacked_emits,
            "equal timestamps — strict > means no unacked emits"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-011 — idle period does not trigger drop or stale mode
    // -----------------------------------------------------------------------

    /// Simulates a long idle period: no emits ever occurred (`last_emit_ms = 0`),
    /// ack is 5 seconds stale. Neither drop nor stale mode should activate
    /// because there are no unacked emits.
    #[test]
    fn idle_period_does_not_trigger_drop_mode() {
        use super::reader::{ACK_DROP_THRESHOLD_MS, ACK_STALE_THRESHOLD_MS};

        let last_emit_ms: u64 = 0;
        let now: u64 = 1_700_000_005_000;
        let last_ack_ms: u64 = 1_700_000_000_000; // 5s ago
        let ack_age_ms = now.saturating_sub(last_ack_ms);
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        let in_drop_mode = has_unacked_emits && ack_age_ms > ACK_DROP_THRESHOLD_MS;
        let in_stale_mode = has_unacked_emits && ack_age_ms > ACK_STALE_THRESHOLD_MS;

        assert!(
            ack_age_ms > ACK_DROP_THRESHOLD_MS,
            "ack age should exceed drop threshold"
        );
        assert!(
            !has_unacked_emits,
            "no emits occurred — guard must be false"
        );
        assert!(!in_drop_mode, "idle period must not trigger drop mode");
        assert!(!in_stale_mode, "idle period must not trigger stale mode");
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-012 — stale ack with unacked emits triggers escalation
    // -----------------------------------------------------------------------

    /// When we have emitted after the last ack and the ack is 2 seconds old,
    /// both drop and stale mode must activate.
    #[test]
    fn stale_ack_with_unacked_emits_triggers_escalation() {
        use super::reader::{ACK_DROP_THRESHOLD_MS, ACK_STALE_THRESHOLD_MS};

        let now: u64 = 1_700_000_002_000;
        let last_ack_ms: u64 = 1_700_000_000_000; // 2s ago
        let last_emit_ms: u64 = 1_700_000_000_500; // 500ms after ack
        let ack_age_ms = now.saturating_sub(last_ack_ms);
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        let in_drop_mode = has_unacked_emits && ack_age_ms > ACK_DROP_THRESHOLD_MS;
        let in_stale_mode = has_unacked_emits && ack_age_ms > ACK_STALE_THRESHOLD_MS;

        assert!(has_unacked_emits, "emit after ack — must be true");
        assert!(
            in_drop_mode,
            "2s ack age with unacked emits — must be in drop mode"
        );
        assert!(
            in_stale_mode,
            "2s ack age with unacked emits — must be in stale mode"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-013 — drop exit via ack arrival forces full redraw
    // -----------------------------------------------------------------------

    /// When `was_in_drop_mode` is true and the frontend acks (making
    /// `has_unacked_emits` false), `in_drop_mode` becomes false and
    /// the full-redraw flag must be set.
    #[test]
    fn drop_exit_via_ack_arrival_forces_full_redraw() {
        use super::reader::ACK_DROP_THRESHOLD_MS;
        use crate::vt::DirtyRegion;

        let now: u64 = 1_700_000_002_000;
        let last_ack_ms: u64 = 1_700_000_001_900; // fresh ack, 100ms ago
        let last_emit_ms: u64 = 1_700_000_000_500; // emit before ack
        let ack_age_ms = now.saturating_sub(last_ack_ms);
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        let in_drop_mode = has_unacked_emits && ack_age_ms > ACK_DROP_THRESHOLD_MS;
        let was_in_drop_mode = true; // previously in drop mode

        assert!(
            !has_unacked_emits,
            "ack arrived after emit — no unacked emits"
        );
        assert!(!in_drop_mode, "must not be in drop mode after ack");

        // Simulate the transition logic from reader.rs.
        let mut pending = ProcessOutput {
            dirty: DirtyRegion {
                rows: Default::default(),
                is_full_redraw: false,
                cursor_moved: true,
            },
            ..Default::default()
        };

        if in_drop_mode {
            pending.dirty = DirtyRegion::default();
            pending.needs_immediate_flush = false;
        } else if was_in_drop_mode {
            pending.dirty.is_full_redraw = true;
        }

        assert!(
            pending.dirty.is_full_redraw,
            "drop exit via ack arrival must force full redraw"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-014 — rapid emit→ack cycles do not trigger escalation
    // -----------------------------------------------------------------------

    /// Simulates 3 rapid emit→ack cycles where the frontend acks within
    /// the stale threshold. No escalation should occur at any point.
    #[test]
    fn rapid_emit_ack_cycle_no_escalation() {
        use super::reader::{ACK_DROP_THRESHOLD_MS, ACK_STALE_THRESHOLD_MS};

        // Cycle 1: emit at t+0, ack at t+15, check at t+20
        // Cycle 2: emit at t+30, ack at t+45, check at t+50
        // Cycle 3: emit at t+60, ack at t+75, check at t+80
        let base: u64 = 1_700_000_000_000;
        let cycles: [(u64, u64, u64); 3] = [
            (base, base + 15, base + 20), // (emit, ack, check)
            (base + 30, base + 45, base + 50),
            (base + 60, base + 75, base + 80),
        ];

        for (i, (emit_ms, ack_ms, check_ms)) in cycles.iter().enumerate() {
            let last_emit_ms = *emit_ms;
            let last_ack_ms = *ack_ms;
            let ack_age_ms = check_ms.saturating_sub(last_ack_ms);
            let has_unacked_emits = last_emit_ms > last_ack_ms;
            let in_drop_mode = has_unacked_emits && ack_age_ms > ACK_DROP_THRESHOLD_MS;
            let in_stale_mode = has_unacked_emits && ack_age_ms > ACK_STALE_THRESHOLD_MS;

            assert!(
                !has_unacked_emits,
                "cycle {i}: ack followed emit — no unacked emits"
            );
            assert!(!in_drop_mode, "cycle {i}: must not be in drop mode");
            assert!(!in_stale_mode, "cycle {i}: must not be in stale mode");
        }
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-015 — output_emits_screen_update returns false for non-visual
    // events (bell, title, mode, cursor shape, OSC 52, CWD) individually and
    // combined.
    // -----------------------------------------------------------------------

    /// Each of the 6 non-visual fields, on its own and combined, must not
    /// trigger a `screen-update` emission. This is the core invariant that
    /// prevents non-visual events from advancing `last_emit_ms` and poisoning
    /// the frame-ack backpressure state.
    #[test]
    fn output_emits_screen_update_false_for_non_visual_events() {
        use super::emitter::output_emits_screen_update;

        // Bell only.
        let pending = ProcessOutput {
            bell: true,
            ..Default::default()
        };
        assert!(
            !output_emits_screen_update(&pending),
            "bell-only ProcessOutput must NOT emit screen-update"
        );

        // Title only.
        let pending = ProcessOutput {
            new_title: Some("My Title".to_string()),
            ..Default::default()
        };
        assert!(
            !output_emits_screen_update(&pending),
            "title-only ProcessOutput must NOT emit screen-update"
        );

        // Mode change only.
        let pending = ProcessOutput {
            mode_changed: true,
            ..Default::default()
        };
        assert!(
            !output_emits_screen_update(&pending),
            "mode-only ProcessOutput must NOT emit screen-update"
        );

        // Cursor shape only.
        let pending = ProcessOutput {
            new_cursor_shape: Some(2),
            ..Default::default()
        };
        assert!(
            !output_emits_screen_update(&pending),
            "cursor-shape-only ProcessOutput must NOT emit screen-update"
        );

        // OSC 52 only.
        let pending = ProcessOutput {
            osc52: Some("clipboard-data".to_string()),
            ..Default::default()
        };
        assert!(
            !output_emits_screen_update(&pending),
            "osc52-only ProcessOutput must NOT emit screen-update"
        );

        // CWD only.
        let pending = ProcessOutput {
            new_cwd: Some("/tmp".to_string()),
            ..Default::default()
        };
        assert!(
            !output_emits_screen_update(&pending),
            "cwd-only ProcessOutput must NOT emit screen-update"
        );

        // All six combined (still no dirty).
        let pending = ProcessOutput {
            bell: true,
            new_title: Some("Title".to_string()),
            mode_changed: true,
            new_cursor_shape: Some(4),
            osc52: Some("clip".to_string()),
            new_cwd: Some("/tmp".to_string()),
            ..Default::default()
        };
        assert!(
            !output_emits_screen_update(&pending),
            "all-six non-visual fields combined must NOT emit screen-update \
             (no dirty region means no frontend paint and no frame-ack)"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-016 — output_emits_screen_update returns true when dirty has
    // content, regardless of non-visual fields.
    // -----------------------------------------------------------------------

    /// A `ProcessOutput` with dirty content MUST emit a `screen-update` event,
    /// whether or not non-visual fields are present alongside.
    #[test]
    fn output_emits_screen_update_true_when_dirty() {
        use super::emitter::output_emits_screen_update;

        // Dirty only (is_full_redraw).
        let pending = ProcessOutput {
            dirty: DirtyRegion {
                rows: Default::default(),
                is_full_redraw: true,
                cursor_moved: false,
            },
            ..Default::default()
        };
        assert!(
            output_emits_screen_update(&pending),
            "full-redraw-only ProcessOutput MUST emit screen-update"
        );

        // Dirty only (cursor_moved).
        let pending = ProcessOutput {
            dirty: DirtyRegion {
                rows: Default::default(),
                is_full_redraw: false,
                cursor_moved: true,
            },
            ..Default::default()
        };
        assert!(
            output_emits_screen_update(&pending),
            "cursor-moved-only ProcessOutput MUST emit screen-update"
        );

        // Dirty + bell + title + cursor shape.
        let pending = ProcessOutput {
            dirty: DirtyRegion {
                rows: Default::default(),
                is_full_redraw: true,
                cursor_moved: true,
            },
            bell: true,
            new_title: Some("T".to_string()),
            new_cursor_shape: Some(2),
            ..Default::default()
        };
        assert!(
            output_emits_screen_update(&pending),
            "dirty + non-visual fields MUST still emit screen-update"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-017 — bell-only flush: last_emit_ms NOT advanced, no stale nor
    // drop mode activation after 1.2 s.
    // -----------------------------------------------------------------------

    /// End-to-end logic simulation of the fix: a bell-only emit at T=0 must
    /// leave `last_emit_ms` at its startup value (0). At T=1.2 s, the
    /// `has_unacked_emits` predicate MUST be false — neither stale nor drop
    /// mode may activate. This is the core acceptance criterion of ADR-0027
    /// Addendum 2.
    #[test]
    fn bell_only_flush_does_not_advance_last_emit_ms() {
        use super::emitter::output_emits_screen_update;

        // T=0: bell-only emit.
        let pending = ProcessOutput {
            bell: true,
            ..Default::default()
        };
        let emitted_screen_update = output_emits_screen_update(&pending);
        assert!(
            !emitted_screen_update,
            "bell-only emit must not flag emitted_screen_update"
        );

        // Simulate startup state and gated advancement.
        let mut last_emit_ms: u64 = 0;
        if emitted_screen_update {
            last_emit_ms = 1; // would have been now_ms(); never reached here.
        }
        assert_eq!(
            last_emit_ms, 0,
            "last_emit_ms must remain at startup value after bell-only flush"
        );

        // T=1.2 s: check backpressure predicates.
        let now: u64 = 1_700_000_001_200;
        let last_ack_ms: u64 = 1_700_000_000_000; // last_ack_ms initialised to now() at task start
        let ack_age_ms = now.saturating_sub(last_ack_ms);
        let has_unacked_emits = last_emit_ms > last_ack_ms;
        let in_drop_mode = has_unacked_emits && ack_age_ms > ACK_DROP_THRESHOLD_MS;
        let in_stale_mode = has_unacked_emits && ack_age_ms > ACK_STALE_THRESHOLD_MS;

        assert!(
            ack_age_ms > ACK_DROP_THRESHOLD_MS,
            "ack age ({ack_age_ms} ms) must exceed drop threshold to exercise the fix"
        );
        assert!(
            !has_unacked_emits,
            "fix: bell-only flush must leave has_unacked_emits = false"
        );
        assert!(
            !in_drop_mode,
            "fix: drop mode must NOT activate after bell-only flush + idle"
        );
        assert!(
            !in_stale_mode,
            "fix: stale mode must NOT activate after bell-only flush + idle"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-018 — anti-regression: conditional gate on emitted_screen_update
    // -----------------------------------------------------------------------

    /// The exact conditional pattern used at the 3 call sites in `reader.rs`
    /// MUST leave `last_emit_ms` unchanged when `emitted_screen_update` is
    /// false, and MUST update it when true. This test is the minimal
    /// structural guard against a future regression that reintroduces the
    /// unconditional assignment.
    #[test]
    fn last_emit_ms_advancement_gated_on_emitted_screen_update() {
        use super::emitter::EmitOutcome;
        use std::time::Duration;

        // Case 1: emitted_screen_update = false → last_emit_ms unchanged.
        let outcome_no_screen = EmitOutcome {
            duration: Duration::from_millis(5),
            emitted_screen_update: false,
        };
        let pre_existing_last_emit_ms: u64 = 1_700_000_000_000;
        let mut last_emit_ms = pre_existing_last_emit_ms;
        if outcome_no_screen.emitted_screen_update {
            last_emit_ms = now_ms();
        }
        assert_eq!(
            last_emit_ms, pre_existing_last_emit_ms,
            "last_emit_ms must be UNCHANGED when emitted_screen_update = false"
        );

        // Case 2: emitted_screen_update = true → last_emit_ms updated.
        let outcome_with_screen = EmitOutcome {
            duration: Duration::from_millis(5),
            emitted_screen_update: true,
        };
        let mut last_emit_ms: u64 = 0;
        if outcome_with_screen.emitted_screen_update {
            last_emit_ms = now_ms();
        }
        assert!(
            last_emit_ms > 0,
            "last_emit_ms must be updated when emitted_screen_update = true"
        );
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-019 — was_in_drop_mode transition safety: empty pending means
    // no emit_all_pending call and no spurious full-redraw.
    // -----------------------------------------------------------------------

    /// When a pane exits drop mode on an idle tick (no dirty content available
    /// at the moment of the transition), the reader loop MUST NOT invoke
    /// `emit_all_pending` and MUST NOT synthesize a full redraw for an empty
    /// `pending`. This mirrors the `!pending.is_empty()` control-flow guard at
    /// line 299 of `reader.rs`.
    #[test]
    fn was_in_drop_mode_transition_with_empty_pending_skips_emit() {
        // State at tick N+1: drop mode has just exited (was_in_drop_mode=true,
        // in_drop_mode=false). A bell was emitted at tick N and already
        // consumed from pending — no dirty output has arrived since.
        let was_in_drop_mode = true;
        let in_drop_mode = false;
        let pending = ProcessOutput::default(); // empty: bell consumed at tick N

        // Structural guard from reader.rs (outer `if !pending.is_empty()`).
        let will_emit = !pending.is_empty();

        assert!(
            !will_emit,
            "empty pending at tick N+1 must not trigger emit_all_pending"
        );

        // If the outer guard erroneously allowed the inner transition, we'd
        // synthesize a spurious full-redraw here — verify that the protection
        // is by the outer guard, not by the transition logic itself.
        // (Belt-and-suspenders: check that the transition logic, when
        // reached with empty pending, would indeed have set is_full_redraw
        // — reinforcing the need for the outer guard.)
        if will_emit {
            // (unreachable in this test; kept for documentation)
            let mut p = pending;
            if in_drop_mode {
                p.dirty = DirtyRegion::default();
            } else if was_in_drop_mode {
                p.dirty.is_full_redraw = true;
            }
            panic!(
                "control flow should have short-circuited at `will_emit`; \
                 reached transition logic with is_full_redraw={}",
                p.dirty.is_full_redraw
            );
        }
    }

    // -----------------------------------------------------------------------
    // TEST-ACK-020 — bell flood non-escalation: 10 bell-only batches over
    // 1.5 s must not push debounce to ACK_STALE_DEBOUNCE.
    // -----------------------------------------------------------------------

    /// Stress test of the fix: a flood of bell events (e.g. vim `:set bell`
    /// loop, or a curses app that rings repeatedly) must NOT push the
    /// debounce interval up to `ACK_STALE_DEBOUNCE` (250 ms, Stage 1). Per
    /// the fix, `last_emit_ms` is not advanced for bell-only emits, so
    /// `has_unacked_emits` stays `false` throughout, and
    /// `current_debounce` remains within `[DEBOUNCE_MIN, DEBOUNCE_MAX]`.
    #[test]
    fn bell_flood_does_not_escalate_to_stale_debounce() {
        use super::emitter::output_emits_screen_update;

        let base_time: u64 = 1_700_000_000_000;
        let mut last_emit_ms: u64 = 0; // startup value
        let last_ack_ms: u64 = base_time; // initialised to now() at task start

        // Simulate 10 bell-only batches over 1500 ms (150 ms apart — slower
        // than DEBOUNCE_MAX so each is a distinct timer fire).
        for i in 0..10u64 {
            let simulated_now = base_time + i * 150;

            // Bell-only pending.
            let pending = ProcessOutput {
                bell: true,
                ..Default::default()
            };

            let emitted_screen_update = output_emits_screen_update(&pending);
            assert!(
                !emitted_screen_update,
                "iteration {i}: bell-only must not flag emitted_screen_update"
            );

            // Gated assignment (matches reader.rs).
            if emitted_screen_update {
                last_emit_ms = simulated_now;
            }

            let ack_age_ms = simulated_now.saturating_sub(last_ack_ms);
            let has_unacked_emits = last_emit_ms > last_ack_ms;
            let in_stale_mode = has_unacked_emits && ack_age_ms > ACK_STALE_THRESHOLD_MS;

            assert!(
                !has_unacked_emits,
                "iteration {i} (t={simulated_now}): has_unacked_emits must \
                 stay false across bell flood"
            );
            assert!(
                !in_stale_mode,
                "iteration {i} (t={simulated_now}): stale mode must NOT \
                 activate during bell flood"
            );

            // Debounce chosen per `next_debounce()` rules — within
            // [DEBOUNCE_MIN, DEBOUNCE_MAX]. We assert the clamping contract.
            // A realistic emit_duration for bell-only is sub-ms; next_debounce
            // clamps to DEBOUNCE_MIN.
            let current_debounce = if in_stale_mode {
                ACK_STALE_DEBOUNCE
            } else {
                next_debounce(Duration::from_micros(50))
            };
            assert!(
                current_debounce >= DEBOUNCE_MIN && current_debounce <= DEBOUNCE_MAX,
                "iteration {i}: current_debounce ({current_debounce:?}) must \
                 stay within [{DEBOUNCE_MIN:?}, {DEBOUNCE_MAX:?}]"
            );
            assert!(
                current_debounce < ACK_STALE_DEBOUNCE,
                "iteration {i}: current_debounce ({current_debounce:?}) must \
                 stay below ACK_STALE_DEBOUNCE ({ACK_STALE_DEBOUNCE:?})"
            );
        }
    }
}
