<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0006 — Coarse-grained IPC: one command per user action

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm's backend and frontend communicate via Tauri's IPC: `invoke()` for commands (frontend → backend), and typed events via `AppHandle::emit()` for notifications (backend → frontend). The IPC design must handle:

- Session management actions: create/close tab, split/close pane, rename tab, reorder tab
- SSH connection management: open connection, close connection, reconnect, save/edit/delete connection
- PTY I/O: sending keyboard input to a pane
- Terminal rendering: delivering screen updates, scroll position changes
- Preferences: read/write preferences, theme management
- Scrollback: scroll position control, search
- Credential prompts: request/provide credentials

The key design question is the granularity of the command API: should commands map one-to-one with user actions (coarse-grained), or should the frontend compose multiple fine-grained commands to implement each action?

## Decision

The IPC command API is **coarse-grained**: each `#[tauri::command]` corresponds to a meaningful user action, not to a data field update or a sub-step of a user action. Each command receives a fully specified intent and returns a fully specified result. The frontend does not need to make multiple sequential `invoke()` calls to complete a single user action.

Examples of correct granularity:
- `create_tab(config: CreateTabConfig) -> Result<TabState>` — not `allocate_tab_id()` + `set_tab_config(id, config)` + `spawn_pty(id)`
- `open_ssh_connection(pane_id, connection_id) -> Result<()>` — not `resolve_credentials()` + `tcp_connect()` + `ssh_handshake()`
- `send_input(pane_id, data: Vec<u8>) -> Result<()>` — acceptable as a stream command because keyboard input is already a coarse user action (one keypress or paste = one IPC call)
- `update_preferences(patch: PreferencesPatch) -> Result<Preferences>` — updates the entire preferences object, not individual fields

Events are always emitted by the backend and consumed by the frontend. Events are fine-grained where necessary for real-time rendering (`screen-update`, `scroll-position-changed`) but the frontend remains a passive receiver — it never emits events to the backend.

## Alternatives considered

**Fine-grained command API (one command per field or sub-operation)**
The frontend would compose `invoke('set_tab_label', {id, label})` + `invoke('set_tab_order', {id, order})` etc. This is more flexible but introduces atomicity problems (what if the second call fails after the first succeeds?), increases the round-trip overhead for each user action, and makes the command API larger and harder to maintain. Not chosen.

**GraphQL-style mutation API**
A single `invoke('mutation', {type: ..., payload: ...})` with a dynamic payload. Type safety is lost without significant boilerplate. Tauri's typed command system is a better fit for static, well-typed IPC. Not chosen.

**Bidirectional event stream (both directions)**
Having the frontend emit events upward (in addition to commands) would blur the separation between the two layers. Events are for state notifications, commands are for intent. Allowing the frontend to emit events to the backend would create a bidirectional coupling that is harder to reason about. Not chosen; the current model (commands for intent, events for state) is unidirectional and clear.

## Consequences

**Positive:**
- Each command is a transaction: the backend validates the full intent, executes it atomically, and returns a complete result. No partial-state scenarios from incomplete multi-call sequences.
- The command surface is small and auditable. Adding a new user action means adding one command, not a family of micro-commands.
- Tauri's capability system grants permissions per command: the frontend receives only the commands it needs. Coarse commands make this list short and reviewable.
- Each command is fully serializable with `serde` — no raw pointers, handles, or platform objects cross the boundary.

**Negative / risks:**
- Command input types must be designed to carry all the information the backend needs to complete the action. For complex actions (creating an SSH connection with credentials), the input type may be large. This is acceptable: large types are preferable to chatty sequences.
- `send_input` is the exception to strict coarse granularity: it is called on every keypress and paste. Its payload is a `Vec<u8>` (already the most compact representation of keyboard input). Performance of this path must be validated; if Tauri IPC overhead becomes significant at very high typing rates, a lower-level channel (e.g., a WebSocket to a local listener) may be needed as a future optimization. This risk is noted but not addressed in v1.

**Note — `send_input` exception must be documented in the IPC contract:**
The `send_input` command is an explicitly documented exception to the coarse-grained principle: it has fine granularity and high call frequency (one call per keypress or paste). This exception MUST be reflected in the IPC contract documentation (ARCHITECTURE.md §4.2) so that future maintainers understand it is a deliberate architectural compromise — not an oversight. Any future addition of similarly high-frequency commands must go through an explicit design decision and be documented the same way.

**Debt:**
The `send_input` performance question is a known open risk. It is unlikely to be observable at normal typing rates (< 100 chars/s) but could manifest with paste of very large payloads. For v1, the risk is accepted; a mitigation path (alternative input transport) exists if needed.
