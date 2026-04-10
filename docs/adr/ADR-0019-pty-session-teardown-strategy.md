<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0019 — PTY session teardown strategy

**Date:** 2026-04-10
**Status:** Accepted

## Context

When the user closes a terminal pane (or the last pane of a tab), TauTerm must
tear down the associated PTY session cleanly. "Clean teardown" means:

1. The foreground process group receives SIGHUP so that it can exit or clean up
   (FS-PTY-007).
2. The PTY read task stops — no subsequent reads on a dead fd.
3. No resources (fd, memory, Tokio task handles) leak after the session is gone.

The implementation involves three actors:

- `LinuxPtySession` (`src-tauri/src/platform/pty_linux/session.rs`) — holds an
  `Arc<Mutex<MasterPty>>` (the `portable-pty` master handle) and an
  `Arc<Mutex<Child>>`.  It implements `PtySession::close(self: Box<Self>)`.
- `PtyTaskHandle` (`src-tauri/src/session/pty_task/`) — holds Tokio
  `AbortHandle`s for Task 1 (blocking PTY reader) and Task 2 (async coalescer).
  Dropped when the `PaneSession` is removed from the `SessionRegistry`.
- Task 1 (blocking, `spawn_blocking`) — loops on `reader.lock().read()`.  Exits
  naturally when `read()` returns `Err` (EIO) or `Ok(0)` (EOF).

The kernel delivers SIGHUP to the foreground process group of a PTY when the
master side of the PTY is closed.  The triggering condition is the last file
descriptor referencing the master side reaching refcount zero.

### Observed design

`LinuxPtySession::close()` is a one-line body with an empty block comment:

```rust
fn close(self: Box<Self>) {
    // Drop self — Arc refcounts reach zero, master fd is dropped, kernel
    // delivers SIGHUP to the foreground process group (FS-PTY-007).
    // portable-pty's MasterPty Drop impl closes the underlying fd.
}
```

Dropping `self` (a `Box<LinuxPtySession>`) releases the `Arc<Mutex<MasterPty>>`
held inside the struct.  If no other `Arc` clones of the master exist — which is
guaranteed by design: only `LinuxPtySession` holds it — the refcount reaches
zero and `MasterPty::drop()` is called by `portable-pty`.  The `portable-pty`
crate closes the underlying file descriptor in its `Drop` implementation.  The
kernel then delivers SIGHUP to the foreground process group.

Task 1 holds a separate `Arc<Mutex<Box<dyn Read + Send>>>` (the reader side,
also from `portable-pty`).  After the master fd is closed, Task 1's next
`read()` call returns `Err(EIO)`, causing it to break its loop naturally,
drop the channel sender, and signal Task 2 to flush and exit.

## Decision

Use the **drop-cascade** approach: `PtySession::close()` does nothing except let
`self` go out of scope.  Ownership rules propagate the close through the
`Arc<Mutex<MasterPty>>` refcount → `MasterPty::drop()` → fd close → kernel
SIGHUP delivery.

No explicit coordination is performed between `close()` and the two PTY tasks
before the fd is closed.  The tasks observe the fd close as a natural EOF/EIO
signal and exit on their own.

## Alternatives considered

**Explicit ordered shutdown (coordinate tasks before closing the fd)**

Send a cancellation signal to Task 1, wait for it to drain its current read and
send the final `ProcessOutput` to the channel, then wait for Task 2 to flush and
exit, and only then close the master fd.

This approach guarantees that all output produced before the close command is
visible to the user before the session ends.  However, it requires:

- A cancellation channel or `CancellationToken` (one per pane).
- `close()` to become `async` or to block on a `JoinHandle`, which conflicts
  with Tauri's command handler execution model and complicates the `PtySession`
  trait signature.
- Careful handling of the case where Task 1 is blocked in a `read()` that will
  never return (e.g., a long-lived idle process) — a timeout or second signal
  would be required.

The added complexity is not justified for the v1 use case.  The window between
the close command and the last byte of output being rendered is typically < 12 ms
(one debounce tick), which is imperceptible.  Not chosen.

**Explicit `close(fd)` syscall in `PtySession::drop()` before dropping the `portable-pty` handle**

Call `libc::close(master_fd)` explicitly in the `Drop` implementation of
`LinuxPtySession`, before the `Arc<Mutex<MasterPty>>` is released.

This would make the fd-close guarantee independent of `portable-pty`'s `Drop`
implementation, providing defense in depth.  However, it introduces a
double-close risk: if `portable-pty` also closes the fd in its `Drop`, the same
fd number may have been reused by the OS, and the second `close()` would close an
unrelated fd (a silent, hard-to-debug bug).  The `Arc<Mutex<MasterPty>>` clone
held by Task 1's reader further complicates ownership: the fd may still be
referenced after `LinuxPtySession` is dropped if Task 1 holds the last `Arc`.

Not chosen without verified `portable-pty` behavior (see Debt).

## Consequences

**Positive:**
- Drop-cascade maps directly to Rust's ownership model — no extra mechanism is
  needed.  The teardown path has zero runtime overhead beyond the normal `Arc`
  decrement.
- Task 1 and Task 2 exit on their own schedule after observing EIO/EOF.  No
  race conditions are introduced: both tasks hold their own `Arc` references and
  will not access freed memory.
- The `PtySession` trait interface remains synchronous and simple.

**Negative / risks:**
- No flush guarantee before teardown.  Output that was written to the PTY master
  but not yet read by Task 1 (buffered in the kernel PTY line discipline) will be
  lost if the master fd is closed before Task 1 reads it.  In practice, the PTY
  ring buffer drains faster than human perception, but a rapidly-exiting process
  that writes to stdout just before exit may lose its last lines.
- The timing of SIGHUP delivery relative to Task 1's last read is
  non-deterministic: the kernel may buffer some output after the master fd is
  closed, which Task 1 will read via the slave-side reader `Arc`.
- Dependency on `portable-pty`'s `Drop` behavior: if `portable-pty` does not
  close the underlying fd on `MasterPty::drop()`, SIGHUP will never be delivered
  and the foreground process group will not be notified.  This is an external
  dependency that has not been formally verified against the `portable-pty` source.

## Debt

The comment in `LinuxPtySession::close()` reads: "portable-pty's MasterPty Drop
impl closes the underlying fd."  This is an assumption, not a verified fact.  A
future task must audit the `portable-pty` source (specifically `MasterPty::drop`
in `src/unix.rs` or equivalent) to confirm that the underlying fd is closed —
not merely that the Rust wrapper is dropped while the fd remains open.  If the
audit shows that `portable-pty` does not close the fd, an explicit `libc::close`
call must be added to `LinuxPtySession::drop()` with careful handling of the
double-close risk.
