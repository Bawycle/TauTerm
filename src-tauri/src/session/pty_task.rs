// SPDX-License-Identifier: MPL-2.0

//! PTY blocking reader feeding the shared coalescer (Task 1 only).
//!
//! ## Architecture (post-ADR-0028)
//!
//! The shared async coalescer (`session/output/coalescer.rs`, formerly
//! "Task 2") is reused by both PTY and SSH pipelines. This module now owns
//! only the PTY-specific Task 1 (blocking reader on the master fd) plus the
//! caller-side termination block (process reaping + `ProcessExited`
//! notification) that runs after the coalescer's `JoinHandle` resolves.
//!
//! ### Task 1 — reader (`spawn_blocking`)
//!
//! Reads raw bytes from the PTY master, feeds them to `VtProcessor`, writes
//! any DSR/DA/CPR responses back to the PTY (after releasing the VT
//! write-lock), and forwards the resulting [`ProcessOutput`] through a
//! bounded channel (capacity 256) to the coalescer. Reaching EOF closes
//! the channel naturally.
//!
//! ### Coalescer — async, shared
//!
//! Lives in `crate::session::output`. Spawned via
//! [`tauri::async_runtime::spawn`] (NOT [`tokio::spawn`]) so that current
//! ordering invariants relied upon by TEST-ACK-018/019 and DEL-ASYNC-PTY-009
//! continue to hold (ADR-0028 Decisions §2, Risk #13).
//!
//! Back-pressure: dirty regions are coalesced over `DEBOUNCE_*` before
//! emitting a single `screen-update` event. This prevents flooding the
//! frontend when high-volume apps (`yes`, `seq`) write faster than the
//! frontend can consume events.

mod reader;

pub use reader::spawn_pty_read_task;

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
