// SPDX-License-Identifier: MPL-2.0

//! IPC throughput benchmarks — measures the cost of building and serializing
//! the events that cross the Tauri IPC boundary from backend to frontend.
//!
//! Benchmarks in this file cover:
//! - `build_screen_update_event` (full redraw and partial update paths)
//! - `build_scrolled_viewport_event`
//! - `serde_json::to_string` on a full `ScreenUpdateEvent`
//! - `RwLock` write contention (writer blocked by concurrent readers)
//! - `RwLock` read contention (reader blocked by concurrent writer)

use std::{
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use criterion::{Criterion, criterion_group, criterion_main};
use parking_lot::RwLock;
use tau_term_lib::{
    events::types::{CellAttrsDto, CellUpdate, CursorState, ScreenUpdateEvent},
    session::{
        ids::PaneId,
        pty_task::{build_screen_update_event, build_scrolled_viewport_event},
    },
    vt::{VtProcessor, screen_buffer::DirtyRegion},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an `Arc<RwLock<VtProcessor>>` pre-filled with printable ASCII content.
fn make_filled_vt(cols: u16, rows: u16) -> Arc<RwLock<VtProcessor>> {
    let content: Vec<u8> = (0u8..128)
        .cycle()
        .filter(|b| b.is_ascii_graphic())
        .take(cols as usize * rows as usize)
        .collect();
    let vt = Arc::new(RwLock::new(VtProcessor::new(cols, rows, 10_000, 0, false)));
    vt.write().process(&content);
    vt
}

// ---------------------------------------------------------------------------
// bench_build_screen_update_full_redraw
// ---------------------------------------------------------------------------

/// Measures the hot path for building a full-redraw `ScreenUpdateEvent` (220×50).
///
/// A full redraw clones the entire screen snapshot — this bench quantifies that cost.
fn bench_build_screen_update_full_redraw(c: &mut Criterion) {
    let vt = make_filled_vt(220, 50);
    let pane_id = PaneId("bench".into());

    let mut dirty = DirtyRegion::default();
    dirty.mark_full_redraw();

    let mut group = c.benchmark_group("screen_update_event");
    group.bench_function("full_redraw_220x50", |b| {
        b.iter(|| {
            let event =
                build_screen_update_event(black_box(&pane_id), black_box(&vt), black_box(&dirty));
            black_box(event);
        })
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// bench_build_screen_update_partial
// ---------------------------------------------------------------------------

/// Measures the partial-update path: all 50 rows dirty, but `is_full_redraw` is false.
///
/// The partial path reads rows individually via `active_buf_ref().get_row()` — no
/// full snapshot clone. This bench isolates that cost vs. the full-redraw path.
fn bench_build_screen_update_partial(c: &mut Criterion) {
    let vt = make_filled_vt(220, 50);
    let pane_id = PaneId("bench".into());

    // All 50 rows dirty, but is_full_redraw stays false.
    let mut dirty = DirtyRegion::default();
    for row in 0u16..50 {
        dirty.mark_row(row);
    }

    let mut group = c.benchmark_group("screen_update_event");
    group.bench_function("partial_all_50_rows_220x50", |b| {
        b.iter(|| {
            let event =
                build_screen_update_event(black_box(&pane_id), black_box(&vt), black_box(&dirty));
            black_box(event);
        })
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// bench_build_scrolled_viewport
// ---------------------------------------------------------------------------

/// Measures `build_scrolled_viewport_event` with 1000 scrollback lines and k=500.
///
/// This exercises the composite viewport path: half the rows come from scrollback,
/// half from the live screen.
fn bench_build_scrolled_viewport(c: &mut Criterion) {
    let vt = Arc::new(RwLock::new(VtProcessor::new(80, 24, 10_000, 0, false)));
    {
        let mut proc = vt.write();
        let row: Vec<u8> = std::iter::repeat_n(b'X', 80).collect();
        for _ in 0..1_000 {
            proc.process(&row);
            proc.process(b"\r\n");
        }
    }
    let pane_id = PaneId("bench".into());

    let mut group = c.benchmark_group("scrolled_viewport");
    group.bench_function("build_scroll_500_of_1000", |b| {
        b.iter(|| {
            let event = build_scrolled_viewport_event(
                black_box(&pane_id),
                black_box(&vt),
                black_box(500i64),
            );
            black_box(event);
        })
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// bench_serde_json_full_redraw
// ---------------------------------------------------------------------------

/// Measures `serde_json::to_string` on a fully-populated 220×50 `ScreenUpdateEvent`.
///
/// This is the serialization cost of the IPC boundary: Tauri serializes the event
/// to JSON before sending it to the WebView. The result size (~500 KB) is load-bearing
/// — optimizing the struct layout shows up directly here.
fn bench_serde_json_full_redraw(c: &mut Criterion) {
    let cells: Vec<CellUpdate> = (0..50u16)
        .flat_map(|row| {
            (0..220u16).map(move |col| CellUpdate {
                row,
                col,
                content: "A".to_string(),
                width: 1,
                attrs: CellAttrsDto {
                    fg: None,
                    bg: None,
                    bold: None,
                    dim: None,
                    italic: None,
                    underline: None,
                    blink: None,
                    inverse: None,
                    hidden: None,
                    strikethrough: None,
                    underline_color: None,
                },
                hyperlink: None,
            })
        })
        .collect();

    let event = ScreenUpdateEvent {
        pane_id: PaneId("bench".into()),
        cells,
        cursor: CursorState {
            row: 0,
            col: 0,
            visible: true,
            shape: 0,
            blink: true,
        },
        scrollback_lines: 0,
        is_full_redraw: true,
        cols: 220,
        rows: 50,
        scroll_offset: 0,
    };

    let mut group = c.benchmark_group("serde_json_ipc");
    group.throughput(criterion::Throughput::Elements(1));
    group.bench_function("serialize_full_redraw_220x50", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&event))
                .expect("ScreenUpdateEvent must be serializable");

            black_box(json);
        })
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// bench_rwlock_contention — write_under_reader + read_under_writer
// ---------------------------------------------------------------------------

/// Measures RwLock write latency when a background reader holds the lock continuously.
///
/// Simulates Task 2 (emitter) holding a read-lock while Task 1 (reader) tries to
/// acquire a write-lock to process PTY data.
///
/// Also measures the inverse: read-lock acquisition latency while a background
/// writer (Task 1) holds the lock continuously.
fn bench_rwlock_contention(c: &mut Criterion) {
    let data: &[u8] = b"hello world\r\n";

    // --- write_under_reader ---
    {
        let vt = Arc::new(RwLock::new(VtProcessor::new(220, 50, 10_000, 0, false)));
        let vt_r = Arc::clone(&vt);
        let running = Arc::new(AtomicBool::new(true));
        let running_r = Arc::clone(&running);

        // Background reader thread — simulates Task 2 holding the read-lock.
        // `_g` is a named binding; the read-guard is dropped at end of the
        // loop body (after yield_now), not at end of the let statement, so
        // the lock is genuinely held across the yield.
        let reader_handle = thread::spawn(move || {
            while running_r.load(Ordering::Relaxed) {
                let _g = vt_r.read();
                thread::yield_now();
            }
        });

        let mut group = c.benchmark_group("rwlock_contention");
        group.measurement_time(Duration::from_secs(5));
        group.bench_function("write_under_reader", |b| {
            b.iter(|| {
                let mut w = vt.write();
                w.process(black_box(data));
            })
        });
        group.finish();

        running.store(false, Ordering::Relaxed);
        reader_handle.join().ok();
    }

    // --- read_under_writer ---
    {
        let vt = Arc::new(RwLock::new(VtProcessor::new(220, 50, 10_000, 0, false)));
        let vt_w = Arc::clone(&vt);
        let running = Arc::new(AtomicBool::new(true));
        let running_r = Arc::clone(&running);
        let data_w: &'static [u8] = b"hello world\r\n";

        let pane_id = PaneId("bench".into());
        let mut dirty = DirtyRegion::default();
        dirty.mark_full_redraw();

        // Background writer thread — simulates Task 1 processing PTY data.
        let writer_handle = thread::spawn(move || {
            while running_r.load(Ordering::Relaxed) {
                let mut w = vt_w.write();
                w.process(data_w);
                drop(w);
                thread::yield_now();
            }
        });

        let mut group = c.benchmark_group("rwlock_contention");
        group.measurement_time(Duration::from_secs(5));
        group.bench_function("read_under_writer", |b| {
            b.iter(|| {
                let event = build_screen_update_event(
                    black_box(&pane_id),
                    black_box(&vt),
                    black_box(&dirty),
                );
                black_box(event);
            })
        });
        group.finish();

        running.store(false, Ordering::Relaxed);
        writer_handle.join().ok();
    }
}

// ---------------------------------------------------------------------------
// bench_serde_json_full_redraw_default_attrs
// ---------------------------------------------------------------------------

/// Measures `serde_json::to_string` on a 220×50 `ScreenUpdateEvent` where every
/// cell has all-default attributes (bold=false, underline=0, fg=None, etc.).
///
/// With P-IPC1 (`skip_serializing_if` on boolean/u8 fields), these attributes
/// are omitted from JSON entirely — this bench quantifies the resulting payload
/// size reduction and serialization speedup compared to `bench_serde_json_full_redraw`.
fn bench_serde_json_full_redraw_default_attrs(c: &mut Criterion) {
    let cells: Vec<CellUpdate> = (0..50u16)
        .flat_map(|row| {
            (0..220u16).map(move |col| CellUpdate {
                row,
                col,
                content: "a".to_string(),
                width: 1,
                attrs: CellAttrsDto {
                    fg: None,
                    bg: None,
                    bold: None,
                    dim: None,
                    italic: None,
                    underline: None,
                    blink: None,
                    inverse: None,
                    hidden: None,
                    strikethrough: None,
                    underline_color: None,
                },
                hyperlink: None,
            })
        })
        .collect();

    let event = ScreenUpdateEvent {
        pane_id: PaneId("bench".into()),
        cells,
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

    let mut group = c.benchmark_group("default_attrs");
    group.throughput(criterion::Throughput::Elements(1));
    group.bench_function("serialize_full_redraw_220x50_default_attrs", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&event))
                .expect("ScreenUpdateEvent must be serializable");
            black_box(json);
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_build_screen_update_full_redraw,
    bench_build_screen_update_partial,
    bench_build_scrolled_viewport,
    bench_serde_json_full_redraw,
    bench_serde_json_full_redraw_default_attrs,
    bench_rwlock_contention,
);
criterion_main!(benches);
