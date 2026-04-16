<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0027 — Frame-ack backpressure for PTY output

**Date:** 2026-04-15
**Status:** Accepted

## Context

The render coalescing pipeline (ADR-0020) provides one-sided back-pressure:
the bounded MPSC channel between Task 1 (PTY reader) and Task 2 (async
coalescer) stalls the read loop when the channel fills up. However, the
backend has no visibility into whether the frontend has actually rendered the
most recent `screen-update` event. The adaptive debounce in Task 2 uses emit
wall-clock duration as a proxy for frontend load — but this is an
approximation. If the WebView event queue grows faster than the frontend can
paint (e.g., sustained `cat /dev/urandom`, WebKitGTK compositor stalls during
tab switch, or garbage collection pauses), the terminal freezes while the
backend continues emitting events the frontend cannot process.

P-HT-2 (adaptive debounce) adjusts the timer interval between 12 ms and
100 ms based on how long `emit()` takes, but it cannot distinguish "emit was
fast because the WebView queue accepted the event" from "the frontend actually
rendered it." The missing signal is an explicit acknowledgement from the
frontend after each paint.

## Decision

Adopt a per-pane `Arc<AtomicU64>` frame-ack mechanism with two-stage
escalation.

The frontend calls a new `frame_ack` IPC command after each
`flushRafQueue()` paint cycle. The backend's Task 2 reads the stored
timestamp on each timer tick to determine how stale the last ack is, and
escalates its coalescing strategy accordingly.

## Options evaluated

### Option A — `Arc<AtomicU64>` advisory timestamp — SELECTED

Each `PaneSession` holds an `Arc<AtomicU64>` storing the last ack timestamp
(milliseconds since epoch). The `frame_ack` command handler writes the
current timestamp with `Relaxed` ordering. Task 2 reads the timestamp with
`Relaxed` ordering in the timer arm, computes the ack age, and applies the
escalation policy.

**Pros:**

- No additional wakeup mechanism — Task 2 already wakes on the timer tick
  and the channel receive. The ack timestamp is an advisory read, not a new
  `select!` branch.
- Matches the existing `write_epoch` pattern used in `PreferencesStore`
  (§7.6.1 of the architecture): an `AtomicU64` counter read on a periodic
  check, with no cross-task synchronization beyond the atomic.
- Minimal structural change to the coalescer: the timer arm gains a
  conditional branch, not a new select arm.
- `Relaxed` ordering is sufficient: the timestamp is advisory. A stale read
  (one tick behind) causes at most one extra debounce cycle at the higher
  interval — no correctness issue.

**Cons:**

- Per-pane `Arc<AtomicU64>` adds one 8-byte allocation per pane session. Negligible.
- `SystemTime` is not monotonic — backward NTP jumps could cause a spurious
  stale-ack detection. Mitigated by using `saturating_sub` on the duration
  computation (see Design Details).

### Option B — `tokio::sync::watch` channel — REJECTED

A `watch` channel per pane, where the frontend ack sends a unit value and
Task 2 subscribes.

Rejected because it adds a third `select!` branch to Task 2 (alongside the
MPSC channel and the interval timer), changing the coalescer structure. The
watch receiver's `changed()` method requires `&mut self`, which complicates
borrow patterns in the existing `select!` macro. The advisory atomic read is
simpler and achieves the same goal — Task 2 does not need to wake on ack
arrival; it checks ack staleness on the timer tick it was already going to
take.

### Option C — New MPSC channel (frontend → backend ack stream) — REJECTED

A dedicated bounded MPSC channel per pane for ack messages, with Task 2
receiving on it as a third `select!` branch.

Rejected for the same structural reason as Option B (third `select!` branch).
Additionally, an MPSC channel allocates a fixed-capacity buffer (even if
only one value is ever in-flight), which is wasteful for a single-timestamp
signal. The atomic is the right primitive for "latest value wins" semantics.

## Design details

### Constants

| Constant | Value | Rationale |
|----------|-------|-----------|
| `ACK_STALE` | 200 ms | 2x margin over observed WebKitGTK compositor stalls (~100 ms). Below this threshold, the frontend is keeping up — no intervention needed. |
| `ACK_STALE_DEBOUNCE` | 250 ms | Debounce interval applied when ack age exceeds `ACK_STALE`. Significantly slower than the normal adaptive range (12–100 ms), giving the frontend time to catch up without dropping data. |
| `ACK_DROP` | 1000 ms | Threshold for entering drop mode. Above observed WebKitGTK tab-switch rAF blocking (200–500 ms). If the frontend has not painted for 1 second, it is likely invisible or severely stalled — continuing to emit cell diffs wastes IPC bandwidth and WebView queue capacity. |

### Two-stage escalation

**Stage 1 (ack age > ACK_STALE, ≤ ACK_DROP):** the debounce interval is
escalated from its current adaptive value (12–100 ms) to `ACK_STALE_DEBOUNCE`
(250 ms). All events are still emitted — no data is lost. The effect is a
4–20x reduction in event rate, giving the frontend breathing room.

**Stage 2 (ack age > ACK_DROP):** dirty cell updates are dropped entirely.
Non-visual events are preserved: mode changes, cursor shape, bell, OSC 52,
title, and CWD updates. These are low-frequency and semantically important
(e.g., a mode change affects keyboard encoding; a bell triggers a
notification). `cursor_moved` is also dropped in Stage 2 — acceptable because
the full-redraw on exit from drop mode will resync the cursor position.

**Drop → normal transition:** when the ack age drops below `ACK_STALE`
(frontend resumed painting), a full-redraw flag is set on the next emission.
This forces the frontend to re-render the entire grid, resyncing any cells
that were dropped during Stage 2.

### Clock handling

`SystemTime::now()` is used for both the frontend ack timestamp and the
backend staleness check. `SystemTime` is not monotonic — backward NTP
adjustments could produce a negative duration. The backend computes ack age
as:

```rust
let age = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as u64
    .saturating_sub(last_ack_ms);
```

`saturating_sub` ensures a backward jump produces age 0 (treated as
"just acked"), which is the safe default — the backend does not escalate on a
clock anomaly.

### CPR/DSR path

The immediate-flush path for CPR (Cursor Position Report) and DSR (Device
Status Report) responses is completely unaffected by the frame-ack mechanism.
These responses are written directly to the PTY master fd from within the VT
processor, bypassing the coalescer entirely. Frame-ack staleness has no
bearing on terminal query responses.

### SSH panes — known limitation

`ssh_task.rs` has its own emit loop that is structurally different from the
local PTY pipeline (no Task 1/Task 2 split, no MPSC channel). The frame-ack
mechanism is not wired into the SSH emit path. This is a known limitation —
SSH panes under heavy output will continue to exhibit the pre-ADR-0027
behavior (adaptive debounce only, no frontend-driven back-pressure).
Extending frame-ack to SSH panes is deferred to a follow-up change.

## Consequences

**Positive:**

- Closes the back-pressure loop: the backend now has a direct signal of
  frontend rendering capacity, rather than inferring it from emit duration.
- Prevents terminal freeze under sustained heavy output: Stage 1 slows the
  event rate; Stage 2 drops visual-only updates, preventing unbounded WebView
  queue growth.
- Full-redraw on drop exit guarantees visual consistency — the frontend grid
  is never permanently stale after a drop period.
- Minimal structural impact on the coalescer: one `AtomicU64` read and a
  conditional branch in the timer arm. No new `select!` branches, no new
  channels.

**Negative / risks:**

- ~60 IPC round-trips/s per active pane (one `frame_ack` call per rAF cycle
  at 60 Hz). This is comparable to `send_input` frequency during active
  typing and well within Tauri's IPC throughput budget.
- `SystemTime` non-monotonicity: mitigated by `saturating_sub` (see Clock
  handling above), but a large forward NTP jump could cause a false
  escalation. In practice, NTP adjustments are small (< 100 ms) and
  infrequent.
- SSH panes are not covered. Users with SSH sessions under heavy output will
  not benefit from frame-ack until the follow-up change.
- The `frame_ack` command is a fire-and-forget call with no return value. If
  the command fails (e.g., invalid pane ID after close), the failure is
  silently ignored — this is acceptable because the ack is advisory.

## Addendum: idle-period false escalation fix (2026-04-15)

### Bug

The original design computed `ack_age = now − last_frame_ack` and escalated
when `ack_age` exceeded the thresholds. This conflates "frontend is
overwhelmed" with "there was nothing to ack." During idle periods (no PTY
output), the ack timer ages indefinitely because no events are emitted and
therefore no acks arrive. When new output arrives after an idle period longer
than `ACK_DROP` (1 second), Task 2 enters drop mode on the first tick —
suppressing dirty cells even though the frontend was healthy and simply idle.

This caused two E2E regressions: `ls-al.spec.ts` (WebDriver roundtrips
between test phases exceeded 1 second) and `perf-p12a-frame-render.spec.ts`
(1-second IDLE workload pause aged the ack past the drop threshold).

### Fix

Track `last_emit_ms` — the wall-clock time of the most recent
`emit_all_pending()` call — as a local variable in Task 2. Gate the
escalation conditions on `has_unacked_emits`:

```rust
let last_ack_ms = ack_ms_e.load(Ordering::Relaxed);
let ack_age_ms = now_ms().saturating_sub(last_ack_ms);
let has_unacked_emits = last_emit_ms > last_ack_ms;
let in_drop_mode = has_unacked_emits && ack_age_ms > ACK_DROP_THRESHOLD_MS;
let in_stale_mode = has_unacked_emits && ack_age_ms > ACK_STALE_THRESHOLD_MS;
```

The atomic is loaded once into `last_ack_ms` to avoid a TOCTOU between the
`ack_age_ms` and `has_unacked_emits` computations.

**Initialization:** `last_emit_ms = 0`. Since `last_frame_ack_ms` is
initialized to `now()` (a large epoch-ms value), `0 > now()` is false, so
`has_unacked_emits` is false at startup — no false escalation.

**After frontend acks:** `last_frame_ack_ms` is updated to current time,
which is ≥ `last_emit_ms`. `has_unacked_emits` becomes false. No escalation
during subsequent idle periods.

**After new output:** `last_emit_ms` is set to `now()` after each
`emit_all_pending()` call (timer arm, CPR/DSR immediate-flush, and
channel-closed flush paths). It exceeds the old `last_frame_ack_ms`, arming
the escalation — but it only fires if the frontend fails to ack within the
threshold windows.

> **Note (2026-04-16):** the paragraph above ("After new output...") is
> superseded by Addendum 2 at the end of this document. See Addendum 2 for
> the corrected timing rule.

### NTP edge case

A backward NTP jump between an `emit_all_pending()` call (which sets
`last_emit_ms`) and a subsequent `record_frame_ack()` call (which stores
`last_ack_ms`) could make `last_ack_ms` lower than `last_emit_ms` even
though the ack arrived after the emit. This is a false positive — the system
escalates unnecessarily for one cycle. This is safe (conservative) and
self-correcting on the next ack.

### Snapshot-refetch ack

The `frameAck()` call in `triggerSnapshotRefetch()` (previously disabled for
E2E debugging) is re-enabled. This ack signals **data consumption** (the
snapshot has been applied to Svelte reactive state), not paint completion.
This is intentional: the snapshot path is a recovery mechanism, not the
steady-state rendering path. Without this ack, the backend's ack timer would
keep aging during the async snapshot fetch and could enter drop mode —
suppressing the very events the frontend needs to send a normal ack from
`flushRafQueue`.

## Addendum 2: non-visual events must not advance `last_emit_ms` (2026-04-16)

> **This Addendum 2 supersedes the paragraph "After new output: `last_emit_ms`
> is set to `now()` after each `emit_all_pending()` call..." of Addendum 1
> above; the timing described there is no longer accurate.** `last_emit_ms`
> is advanced only when the emit call produced a `screen-update` event, not
> unconditionally.

### Bug

Addendum 1 gated escalation on `has_unacked_emits = last_emit_ms > last_ack_ms`
and set `last_emit_ms = now()` **unconditionally** after every
`emit_all_pending()` call. This was incorrect: not every invocation of
`emit_all_pending()` produces a `screen-update` event. The frontend only
acknowledges paint cycles via `flushRafQueue()`, which is triggered
**exclusively** by `screen-update` events. Every other event emitted by the
coalescer — `bell-triggered`, `mode-state-changed`, `cursor-style-changed`,
`osc52-write-requested`, `notification-changed`, title and CWD updates — is
consumed by the frontend without producing an ack.

Consequence: when `emit_all_pending()` flushed a non-visual-only batch (e.g.
bash responded to Del at end-of-line with a single BEL byte `0x07`),
`last_emit_ms` was advanced past `last_ack_ms`, so `has_unacked_emits`
stayed true indefinitely even though no paint was ever owed. After 1 second
of user idle, Task 2 entered drop mode; the next keystroke's dirty cells
were silently suppressed by the drop-mode gate, and the pane froze
permanently from the user's perspective (the cursor kept blinking
client-side, but no `screen-update` event reached the frontend anymore).

The same mechanism also triggered **Stage 1** (stale mode, `ACK_STALE_DEBOUNCE`
= 250 ms) on every isolated non-visual event — not only full freezes but
also a ~250 ms input-to-paint latency regression visible after every bell,
title change (OSC 2), or CWD change (OSC 7). Real-world triggers include
Starship / Powerlevel10k prompts that emit OSC 2 or OSC 7 on every command.

### Fix

Return an `EmitOutcome` from `emit_all_pending()` carrying an explicit flag
`emitted_screen_update`. The debounce timer arm advances `last_emit_ms`
only when that flag is true:

```rust
#[derive(Debug, Clone, Copy)]
pub(super) struct EmitOutcome {
    pub duration: Duration,
    pub emitted_screen_update: bool,
}

// In Task 2 debounce timer arm:
let outcome = emit_all_pending(...);
if outcome.emitted_screen_update { last_emit_ms = now_ms(); }
current_debounce = if in_stale_mode { ACK_STALE_DEBOUNCE } else { next_debounce(outcome.duration) };
```

The same guard applies at the CPR/DSR immediate-flush call site. The
channel-closed flush path discards the outcome (the task is exiting).

### Semantic invariant

**Non-visual events never perturb the frame-ack timestamp invariant.**
`last_emit_ms` is defined as *the wall-clock time of the most recent
`screen-update` emission* — the only event type the frontend acknowledges.
Any future non-visual event type added to the coalescer inherits this
invariant automatically through `EmitOutcome.emitted_screen_update`: the
type system forbids accidentally advancing the timestamp from a non-visual
emit path.

### Scope

**This addendum applies to the local PTY pipeline only.** SSH panes
(`ssh_task.rs`) have no frame-ack mechanism and remain under the known
limitation documented in the main ADR body. Extending frame-ack (and this
invariant) to SSH panes is deferred to a follow-up change.

### Trigger example

The user-reported trigger was the **Del** key pressed at end-of-line in
bash. Bash responds with a single BEL byte (`0x07`) and no visible dirty
cells; after ~1 second of user idle, the next keystroke's output was
dropped. The same pattern applies to any non-visual-only backend event —
most commonly Starship or Powerlevel10k prompts emitting OSC 2 (window
title) or OSC 7 (working directory) on every command execution.

### Anti-regression tests

The invariant is guarded by:

- **Rust unit (`src-tauri/src/session/pty_task.rs`):** TEST-ACK-015,
  TEST-ACK-016, TEST-ACK-017, TEST-ACK-018, TEST-ACK-019, TEST-ACK-020 —
  cover `output_emits_screen_update` for all six non-visual fields, the
  gated-assignment logic, bell flood (Stage 1 non-activation), and the
  `was_in_drop_mode` idle-tick transition.
- **Rust integration (`src-tauri/tests/async_concurrency.rs`):**
  DEL-ASYNC-PTY-009 — Task 2 gating logic replayed on synthetic
  `ProcessOutput`, with a latency assertion (< 150 ms) that fails if
  Stage 1 is incorrectly armed.
- **E2E (`tests/e2e/del-key-freeze.spec.ts`):** DEL-E2E-006 (BEL + pause
  ≥ 1.2 s + follow-up text), DEL-E2E-007 (OSC 2 title + pause), DEL-E2E-008
  (OSC 7 CWD + pause).
- **Frontend vitest
  (`src/lib/composables/__tests__/useTerminalPane.frame-ack.test.ts`):**
  ACK-FE-006 — asserts that receiving `bell-triggered`, `mode-state-changed`
  or `cursor-style-changed` schedules no rAF and invokes no `frame_ack`.

## Addendum 3: Frontend ack obligation after snapshot consumption (2026-04-16)

> **This Addendum 3 complements Addendum 2 on a disjoint axis. It does NOT
> supersede any previous addendum.** Addendum 2 gates backend `last_emit_ms`
> advancement on actual `screen-update` emission. Addendum 3 mandates
> frontend `frame_ack` invocation after snapshot consumption — an
> independent invariant on the other side of the IPC boundary. The two
> invariants hold simultaneously and guard against orthogonal failure modes.

### Bug

In dev mode (`pnpm tauri dev`), the **initial pane** was frozen on startup:
the user could not type anything until a second tab or pane was created. In
production builds the problem was not observed. The root cause was a missing
`frame_ack` call on the mount path of `useTerminalPane.svelte.ts`: `onMount`
subscribed to `screen-update` in buffering mode, fetched the initial screen
snapshot, applied it, replayed buffered updates, and returned — without ever
calling `frameAck(paneId)`.

If the first backend `screen-update` emission reached the WebView queue
before the frontend listener attachment completed (reliably triggered by
Vite's slow module loading in dev), the event was silently dropped —
**yet it still advanced the backend's `last_emit_ms`**. After 1 second of
user idle, Task 2 observed
`has_unacked_emits = (last_emit_ms > last_ack_ms) = true`
and `ack_age > ACK_DROP_THRESHOLD_MS`, and entered drop mode. The next
keystroke's dirty cells were silently suppressed by the drop-mode gate and
the pane appeared permanently frozen.

### Two call sites, distinct root causes

Two frontend call sites consumed a pane screen snapshot without acking. The
same symptom masked two architecturally independent races:

- **`onMount` — pre-attach race.** The frontend event listener is attached
  inside `onMount`, after the backend pane has already started producing
  output. The first `screen-update` emission can win the race against
  listener attachment, especially in dev mode where Vite's module-graph
  resolution inflates startup latency well past the backend's 12 ms
  `DEBOUNCE_MIN` window. The lost event advanced `last_emit_ms` server-side
  with no matching ack.

- **`triggerSnapshotRefetch` — ack starvation during async fetch.** Here
  the listener is already attached, but the ack timer keeps aging through
  the entire duration of the `await commands.getPaneScreenSnapshot(...)`
  round-trip. Any `screen-update` emitted between the refetch request and
  its resolution is coalesced into the snapshot, so no separate ack fires —
  the snapshot itself carries the obligation to ack.

Both call sites converge on the same fix pattern ("ack immediately after
consuming a snapshot"), but they are not the same race, and patching only
one would leave the other exposed. Addendum 3 addresses both at once by
relocating the contract into a shared helper.

### Not a dev-only bug

> Dev mode reliably reproduces a race that is theoretically present in
> production under GC pauses, IO stalls, or any other factor that delays
> listener attachment past the 12 ms `DEBOUNCE_MIN` window. The fix applies
> universally — production has simply been lucky, not safe.

The dev-mode timing profile (Vite module loading, HMR bookkeeping) collapses
the probability of the `onMount` race to effectively 1. In production the
same race is latent: an unlucky GC pause during startup, an SSD hiccup, or
heavy system load can reproduce it at arbitrarily low probability. Fixing
only dev would preserve a silent tail-latency bug in production.

### Fix and enforcement

The fix is a new helper `fetchAndAckSnapshot(paneId)` exported from
`src/lib/ipc/snapshot.ts` (re-exported by `src/lib/ipc/index.ts`). This
helper is the **single supported entry point** for consuming a pane screen
snapshot from the frontend. It performs the `invoke` round-trip, and on
successful resolution it calls `frameAck(paneId)` before returning the
snapshot to the caller. On fetch failure, no ack is emitted (no snapshot
was consumed — the invariant is preserved).

Both historical call sites (`onMount` and `triggerSnapshotRefetch` in
`useTerminalPane.svelte.ts`) are migrated to the helper. Calling
`commands.getPaneScreenSnapshot` (or the `getPaneScreenSnapshot` re-export)
directly is **prohibited** by `src/CLAUDE.md`.

Rationale for the helper-as-SSoT pattern: enforcement by API design strictly
dominates enforcement by convention. A future caller (search indexer,
debug dump, test harness) that imports the helper cannot forget the ack —
the ack is a post-condition of consuming the snapshot, encoded in the type
signature's contract, not in a reviewer's memory.

### Why not backend-side auto-ack

> A backend-side auto-ack inside `get_pane_screen_snapshot` was considered
> and rejected. `last_ack_ms` means "the frontend has painted up to this
> point". A future caller that fetches a snapshot without rendering
> (search index, debug dump, programmatic export) would incorrectly
> advance the timestamp, silencing legitimate back-pressure escalation for
> any subsequent real paint cycle. The invariant must stay frontend-owned:
> only a caller that actually renders the content has the authority to ack.

This mirrors the decision in Addendum 2 — the ack semantics are
render-coupled, not fetch-coupled. Moving the ack server-side would
re-open the conflation that Addendum 2 closed on the emit side.

### Anti-regression tests

The invariant is guarded by:

- **Frontend vitest (helper, `src/lib/ipc/__tests__/snapshot.test.ts`):**
  ACK-FE-009-A (happy path — `frameAck` called exactly once on fetch
  success, across multiple pane IDs), ACK-FE-009-B (error path —
  `frameAck` is NOT called when the binding rejects, because no snapshot
  was consumed).
- **Frontend vitest
  (`src/lib/composables/__tests__/useTerminalPane.frame-ack.test.ts`):**
  ACK-FE-007 (onMount with non-empty `pendingUpdates`: a deferred snapshot
  promise allows listener attachment before resolution; buffered events
  fire during the fetch; exactly one `frame_ack` observed after mount;
  extended with an idempotence check — subsequent standard events flushed
  via rAF produce a second, independent ack), ACK-FE-008 (onMount with
  empty `pendingUpdates`: the race where the listener attaches after the
  first backend emit and has nothing to buffer; `frame_ack` is called
  unconditionally, not gated on `pendingUpdates.length > 0`).
- **Rust unit (`src-tauri/src/session/registry.rs`):** TEST-ACK-021 —
  `record_frame_ack` idempotence under rapid successive calls; monotonic
  non-decreasing `last_frame_ack_ms`; no panic on unknown `PaneId`.
