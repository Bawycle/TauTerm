// SPDX-License-Identifier: MPL-2.0

/**
 * setupViewListeners — IPC event listener setup for useTerminalView.
 *
 * Registers all backend→frontend event listeners that update the ViewState bag.
 * Must be called from inside onMount so that event handlers close over the bag
 * reference and can mutate its reactive properties.
 *
 * Returns an array of unlisten functions. The closeUnlisten handle (WM close)
 * is registered separately by the caller (it needs to be exposed on the bag
 * for targeted removal before window.destroy()).
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  getConnections,
  hasForegroundProcess,
} from '$lib/ipc/commands';
import {
  onSessionStateChanged,
  onSshStateChanged,
  onHostKeyPrompt,
  onCredentialPrompt,
  onNotificationChanged,
  onModeStateChanged,
  onFullscreenStateChanged,
} from '$lib/ipc/events';
import {
  sessionState,
  applySessionDelta,
  collectLeafPanes,
} from '$lib/state/session.svelte';
import {
  setHostKeyPrompt,
  setCredentialPrompt,
  applySshStateChanged,
  setBracketedPaste,
} from '$lib/state/ssh.svelte';
import { applyNotificationChanged } from '$lib/state/notifications.svelte';
import { setFullscreen } from '$lib/state/fullscreen.svelte';
import type { ViewState } from './useTerminalView.core.svelte';
import type { PaneId } from '$lib/ipc/types';

/**
 * Register all IPC event listeners that update the ViewState bag.
 *
 * @param bag      - The ViewState bag (mutable reactive object).
 * @param doClosePane - Async function called when autoClose notification fires.
 * @returns        - Array of unlisten functions to call on destroy.
 *                   The closeUnlisten handle is written directly onto bag.closeUnlisten.
 */
export async function setupViewListeners(
  bag: ViewState,
  doClosePane: (paneId: PaneId) => Promise<void>,
): Promise<Array<() => void>> {
  const unlistens: Array<() => void> = [];

  // ── Fetch saved connections ───────────────────────────────────────────────
  try {
    bag.savedConnections = await getConnections();
  } catch {
    // Non-fatal
  }

  // ── Sync initial fullscreen state + WM close handler ─────────────────────
  try {
    const appWindow = getCurrentWindow();

    const isFs = await appWindow.isFullscreen();
    setFullscreen(isFs);

    // FS-PTY-008: intercept WM close button to check for active non-shell processes.
    //
    // Tauri 2 pattern: onCloseRequested wrapper calls this.destroy() automatically
    // if the handler does NOT call event.preventDefault(). So:
    //   - No active processes → don't prevent → wrapper calls destroy() → window closes.
    //   - Active processes → prevent → show dialog → user confirms → destroy() manually.
    //
    // Never use close() for programmatic closes: close() re-emits CloseRequested,
    // and if no listener calls destroy() in response, the window stays open.
    const closeUnlisten = await appWindow.onCloseRequested(async (event) => {
      const allPanes = sessionState.tabs.flatMap((tab) => collectLeafPanes(tab.layout));
      // .catch(() => false): IPC error → treat as no foreground process (fail-open, allows close)
      const activeFlags = await Promise.all(
        allPanes.map((p) => hasForegroundProcess(p.paneId).catch(() => false)),
      );
      const activeCount = activeFlags.filter(Boolean).length;

      if (activeCount > 0) {
        event.preventDefault();
        bag.pendingWindowClose = { paneCount: activeCount };
      }
      // activeCount === 0: don't prevent → Tauri wrapper calls destroy() automatically.
    });
    // Expose the unlisten handle on the state bag so the orchestrator can
    // remove it before calling destroy() in close-confirmation handlers.
    bag.closeUnlisten = closeUnlisten;
  } catch {
    /* non-fatal — Tauri window APIs unavailable in test/non-Tauri environments */
  }

  // ── IPC event listeners ───────────────────────────────────────────────────

  unlistens.push(await onSessionStateChanged(applySessionDelta));
  unlistens.push(await onHostKeyPrompt(setHostKeyPrompt));
  unlistens.push(await onCredentialPrompt(setCredentialPrompt));
  unlistens.push(
    await onSshStateChanged((ev) => {
      applySshStateChanged(ev);
    }),
  );
  unlistens.push(
    await onNotificationChanged(async (ev) => {
      const action = applyNotificationChanged(ev);
      if (action?.type === 'autoClose') {
        await doClosePane(action.paneId);
      }
    }),
  );
  unlistens.push(
    await onModeStateChanged((mode) => {
      setBracketedPaste(mode.paneId, mode.bracketedPaste);
    }),
  );
  // Listen for WM-driven fullscreen changes.
  // The backend emits this event after a 200 ms delay to let the WM stabilise
  // the window geometry before the frontend reads dimensions. Restoring focus
  // here (rather than in the onclick handler) ensures the element is focused
  // only once the window is in its final state.
  unlistens.push(
    await onFullscreenStateChanged((ev) => {
      setFullscreen(ev.isFullscreen);
      if (!document.querySelector('[role="dialog"][aria-modal="true"]')) {
        bag.activeViewportEl?.focus({ preventScroll: true });
      }
    }),
  );

  return unlistens;
}
