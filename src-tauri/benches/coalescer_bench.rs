// SPDX-License-Identifier: MPL-2.0

//! Coalescer benchmarks (ADR-0028 Commit 6).
//!
//! Three benchmarks targeting the shared `session::output` pipeline hot paths:
//!
//! 1. [`bench_coalescer_throughput`] — end-to-end emit latency for a synthetic
//!    burst of N `ProcessOutput` chunks (10, 100, 1 000) flowing through the
//!    coalescer's `tokio::select!` / debounce / frame-ack state machine.
//! 2. [`bench_process_output_merge`] — cost of `ProcessOutput::merge` as a
//!    function of accumulated dirty rows (1, 10, 100 rows × 200 cells each).
//! 3. [`bench_dsr_response_coalescing`] — cost of the
//!    `responses.into_iter().flatten().collect()` merge step + a no-op write
//!    simulation, for 10 / 100 / 256 (effective cap per ADR-0028) responses.
//!
//! ## Mocking strategy (mirror)
//!
//! `Coalescer`, `CoalescerConfig`, `CoalescerContext`, `run`, and `ProcessOutput`
//! are `pub(crate)` in the production crate. Benches live in a separate binary
//! crate and cannot reach `pub(crate)` items. We use the SAME mirror approach
//! as `src-tauri/tests/ssh_coalescer_integration.rs` (Commit 5): replicate the
//! small, stable types and the coalescer event loop here, locked structurally
//! to the production code by code review. Reviewers MUST update both in
//! lockstep when:
//!
//! - `ProcessOutput` field shape changes (add field → mirror it here).
//! - `coalescer::run` event loop reshapes (e.g. new branch in `tokio::select!`).
//! - `output_emits_screen_update` predicate changes (extend `emits_screen_update`).
//!
//! This keeps production-code visibility minimal (zero churn). The alternative
//! — widening 5+ types to `#[doc(hidden)] pub` solely for benches — was rejected
//! per ADR-0028 Commit 6 scoping: production API surface stays narrow.
//!
//! ## Baseline
//!
//! Criterion writes results to `target/criterion/` (gitignored). No baseline is
//! checked in. Manual baseline procedure documented in `docs/testing/TESTING.md`
//! at Commit 8 of ADR-0028. The first run after this commit establishes the
//! reference numbers; future regressions require manual `criterion --save-baseline`.

#![allow(clippy::too_many_lines)]

use std::hint::black_box;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tau_term_lib::vt::screen_buffer::DirtyRegion;
use tokio::runtime::Builder;
use tokio::sync::mpsc;

// ===========================================================================
// Mirrors — kept structurally identical to production. See module-level note.
// ===========================================================================

// ---------------------------------------------------------------------------
// Mirror of `session::output::process_output::ProcessOutput`.
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct MirrorProcessOutput {
    dirty: DirtyRegion,
    mode_changed: bool,
    new_title: Option<String>,
    new_cursor_shape: Option<u8>,
    bell: bool,
    osc52: Option<String>,
    new_cwd: Option<String>,
    needs_immediate_flush: bool,
}

impl MirrorProcessOutput {
    /// Mirror of `ProcessOutput::merge`. Field-for-field identical.
    fn merge(&mut self, other: MirrorProcessOutput) {
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

    /// Mirror of `output_emits_screen_update(&pending)`.
    fn emits_screen_update(&self) -> bool {
        !self.dirty.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Mirror of `CoalescerConfig::for_tests` — shrunken thresholds so a Criterion
// iteration completes in single-digit virtual milliseconds. Production
// constants are too large for a tight bench loop; the structural ratios
// (stale ≈ 1/5 drop, stale_debounce > debounce_max) are preserved.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct MirrorCoalescerConfig {
    debounce_min: Duration,
    debounce_max: Duration,
    debounce_scale: f64,
    debounce_decay: f64,
    ack_stale_threshold_ms: u64,
    ack_stale_debounce: Duration,
    ack_drop_threshold_ms: u64,
}

impl MirrorCoalescerConfig {
    fn for_bench() -> Self {
        // Identical to `CoalescerConfig::for_tests()`. Renamed to surface intent.
        Self {
            debounce_min: Duration::from_millis(1),
            debounce_max: Duration::from_millis(10),
            debounce_scale: 1.2,
            debounce_decay: 0.5,
            ack_stale_threshold_ms: 20,
            ack_stale_debounce: Duration::from_millis(25),
            ack_drop_threshold_ms: 100,
        }
    }
}

fn next_debounce(emit_duration: Duration, cfg: &MirrorCoalescerConfig) -> Duration {
    emit_duration
        .mul_f64(cfg.debounce_scale)
        .clamp(cfg.debounce_min, cfg.debounce_max)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Observer — counts emit decisions. We do not store full event payloads here
// (unlike the integration test) to keep the bench overhead minimal — only
// the count is load-bearing for throughput measurement.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct EmitObserver {
    screen_updates: AtomicUsize,
    other_events: AtomicUsize,
}

impl EmitObserver {
    fn screen_update_count(&self) -> usize {
        self.screen_updates.load(Ordering::Relaxed)
    }
}

/// Mirror of `emit_all_pending` — drains `pending` into observer counters.
/// Returns `true` iff a `screen-update` would have been emitted.
fn mirror_emit_all_pending(observer: &EmitObserver, pending: &mut MirrorProcessOutput) -> bool {
    let emitted_screen_update = pending.emits_screen_update();

    if !pending.dirty.is_empty() {
        observer.screen_updates.fetch_add(1, Ordering::Relaxed);
    }
    if pending.mode_changed {
        observer.other_events.fetch_add(1, Ordering::Relaxed);
    }
    if pending.new_cursor_shape.is_some() {
        observer.other_events.fetch_add(1, Ordering::Relaxed);
    }
    if pending.bell {
        observer.other_events.fetch_add(1, Ordering::Relaxed);
    }
    if pending.osc52.take().is_some() {
        observer.other_events.fetch_add(1, Ordering::Relaxed);
    }
    if pending.new_title.take().is_some() {
        observer.other_events.fetch_add(1, Ordering::Relaxed);
    }
    if pending.new_cwd.take().is_some() {
        observer.other_events.fetch_add(1, Ordering::Relaxed);
    }

    *pending = MirrorProcessOutput::default();
    emitted_screen_update
}

// ---------------------------------------------------------------------------
// Mirror of `coalescer::run`. Same `tokio::select!` shape, same Stage 1 / 2
// logic. See `src-tauri/src/session/output/coalescer.rs::run`.
// ---------------------------------------------------------------------------

#[allow(clippy::needless_pass_by_value)]
async fn mirror_coalescer_run(
    cfg: MirrorCoalescerConfig,
    observer: Arc<EmitObserver>,
    last_frame_ack_ms: Arc<AtomicU64>,
    mut rx: mpsc::Receiver<MirrorProcessOutput>,
) {
    let mut pending = MirrorProcessOutput::default();
    let mut current_debounce = cfg.debounce_min;
    let mut was_in_drop_mode = false;
    let mut last_emit_ms: u64 = 0;

    let sleep_fut = tokio::time::sleep(current_debounce);
    tokio::pin!(sleep_fut);

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(output) => {
                        let flush_now = output.needs_immediate_flush;
                        pending.merge(output);
                        if flush_now {
                            while let Ok(more) = rx.try_recv() {
                                pending.merge(more);
                            }
                            if !pending.is_empty() {
                                let t0 = std::time::Instant::now();
                                let emitted = mirror_emit_all_pending(&observer, &mut pending);
                                let dur = t0.elapsed();
                                current_debounce = next_debounce(dur, &cfg);
                                if emitted {
                                    last_emit_ms = now_ms();
                                }
                            } else {
                                pending.needs_immediate_flush = false;
                            }
                            sleep_fut.as_mut().reset(tokio::time::Instant::now() + current_debounce);
                        }
                    }
                    None => {
                        if !pending.is_empty() {
                            let _ = mirror_emit_all_pending(&observer, &mut pending);
                        }
                        break;
                    }
                }
            }

            _ = &mut sleep_fut => {
                while let Ok(output) = rx.try_recv() {
                    pending.merge(output);
                }

                let last_ack_ms = last_frame_ack_ms.load(Ordering::Relaxed);
                let ack_age_ms = now_ms().saturating_sub(last_ack_ms);
                let has_unacked_emits = last_emit_ms > last_ack_ms;
                let in_drop_mode =
                    has_unacked_emits && ack_age_ms > cfg.ack_drop_threshold_ms;
                let in_stale_mode =
                    has_unacked_emits && ack_age_ms > cfg.ack_stale_threshold_ms;

                if !pending.is_empty() {
                    if in_drop_mode {
                        pending.dirty = DirtyRegion::default();
                        pending.needs_immediate_flush = false;
                    } else if was_in_drop_mode {
                        pending.dirty.is_full_redraw = true;
                    }

                    if !pending.is_empty() {
                        let t0 = std::time::Instant::now();
                        let emitted = mirror_emit_all_pending(&observer, &mut pending);
                        let dur = t0.elapsed();
                        if emitted {
                            last_emit_ms = now_ms();
                        }
                        current_debounce = if in_stale_mode {
                            cfg.ack_stale_debounce
                        } else {
                            next_debounce(dur, &cfg)
                        };
                    } else {
                        pending = MirrorProcessOutput::default();
                    }
                } else {
                    current_debounce = cfg
                        .debounce_min
                        .max(current_debounce.mul_f64(cfg.debounce_decay));
                }

                was_in_drop_mode = in_drop_mode;
                sleep_fut.as_mut().reset(tokio::time::Instant::now() + current_debounce);
            }
        }
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Build a synthetic `ProcessOutput` whose `dirty` covers `rows_count` rows.
///
/// The 200-cells-per-row figure cited in the bench description is a property
/// of the *event-builder cost* (not measured here — covered by `ipc_throughput`).
/// At the `ProcessOutput::merge` layer, only the row count of `DirtyRows` is
/// load-bearing, so we mark `rows_count` distinct rows on the dirty region.
fn dirty_with_rows(rows_count: u16) -> MirrorProcessOutput {
    let mut dirty = DirtyRegion::default();
    for r in 0..rows_count {
        dirty.mark_row(r);
    }
    MirrorProcessOutput {
        dirty,
        ..Default::default()
    }
}

fn dirty_full_redraw() -> MirrorProcessOutput {
    let mut dirty = DirtyRegion::default();
    dirty.mark_full_redraw();
    MirrorProcessOutput {
        dirty,
        ..Default::default()
    }
}

// ===========================================================================
// bench_coalescer_throughput
// ===========================================================================

/// Measures end-to-end coalescer latency: send N `ProcessOutput` chunks into
/// the bounded mpsc channel, drop the sender, await the coalescer task to
/// drain. The total wall-clock time of one iteration is the metric.
///
/// Bench exercises the production-shape pipeline end-to-end:
///
/// - bounded `mpsc::channel(256)` (matching ADR-0027 sizing)
/// - real `tokio::select!` event loop (mirror)
/// - real adaptive debounce arithmetic
/// - real `frame-ack` two-stage logic (ack_age never crosses thresholds in
///   the throughput bench because `last_ack` is bumped to "now" — Stage 1/2
///   are deliberately inert here; that path is covered by SSH-COALESCE-002/003)
///
/// Variations: N ∈ {10, 100, 1 000}.
///
/// Each iteration spins up its own runtime + coalescer task. This includes
/// some constant overhead (~100-200 µs runtime startup) — comparing the three
/// N values lets us subtract that out if needed for regression analysis.
fn bench_coalescer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("coalescer_throughput");

    for &n in &[10usize, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                // Per-iteration runtime: matches production isolation. Cost
                // is included in the measurement but constant across N.
                let rt = Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("tokio runtime must build");

                rt.block_on(async {
                    let cfg = MirrorCoalescerConfig::for_bench();
                    let observer = Arc::new(EmitObserver::default());
                    // Fresh ack at "now" — ack-stale / drop never engage.
                    let last_ack = Arc::new(AtomicU64::new(now_ms()));

                    let (tx, rx) = mpsc::channel::<MirrorProcessOutput>(256);
                    let obs = observer.clone();
                    let ack = last_ack.clone();
                    let coalescer = tokio::spawn(async move {
                        mirror_coalescer_run(cfg, obs, ack, rx).await;
                    });

                    for _ in 0..n {
                        // Use `await` (not `try_send`): the bench measures
                        // back-pressure-tolerant throughput, not drop rate.
                        tx.send(black_box(dirty_full_redraw()))
                            .await
                            .expect("send must succeed");
                    }
                    drop(tx);
                    coalescer.await.expect("coalescer must finish");

                    // Sanity: at least one emit happened. Prevents the
                    // compiler from eliding the entire pipeline if a future
                    // refactor unexpectedly short-circuits emits.
                    black_box(observer.screen_update_count());
                });
            });
        });
    }
    group.finish();
}

// ===========================================================================
// bench_process_output_merge
// ===========================================================================

/// Measures the cost of `ProcessOutput::merge` as a function of accumulated
/// dirty rows. The merge step is called once per upstream chunk in the hot
/// loop (`pending.merge(output)`); its cost dominates when bursts arrive
/// faster than the debounce can fire.
///
/// Variations: 1, 10, 100 dirty rows per merged chunk.
///
/// Note on "200 cells per row" (from the Commit 6 brief): cell-level cost
/// belongs to the event-builder path (already measured by `ipc_throughput::
/// bench_build_screen_update_partial`). At the `ProcessOutput::merge` layer,
/// `DirtyRows` stores rows as a bitset (`u128`-based), so per-row cost is
/// O(1) bit-set arithmetic regardless of cell content. The bench measures
/// the merge primitive faithfully without conflating it with rendering cost.
fn bench_process_output_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_output_merge");

    for &rows in &[1u16, 10, 100] {
        group.throughput(Throughput::Elements(u64::from(rows)));
        group.bench_with_input(
            BenchmarkId::from_parameter(rows),
            &rows,
            |b, &rows_count| {
                // Pre-build the source chunk once per bench (setup cost
                // excluded from the timing).
                let source = dirty_with_rows(rows_count);
                b.iter(|| {
                    let mut acc = MirrorProcessOutput::default();
                    // Single merge — repeated by Criterion's iter count.
                    acc.merge(black_box(source.clone()));
                    black_box(acc);
                });
            },
        );
    }
    group.finish();
}

// ===========================================================================
// bench_dsr_response_coalescing
// ===========================================================================

/// Measures the cost of merging N DSR/CPR responses into a single contiguous
/// `Vec<u8>` and writing it to a mock channel. This path is the SSH writer's
/// per-drain hot loop (`responses.into_iter().flatten().collect()` + one
/// `ch.data().await`).
///
/// Variations: N ∈ {10, 100, 1 000}.
///
/// Note: production caps `pending_responses` at 256 entries (ADR-0028 Commit
/// 4 — `MAX_PENDING_RESPONSES`). The 1 000 variant therefore measures the
/// merge cost at a payload size larger than production can ever produce,
/// which is intentional — it documents headroom and surfaces any
/// quadratic-in-N regression should the cap ever be raised. The 256-effective
/// case lies between the 100 and 1 000 buckets; results scale linearly so
/// readers can interpolate.
///
/// Each DSR response is `\x1b[<row>;<col>R` — 8 bytes typical. We use that
/// canonical reply (`\x1b[24;80R`) to keep the bench faithful to real CPR
/// reply sizes.
fn bench_dsr_response_coalescing(c: &mut Criterion) {
    let mut group = c.benchmark_group("dsr_response_coalescing");

    for &n in &[10usize, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            // Pre-build the responses Vec once. We clone it inside `iter`
            // because `into_iter().flatten().collect()` consumes `responses`,
            // which must be reproduced fresh for each iteration.
            let template: Vec<Vec<u8>> = (0..n).map(|_| b"\x1b[24;80R".to_vec()).collect();

            // Mock "channel" — a `Mutex<Vec<Vec<u8>>>` mirrors the production
            // shape (`tokio::sync::Mutex<russh::Channel>`). We use std Mutex
            // since the bench is fully synchronous below.
            let mock_channel: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));

            b.iter(|| {
                // Reproduce the responses for this iteration.
                let responses = template.clone();

                // Production hot path: merge + single write.
                let merged: Vec<u8> = responses.into_iter().flatten().collect();
                {
                    // Mirror `let ch = channel.lock().await; ch.data(merged.as_slice()).await?;`
                    // The std Mutex stand-in cannot block here (single thread),
                    // so the lock cost is sub-µs. The dominant cost is the
                    // merge above — exactly what we want to measure.
                    let mut ch = mock_channel.lock().expect("mock channel lock");
                    ch.push(black_box(merged));
                    // Drain to keep the mock channel from growing unbounded
                    // across iterations.
                    ch.clear();
                }
                black_box(&mock_channel);
            });
        });
    }
    group.finish();
}

// ===========================================================================
// Criterion harness
// ===========================================================================

criterion_group!(
    benches,
    bench_coalescer_throughput,
    bench_process_output_merge,
    bench_dsr_response_coalescing,
);
criterion_main!(benches);
