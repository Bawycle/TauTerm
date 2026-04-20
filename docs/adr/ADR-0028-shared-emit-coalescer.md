<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0028 — Shared `EmitCoalescer` for PTY and SSH panes

**Date:** 2026-04-16
**Status:** Accepted

## Context

The PTY pipeline runs a two-task design (ADR-0020, ADR-0027): Task 1
reads the PTY master via `spawn_blocking`, processes bytes through the
shared `VtProcessor`, and forwards the resulting `ProcessOutput` over a
bounded MPSC channel to Task 2. Task 2 coalesces multiple `ProcessOutput`
values via `ProcessOutput::merge`, applies the adaptive debounce
(P-HT-2), reads the per-pane `last_frame_ack_ms` atomic to escalate the
ADR-0027 two-stage backpressure, and emits `screen-update` plus the
non-visual events to the frontend.

The SSH pipeline (`session/ssh_task.rs`) runs a single async task that
reads `russh::ChannelMsg::Data` and `ExtendedData`, processes bytes
through `VtProcessor`, and emits `screen-update` immediately —
without coalescing, without adaptive debounce, and without reading
`last_frame_ack_ms`. ADR-0027 explicitly listed this as a known
limitation.

Three concrete defects accumulate from this divergence:

1. **OOM-class flooding under heavy SSH output.** A `cat large_file` on
   a fast SSH connection produces the same per-event firehose that
   commit `298d373` fixed for local PTY by introducing the bounded
   channel. The frontend has no rendering protection and the WebView
   event queue grows unbounded.
2. **Missing non-visual events.** `ssh_task.rs` extracts only
   `mode_changed` and `new_title` from the VT processor. Bell, OSC 52,
   cursor shape changes, OSC 7 CWD updates, and CPR/DSR responses are
   dropped on the floor for SSH panes. Each new emit-side feature
   added to PTY widens this silent gap.
3. **Code duplication.** `Data` and `ExtendedData` arms in
   `ssh_task.rs` re-implement the same VT extraction block (lines
   77–102 vs 107–133), twice diverged from PTY's reader.

A naive port of the Task 2 coalescer into `ssh_task.rs` would create a
third duplication and a maintenance forcing function: every future
ADR-0027-style addendum would need to be applied in three places.

## Decision

Extract Task 2 of the PTY pipeline into a shared module
`src-tauri/src/session/output/` that owns:

- `ProcessOutput` (moved from `session/pty_task.rs`)
- `output_emits_screen_update` and `EmitOutcome` (moved from
  `session/pty_task/emitter.rs`)
- `emit_all_pending` (moved from `session/pty_task/emitter.rs`)
- `build_screen_update_event`, `build_scrolled_viewport_event`,
  `build_mode_state_event` (moved from
  `session/pty_task/event_builders.rs`)
- A new `Coalescer` value type and a `pub(crate) async fn run(...)`
  containing the `select!` loop with the same adaptive-debounce and
  frame-ack escalation logic as the current PTY Task 2.

Both `pty_task` and `ssh_task` become consumers of this shared module.
PTY keeps its `spawn_blocking` reader (Task 1) feeding the channel.
SSH gains a 2-task structure: an async channel reader (no
`spawn_blocking` — `russh::Channel::wait()` is already async) feeding a
coalescer task identical in shape to PTY's Task 2.

Termination semantics stay caller-owned: the coalescer's `run`
function exits when its `Receiver<ProcessOutput>` is closed. The
caller awaits the `JoinHandle` and runs its source-specific
termination block — `mark_pane_terminated` + `ProcessExited`
notification for PTY; `SshLifecycleState::Closed` event for SSH.

## Consequences

**Positive:**

- SSH panes inherit the bounded-channel back-pressure that protected
  PTY panes against OOM-class flooding (commit `298d373`).
- SSH panes inherit ADR-0027 frame-ack two-stage escalation. The
  per-pane `Arc<AtomicU64> last_frame_ack_ms` already exists on every
  `PaneSession` and is wired to both source types via the same
  `CoalescerContext` field — no dispatcher in the `frame_ack` command,
  no PTY/SSH branch in the frontend.
- SSH panes emit bell, OSC 52, cursor shape, CWD, and CPR/DSR
  responses correctly. The previously dropped events stop being
  dropped.
- `Data` / `ExtendedData` duplication in `ssh_task.rs` collapses to a
  single private helper `extract_process_output(vt, bytes)`.
- Future emit-side changes (e.g. eventual per-pane render budgets,
  scroll-follow flags) are applied once in `session/output/` and
  benefit both pipelines automatically.
- Test gating logic (TEST-ACK-001 through TEST-ACK-020) becomes
  source-agnostic: a test asserting "non-visual events do not advance
  `last_emit_ms`" now covers both PTY and SSH with one test.
- The signature pin `_OUTPUT_EMITS_SIGNATURE_PIN` is preserved at its
  new location and continues to enforce the frontend/backend ack
  contract from ADR-0027 Addendum 2.

**Negative / risks:**

- One-shot churn: `session/output/` is created; `session/pty_task.rs`
  loses ~150 lines (Task 2, ProcessOutput, emitter, event_builders);
  `session/ssh_task.rs` is rewritten as a 2-task pipeline. The
  `pty_task` module retains only `spawn_pty_read_task` (Task 1) plus
  `PtyTaskHandle`. All `pub use` re-exports used by benchmarks
  (`build_screen_update_event`, `build_scrolled_viewport_event`) move
  to `session/output` — internal callers update their imports. No
  external API change crosses the IPC boundary.
- SSH gains one MPSC channel + one extra task per pane. Memory cost:
  ~256 × `sizeof::<ProcessOutput>()` ≈ a few KB per SSH pane.
- Lock-ordering rule for SSH responses (VT write-lock released before
  acquiring the SSH channel mutex) becomes load-bearing for
  correctness. Documented in the new `ssh_task.rs` and enforced by
  the code shape of `extract_process_output` (which drops the VT lock
  at the end of its tuple-return expression). `await_holding_lock`
  Clippy lint protects against accidental future regressions on the
  async side; the PTY-style structural pattern protects the
  `parking_lot::RwLock` side.
- Existing PTY tests (TEST-ACK-001 through TEST-ACK-020,
  TEST-ADPT-001 through TEST-ADPT-005, TEST-PIPC2-UNIT-001/002/003,
  TEST-SB-VIEWPORT-001 through 005) all live under
  `pty_task.rs`'s `mod tests`. They need to move to
  `session/output/` and the `super::reader::*` imports rewritten to
  `super::*`. Test IDs are preserved.

## Security Considerations

This ADR closes a known OOM-class flooding vector in the SSH read path.
Prior to ADR-0028, a malicious or compromised SSH server could spam
`screen-update` events at the rate of `cat /dev/urandom`, growing the
WebView event queue without bound. The shared coalescer applies the same
`mpsc::channel(256)` back-pressure and frame-ack escalation (ADR-0027)
to SSH that already protected local PTY panes after commit `298d373`.

Worst-case memory per SSH pane: ~1 MB (256 ProcessOutput entries × max
DirtyRegion size, accumulated `pending`).

VT input caps (defensive measures against SSH-untrusted byte vectors):
- `pending_responses` (DSR/CPR/DA): capped at 256 entries; oldest dropped
  with `tracing::warn!` on overflow. Closes DSR amplification (an
  unbounded `Vec<Vec<u8>>` was previously fed by adversarial server).
- OSC 52 string: capped at 1 MB; silently dropped above the cap (no event
  emitted). Threat model unchanged from PTY: gated by `allow_osc52_write`
  (default false).
- OSC 0/2 title: truncated at 4096 chars after C0/C1 strip; warn-log on
  truncation.

Lock ordering is load-bearing: VT write-lock MUST be released before
acquiring the SSH channel mutex. Enforced structurally by
`extract_process_output` returning by value (drops the parking_lot guard
at scope exit) and by test SEC-SSH-LOCK-NO-WRITE-WITH-CHANNEL-001.

DSR/CPR responses are merged into a single `ch.data()` call per drain
(merged buffer bounded ~8 KB by the 256-entry cap), eliminating SSH
channel mutex thrashing under DSR flood.

Logging rule: PTY/SSH raw bytes MUST NOT be logged at any tracing level —
they may contain user passwords typed into prompts. See
`src-tauri/CLAUDE.md`.

Inherited debt (out of scope, tracked separately in TODO.md):
- SSH channel mutex held during `Channel::wait()` blocks input/resize
  during long server idle.
- OSC 8 hyperlink URI not validated.
- OSC 7 cwd not sanitized for Unicode bidi (RTL override, homoglyphs).
- OSC 0/2 title not sanitized for Unicode bidi.
- Frame-ack spam from a compromised WebView (post-XSS) can disable
  Stage 1/Stage 2 escalation; the `mpsc(256)` cap remains as
  defense-in-depth.

## Performance Considerations

- ~1 MB worst-case channel buffer per SSH pane (256 `ProcessOutput`
  entries × max `DirtyRegion` size). Acceptable for the target
  workload (dozens of panes, not thousands).
- `CoalescerConfig` layout fits 2 cache lines (Copy-only types);
  no heap allocation in the hot path.
- `process_chunk` sync work is bounded by the `russh` receive window.
  If `tokio_console` shows a worker blocked > 100 µs on burst, wrap
  with `block_in_place` (deferred decision — Risk #9 in the
  implementation plan).

## Alternatives Considered

### Option A — `session/pty_task/coalescer.rs` (TODO's first sketch) — REJECTED

Putting the shared module under `pty_task/` keeps the file path tied to
a source name that is no longer accurate once SSH consumes it. Naming
debt — explicitly contrary to the "no conscious technical debt" beta
quality bar.

### Option B — `session/coalescer.rs` (single file, sibling to `pty_task` and `ssh_task`) — REJECTED

Lifts the coalescer name out of `pty_task/`, but leaves
`ProcessOutput`, `EmitOutcome`, `output_emits_screen_update`,
`emit_all_pending` and the event builders stranded inside
`pty_task/`. SSH would have to import from `pty_task` to access
non-PTY-specific types — same naming debt at finer granularity.

### Option C — sub-module `session/output/` — SELECTED

Groups every type and function whose responsibility is "turn
VtProcessor side-effects into IPC events" under one module,
irrespective of source. Aligns module names with responsibilities
(SoC). Absorbs all future emit-side changes without re-touching
`pty_task` or `ssh_task`.

### Termination — passing a `TerminationKind` enum to the coalescer — REJECTED

Would force `session/output/` to know `mark_pane_terminated`,
`SshLifecycleState::Closed`, and `PaneNotificationDto::ProcessExited`
— pulling SSH and registry-lifecycle types into a module whose entire
purpose is to be source-agnostic. Variant B (caller owns termination
after `JoinHandle.await`) keeps the coalescer pure.

### Coalescer behind an `Emitter` trait — REJECTED

The current testing strategy (mirror the gating predicate inline,
exercise `output_emits_screen_update` directly) already produces fully
deterministic, runtime-free tests for every gating decision. A trait
exists only to enable mocking that is not needed. YAGNI.

### Frame-ack dispatcher in the `frame_ack` command — REJECTED

`PaneSession.last_frame_ack_ms` already exists for every pane and
`registry.record_frame_ack` already dispatches by `pane_id`. The
SSH-side coalescer reads the same atomic via the same
`CoalescerContext` field. No dispatcher needed; the lookup is the
dispatcher.

## Related ADRs

- ADR-0001 — Tauri 2 as application framework (notes IPC back-pressure
  as a known performance risk)
- ADR-0007 — SSH via russh (establishes the async-only nature of
  `russh::Channel::wait()`)
- ADR-0019 — PTY session teardown strategy (PTY termination semantics
  this ADR consumes via `mark_pane_terminated`)
- ADR-0020 — Render coalescing strategy (the Task 1 / Task 2 split
  this ADR generalizes)
- ADR-0026 — IPC type codegen strategy (no IPC surface change in this
  ADR; mentioned because event types whose definitions move to
  `session/output/event_builders.rs` are already specta-derived and
  the codegen output is unchanged)
- ADR-0027 — Frame-ack backpressure for PTY output (the mechanism
  this ADR extends to SSH; all three addenda apply unchanged to the
  shared coalescer)
