<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0004 — Svelte 5 runes as frontend state management

**Date:** 2026-04-04
**Status:** Accepted

## Context

The TauTerm frontend is a reactive Svelte application that must manage:
- Session topology state (tabs, panes, active pane) — driven by backend events
- Per-pane terminal rendering state (screen buffer cells, cursor, scrollback position)
- SSH lifecycle state per pane
- Notification indicators per tab / pane
- Preferences state
- Transient UI state (search query, open dialogs, drag state)

State management choices include: Svelte 5's built-in runes (`$state`, `$derived`, `$effect`), Svelte 4-style global stores (`writable`, `readable`), external stores (Zustand, Jotai), or a full state machine framework (XState).

The project is already committed to Svelte 5 (per CLAUDE.md). The question is whether runes alone are sufficient or whether a supplementary state management layer is required.

## Decision

Use **Svelte 5 runes exclusively** (`$state`, `$derived`, `$effect`) for all frontend state management. No centralized store library. No external state management framework.

State is owned at the component level or in shared state modules (plain `.svelte.ts` files with exported `$state` variables) when cross-component sharing is needed. Components receive state as props or import from shared modules; they never reach up the component tree to mutate a parent's state.

## Alternatives considered

**Svelte 4-style global stores (writable/readable)**
Compatible with Svelte 5 but represent the previous paradigm. Runes provide the same capabilities with better TypeScript integration, finer-grained reactivity, and direct integration with Svelte 5's component model. Not chosen: no reason to use the legacy model in a greenfield Svelte 5 project.

**XState for terminal lifecycle state machines**
XState is a full actor-model state machine library. The SSH session lifecycle and PTY session lifecycle are legitimately finite state machines. However, the authoritative state machine for these lifecycles lives in the **Rust backend** (see ARCHITECTURE.md §5). The frontend merely reflects the state it receives via IPC events; it does not independently maintain or transition lifecycle states. Introducing XState on the frontend for state that is owned and driven by the backend would create a redundant, potentially divergent state representation. Not chosen.

**React + Zustand / Jotai**
React was not selected as the UI framework; TauTerm uses Svelte. These options are not applicable.

**Shared Svelte stores (a custom reactive store layer)**
Could be implemented as `.svelte.ts` modules with exported `$state` variables and mutation functions. This is essentially what this decision recommends — the distinction from "centralized store" is that state is co-located with the concern it represents (e.g., `sessionStore.svelte.ts` for session topology), not dumped into a single global object.

## Consequences

**Positive:**
- Svelte 5 runes integrate with the compiler's reactivity system: `$derived` values are lazy and only recompute when their dependencies change, avoiding over-rendering.
- No dependency on external state management libraries reduces bundle size and simplifies the mental model.
- `$effect` is the natural integration point for Tauri event listeners: `$effect(() => { const unlisten = await listen('session-state-changed', handler); return unlisten; })`.
- Unidirectional data flow is enforced by convention: rune state is declared in the owning module, mutations go through the module's exported functions, consumers react to changes. This is the same pattern as a store without the indirection.

**Negative / risks:**
- As the application grows, the discipline of not mutating state from unrelated modules must be maintained by convention rather than enforced by a framework. This is a governance risk, not a technical one.
- Very high-frequency state updates (screen buffer updates at high terminal output rates) may create performance pressure. `$state` with fine-grained cell-level updates could cause excessive reactivity. Mitigation: the terminal rendering area uses a DOM-based cell rendering approach that bypasses Svelte reactivity for cell-level updates; only coarser state (scrollback length, cursor position, pane metadata) flows through runes. The terminal rendering strategy was resolved by ADR-0008 (DOM-based cell rendering).

**Debt:**
None for this decision. The terminal rendering strategy is resolved (ADR-0008). Runes are compatible with the DOM-based cell rendering approach for all coarser state.
