// SPDX-License-Identifier: MPL-2.0

//! Source-agnostic emit pipeline shared by PTY and SSH read tasks.
//!
//! Contains:
//!
//! - [`ProcessOutput`] — the chunk-of-output value type produced by VT processing.
//! - [`Coalescer`] / [`CoalescerConfig`] / [`CoalescerContext`] — the async
//!   coalescer + adaptive-debounce + frame-ack backpressure machinery (formerly
//!   "Task 2" of the PTY pipeline).
//! - [`run`] — the coalescer entry point: consumes a `mpsc::Receiver<ProcessOutput>`
//!   and emits `screen-update` (and other) events on a debounce timer.
//! - [`build_screen_update_event`] / [`build_scrolled_viewport_event`] /
//!   [`build_mode_state_event`] — DTO constructors used by both the coalescer
//!   and the input commands.
//!
//! ## Caller-managed termination
//!
//! [`run`] returns when its `Receiver<ProcessOutput>` returns `None`. It performs
//! NO source-specific termination work (process reaping, lifecycle mutation,
//! exit notifications) — that responsibility lies with the caller (PTY or SSH)
//! AFTER awaiting the coalescer's `JoinHandle`. This keeps the coalescer fully
//! source-agnostic.
//!
//! See ADR-0027 (frame-ack backpressure for PTY output) and ADR-0028 (shared
//! emit coalescer) for design rationale.

mod coalescer;
mod emitter;
mod event_builders;
mod process_output;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Re-exports — public coalescer API
// ---------------------------------------------------------------------------

pub(crate) use coalescer::{Coalescer, CoalescerConfig, CoalescerContext, run};
pub(crate) use process_output::ProcessOutput;

// `emit_all_pending` is intentionally NOT re-exported: post-ADR-0028
// Commit 3, both PTY and SSH pipelines feed the coalescer (`run`), which
// owns the only call site. The function is reachable as
// `super::emitter::emit_all_pending` from inside `coalescer.rs`.

// ---------------------------------------------------------------------------
// Re-exports — event builders
// ---------------------------------------------------------------------------

// `build_mode_state_event` is intentionally NOT re-exported: its only call
// site is inside `emitter::emit_all_pending`, which imports it directly via
// `super::event_builders::`. Keeping it module-private avoids cluttering
// the `session::output` surface (ADR-0028 Decisions §1).

// Exposed for benchmarks (criterion benches) only — not part of the stable
// public API. `#[doc(hidden)] pub` qualifiers preserved on the function
// definitions themselves.
pub use event_builders::{build_screen_update_event, build_scrolled_viewport_event};
