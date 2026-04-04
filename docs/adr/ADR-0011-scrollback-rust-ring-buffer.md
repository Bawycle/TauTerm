<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0011 — Scrollback storage: Rust ring buffer in backend

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm must provide a scrollback buffer for each pane: a history of lines that have scrolled off the top of the visible terminal area, navigable by the user (FS-SB-001 through FS-SB-008). The scrollback buffer can grow large: the default is 10,000 lines (FS-SB-002); users may configure larger values.

The key design question is where the scrollback is stored and how much of it is transmitted to the frontend:

**Option A: Scrollback stored in the Rust backend, only the visible viewport transmitted**
The `ScreenBuffer` in the `vt` module holds the scrollback as a ring buffer. The frontend renders a viewport of visible rows (current screen + a small number of buffered rows for smooth scrolling). Navigating the scrollback triggers IPC calls to fetch the required rows.

**Option B: Full scrollback transmitted to the frontend on each screen update**
The backend transmits the complete scrollback buffer in every `screen-update` event or provides it on demand. The frontend stores the full history in JavaScript memory and renders it locally.

## Decision

Use **Option A — scrollback stored in the Rust backend as a ring buffer, only the viewport transmitted**.

The `ScreenBuffer` (`vt/screen_buffer.rs`) owns a ring buffer of scrollback lines. Its capacity is configurable via `PreferencesStore` (default: 10,000 lines; FS-SB-002). When the ring is full, the oldest line is evicted. The frontend receives only the rows needed for the current viewport via `screen-update` events (cell diffs for the visible area) and `get_pane_screen_snapshot` (full snapshot for initial render or after reconnect).

Scrollback navigation is implemented as scroll offset state in `scroll.svelte.ts`. When the user scrolls beyond the currently buffered rows, the frontend invokes `scroll_pane` which returns a `ScrollPositionState` containing the new viewport rows fetched from the backend ring buffer.

**Scrollback exclusion rule:** Only lines scrolled off the top of a full-screen scroll region enter the ring buffer. Lines evicted from a partial DECSTBM region (margins not spanning the full screen) are discarded (FS-VT-053, FS-SB-004). This prevents multiplexers (tmux, screen) from polluting the scrollback with their status bars.

## Alternatives considered

**Full scrollback in the frontend (Option B)**
Store the complete scrollback in a JavaScript array in the frontend. Updates to the scrollback arrive with each `screen-update` event.

Rejected for two reasons:

1. **Memory attack surface.** A remote program (via an SSH session) can emit arbitrary output at arbitrary rates. With the scrollback stored in JavaScript, an application that emits 100,000 lines of output forces 100,000 lines × (column width × cell size) into the WebView process's V8 heap. At 1 MB per 10,000 lines, a 1,000,000-line attack would consume 100 MB in the renderer process — on top of everything else. The Rust backend ring buffer has a configured upper bound enforced in native code. JavaScript arrays have no enforced bound outside explicit application logic. A fixed ring buffer in Rust is a more reliable defense against scrollback exhaustion attacks (FS-SEC reference: resource limits).

2. **Serialization overhead.** Transmitting the complete scrollback on every screen update is architecturally wrong: `screen-update` events are high-frequency (tens per second during active output). Appending new lines to the serialized scrollback payload on every event produces quadratic serialization cost (each event includes all previous lines). Even transmitting only new lines requires the frontend to maintain a complete local accumulator, which has the same memory footprint problem as Option B.

**Hybrid: frontend caches recent scrollback, fetches older lines on demand**
The frontend keeps a fixed-size cache of recently received scrollback lines; older lines are fetched from the backend on demand. This reduces the average memory footprint but adds complexity (cache invalidation on resize, on reconnect, on theme change). Not chosen for v1: the simple model (backend ring buffer, fetch-on-scroll) is correct and sufficient.

## Consequences

**Positive:**
- Scrollback size is bounded by the Rust ring buffer capacity, regardless of application output rate. The frontend's memory footprint for scrollback is limited to the visible viewport plus a small prefetch buffer.
- The ring buffer eviction policy is O(1) — no memory allocation on each line eviction, no GC pressure in the renderer.
- Scrollback search (`search_pane`) operates on the ring buffer in Rust, where it can iterate efficiently without serialization cost. Only the search results (positions) are transmitted to the frontend.
- The scrollback default of 10,000 lines is a documented, user-adjustable preference with a visible memory estimate in the UI (FS-SB-002).

**Negative / risks:**
- Scrollback navigation requires IPC round-trips when the user scrolls beyond the prefetched buffer. At typical scroll rates, this is not perceptible, but extremely fast keyboard scrolling could produce visible latency. Mitigation: prefetch a configurable number of lines ahead of the current scroll position.
- On pane resize, the ring buffer may need to reflow lines (changing the line width changes which content is on which line). Reflow of a full 10,000-line scrollback is O(n) and should be profiled. Mitigation: defer reflow to when the scrollback is accessed, not on every resize event.
- `get_pane_screen_snapshot` must include enough scrollback rows for the initial render without a subsequent `scroll_pane` call. The snapshot includes the current screen plus the immediately preceding buffer (configurable, default: one screen height above the current position).
