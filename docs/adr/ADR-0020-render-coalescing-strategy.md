<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0020 — Render coalescing strategy

**Date:** 2026-04-10
**Status:** Accepted

## Context

PTY output can arrive at rates far exceeding the frontend's ability to render.
A naive pipeline — emit one `screen-update` Tauri event per `read()` call —
would flood the WebView event queue under sustained output (e.g.,
`cat /dev/urandom | head -c 50M`, or a `vim` redraw on window resize), causing
unbounded memory growth and increasing latency for subsequent user input
(`send_input` commands compete for the Tauri command queue).

The PTY read loop runs in `spawn_blocking` (Task 1) because `portable-pty`'s
master reader is a synchronous `Box<dyn Read + Send>` that blocks the OS thread.
Tokio's `spawn` cannot be used without wrapping the read in a busy-wait loop or
`AsyncFd`, both of which add complexity.  Task 2 (async Tokio) is responsible
for emitting Tauri events.

The architecture document (§6.2, §6.5) identified back-pressure as a known
performance risk.  The accepted solution is coalescing multiple reads into a
single `screen-update` event per render frame.

### Observed design

`src-tauri/src/session/pty_task/reader.rs` implements a two-task pipeline:

```
Task 1 (spawn_blocking):
  loop {
    let n = reader.lock().read(&mut buf);       // blocking OS read, 4096-byte chunk
    { let mut proc = vt.write();                // write-lock: brief (parse + side-effects)
      let output = proc.process(&buf[..n]);
    }                                           // lock released
    tx.blocking_send(output)                    // blocks if channel is full (back-pressure)
  }

Task 2 (async Tokio):
  loop {
    select! {
      msg = rx.recv()  => { pending.merge(msg) }
      _ = interval.tick() => {
        while let Ok(msg) = rx.try_recv() { pending.merge(msg) }
        if !pending.is_empty() { emit_all_pending(...); }
      }
    }
  }
```

Constants:
- Channel capacity: 256 slots (`tokio::sync::mpsc::channel::<ProcessOutput>(256)`).
- Debounce interval: `SCREEN_UPDATE_DEBOUNCE = Duration::from_millis(12)`.
- Read buffer: 4096 bytes per chunk.

`ProcessOutput::merge()` is a fold operation that unions dirty cell sets, OR-s
boolean flags (`mode_changed`, `bell`), and keeps the last non-None title/cursor
shape change.

The `interval` uses `MissedTickBehavior::Delay`: if emitting takes longer than
12 ms, the next tick is delayed rather than skipped, preventing bursting.

## Decision

Use a **two-task pipeline** with a bounded 256-slot MPSC channel and a 12 ms
debounce timer for coalescing `screen-update` events.

- Task 1 reads PTY bytes in 4096-byte chunks (blocking), processes each chunk
  through the `VtProcessor` (write-lock held only for the processing window),
  then sends a `ProcessOutput` value to the channel via `blocking_send`.  The
  write-lock is released before the send.
- Task 2 coalesces incoming `ProcessOutput` values via `merge()` on every
  channel receive, and drains any remaining values via non-blocking `try_recv()`
  on each 12 ms timer tick before emitting a single `screen-update` event.
- Channel saturation causes `blocking_send` in Task 1 to block the
  `spawn_blocking` OS thread.  This applies back-pressure to the PTY kernel
  buffer, slowing the overall pipeline without dropping data.

## Alternatives considered

**Single task (read + emit in one loop)**

Process bytes and emit in the same `spawn_blocking` task, with no intermediate
channel.

This eliminates the channel overhead but creates a deadlock risk: Tauri's
`AppHandle::emit()` internally acquires locks in the WebView event queue.  If
Task 1 holds the VT write-lock while calling `emit()`, and a concurrent Tauri
command handler (e.g., `get_pane_screen_snapshot`) is waiting on the read-lock
while `emit()` waits for the WebView queue, a deadlock is possible depending on
Tauri's internal threading model.  The two-task design keeps the VT lock and
the Tauri event emission in separate tasks, eliminating this risk.  Not chosen.

**Unbounded channel**

Replace the 256-slot bounded channel with an unbounded channel
(`tokio::sync::mpsc::unbounded_channel`).

This removes back-pressure entirely.  Under sustained burst output, Task 2 may
fall arbitrarily far behind Task 1, causing the channel queue to grow without
bound and eventually exhausting heap memory.  Not chosen.

**RequestAnimationFrame-only coalescing (frontend-side)**

Let the backend emit events at full rate and have the Svelte frontend coalesce
them in a `requestAnimationFrame` loop.

This does not reduce the number of IPC events crossing the Tauri boundary —
it only reduces DOM updates.  The Tauri event system still queues each event
in the WebView's JavaScript event queue, which can overflow or cause GC pressure
under burst conditions.  Back-pressure is not achievable from the frontend:
once an event is emitted by the backend, it has already consumed IPC queue
capacity.  Frontend-side coalescing is still applied (via `requestAnimationFrame`
batching in the renderer), but it is not a substitute for backend coalescing.
Not chosen as the primary strategy.

**Larger read buffer (> 4096 bytes)**

Increasing the read buffer to 16 384 or 65 536 bytes would process more bytes
per VT write-lock acquisition, reducing lock contention and context switches.

The tradeoff is increased latency for the first batch of bytes after a period
of silence: a 65 536-byte buffer may wait for the OS to fill it before
returning from `read()`, which is not how PTY reads work in practice (PTY reads
return as soon as any data is available), but larger buffers do increase the
size of each processing batch.  4096 bytes is a standard page size and a
well-established PTY read buffer size.  This is a tuning parameter; 4096 is
retained pending profiling data.  Not changed at this time.

## Rationale for 12 ms debounce

12 ms ≈ one frame at 60 Hz (16.7 ms per frame), leaving 4 ms of headroom for
event dispatch and DOM layout.  The worst-case keypress-to-echo latency under
this scheme is two debounce ticks (the keypress arrives just after a tick fires,
and the echo is emitted on the next tick): 2 × 12 ms = 24 ms, below the 25 ms
interactive threshold.

At 120 Hz (8.3 ms per frame), 12 ms spans ~1.4 frames.  A tighter interval
(e.g., 8 ms) would be more appropriate for 120 Hz displays but would increase
CPU overhead for high-throughput sessions on 60 Hz hardware.  12 ms is a
conservative default pending display-frequency-aware adaptation.

## Rationale for 256-slot channel capacity

256 slots × sizeof(ProcessOutput) ≈ 256 × ~200 bytes (estimated) ≈ 50 KB
maximum channel queue size.  This bounds memory consumption under burst
conditions while providing enough slack that Task 2 can fall up to 256 ticks
behind Task 1 before back-pressure engages.  At 12 ms per tick, 256 slots
represents ~3 seconds of burst budget at one `ProcessOutput` per tick — in
practice, bursts are much shorter.

## Consequences

**Positive:**
- Bounded memory: the channel queue cannot grow beyond 256 × sizeof(ProcessOutput).
- Back-pressure is automatic: Task 1 stalls on `blocking_send` when Task 2 is
  slow, propagating pressure to the kernel PTY buffer.  No data is dropped.
- The VT write-lock and Tauri event emission are fully decoupled across tasks,
  eliminating the deadlock risk identified in the single-task alternative.
- The drain loop (`try_recv` on tick) prevents application redraws that arrive
  in bursts (e.g., `CSI 2J` + full screen repaint) from being split across two
  `screen-update` events, reducing visual flicker.

**Negative / risks:**
- 12 ms added latency on the echo path in the worst case.  This is imperceptible
  for interactive typing but measurable in automated tests that send input and
  immediately read the screen snapshot.
- Task 1 runs on Tokio's blocking thread pool.  Each active pane occupies one
  OS thread for the duration of the session.  With many panes open (e.g., 20),
  this adds 20 OS threads.  Tokio's default blocking thread pool limit is 512;
  this is not expected to be a constraint in v1.
- `MissedTickBehavior::Delay` means that if emitting is slow (e.g., the WebView
  is busy), the effective emit rate drops below 1 event per 12 ms.  This is the
  correct behavior under load but means latency is not strictly bounded at 24 ms
  when the system is saturated.
