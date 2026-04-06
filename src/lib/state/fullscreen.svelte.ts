// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive fullscreen state — tracks whether the application window is in
 * fullscreen mode.
 *
 * Provides:
 *   - `fullscreenState` — read-only reactive accessor
 *   - `setFullscreen()` — called from useTerminalView on IPC events and
 *     command responses
 *
 * Source of truth: the backend window state, synced via:
 *   - `getCurrentWindow().isFullscreen()` on mount
 *   - `toggle_fullscreen` command response
 *   - `fullscreen-state-changed` event (WM-driven changes)
 */

const _fs = $state<{ value: boolean }>({ value: false });

/**
 * Read-only reactive accessor for the current fullscreen state.
 * `value` is `false` until the first sync from the window.
 */
export const fullscreenState = {
  get value(): boolean {
    return _fs.value;
  },
};

/**
 * Update the reactive fullscreen state.
 * Called from useTerminalView composable only — not from components directly.
 */
export function setFullscreen(v: boolean): void {
  _fs.value = v;
}
