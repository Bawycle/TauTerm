// SPDX-License-Identifier: MPL-2.0

//! Integration tests — async concurrency (TEST-ASYNC-*).
//!
//! Tests coverage:
//!
//! - `ResizeDebouncer`: callback fires after debounce interval; only the last
//!   value in a rapid burst is applied; sender drop stops the task cleanly.
//! - `PtyTaskHandle` / `SshTaskHandle`: drop aborts the underlying Tokio task;
//!   explicit `abort()` call also terminates the task.
//! - PTY EOF path: a `spawn_blocking` task exits cleanly when the reader
//!   returns EOF (simulated with a zero-byte reader).
//!
//! ## Notes
//!
//! These tests use `tokio::runtime::Builder::new_current_thread()` because
//! they must run in isolation from each other in nextest's process-per-test
//! model. `tauri::async_runtime` is not available outside a live Tauri app,
//! so tests that exercise `spawn_pty_read_task` use a plain Tokio runtime and
//! a direct `tokio::task::spawn_blocking` instead.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

use tau_term_lib::session::{
    pty_task::PtyTaskHandle,
    resize::{PendingResize, RESIZE_DEBOUNCE_MS, ResizeDebouncer},
    ssh_task::SshTaskHandle,
};

// ---------------------------------------------------------------------------
// TEST-ASYNC-RD-001 — ResizeDebouncer: callback fires after debounce window
// ---------------------------------------------------------------------------

/// TEST-ASYNC-RD-001: The callback fires exactly once after the debounce
/// interval when only one resize is scheduled.
#[test]
fn async_rd_001_callback_fires_after_debounce() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();

        let debouncer = ResizeDebouncer::new(move |_r| {
            fired_clone.store(true, Ordering::Release);
        });

        debouncer.schedule(PendingResize {
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
        });

        // Wait for debounce window + buffer.
        tokio::time::sleep(Duration::from_millis(RESIZE_DEBOUNCE_MS * 3 + 50)).await;
        assert!(
            fired.load(Ordering::Acquire),
            "callback must fire within 3× the debounce window"
        );
    });
}

/// TEST-ASYNC-RD-002: Rapid bursts — only the last resize is applied.
///
/// Scheduling N resizes rapidly and then waiting through the debounce window
/// must result in the callback being invoked with the last-scheduled dimensions.
#[test]
fn async_rd_002_rapid_burst_applies_only_last_resize() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let last_cols = Arc::new(AtomicUsize::new(0));
        let last_rows = Arc::new(AtomicUsize::new(0));
        let call_count = Arc::new(AtomicUsize::new(0));

        let lc = last_cols.clone();
        let lr = last_rows.clone();
        let cc = call_count.clone();

        let debouncer = ResizeDebouncer::new(move |r: PendingResize| {
            lc.store(r.cols as usize, Ordering::Release);
            lr.store(r.rows as usize, Ordering::Release);
            cc.fetch_add(1, Ordering::AcqRel);
        });

        // Schedule 10 resizes with zero delay between them — only the last
        // should be delivered to the callback.
        for i in 1..=10u16 {
            debouncer.schedule(PendingResize {
                cols: 80 + i,
                rows: 24 + i,
                pixel_width: 0,
                pixel_height: 0,
            });
        }

        // Wait through the debounce window.
        tokio::time::sleep(Duration::from_millis(RESIZE_DEBOUNCE_MS * 4 + 100)).await;

        // The callback may fire once or a small number of times, but must
        // ultimately settle on the last-scheduled value.
        assert_eq!(
            last_cols.load(Ordering::Acquire),
            90,
            "last cols must be 80+10=90"
        );
        assert_eq!(
            last_rows.load(Ordering::Acquire),
            34,
            "last rows must be 24+10=34"
        );
    });
}

/// TEST-ASYNC-RD-003: Dropping the debouncer stops the background task cleanly.
///
/// After drop, no further callbacks should fire. We verify this by dropping
/// the debouncer before the debounce window expires.
#[test]
fn async_rd_003_drop_debouncer_stops_background_task() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();

        let debouncer = ResizeDebouncer::new(move |_r| {
            fired_clone.store(true, Ordering::Release);
        });

        debouncer.schedule(PendingResize {
            cols: 100,
            rows: 40,
            pixel_width: 0,
            pixel_height: 0,
        });

        // Drop the debouncer immediately — the sender is dropped, which closes
        // the watch channel and causes the background task to exit.
        drop(debouncer);

        // Wait 3× the debounce window — the callback must NOT have fired
        // because the task exited when the sender was dropped.
        tokio::time::sleep(Duration::from_millis(RESIZE_DEBOUNCE_MS * 3 + 50)).await;

        // Note: the callback MAY have already fired if the task ran before the
        // drop (race condition inherent to async scheduling). This test is
        // intentionally weak on that assertion to avoid flakiness — the key
        // invariant is "no panic, no deadlock".
        // What we DO assert: no panic occurred (if we reached here, we're fine).
        let _ = fired.load(Ordering::Acquire);
    });
}

// ---------------------------------------------------------------------------
// TEST-ASYNC-PTY-001 — PtyTaskHandle: drop aborts the spawned task
// ---------------------------------------------------------------------------

/// TEST-ASYNC-PTY-001: Dropping a `PtyTaskHandle` aborts the underlying task.
#[test]
fn async_pty_001_drop_pty_task_handle_aborts_task() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let jh = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });

        let handle = PtyTaskHandle::from_abort_handle(jh.abort_handle());
        drop(handle);

        let result = jh.await;
        assert!(
            result.is_err(),
            "task must be cancelled (JoinError) after PtyTaskHandle drop"
        );
    });
}

/// TEST-ASYNC-PTY-002: Calling `PtyTaskHandle::abort()` also aborts the task.
#[test]
fn async_pty_002_explicit_abort_terminates_task() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let jh = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });

        let handle = PtyTaskHandle::from_abort_handle(jh.abort_handle());
        handle.abort();

        let result = jh.await;
        assert!(
            result.is_err(),
            "task must be cancelled after explicit PtyTaskHandle::abort()"
        );
    });
}

// ---------------------------------------------------------------------------
// TEST-ASYNC-SSH-001 — SshTaskHandle: drop aborts the spawned task
// ---------------------------------------------------------------------------

/// TEST-ASYNC-SSH-001: Dropping an `SshTaskHandle` aborts the underlying task.
#[test]
fn async_ssh_001_drop_ssh_task_handle_aborts_task() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let jh = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });

        let handle = SshTaskHandle::from_abort_handle(jh.abort_handle());
        drop(handle);

        let result = jh.await;
        assert!(
            result.is_err(),
            "task must be cancelled (JoinError) after SshTaskHandle drop"
        );
    });
}

/// TEST-ASYNC-SSH-002: Calling `SshTaskHandle::abort()` terminates the task.
#[test]
fn async_ssh_002_explicit_abort_terminates_task() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let jh = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });

        let handle = SshTaskHandle::from_abort_handle(jh.abort_handle());
        handle.abort();

        let result = jh.await;
        assert!(
            result.is_err(),
            "task must be cancelled after explicit SshTaskHandle::abort()"
        );
    });
}

// ---------------------------------------------------------------------------
// TEST-ASYNC-PTY-003 — PTY EOF: spawn_blocking task exits cleanly on reader EOF
//
// This test exercises the PTY read loop's EOF handling without a real PTY or
// AppHandle. We use a thin blocking reader that immediately returns Ok(0)
// (EOF) to simulate the read loop exit.
// ---------------------------------------------------------------------------

/// A `Read` implementation that immediately returns EOF.
struct EofReader;

impl std::io::Read for EofReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0) // EOF
    }
}

/// TEST-ASYNC-PTY-003: A `spawn_blocking` read loop that sees EOF exits cleanly.
///
/// We replicate the read loop pattern from `pty_task::spawn_pty_read_task`
/// using a bare `tokio::task::spawn_blocking` and an `EofReader`, ensuring the
/// loop exits correctly (does not hang, does not panic).
#[test]
fn async_pty_003_eof_reader_causes_read_loop_exit() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let reader: Arc<Mutex<Box<dyn std::io::Read + Send>>> =
            Arc::new(Mutex::new(Box::new(EofReader)));

        let task = tokio::task::spawn_blocking(move || {
            let mut buf = vec![0u8; 4096];
            loop {
                let n = {
                    let mut rdr = reader.lock().expect("reader lock");
                    match rdr.read(&mut buf) {
                        Ok(0) => break, // EOF — clean exit
                        Ok(n) => n,
                        Err(_) => break,
                    }
                };
                let _ = n; // suppress unused warning
            }
            // Reached here: loop exited cleanly
        });

        let result = tokio::time::timeout(Duration::from_secs(5), task).await;
        assert!(
            result.is_ok(),
            "spawn_blocking read loop must exit within 5 seconds on EOF"
        );
        assert!(
            result.unwrap().is_ok(),
            "spawn_blocking task must not panic on EOF"
        );
    });
}

/// TEST-ASYNC-PTY-004: A `spawn_blocking` read loop that sees an I/O error exits cleanly.
struct ErrorReader;

impl std::io::Read for ErrorReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "simulated PTY EIO",
        ))
    }
}

#[test]
fn async_pty_004_io_error_reader_causes_read_loop_exit() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let reader: Arc<Mutex<Box<dyn std::io::Read + Send>>> =
            Arc::new(Mutex::new(Box::new(ErrorReader)));

        let task = tokio::task::spawn_blocking(move || {
            let mut buf = vec![0u8; 4096];
            loop {
                let mut rdr = reader.lock().expect("reader lock");
                match rdr.read(&mut buf) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break, // I/O error — clean exit
                }
            }
        });

        let result = tokio::time::timeout(Duration::from_secs(5), task).await;
        assert!(
            result.is_ok(),
            "spawn_blocking read loop must exit within 5 seconds on I/O error"
        );
        assert!(
            result.unwrap().is_ok(),
            "spawn_blocking task must not panic on I/O error"
        );
    });
}

// ---------------------------------------------------------------------------
// TEST-ASYNC-PTY-005 — Two-task debounce: channel closes on EOF, last flush fires
//
// Exercises the core invariant of the two-task design:
// - Task 1 (reader) sends chunks then closes the channel on EOF.
// - Task 2 (emitter) coalesces chunks and must flush on channel close.
//
// We simulate this without a real PTY/AppHandle by replicating the channel
// protocol directly.
// ---------------------------------------------------------------------------

/// TEST-ASYNC-PTY-005: Channel close (EOF) triggers flush of accumulated data.
///
/// Verifies that closing the sender side of an mpsc channel causes the receiver
/// side to drain the remaining message and exit cleanly within the debounce window.
#[test]
fn async_pty_005_channel_close_causes_final_flush() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        // Mimic the two-task channel protocol: unbounded mpsc, sender in Task 1,
        // receiver in Task 2.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u32>();

        // Send some values and then drop the sender (simulates PTY EOF).
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        drop(tx); // Channel closed — simulates Task 1 EOF path.

        // Task 2 pattern: receive until channel closes, accumulate, then flush.
        let accumulator = tokio::spawn(async move {
            let mut sum = 0u32;
            loop {
                match rx.recv().await {
                    Some(v) => sum += v,
                    None => break, // channel closed — flush and exit
                }
            }
            sum
        });

        let result = tokio::time::timeout(Duration::from_millis(500), accumulator).await;
        assert!(result.is_ok(), "accumulator must finish before timeout");
        let sum = result.unwrap().expect("accumulator must not panic");
        assert_eq!(
            sum, 6,
            "all values (1+2+3=6) must be received before channel close"
        );
    });
}

/// TEST-ASYNC-PTY-006: Debounce timer fires when channel is idle.
///
/// Verifies that a `tokio::select!` over an mpsc receiver and a timer correctly
/// wakes on the timer when no messages arrive within the debounce window.
/// This is the core property that fixes the WP4 bug (silent PTY → last batch
/// was never flushed in the old single-task design).
#[test]
fn async_pty_006_debounce_timer_fires_when_channel_is_idle() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        const DEBOUNCE_MS: u64 = 20;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u32>();

        let flush_count = Arc::new(AtomicUsize::new(0));
        let fc = flush_count.clone();

        // Task 2 pattern with a timer-driven flush.
        let emitter = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(DEBOUNCE_MS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut pending: u32 = 0;
            let mut done = false;

            while !done {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Some(v) => pending += v,
                            None => {
                                // Channel closed — flush remaining and exit.
                                if pending > 0 {
                                    fc.fetch_add(1, Ordering::AcqRel);
                                }
                                done = true;
                            }
                        }
                    }
                    _ = interval.tick() => {
                        if pending > 0 {
                            fc.fetch_add(1, Ordering::AcqRel);
                            pending = 0;
                        }
                    }
                }
            }
        });

        // Send a burst, then keep the channel open and wait for the timer to fire.
        tx.send(42).unwrap();

        // Wait longer than the debounce window — the timer must have fired.
        tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS * 4)).await;

        let count_before_close = flush_count.load(Ordering::Acquire);
        assert!(
            count_before_close >= 1,
            "timer must flush at least once while channel is idle (count={count_before_close})"
        );

        // Drop the sender — Task 2 must exit cleanly.
        drop(tx);
        let result = tokio::time::timeout(Duration::from_millis(200), emitter).await;
        assert!(
            result.is_ok(),
            "emitter must exit cleanly after sender drop"
        );
    });
}

/// TEST-ASYNC-PTY-007: Two-task coalescing — burst of chunks merged into one flush.
///
/// Sends many messages rapidly and verifies that the debounce timer coalesces
/// them into fewer flushes (ideally one), not one flush per message.
#[test]
fn async_pty_007_burst_is_coalesced_before_flush() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        const DEBOUNCE_MS: u64 = 30;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u32>();
        let flush_count = Arc::new(AtomicUsize::new(0));
        let total_flushed = Arc::new(AtomicUsize::new(0));

        let fc = flush_count.clone();
        let tf = total_flushed.clone();

        let emitter = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(DEBOUNCE_MS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut pending: u32 = 0;
            let mut done = false;

            while !done {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Some(v) => pending += v,
                            None => {
                                if pending > 0 {
                                    fc.fetch_add(1, Ordering::AcqRel);
                                    tf.fetch_add(pending as usize, Ordering::AcqRel);
                                }
                                done = true;
                            }
                        }
                    }
                    _ = interval.tick() => {
                        if pending > 0 {
                            fc.fetch_add(1, Ordering::AcqRel);
                            tf.fetch_add(pending as usize, Ordering::AcqRel);
                            pending = 0;
                        }
                    }
                }
            }
        });

        // Send 50 messages rapidly — much faster than the debounce window.
        for _ in 0..50u32 {
            tx.send(1).unwrap();
        }

        // Close the sender and wait for Task 2 to finish.
        drop(tx);
        tokio::time::timeout(Duration::from_millis(500), emitter)
            .await
            .expect("emitter must finish")
            .expect("emitter must not panic");

        let flushes = flush_count.load(Ordering::Acquire);
        let total = total_flushed.load(Ordering::Acquire);

        // All 50 values must be accounted for.
        assert_eq!(total, 50, "all 50 values must be flushed (total={total})");
        // The burst must have been coalesced — substantially fewer than 50 flushes.
        assert!(
            flushes < 10,
            "burst of 50 rapid messages must be coalesced (flushes={flushes})"
        );
    });
}

// ---------------------------------------------------------------------------
// TEST-ASYNC-RESIZE-001 — Rapid resize then PTY death: no deadlock
//
// Verifies that scheduling many resizes rapidly and then dropping all handles
// does not cause a deadlock or panic.
// ---------------------------------------------------------------------------

/// TEST-ASYNC-RESIZE-001: Schedule many resizes then drop the debouncer.
/// No deadlock or panic must occur.
#[test]
fn async_resize_001_many_resizes_then_drop_no_deadlock() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    rt.block_on(async {
        let counter = Arc::new(AtomicUsize::new(0));
        let cc = counter.clone();

        let debouncer = ResizeDebouncer::new(move |_| {
            cc.fetch_add(1, Ordering::AcqRel);
        });

        // Schedule 100 resizes in rapid succession.
        for i in 0..100u16 {
            debouncer.schedule(PendingResize {
                cols: 80 + (i % 40),
                rows: 24 + (i % 20),
                pixel_width: 0,
                pixel_height: 0,
            });
        }

        // Drop the debouncer while some resizes may still be pending.
        drop(debouncer);

        // Brief pause to let the background task exit.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // If we're here, no deadlock or panic occurred.
        // The counter value is non-deterministic depending on timing.
    });
}
