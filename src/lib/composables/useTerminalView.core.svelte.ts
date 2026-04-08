// SPDX-License-Identifier: MPL-2.0

/**
 * useTerminalView — core state, derived, effects, mount/destroy, IPC listeners.
 *
 * Called synchronously from useTerminalView() during component initialisation so
 * that $state / $derived / $effect / onMount / onDestroy are all in the correct
 * Svelte 5 reactive scope.
 */

import { onMount, onDestroy } from 'svelte';
import {
  getSessionState,
  getPreferences,
} from '$lib/ipc/commands';
import {
  setInitialSession,
} from '$lib/state/session.svelte';
import {
  setPreferences, setPreferencesFallback, preferences,
} from '$lib/state/preferences.svelte';
import { fullscreenState } from '$lib/state/fullscreen.svelte';
import { setupViewListeners } from './useTerminalView.lifecycle.svelte';
import type { SshConnectionConfig, PaneId } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// State bag — all reactive variables owned by this module, returned as a
// plain object of getters/setters so handlers in other modules can close over
// them without breaking reactivity.
// ---------------------------------------------------------------------------

export interface ViewState {
  // Search
  get searchOpen(): boolean;
  set searchOpen(v: boolean);
  get searchMatches(): import('$lib/ipc/types').SearchMatch[];
  set searchMatches(v: import('$lib/ipc/types').SearchMatch[]);
  get searchCurrentIdx(): number;
  set searchCurrentIdx(v: number);

  // Preferences panel
  get prefsOpen(): boolean;
  set prefsOpen(v: boolean);

  // Dimensions overlay
  get activePaneCols(): number | null;
  set activePaneCols(v: number | null);
  get activePaneRows(): number | null;
  set activePaneRows(v: number | null);
  get dimsVisible(): boolean;

  // Connection manager
  get connectionManagerOpen(): boolean;
  set connectionManagerOpen(v: boolean);
  get savedConnections(): SshConnectionConfig[];
  set savedConnections(v: SshConnectionConfig[]);
  get connectionOpenError(): boolean;
  set connectionOpenError(v: boolean);

  // Context menu hint
  get contextMenuHintVisible(): boolean;
  set contextMenuHintVisible(v: boolean);
  get contextMenuHintDismissed(): boolean;
  set contextMenuHintDismissed(v: boolean);

  // Close confirmation (tab/pane)
  get pendingClose(): PendingClose | null;
  set pendingClose(v: PendingClose | null);
  get closeConfirmCancelBtn(): HTMLButtonElement | undefined;
  set closeConfirmCancelBtn(v: HTMLButtonElement | undefined);

  // Close confirmation (window)
  get pendingWindowClose(): { paneCount: number } | null;
  set pendingWindowClose(v: { paneCount: number } | null);
  get windowCloseConfirmCancelBtn(): HTMLButtonElement | undefined;
  set windowCloseConfirmCancelBtn(v: HTMLButtonElement | undefined);

  // Tab rename
  get requestedRenameTabId(): string | null;
  set requestedRenameTabId(v: string | null);

  // Focus management
  get activeViewportEl(): HTMLElement | null;
  set activeViewportEl(v: HTMLElement | null);

  // Derived
  get activeThemeLineHeight(): number | undefined;
  get isFullscreen(): boolean;

  // Unlisten handle for WM close (needs targeted removal before window.destroy())
  closeUnlisten: (() => void) | null;
}

export type PendingClose = { kind: 'tab'; tabId: string } | { kind: 'pane'; paneId: PaneId };

// ---------------------------------------------------------------------------
// Factory — must be called synchronously from the composable entry point so
// Svelte 5 runes are initialised in the component's reactive scope.
// ---------------------------------------------------------------------------

export function createViewState(doClosePane: (paneId: PaneId) => Promise<void>): ViewState {
  // --- Local reactive variables ---

  let searchOpen = $state(false);
  let searchMatches = $state<import('$lib/ipc/types').SearchMatch[]>([]);
  let searchCurrentIdx = $state(0);

  let prefsOpen = $state(false);

  let activePaneCols = $state<number | null>(null);
  let activePaneRows = $state<number | null>(null);
  let dimsVisible = $state(false);
  let dimsHideTimer: ReturnType<typeof setTimeout> | null = null;

  let connectionManagerOpen = $state(false);
  let savedConnections = $state<SshConnectionConfig[]>([]);
  let connectionOpenError = $state(false);

  let contextMenuHintVisible = $state(false);
  let contextMenuHintDismissed = $state(false);
  let contextMenuHintTimer: ReturnType<typeof setTimeout> | null = null;

  let pendingClose = $state<PendingClose | null>(null);
  let closeConfirmCancelBtn = $state<HTMLButtonElement | undefined>(undefined);

  let pendingWindowClose = $state<{ paneCount: number } | null>(null);
  let windowCloseConfirmCancelBtn = $state<HTMLButtonElement | undefined>(undefined);

  let requestedRenameTabId = $state<string | null>(null);

  let activeViewportEl = $state<HTMLElement | null>(null);

  // --- Derived ---

  const activeThemeLineHeight = $derived(
    preferences.value?.themes.find((t) => t.name === preferences.value?.appearance.themeName)
      ?.lineHeight,
  );

  // --- Effects ---

  $effect(() => {
    const _c = activePaneCols;
    const _r = activePaneRows;
    if (_c === null || _r === null) return;
    dimsVisible = true;
    if (dimsHideTimer !== null) clearTimeout(dimsHideTimer);
    dimsHideTimer = setTimeout(() => {
      dimsVisible = false;
      dimsHideTimer = null;
    }, 2000);
    return () => {
      if (dimsHideTimer !== null) {
        clearTimeout(dimsHideTimer);
        dimsHideTimer = null;
      }
    };
  });

  $effect(() => {
    if (
      preferences.value !== undefined &&
      !preferences.value.appearance.contextMenuHintShown &&
      !contextMenuHintDismissed
    ) {
      contextMenuHintTimer = setTimeout(() => {
        contextMenuHintVisible = true;
      }, 2000);
    }
    return () => {
      if (contextMenuHintTimer) {
        clearTimeout(contextMenuHintTimer);
        contextMenuHintTimer = null;
      }
    };
  });

  // --- Focus guard (safety net) ---
  // Recaptures focus to the active terminal viewport whenever focus lands on
  // document.body (e.g. after a transient element like ScrollToBottomButton
  // disappears). Skipped when a modal dialog is open.

  function onFocusIn(e: FocusEvent) {
    if (e.target !== document.body) return;
    if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    const el = activeViewportEl;
    if (!el || !document.contains(el)) return;
    el.focus({ preventScroll: true });
  }

  // --- Mount / Destroy ---

  let unlistens: Array<() => void> = [];
  let closeUnlisten: (() => void) | null = null;

  onMount(async () => {
    document.addEventListener('focusin', onFocusIn, { capture: true });
    try {
      const state = await getSessionState();
      setInitialSession(state);
    } catch {
      // Backend not ready — populated by first session-state-changed event
    }

    try {
      const prefs = await getPreferences();
      setPreferences(prefs);
    } catch {
      setPreferencesFallback();
    }

    const listenerUnlistens = await setupViewListeners(bag, doClosePane);
    for (const u of listenerUnlistens) unlistens.push(u);
  });

  onDestroy(() => {
    document.removeEventListener('focusin', onFocusIn, { capture: true });
    bag.closeUnlisten?.();
    bag.closeUnlisten = null;
    for (const unlisten of unlistens) unlisten();
    unlistens = [];
  });

  // --- State bag (getter/setter object) ---

  const bag: ViewState = {
    get searchOpen() {
      return searchOpen;
    },
    set searchOpen(v) {
      searchOpen = v;
    },
    get searchMatches() {
      return searchMatches;
    },
    set searchMatches(v) {
      searchMatches = v;
    },
    get searchCurrentIdx() {
      return searchCurrentIdx;
    },
    set searchCurrentIdx(v) {
      searchCurrentIdx = v;
    },

    get prefsOpen() {
      return prefsOpen;
    },
    set prefsOpen(v) {
      prefsOpen = v;
    },

    get activePaneCols() {
      return activePaneCols;
    },
    set activePaneCols(v) {
      activePaneCols = v;
    },
    get activePaneRows() {
      return activePaneRows;
    },
    set activePaneRows(v) {
      activePaneRows = v;
    },
    get dimsVisible() {
      return dimsVisible;
    },

    get connectionManagerOpen() {
      return connectionManagerOpen;
    },
    set connectionManagerOpen(v) {
      connectionManagerOpen = v;
    },
    get savedConnections() {
      return savedConnections;
    },
    set savedConnections(v) {
      savedConnections = v;
    },
    get connectionOpenError() {
      return connectionOpenError;
    },
    set connectionOpenError(v) {
      connectionOpenError = v;
    },

    get contextMenuHintVisible() {
      return contextMenuHintVisible;
    },
    set contextMenuHintVisible(v) {
      contextMenuHintVisible = v;
    },
    get contextMenuHintDismissed() {
      return contextMenuHintDismissed;
    },
    set contextMenuHintDismissed(v) {
      contextMenuHintDismissed = v;
    },

    get pendingClose() {
      return pendingClose;
    },
    set pendingClose(v) {
      pendingClose = v;
    },
    get closeConfirmCancelBtn() {
      return closeConfirmCancelBtn;
    },
    set closeConfirmCancelBtn(v) {
      closeConfirmCancelBtn = v;
    },

    get pendingWindowClose() {
      return pendingWindowClose;
    },
    set pendingWindowClose(v) {
      pendingWindowClose = v;
    },
    get windowCloseConfirmCancelBtn() {
      return windowCloseConfirmCancelBtn;
    },
    set windowCloseConfirmCancelBtn(v) {
      windowCloseConfirmCancelBtn = v;
    },

    get requestedRenameTabId() {
      return requestedRenameTabId;
    },
    set requestedRenameTabId(v) {
      requestedRenameTabId = v;
    },

    get activeViewportEl() {
      return activeViewportEl;
    },
    set activeViewportEl(v) {
      activeViewportEl = v;
    },

    get activeThemeLineHeight() {
      return activeThemeLineHeight;
    },
    get isFullscreen() {
      return fullscreenState.value;
    },

    closeUnlisten,
  };

  return bag;
}
