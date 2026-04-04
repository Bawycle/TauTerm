<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0003 — VT parser: use the `vte` crate

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm must parse the ECMA-48 / ANSI / xterm escape sequence stream produced by PTY output. This includes:
- C0 control characters (BEL, BS, TAB, LF, CR, ESC)
- CSI sequences (SGR, cursor movement, DECSET/DECRST, mouse reporting, scroll regions)
- OSC sequences (tab title, hyperlinks, clipboard)
- DCS sequences
- Correct handling of partial sequences split across read boundaries (FS-VT-005)
- Sequence size limits (FS-SEC-005: 4096 bytes max per sequence)

Implementing a conformant VT state machine from scratch is a multi-week effort with significant correctness risk. The state machine must conform to Paul Flo Williams' "A parser for DEC's ANSI-compatible video terminals" (the standard reference for this state machine).

## Decision

Use the **`vte` crate** as the VT parser.

`vte` implements the Williams state machine in Rust, processes the PTY byte stream incrementally (handles split sequences correctly), and invokes callbacks via a `Perform` trait that the screen buffer implementation must satisfy. It handles CSI, OSC, DCS, and ESC sequences. It is the parser used by Alacritty and other Rust terminal emulators, giving it significant battle-testing.

TauTerm will implement the `vte::Perform` trait on a `ScreenBuffer` type (or a `VtProcessor` adapter that owns the buffer and dispatches to it). This is the primary extension point for terminal emulation behavior.

## Alternatives considered

**Implement a bespoke VT parser**
Complete control over the parser but at very high implementation cost and correctness risk. The Williams state machine has 14 states and handles dozens of edge cases (mid-sequence UTF-8, escaped ESC inside OSC, etc.). Three to six weeks of work that `vte` already provides. Not chosen.

**Use `alacritty-terminal` as a combined parser + screen buffer**
`alacritty-terminal` bundles a VT parser, screen buffer, and selection model. This would give TauTerm a battle-tested terminal emulation layer quickly, but at the cost of tightly coupling the screen buffer design to Alacritty's data model, which is not aligned with TauTerm's scrollback architecture (particularly the distinction between normal and alternate screens with separate scrollback freeze semantics per FS-SB-005). The crate is also not published with a stable API contract. Not chosen.

**Use `termwiz` (WezTerm's terminal library)**
`termwiz` provides a VT parser and terminal model from the WezTerm project. It is more complete than `vte` in some areas (Kitty keyboard protocol) but significantly heavier as a dependency. Its API is designed for WezTerm's internal use and changes without stability guarantees. Not chosen for v1; reconsider if Kitty keyboard protocol support is added in a future version.

## Consequences

**Positive:**
- Correct incremental parsing with no need to manage sequence buffering manually.
- The `Perform` trait is TauTerm's authoritative extension point for implementing all VT behavior (cursor movement, SGR, screen mode changes, OSC handlers).
- Sequence-level callback design is well-suited to the screen buffer update model: each callback updates the buffer and may trigger a `screen-update` event to the frontend.
- Security: `vte` does not perform any actions itself — all behavior is in TauTerm's `Perform` implementation. Sequence length enforcement (FS-SEC-005) is applied in the `Perform` callbacks before processing.

**Negative / risks:**
- `vte` does not enforce sequence length limits internally; TauTerm must enforce the 4096-byte limit (FS-SEC-005) in its `Perform` implementation by tracking accumulated lengths and discarding overlong sequences.
- `vte` provides a low-level callback interface; TauTerm is responsible for all semantic behavior (what a given CSI sequence means). This is intentional but places the correctness burden on TauTerm's `Perform` implementation.
- If the `vte` crate's API changes incompatibly, the impact is bounded to the `VtProcessor` / `Perform` implementation — a contained surface.

**Open question (not blocking v1):**
The Kitty keyboard protocol is out of scope for v1 (FS-KBD, domain constraints in FS.md §4). If it is added in a future version, the parser may need to be extended or replaced. The `Perform` trait extension point does not foreclose this; it would be an additive change.

**Note — `vte` internal buffer limit vs TauTerm's 4096-byte enforcement:**
FS-SEC-005 requires TauTerm to enforce a maximum of 4096 bytes per OSC and DCS sequence parameter. This enforcement is applied in TauTerm's `Perform` callbacks (e.g., `osc_dispatch`, `hook`/`put`/`unhook` for DCS). However, this enforcement is only effective if `vte` accumulates and delivers the full parameter content to the callbacks before TauTerm can inspect it. If `vte`'s internal parameter buffer (`PARAMS_MAX` or equivalent constant) is smaller than 4096 bytes, `vte` will silently truncate overlong sequences before invoking the callbacks — the excess bytes are already lost, and TauTerm's enforcement in `Perform` becomes dead code for sequences that exceed `vte`'s internal limit.

**Action required at implementation:** the implementor MUST verify the buffer limit of the specific `vte` version pinned in `Cargo.toml` (check the `PARAMS_MAX` or equivalent constant in the `vte` source). If the limit is below 4096 bytes, one of the following mitigations MUST be applied: (a) pin a version or fork where the limit is at or above 4096 bytes, (b) patch `vte` via a Cargo patch override, or (c) enforce the limit at the byte-stream level before feeding data to `vte`. This check is a precondition for considering FS-SEC-005 correctly implemented.

**Debt:**
None for v1 scope. The Kitty protocol deferral is documented as a known v1 limitation in FS.md §4.
