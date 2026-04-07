// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView composable — reactive logic extracted from TerminalView.svelte.
 *
 * Manages:
 *   - IPC event subscription lifecycle (session, ssh, notifications, mode state)
 *   - Session state delta merge via state/session.svelte
 *   - SSH auth prompts via state/ssh.svelte
 *   - Notification tracking via state/notifications.svelte
 *   - Preferences loading and updates via state/preferences.svelte
 *   - Search state
 *   - Close confirmation dialog state
 *   - Terminal dimensions visibility (status bar)
 *   - Shortcut resolution helpers
 *
 * Returns a reactive object whose properties are read by TerminalView.svelte.
 * All IPC commands are invoked via $lib/ipc/commands.
 *
 * Separation of concerns:
 *   - This file: reactive logic, IPC orchestration, state transitions
 *   - TerminalView.svelte: template markup and DOM event binding
 */

import { onMount, onDestroy } from 'svelte';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  getSessionState,
  createTab,
  closeTab,
  splitPane,
  closePane,
  setActivePane,
  setActiveTab,
  getPreferences,
  updatePreferences,
  getConnections,
  saveConnection,
  deleteConnection,
  openSshConnection,
  closeSshConnection,
  acceptHostKey,
  rejectHostKey,
  provideCredentials,
  getClipboard,
  sendInput,
  searchPane,
  markContextMenuUsed,
  toggleFullscreen as ipcToggleFullscreen,
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
  setInitialSession,
  applySessionDelta,
  removeTab,
  addTab,
  updateTab,
  setActiveTabId,
  getActiveTab,
  collectLeafPanes,
  getActivePanes,
  findNeighbourPaneId,
} from '$lib/state/session.svelte';
import {
  sshStates,
  hostKeyPrompt,
  credentialPrompt,
  setHostKeyPrompt,
  clearHostKeyPrompt,
  setCredentialPrompt,
  clearCredentialPrompt,
  applySshStateChanged,
  setBracketedPaste,
  getBracketedPaste,
} from '$lib/state/ssh.svelte';
import {
  tabNotifications,
  terminatedPanes,
  applyNotificationChanged,
  clearTabNotification,
} from '$lib/state/notifications.svelte';
import { preferences, setPreferences, setPreferencesFallback } from '$lib/state/preferences.svelte';
import { fullscreenState, setFullscreen } from '$lib/state/fullscreen.svelte';
import { applyPreferencesUpdate } from '$lib/preferences/applyUpdate';
import { applyLocaleChange } from '$lib/state/locale.svelte';
import { pasteToBytes } from '$lib/terminal/paste.js';
import type {
  TabState,
  PaneId,
  TabId,
  SshConnectionConfig,
  SearchQuery,
  SearchMatch,
  PreferencesPatch,
} from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Re-export state references so TerminalView.svelte can bind to them directly
// ---------------------------------------------------------------------------

export {
  sessionState,
  sshStates,
  hostKeyPrompt,
  credentialPrompt,
  tabNotifications,
  terminatedPanes,
  preferences,
  fullscreenState,
  getActiveTab,
  getActivePanes,
  collectLeafPanes,
};

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export function useTerminalView() {
  // -------------------------------------------------------------------------
  // Local reactive state (UI-only, not shared across components)
  // -------------------------------------------------------------------------

  let searchOpen = $state(false);
  let searchMatches = $state<SearchMatch[]>([]);
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

  // FS-PTY-008: close confirmation dialog state.
  type PendingClose = { kind: 'tab'; tabId: string } | { kind: 'pane'; paneId: PaneId };
  let pendingClose = $state<PendingClose | null>(null);
  let closeConfirmCancelBtn = $state<HTMLButtonElement | undefined>(undefined);

  // FS-PTY-008: window close confirmation dialog state (WM close button).
  let pendingWindowClose = $state<{ paneCount: number } | null>(null);
  let windowCloseConfirmCancelBtn = $state<HTMLButtonElement | undefined>(undefined);

  // Unlisten-before-close: we remove this listener before calling appWindow.close()
  // so that the CloseRequested event is not intercepted a second time.
  // This is stored outside unlistens[] to allow targeted removal.
  let closeUnlisten: (() => void) | null = null;

  // FS-KBD-003: F2 rename — ID to trigger rename on TabBar, cleared after TabBar acts.
  let requestedRenameTabId = $state<string | null>(null);

  // -------------------------------------------------------------------------
  // Derived
  // -------------------------------------------------------------------------

  const activeThemeLineHeight = $derived(
    preferences.value?.themes.find((t) => t.name === preferences.value?.appearance.themeName)
      ?.lineHeight,
  );

  // -------------------------------------------------------------------------
  // Effects
  // -------------------------------------------------------------------------

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

  // -------------------------------------------------------------------------
  // Mount / destroy
  // -------------------------------------------------------------------------

  let unlistens: Array<() => void> = [];

  onMount(async () => {
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

    try {
      savedConnections = await getConnections();
    } catch {
      // Non-fatal
    }

    // Sync initial fullscreen state and register WM close handler.
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
      closeUnlisten = await appWindow.onCloseRequested(async (event) => {
        const allPanes = sessionState.tabs.flatMap((tab) => collectLeafPanes(tab.layout));
        // .catch(() => false): IPC error → treat as no foreground process (fail-open, allows close)
        const activeFlags = await Promise.all(
          allPanes.map((p) => hasForegroundProcess(p.paneId).catch(() => false)),
        );
        const activeCount = activeFlags.filter(Boolean).length;

        if (activeCount > 0) {
          event.preventDefault();
          pendingWindowClose = { paneCount: activeCount };
        }
        // activeCount === 0: don't prevent → Tauri wrapper calls destroy() automatically.
      });
    } catch {
      /* non-fatal — Tauri window APIs unavailable in test/non-Tauri environments */
    }

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
    // Listen for WM-driven fullscreen changes
    unlistens.push(
      await onFullscreenStateChanged((ev) => {
        setFullscreen(ev.isFullscreen);
      }),
    );
  });

  onDestroy(() => {
    closeUnlisten?.();
    closeUnlisten = null;
    for (const unlisten of unlistens) unlisten();
    unlistens = [];
  });

  // -------------------------------------------------------------------------
  // Tab management
  // -------------------------------------------------------------------------

  async function handleTabClick(tabId: TabId) {
    clearTabNotification(tabId);
    setActiveTabId(tabId);
    try {
      await setActiveTab(tabId);
    } catch {
      // Non-fatal
    }
  }

  async function handleTabClose(tabId: string) {
    const tab = sessionState.tabs.find((t) => t.id === tabId);
    if (!tab) return;
    // FS-PTY-008: check all panes in the tab for non-shell foreground processes.
    const panes = collectLeafPanes(tab.layout);
    // .catch(() => false): IPC error → treat as no foreground process (fail-open, allows close)
    const checks = await Promise.all(
      panes.map((p) => hasForegroundProcess(p.paneId).catch(() => false)),
    );
    if (checks.some(Boolean)) {
      pendingClose = { kind: 'tab', tabId };
      return;
    }
    await doCloseTab(tabId);
  }

  async function doCloseTab(tabId: string) {
    try {
      await closeTab(tabId);
      removeTab(tabId);
      if (sessionState.tabs.length === 0) {
        // destroy() forces the window closed without firing CloseRequested.
        // Using close() here would re-emit CloseRequested; if no JS listener
        // calls destroy() in response, the window would stay open.
        await getCurrentWindow().destroy();
      }
    } catch {
      // Non-fatal
    }
  }

  async function handleNewTab() {
    try {
      const login = sessionState.tabs.length === 0;
      const newTab: TabState = await createTab({ cols: 80, rows: 24, login });
      addTab(newTab);
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Close confirmation dialog
  // -------------------------------------------------------------------------

  async function handleCloseConfirm() {
    const pending = pendingClose;
    pendingClose = null;
    if (!pending) return;
    if (pending.kind === 'tab') {
      await doCloseTab(pending.tabId);
    } else {
      await doClosePane(pending.paneId);
    }
  }

  function handleCloseCancel() {
    pendingClose = null;
  }

  async function handleWindowCloseConfirm() {
    pendingWindowClose = null;
    // destroy() forces close without re-emitting CloseRequested.
    await getCurrentWindow().destroy();
  }

  function handleWindowCloseCancel() {
    pendingWindowClose = null;
  }

  // -------------------------------------------------------------------------
  // Pane actions
  // -------------------------------------------------------------------------

  async function handlePaneClose(paneId: PaneId) {
    // FS-PTY-008: only show dialog when a non-shell foreground process is active.
    // .catch(() => false): IPC error → treat as no foreground process (fail-open, allows close)
    const hasForeground = await hasForegroundProcess(paneId).catch(() => false);
    if (hasForeground) {
      pendingClose = { kind: 'pane', paneId };
      return;
    }
    await doClosePane(paneId);
  }

  async function doClosePane(paneId: PaneId) {
    try {
      // Resolve the owner tab before the IPC call — after closePane returns null,
      // the pane is gone from the registry and we can no longer look it up.
      // Using the active tab would be wrong when auto-closing a background pane.
      const ownerTab = sessionState.tabs.find((t) =>
        collectLeafPanes(t.layout).some((p) => p.paneId === paneId),
      );
      const updatedTab: TabState | null = await closePane(paneId);
      if (updatedTab === null) {
        if (ownerTab) removeTab(ownerTab.id);
      } else {
        updateTab(updatedTab);
      }
    } catch {
      // Non-fatal
    }
  }

  async function handleSplitPane(direction: 'horizontal' | 'vertical') {
    const activePaneId = getActiveTab()?.activePaneId;
    if (!activePaneId) return;
    try {
      const updatedTab: TabState = await splitPane(activePaneId, direction);
      updateTab(updatedTab);
    } catch {
      // Non-fatal
    }
  }

  async function handleNavigatePane(direction: 'left' | 'right' | 'up' | 'down') {
    const targetPaneId = findNeighbourPaneId(direction);
    if (!targetPaneId) return;
    try {
      await setActivePane(targetPaneId);
    } catch {
      // Non-fatal
    }
  }

  function handleSwitchTab(delta: 1 | -1) {
    if (sessionState.tabs.length <= 1) return;
    const sorted = [...sessionState.tabs].sort((a, b) => a.order - b.order);
    const idx = sorted.findIndex((t) => t.id === sessionState.activeTabId);
    if (idx === -1) return;
    const next = sorted[(idx + delta + sorted.length) % sorted.length];
    handleTabClick(next.id);
  }

  // -------------------------------------------------------------------------
  // Connection manager
  // -------------------------------------------------------------------------

  async function handleConnectionSave(config: SshConnectionConfig) {
    try {
      const id: string = await saveConnection(config);
      const exists = savedConnections.some((c) => c.id === id);
      if (exists) {
        savedConnections = savedConnections.map((c) => (c.id === id ? { ...config, id } : c));
      } else {
        savedConnections = [...savedConnections, { ...config, id }];
      }
    } catch {
      // Non-fatal
    }
  }

  async function handleConnectionDelete(connectionId: string) {
    try {
      await deleteConnection(connectionId);
      savedConnections = savedConnections.filter((c) => c.id !== connectionId);
    } catch {
      // Non-fatal
    }
  }

  async function handleConnectionOpen({
    connectionId,
    target,
  }: {
    connectionId: string;
    target: 'tab' | 'pane';
  }) {
    if (target === 'tab') {
      let newTab: TabState;
      try {
        newTab = await createTab({ cols: 80, rows: 24 });
      } catch {
        connectionOpenError = true;
        return;
      }
      addTab(newTab);
      const panes = collectLeafPanes(newTab.layout);
      if (panes.length > 0) {
        try {
          await openSshConnection(panes[0].paneId, connectionId);
        } catch {
          try {
            await closeTab(newTab.id);
          } catch {
            // close_tab also failed
          }
          removeTab(newTab.id);
          connectionOpenError = true;
          return;
        }
      }
    } else {
      const activePaneId = getActiveTab()?.activePaneId;
      if (!activePaneId) return;
      try {
        await openSshConnection(activePaneId, connectionId);
      } catch {
        connectionOpenError = true;
        return;
      }
    }
    connectionManagerOpen = false;
  }

  // -------------------------------------------------------------------------
  // SSH auth handlers
  // -------------------------------------------------------------------------

  async function handleAcceptHostKey() {
    const prompt = clearHostKeyPrompt();
    if (!prompt) return;
    try {
      await acceptHostKey(prompt.paneId);
    } catch {
      /* non-fatal */
    }
  }

  async function handleRejectHostKey() {
    const prompt = clearHostKeyPrompt();
    if (!prompt) return;
    try {
      await rejectHostKey(prompt.paneId);
    } catch {
      /* non-fatal */
    }
  }

  async function handleProvideCredentials(password: string) {
    const prompt = clearCredentialPrompt();
    if (!prompt) return;
    try {
      await provideCredentials(prompt.paneId, { username: prompt.username, password });
    } catch {
      /* non-fatal */
    }
  }

  async function handleCancelCredentials() {
    const prompt = clearCredentialPrompt();
    if (!prompt) return;
    try {
      await closeSshConnection(prompt.paneId);
    } catch {
      /* non-fatal */
    }
  }

  // -------------------------------------------------------------------------
  // Global paste (Ctrl+Shift+V)
  // -------------------------------------------------------------------------

  async function handleGlobalPaste() {
    const activePaneId = getActiveTab()?.activePaneId;
    if (!activePaneId) return;
    try {
      const text: string = await getClipboard();
      if (!text) return;
      const isBracketed = getBracketedPaste(activePaneId);
      const encoded = pasteToBytes(text, isBracketed);
      if (!encoded) return;
      await sendInput(activePaneId, Array.from(encoded));
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Search
  // -------------------------------------------------------------------------

  async function handleSearch(query: SearchQuery) {
    const activePaneId = getActiveTab()?.activePaneId;
    if (!activePaneId) return;
    try {
      searchMatches = await searchPane(activePaneId, query);
      searchCurrentIdx = searchMatches.length > 0 ? 1 : 0;
    } catch {
      searchMatches = [];
      searchCurrentIdx = 0;
    }
  }

  function handleSearchNext() {
    if (searchMatches.length === 0) return;
    searchCurrentIdx = (searchCurrentIdx % searchMatches.length) + 1;
  }

  function handleSearchPrev() {
    if (searchMatches.length === 0) return;
    searchCurrentIdx = searchCurrentIdx <= 1 ? searchMatches.length : searchCurrentIdx - 1;
  }

  function handleSearchClose() {
    searchOpen = false;
    searchMatches = [];
    searchCurrentIdx = 0;
  }

  // -------------------------------------------------------------------------
  // Context menu hint (FS-UX-002)
  // -------------------------------------------------------------------------

  async function handleContextMenuHintDismiss() {
    if (!contextMenuHintVisible) return;
    contextMenuHintVisible = false;
    contextMenuHintDismissed = true;
    try {
      await markContextMenuUsed();
      if (preferences.value !== undefined) {
        setPreferences({
          ...preferences.value,
          appearance: { ...preferences.value.appearance, contextMenuHintShown: true },
        });
      }
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Preferences
  // -------------------------------------------------------------------------

  async function handlePreferencesUpdate(patch: PreferencesPatch) {
    try {
      const updated = await applyPreferencesUpdate(
        patch,
        (_cmd, args) => updatePreferences((args as { patch: PreferencesPatch }).patch),
        applyLocaleChange,
      );
      setPreferences(updated);
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Shortcut resolution (FS-KBD-002)
  // -------------------------------------------------------------------------

  const defaultShortcuts: Record<string, string> = {
    new_tab: 'Ctrl+Shift+T',
    close_tab: 'Ctrl+Shift+W',
    paste: 'Ctrl+Shift+V',
    search: 'Ctrl+Shift+F',
    preferences: 'Ctrl+,',
    next_tab: 'Ctrl+Tab',
    prev_tab: 'Ctrl+Shift+Tab',
    rename_tab: 'F2',
    toggle_fullscreen: 'F11',
    split_pane_h: 'Ctrl+Shift+D',
    split_pane_v: 'Ctrl+Shift+E',
    close_pane: 'Ctrl+Shift+Q',
    navigate_pane_left: 'Ctrl+Shift+ArrowLeft',
    navigate_pane_right: 'Ctrl+Shift+ArrowRight',
    navigate_pane_up: 'Ctrl+Shift+ArrowUp',
    navigate_pane_down: 'Ctrl+Shift+ArrowDown',
  };

  function effectiveShortcut(actionId: string): string {
    return preferences.value?.keyboard?.bindings?.[actionId] ?? defaultShortcuts[actionId] ?? '';
  }

  function matchesShortcut(event: KeyboardEvent, shortcut: string): boolean {
    if (!shortcut) return false;
    const parts = shortcut.split('+');
    const requiredCtrl = parts.includes('Ctrl');
    const requiredAlt = parts.includes('Alt');
    const requiredShift = parts.includes('Shift');
    const keyParts = parts.filter((p) => !['Ctrl', 'Alt', 'Shift', 'Meta'].includes(p));
    if (keyParts.length !== 1) return false;
    const requiredKey = keyParts[0];
    if (event.ctrlKey !== requiredCtrl) return false;
    if (event.altKey !== requiredAlt) return false;
    if (event.shiftKey !== requiredShift) return false;
    const eventKey = event.key;
    if (requiredKey.length === 1) {
      return eventKey.toLowerCase() === requiredKey.toLowerCase();
    }
    return eventKey === requiredKey;
  }

  async function handleToggleFullscreen(): Promise<void> {
    try {
      const result = await ipcToggleFullscreen();
      setFullscreen(result.isFullscreen);
    } catch {
      /* non-fatal */
    }
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if ((event.target as Element)?.closest?.('[role="dialog"], [role="alertdialog"]')) return;

    if (matchesShortcut(event, effectiveShortcut('toggle_fullscreen'))) {
      event.preventDefault();
      handleToggleFullscreen();
      return;
    }

    if (matchesShortcut(event, effectiveShortcut('new_tab'))) {
      event.preventDefault();
      handleNewTab();
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('close_tab'))) {
      event.preventDefault();
      if (sessionState.activeTabId) handleTabClose(sessionState.activeTabId);
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('search'))) {
      event.preventDefault();
      searchOpen = true;
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('paste'))) {
      event.preventDefault();
      handleGlobalPaste();
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('preferences'))) {
      event.preventDefault();
      prefsOpen = true;
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('next_tab'))) {
      event.preventDefault();
      handleSwitchTab(1);
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('prev_tab'))) {
      event.preventDefault();
      handleSwitchTab(-1);
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('rename_tab'))) {
      event.preventDefault();
      if (sessionState.activeTabId) {
        requestedRenameTabId = sessionState.activeTabId;
      }
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('split_pane_h'))) {
      event.preventDefault();
      handleSplitPane('horizontal');
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('split_pane_v'))) {
      event.preventDefault();
      handleSplitPane('vertical');
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('close_pane'))) {
      event.preventDefault();
      const activePaneId = getActiveTab()?.activePaneId;
      if (activePaneId) handlePaneClose(activePaneId);
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('navigate_pane_left'))) {
      event.preventDefault();
      handleNavigatePane('left');
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('navigate_pane_right'))) {
      event.preventDefault();
      handleNavigatePane('right');
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('navigate_pane_up'))) {
      event.preventDefault();
      handleNavigatePane('up');
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('navigate_pane_down'))) {
      event.preventDefault();
      handleNavigatePane('down');
      return;
    }
  }

  // -------------------------------------------------------------------------
  // Dimensions callback from TerminalPane
  // -------------------------------------------------------------------------

  function handleDimensionsChange(paneId: PaneId, c: number, r: number) {
    const activeTab = getActiveTab();
    if (paneId === activeTab?.activePaneId) {
      activePaneCols = c;
      activePaneRows = r;
    }
  }

  // -------------------------------------------------------------------------
  // Return all state and handlers needed by TerminalView.svelte
  // -------------------------------------------------------------------------

  return {
    // State (read-only from the template)
    get searchOpen() {
      return searchOpen;
    },
    set searchOpen(v: boolean) {
      searchOpen = v;
    },
    get searchMatches() {
      return searchMatches;
    },
    get searchCurrentIdx() {
      return searchCurrentIdx;
    },
    get prefsOpen() {
      return prefsOpen;
    },
    set prefsOpen(v: boolean) {
      prefsOpen = v;
    },
    get activePaneCols() {
      return activePaneCols;
    },
    get activePaneRows() {
      return activePaneRows;
    },
    get dimsVisible() {
      return dimsVisible;
    },
    get connectionManagerOpen() {
      return connectionManagerOpen;
    },
    set connectionManagerOpen(v: boolean) {
      connectionManagerOpen = v;
    },
    get savedConnections() {
      return savedConnections;
    },
    get connectionOpenError() {
      return connectionOpenError;
    },
    set connectionOpenError(v: boolean) {
      connectionOpenError = v;
    },
    get contextMenuHintVisible() {
      return contextMenuHintVisible;
    },
    get pendingClose() {
      return pendingClose;
    },
    get closeConfirmCancelBtn() {
      return closeConfirmCancelBtn;
    },
    set closeConfirmCancelBtn(v: HTMLButtonElement | undefined) {
      closeConfirmCancelBtn = v;
    },
    get pendingWindowClose() {
      return pendingWindowClose;
    },
    get windowCloseConfirmCancelBtn() {
      return windowCloseConfirmCancelBtn;
    },
    set windowCloseConfirmCancelBtn(v: HTMLButtonElement | undefined) {
      windowCloseConfirmCancelBtn = v;
    },
    get requestedRenameTabId() {
      return requestedRenameTabId;
    },
    set requestedRenameTabId(v: string | null) {
      requestedRenameTabId = v;
    },
    get activeThemeLineHeight() {
      return activeThemeLineHeight;
    },

    get isFullscreen() {
      return fullscreenState.value;
    },

    // Handlers
    handleTabClick,
    handleTabClose,
    handleNewTab,
    handleCloseConfirm,
    handleCloseCancel,
    handleWindowCloseConfirm,
    handleWindowCloseCancel,
    handlePaneClose,
    handleSplitPane,
    handleNavigatePane,
    handleSwitchTab,
    handleConnectionSave,
    handleConnectionDelete,
    handleConnectionOpen,
    handleAcceptHostKey,
    handleRejectHostKey,
    handleProvideCredentials,
    handleCancelCredentials,
    handleGlobalPaste,
    handleSearch,
    handleSearchNext,
    handleSearchPrev,
    handleSearchClose,
    handleContextMenuHintDismiss,
    handlePreferencesUpdate,
    handleGlobalKeydown,
    handleDimensionsChange,
    handleToggleFullscreen,
  };
}
