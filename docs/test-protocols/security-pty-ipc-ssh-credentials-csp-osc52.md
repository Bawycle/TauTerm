<!-- SPDX-License-Identifier: MPL-2.0 -->

# Security Test Protocol — PTY, IPC, SSH, Credentials, CSP, OSC 52

This document catalogs security test scenarios with unique IDs, threat models, and expected results. Each scenario targets a specific attack vector or defense-in-depth boundary.

---

## SSH Coalescer Security (ADR-0028)

### Threat model

SSH panes receive bytes from a remote server — an untrusted input source. The shared `EmitCoalescer` and VT input caps must prevent resource exhaustion, lock inversion, and information leakage.

### SEC-SSH — SSH coalescer security scenarios

| ID | Threat | Precondition | Action | Expected Result |
|---|---|---|---|---|
| SEC-SSH-OOM-001 | OOM via unbounded event queue | SSH pane connected | Inject 1 GB of output data | Memory bounded by `mpsc::channel(256)` + frame-ack escalation; no OOM; process RSS stays within ~50 MB above baseline |
| SEC-SSH-DSR-FLOOD-001 | DSR amplification via unbounded response writes | SSH pane connected | Inject 10,000 DSR queries (`CSI 6 n`) | Responses capped at 256 entries (VT cap); coalesced into single `ch.data()` write; no SSH channel mutex thrashing |
| SEC-SSH-OSC52-SPAM-001 | Memory growth via OSC 52 accumulation | SSH pane connected, `allow_osc52_write = true` | Inject 1,000 distinct OSC 52 sequences | `pending_osc52_write` is `Option` (last-wins); no growth beyond single entry; upstream `OSC_PAYLOAD_MAX = 4096` enforced |
| SEC-SSH-FRAME-ACK-DESYNC-001 | Backpressure bypass via frame-ack withholding | SSH pane connected | Flood output with frame-ack permanently withheld | `mpsc::channel(256)` cap holds as defense-in-depth; sender blocks; no unbounded queue growth even without frame-ack |
| SEC-SSH-LOCK-NO-WRITE-WITH-CHANNEL-001 | Deadlock via lock inversion | SSH pane connected | Structural assertion on `extract_process_output` | VT write-lock is NEVER held when SSH channel mutex is acquired; `extract_process_output` returns by value (drops guard); `#[deny(clippy::await_holding_lock)]` enforces async side |

### VT input caps as defensive measures

The following caps are enforced at the site of accumulation in `vt/processor.rs`, not at the `take_*()` extraction points:

| Cap | Location | Limit | Overflow behavior |
|---|---|---|---|
| `pending_responses` (DSR/CPR/DA) | `vt/processor/dispatch/csi_misc.rs` | 256 entries (`VecDeque`) | Oldest dropped via `pop_front`; `tracing::warn!` emitted |
| OSC 0/2 title | `vt/processor/dispatch/osc.rs` | 4096 chars (after C0/C1 strip) | Truncated; `tracing::warn!` emitted |
| OSC 7 CWD | `vt/processor/dispatch/osc.rs` | 4 KB | Dropped entirely; `tracing::warn!` emitted |
| OSC 52 payload | `vt/processor/dispatch/osc.rs` | 4096 bytes (`OSC_PAYLOAD_MAX`) | Dropped at parse level (upstream guard) |

### Inherited SSH-untrusted vectors (out of scope)

These vectors are identified but not addressed by ADR-0028. They are tracked separately in TODO.md:

| # | Vector | Status | Mitigation |
|---|---|---|---|
| 1 | OSC 8 hyperlink URI not validated (XSS if frontend renders `<a>`) | Open — tracked in TODO | Frontend does not currently render hyperlinks |
| 2 | OSC 7 CWD not sanitized for Unicode bidi (RTL override, homoglyphs) | Open — tracked in TODO | Cap at 4 KB limits blast radius |
| 3 | SSH channel mutex held during `Channel::wait()` (DoS via slow server) | Open — latent debt | Bounded by `russh` receive window |
| 4 | OSC 0/2 title not sanitized for Unicode bidi | Open — tracked in TODO | Truncated at 4096 chars |
| 5 | Frame-ack spam from compromised WebView (post-XSS) disables escalation | Open — defense-in-depth | `mpsc(256)` cap remains as floor |
