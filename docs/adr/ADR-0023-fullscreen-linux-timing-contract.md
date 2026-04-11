<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0023 — Fullscreen Linux timing contract

**Date:** 2026-04-11
**Status:** Accepted

## Context

On Linux (X11 and Wayland via Tauri / WRY), calling `window.set_fullscreen(true)` or
`window.set_fullscreen(false)` returns immediately after sending a request to the
window manager. The WM then applies the geometry change asynchronously. When
`toggle_fullscreen` returns and the frontend ResizeObserver fires, the window
dimensions reported by the WM may still reflect the pre-transition geometry.

This matters because SIGWINCH must be sent to the active PTY session with the
correct final dimensions. If `resize_pane` is called while the window is still
mid-transition (e.g. the ResizeObserver fires before the WM confirms the new
size), the PTY receives an incorrect `cols × rows` value. This causes terminal
applications (vim, tmux, htop) to render incorrectly until a second resize event
arrives.

A secondary concern is focus restoration: the frontend must restore keyboard focus
to the active viewport after the fullscreen transition, but doing so immediately
in the `toggle_fullscreen` `onclick` handler risks restoring focus before the
WebView has finished recompositing, which can silently drop the `focus()` call.

The implementation in `src-tauri/src/commands/system_cmds/window.rs`:

```rust
let app_clone = app.clone();
tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    emit_fullscreen_state_changed(
        &app_clone,
        FullscreenStateChangedEvent { is_fullscreen: target },
    );
});
```

The `toggle_fullscreen` command also returns a `FullscreenState` synchronously
(with `is_fullscreen: target`) so that the frontend has the new target state
immediately — it does not need to wait for the event to update its own state.

## Decision

After calling `window.set_fullscreen()`, the backend waits **~200 ms** (a fixed
`tokio::time::sleep`) before emitting the `fullscreen-state-changed` event to the
frontend.

The frontend restores focus to the active viewport inside the
`fullscreen-state-changed` event handler (`useTerminalView.core.svelte.ts`
`onFullscreenStateChanged`), not in the `toggle_fullscreen` `onclick` handler.
The ResizeObserver fires naturally after the WM confirms the geometry, and its
callback calls `resize_pane` with the correct dimensions.

SIGWINCH is therefore sent with the correct post-transition dimensions, not with
the pre-transition geometry that would be reported by an immediate resize.

## Alternatives considered

**Poll the window state until `is_fullscreen()` changes**

After `set_fullscreen()`, poll `window.is_fullscreen()` in a loop with a short
sleep until the value matches `target`, then emit the event.

This is adaptive and does not waste time when the WM is fast. However, it
requires an explicit polling loop with a timeout (to avoid spinning forever if
the WM never confirms), and adds complexity. The Tauri API does not provide a
future or callback for WM confirmation; polling would need to be implemented
manually. Not chosen for v1.

**Listen to a WM resize event (e.g., Tauri's window resize event)**

Subscribe to Tauri's window-resize event and emit `fullscreen-state-changed`
from inside that handler.

This is more correct than a fixed delay but introduces a coupling to
implementation-specific Tauri events that behave differently between X11 and
Wayland backends. On Wayland, the resize event may fire during the transition
(before the final geometry is stable), which would not improve on the fixed
delay. Not chosen.

**Emit immediately and accept temporarily incorrect PTY dimensions**

Emit `fullscreen-state-changed` synchronously inside `toggle_fullscreen`
(before the WM has confirmed the geometry), and let the ResizeObserver
correct the PTY dimensions when it fires.

This risks a SIGWINCH with incorrect dimensions reaching the PTY before the
correct dimensions are established. The PTY may relay incorrect geometry to
running applications, causing a visible redraw artifact. Not chosen.

## Rationale for 200 ms

200 ms is empirically sufficient for both X11 (KWin, Mutter, Openbox) and
Wayland (Mutter/GNOME Shell, KWin/Plasma) to confirm a fullscreen transition
on a modern system. It is imperceptible to the user (the transition itself takes
150–200 ms visually). The headroom above the typical WM response time
(80–120 ms) accounts for compositing pipelines under moderate load.

This is a conservative fixed value, not a tight bound. A future improvement
(post-v1) could replace it with adaptive polling (alternative 1 above) once
the Tauri API surface stabilizes.

## Consequences

**Positive:**
- Simple implementation — one `tokio::time::sleep` call, no polling loop.
- Focus restoration and PTY resize happen reliably after the WM has confirmed
  the geometry, eliminating the resize artifact in terminal applications.
- Works consistently across X11 and Wayland without WM-specific event handling.

**Negative / risks:**
- 200 ms added latency between the user action and the `fullscreen-state-changed`
  event. This is imperceptible in practice (the visual transition takes at least
  as long) but means the frontend's event-driven state is slightly behind the
  actual window state.
- On very slow systems or heavily loaded compositors, 200 ms may not be
  sufficient. In that case, the ResizeObserver may fire before the event, and
  the PTY would receive two resize signals (one potentially incorrect, one
  correct). This is survivable — terminal applications handle spurious SIGWINCH
  gracefully by redrawing at the new size.
- The fixed delay is not testable deterministically in E2E tests; the
  `inject_ssh_delay`-style injection pattern is not applicable here (the delay
  is in the backend, not the SSH path). E2E tests must account for the 200 ms
  delay when asserting fullscreen state.
