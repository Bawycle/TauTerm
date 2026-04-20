<!-- SPDX-License-Identifier: MPL-2.0 -->

# Functional Test Protocol — PTY, VT, SSH, Preferences, UI, IPC

This document catalogs functional test scenarios with unique IDs, traceability to FS requirements, and expected results. Each scenario is implementable as an automated test or a manual verification step.

---

## SSH Coalescer (ADR-0028)

Scenarios covering the shared `EmitCoalescer` behavior for SSH panes, VT extraction parity, and VT input caps.

### SSH-COALESCE — Coalescer integration

| ID | FS Trace | Precondition | Action | Expected Result |
|---|---|---|---|---|
| SSH-COALESCE-001 | ADR-0028 | SSH pane connected | Inject burst of 100 `ProcessOutput` values in < 1 ms | Values merged; single `screen-update` emitted (not 100) |
| SSH-COALESCE-002 | ADR-0028 | SSH pane connected, frame-ack withheld > 200 ms | Inject output | Debounce escalated to 250 ms (Stage 1) |
| SSH-COALESCE-003 | ADR-0028 | SSH pane connected, frame-ack withheld > 1000 ms | Inject output with dirty cells | Dirty cell updates dropped; non-visual events preserved (Stage 2) |
| SSH-COALESCE-004 | ADR-0028 | Stage 2 active, then frame-ack received | Inject output | Full-redraw flag set on next emit to resync frontend grid |
| SSH-COALESCE-005 | ADR-0028 | SSH pane connected | Close SSH channel (EOF) | Last `screen-update` emitted before `SshLifecycleState::Closed` event |
| SSH-COALESCE-006 | ADR-0028 | SSH pane connected | Inject 1000 BEL bytes (bell flood) | Bell events emitted; `last_emit_ms` NOT advanced; no backpressure escalation |
| SSH-COALESCE-007 | ADR-0028 | SSH pane connected | Inject `ProcessOutput` with `needs_immediate_flush = true` | Debounce timer bypassed; emit occurs immediately |
| SSH-COALESCE-008 | ADR-0028 | SSH pane connected | Drop the MPSC sender (channel close) | `Coalescer::run` exits promptly; `JoinHandle` resolves |

### SSH-EXTRACT — VT extraction parity

| ID | FS Trace | Precondition | Action | Expected Result |
|---|---|---|---|---|
| SSH-EXTRACT-001 | ADR-0028 | SSH pane connected | Inject `\x07` (BEL) via SSH channel | `ProcessOutput.bell == true`; bell event emitted |
| SSH-EXTRACT-002 | ADR-0028 | SSH pane connected, `allow_osc52_write = true` | Inject OSC 52 sequence | `ProcessOutput.osc52_write` populated; `osc52-write` event emitted |
| SSH-EXTRACT-003 | ADR-0028 | SSH pane connected | Inject `CSI ? 12 h` (blinking cursor) | `ProcessOutput.cursor_shape` updated; `cursor-shape-changed` event emitted |
| SSH-EXTRACT-004 | ADR-0028 | SSH pane connected | Inject `OSC 7 ; file:///home/user/dir ST` | `ProcessOutput.cwd` populated; `cwd-changed` event emitted |
| SSH-EXTRACT-005 | ADR-0028 | SSH pane connected | Inject `CSI 6 n` (CPR query) → VT responds | `ProcessOutput.mode_changed` extracted; mode event emitted |

### VT-CAP — VT input caps

| ID | FS Trace | Precondition | Action | Expected Result |
|---|---|---|---|---|
| VT-CAP-TITLE-001 | ADR-0028 | VT processor initialized | Inject `OSC 0 ; <8000 chars> BEL` | Title stored ≤ 4096 chars; `tracing::warn!` emitted on truncation |
| VT-CAP-TITLE-002 | ADR-0028 | VT processor initialized | Inject `OSC 0 ; <C0-stuffed 8000 chars> BEL` | C0/C1 chars stripped first, then truncation at 4096 chars |
| VT-CAP-CWD-001 | ADR-0028 | VT processor initialized | Inject `OSC 7 ; <5 KB URI> ST` | CWD update dropped; `tracing::warn!` emitted |
| VT-CAP-RESPONSES-001 | ADR-0028 | VT processor initialized | Inject 1000 DSR queries (`CSI 6 n`) | `pending_responses.len() <= 256`; `tracing::warn!` on overflow |
| VT-CAP-RESPONSES-002 | ADR-0028 | VT processor initialized | Inject 300 DSR queries | Oldest responses dropped (VecDeque pop_front); newest 256 preserved |
| SEC-VT-CAP-OSC52-OVERSIZE-001 | ADR-0028 | VT processor initialized | Inject OSC 52 with 10 KB payload | Dropped at `OSC_PAYLOAD_MAX = 4096` upstream; `take_osc52_write()` returns `None` |
