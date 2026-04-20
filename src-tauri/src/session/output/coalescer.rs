// SPDX-License-Identifier: MPL-2.0

//! Source-agnostic async coalescer for `ProcessOutput`.
//!
//! Receives `ProcessOutput` values from a bounded `mpsc` channel, coalesces
//! them on an adaptive debounce interval, and emits the resulting events to
//! the frontend via [`super::emitter::emit_all_pending`].
//!
//! ## Frame-ack two-stage backpressure (ADR-0027)
//!
//! Combined with the bounded mpsc capacity and the frontend's `frame_ack` IPC
//! call, the coalescer implements two protective stages:
//!
//! - **Stage 1 (stale)**: when no `frame_ack` has been received for at least
//!   [`CoalescerConfig::ack_stale_threshold_ms`], the debounce interval is
//!   forced to [`CoalescerConfig::ack_stale_debounce`] regardless of the
//!   adaptive computation, slowing the emit rate.
//! - **Stage 2 (drop)**: when no `frame_ack` has been received for at least
//!   [`CoalescerConfig::ack_drop_threshold_ms`], dirty cell updates are
//!   suppressed (cleared); non-visual events (bell, mode, cursor shape, OSC 52,
//!   title, CWD) are still emitted. On exit from drop mode, a full redraw is
//!   forced to refresh the now-stale frontend grid.
//!
//! ## Caller-managed termination (ADR-0028)
//!
//! The [`run`] function performs no source-specific cleanup. When the upstream
//! channel is closed (sender dropped → `recv()` returns `None`), [`run`]
//! flushes any remaining pending output and returns. The caller (PTY task or
//! SSH task) is responsible for awaiting the `JoinHandle` and then performing
//! pipeline-specific termination work (process reaping for PTY, SSH lifecycle
//! mutation for SSH).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use parking_lot::RwLock;
use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::vt::{DirtyRegion, VtProcessor};

use super::ProcessOutput;
use super::emitter::emit_all_pending;

// ---------------------------------------------------------------------------
// Adaptive debounce constants (PTY default — ADR-0027 P-HT-2)
// ---------------------------------------------------------------------------

/// Minimum debounce window — floor for adaptive scaling and idle decay.
pub(crate) const DEBOUNCE_MIN: Duration = Duration::from_millis(12);

/// Maximum debounce window — cap to avoid perceptible input latency.
pub(crate) const DEBOUNCE_MAX: Duration = Duration::from_millis(100);

/// Multiplier applied to the measured emit duration to compute the next
/// debounce interval. A value slightly above 1.0 gives the frontend a
/// comfortable margin to process the event before the next one arrives.
pub(crate) const DEBOUNCE_SCALE: f64 = 1.2;

/// Decay factor applied on idle ticks (no pending output). Exponentially
/// shrinks the debounce interval back toward `DEBOUNCE_MIN` when the source
/// is quiet, ensuring low latency for interactive use after a burst.
pub(crate) const DEBOUNCE_DECAY: f64 = 0.5;

/// Ack age above which debounce is escalated (Stage 1).
pub(crate) const ACK_STALE_THRESHOLD_MS: u64 = 200;

/// Debounce interval during stale-ack mode.
pub(crate) const ACK_STALE_DEBOUNCE: Duration = Duration::from_millis(250);

/// Ack age above which dirty updates are dropped (Stage 2).
pub(crate) const ACK_DROP_THRESHOLD_MS: u64 = 1000;

// ---------------------------------------------------------------------------
// Helpers (pure)
// ---------------------------------------------------------------------------

/// Wall-clock millisecond timestamp (epoch).
///
/// Used for ack-age and emit-age tracking. A monotonic clock would be
/// preferable, but `frame_ack` timestamps are recorded on the same wall clock
/// (see `SessionRegistry::record_frame_ack`), so the two MUST agree.
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute the next debounce interval from the measured emit duration.
///
/// The result is `emit_duration * config.debounce_scale`, clamped to
/// `[config.debounce_min, config.debounce_max]`.
pub(crate) fn next_debounce(emit_duration: Duration, config: &CoalescerConfig) -> Duration {
    emit_duration
        .mul_f64(config.debounce_scale)
        .clamp(config.debounce_min, config.debounce_max)
}

// ---------------------------------------------------------------------------
// CoalescerConfig — tunable thresholds
// ---------------------------------------------------------------------------

/// Tunable thresholds for the coalescer.
///
/// Three constructors:
///
/// - [`CoalescerConfig::pty_default`]: production PTY values (ADR-0027 baseline).
/// - [`CoalescerConfig::ssh_default`]: SSH values — currently identical to PTY
///   defaults, exposed as a separate constructor to prepare for future
///   divergence (e.g. larger debounce on high-RTT links) without a future
///   API break.
/// - [`CoalescerConfig::for_tests`]: shrunken thresholds for deterministic
///   timing in `#[tokio::test(start_paused = true)]` tests.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CoalescerConfig {
    pub debounce_min: Duration,
    pub debounce_max: Duration,
    pub debounce_scale: f64,
    pub debounce_decay: f64,
    pub ack_stale_threshold_ms: u64,
    pub ack_stale_debounce: Duration,
    pub ack_drop_threshold_ms: u64,
}

impl CoalescerConfig {
    /// PTY default thresholds (ADR-0027 baseline).
    pub(crate) const fn pty_default() -> Self {
        Self {
            debounce_min: DEBOUNCE_MIN,
            debounce_max: DEBOUNCE_MAX,
            debounce_scale: DEBOUNCE_SCALE,
            debounce_decay: DEBOUNCE_DECAY,
            ack_stale_threshold_ms: ACK_STALE_THRESHOLD_MS,
            ack_stale_debounce: ACK_STALE_DEBOUNCE,
            ack_drop_threshold_ms: ACK_DROP_THRESHOLD_MS,
        }
    }

    /// SSH default thresholds.
    ///
    /// Currently identical to [`Self::pty_default`] — declared as a separate
    /// constructor to prepare for SSH-specific tuning (e.g. larger debounce
    /// on high-RTT links) without forcing a later API break. Used by
    /// `session::ssh_task` starting in ADR-0028 Commit 3.
    #[allow(dead_code)]
    pub(crate) const fn ssh_default() -> Self {
        Self::pty_default()
    }

    /// Test-friendly thresholds: shrunken so `#[tokio::test(start_paused =
    /// true)]` tests can exercise stale/drop transitions in single-digit
    /// virtual milliseconds.
    ///
    /// Ratios match production constants (`stale ≈ 1/5 drop`,
    /// `stale_debounce > debounce_max`), so behaviour is structurally
    /// equivalent.
    #[allow(dead_code)] // Exposed for future tests in `output/tests.rs`.
    pub(crate) const fn for_tests() -> Self {
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

// ---------------------------------------------------------------------------
// CoalescerContext — dependencies wired by the caller
// ---------------------------------------------------------------------------

/// Bundle of dependencies passed to [`run`].
///
/// Grouping these in a struct keeps the [`run`] signature stable as the
/// pipeline grows; callers can reuse the same context for multiple
/// re-spawns if needed (e.g. after an SSH reconnect).
pub(crate) struct CoalescerContext {
    pub app: AppHandle,
    pub pane_id: PaneId,
    pub vt: Arc<RwLock<VtProcessor>>,
    pub registry: Arc<SessionRegistry>,
    /// Wall-clock millisecond timestamp of the most recent `frame_ack` IPC
    /// from the frontend for this pane. Updated by
    /// `SessionRegistry::record_frame_ack`. Read here to drive frame-ack
    /// stale/drop escalation.
    pub last_frame_ack_ms: Arc<AtomicU64>,
    pub config: CoalescerConfig,
}

// ---------------------------------------------------------------------------
// Coalescer — encapsulated mutable state
// ---------------------------------------------------------------------------

/// Encapsulated mutable state of the coalescer (formerly inline locals of
/// PTY Task 2).
///
/// Held by the [`run`] function; not directly exposed to callers. A separate
/// struct (rather than inline `let mut` in `run`) makes the state shape
/// explicit and benchmarkable in isolation.
pub(crate) struct Coalescer {
    pending: ProcessOutput,
    current_debounce: Duration,
    was_in_drop_mode: bool,
    last_emit_ms: u64,
}

impl Coalescer {
    /// Construct a fresh coalescer state from a configuration.
    ///
    /// `current_debounce` starts at `config.debounce_min`; `last_emit_ms` at
    /// `0` (interpreted as "no emit yet" by the `has_unacked_emits` predicate
    /// — see TEST-ACK-007).
    pub(crate) fn new(config: &CoalescerConfig) -> Self {
        Self {
            pending: ProcessOutput::default(),
            current_debounce: config.debounce_min,
            was_in_drop_mode: false,
            last_emit_ms: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// run — coalescer event loop
// ---------------------------------------------------------------------------

/// Coalescer event loop.
///
/// Receives `ProcessOutput` values from `rx`, coalesces them via
/// [`ProcessOutput::merge`], and emits events on a debounce timer.
///
/// Returns when `rx.recv()` yields `None` (sender dropped → upstream task
/// ended). Any remaining pending output is flushed before return.
///
/// **The function performs no source-specific termination work** (process
/// reaping, lifecycle mutation, exit notifications). The caller is responsible
/// for that — see module-level doc.
pub(crate) async fn run(
    coalescer: Coalescer,
    ctx: CoalescerContext,
    mut rx: mpsc::Receiver<ProcessOutput>,
) {
    let Coalescer {
        mut pending,
        mut current_debounce,
        mut was_in_drop_mode,
        mut last_emit_ms,
    } = coalescer;

    let sleep_fut = tokio::time::sleep(current_debounce);
    tokio::pin!(sleep_fut);

    loop {
        tokio::select! {
            // Receive a chunk from the upstream task.
            msg = rx.recv() => {
                match msg {
                    Some(output) => {
                        let flush_now = output.needs_immediate_flush;
                        pending.merge(output);
                        if flush_now {
                            // CPR/DSR response was sent — bypass debounce to update cursor
                            // state promptly. Tools like vim/neovim/fzf use CPR to sync
                            // their rendering and will stall until this event arrives.
                            // Drain any concurrently buffered output to avoid splitting the
                            // update across two events.
                            while let Ok(more) = rx.try_recv() {
                                pending.merge(more);
                            }
                            if !pending.is_empty() {
                                let outcome = emit_all_pending(
                                    &ctx.app,
                                    &ctx.pane_id,
                                    &ctx.vt,
                                    &ctx.registry,
                                    &mut pending,
                                );
                                current_debounce = next_debounce(outcome.duration, &ctx.config);
                                if outcome.emitted_screen_update {
                                    last_emit_ms = now_ms();
                                }
                            } else {
                                // Nothing to emit, but clear the flag to avoid stale hints.
                                pending.needs_immediate_flush = false;
                            }
                            // Re-arm sleep from now with updated period.
                            sleep_fut.as_mut().reset(tokio::time::Instant::now() + current_debounce);
                        }
                    }
                    None => {
                        // Channel closed — upstream task finished (EOF, error, or close).
                        // Flush any remaining pending output before exiting.
                        // EmitOutcome is discarded: this task is exiting and
                        // neither adaptive debounce nor last_emit_ms matters.
                        if !pending.is_empty() {
                            let _ = emit_all_pending(
                                &ctx.app,
                                &ctx.pane_id,
                                &ctx.vt,
                                &ctx.registry,
                                &mut pending,
                            );
                        }
                        break;
                    }
                }
            }

            // Adaptive debounce timer — flush accumulated output.
            _ = &mut sleep_fut => {
                // Drain any output buffered during this tick before emitting.
                // Prevents splitting application redraw bursts (e.g. CSI 2J + redraw)
                // across two separate screen-update events. try_recv() is non-blocking
                // and returns Err immediately when the channel is empty.
                while let Ok(output) = rx.try_recv() {
                    pending.merge(output);
                }

                // P-HT-6: frame-ack backpressure.
                // Single atomic load to avoid TOCTOU between ack_age and
                // has_unacked_emits checks.
                let last_ack_ms = ctx.last_frame_ack_ms.load(Ordering::Relaxed);
                let ack_age_ms = now_ms().saturating_sub(last_ack_ms);
                let has_unacked_emits = last_emit_ms > last_ack_ms;
                let in_drop_mode =
                    has_unacked_emits && ack_age_ms > ctx.config.ack_drop_threshold_ms;
                let in_stale_mode =
                    has_unacked_emits && ack_age_ms > ctx.config.ack_stale_threshold_ms;

                if !pending.is_empty() {
                    if in_drop_mode {
                        // Stage 2: suppress dirty cell updates + cursor_moved.
                        // Non-visual events preserved: mode_changed, new_cursor_shape,
                        // bell, osc52, new_title, new_cwd.
                        pending.dirty = DirtyRegion::default();
                        pending.needs_immediate_flush = false;
                    } else if was_in_drop_mode {
                        // Exiting drop mode: frontend grid is stale. Force full redraw.
                        pending.dirty.is_full_redraw = true;
                    }

                    if !pending.is_empty() {
                        let outcome = emit_all_pending(
                            &ctx.app,
                            &ctx.pane_id,
                            &ctx.vt,
                            &ctx.registry,
                            &mut pending,
                        );
                        // ADR-0027 Addendum 2: only advance `last_emit_ms`
                        // when a `screen-update` event was actually emitted.
                        // Non-visual events (bell, mode, cursor shape, OSC 52,
                        // title, CWD) produce no frontend render and therefore
                        // no frame-ack — advancing the timestamp for them
                        // would synthesize a phantom "unacked emit" that
                        // could push the pane into drop mode on the next
                        // idle tick past ACK_DROP_THRESHOLD_MS.
                        if outcome.emitted_screen_update {
                            last_emit_ms = now_ms();
                        }
                        current_debounce = if in_stale_mode {
                            ctx.config.ack_stale_debounce
                        } else {
                            next_debounce(outcome.duration, &ctx.config)
                        };
                    } else {
                        // All content was dirty-only and was dropped.
                        pending = ProcessOutput::default();
                    }
                    was_in_drop_mode = in_drop_mode;
                } else {
                    // Idle tick: exponential decay toward minimum.
                    // No stale escalation on idle ticks.
                    // Note: was_in_drop_mode is intentionally NOT updated here.
                    // If drop mode exits during an idle tick (ack arrives, no
                    // pending data), the transition must be preserved so that
                    // the next active tick sees was_in_drop_mode=true and forces
                    // a full redraw to repair the stale frontend grid.
                    current_debounce = ctx
                        .config
                        .debounce_min
                        .max(current_debounce.mul_f64(ctx.config.debounce_decay));
                }

                // Always re-arm after timer fires.
                sleep_fut.as_mut().reset(tokio::time::Instant::now() + current_debounce);
            }
        }
    }
}
