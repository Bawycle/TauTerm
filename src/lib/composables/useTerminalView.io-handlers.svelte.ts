// SPDX-License-Identifier: MPL-2.0

/**
 * useTerminalView — SSH auth handlers, connection manager handlers, paste,
 * search, context menu hint, preferences, keyboard shortcuts, and
 * handleGlobalKeydown.
 *
 * All handlers close over the ViewState bag returned by createViewState().
 * No runes are declared here — state lives entirely in core.svelte.ts.
 */

import {
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
  createTab,
  closeTab,
} from '$lib/ipc/commands';
import {
  clearHostKeyPrompt,
  clearCredentialPrompt,
  getBracketedPaste,
} from '$lib/state/ssh.svelte';
import { preferences, setPreferences } from '$lib/state/preferences.svelte';
import { setFullscreen } from '$lib/state/fullscreen.svelte';
import { applyLocaleChange } from '$lib/state/locale.svelte';
import { applyPreferencesUpdate } from '$lib/preferences/applyUpdate';
import { pasteToBytes } from '$lib/terminal/paste.js';
import {
  sessionState,
  getActiveTab,
  collectLeafPanes,
  addTab,
  removeTab,
} from '$lib/state/session.svelte';
import type {
  SshConnectionConfig,
  SearchQuery,
  SearchMatch,
  PreferencesPatch,
} from '$lib/ipc/types';
import type { ViewState } from './useTerminalView.core.svelte';

export function createIoHandlers(
  s: ViewState,
  handleSwitchTab: (delta: 1 | -1) => void,
  handleNewTab: () => Promise<void>,
  handleTabClose: (tabId: string) => Promise<void>,
  handleSplitPane: (direction: 'horizontal' | 'vertical') => Promise<void>,
  handlePaneClose: (paneId: string) => Promise<void>,
  handleNavigatePane: (direction: 'left' | 'right' | 'up' | 'down') => Promise<void>,
  handleToggleFullscreen: () => Promise<void>,
) {
  // -------------------------------------------------------------------------
  // Connection manager
  // -------------------------------------------------------------------------

  async function handleConnectionSave(config: SshConnectionConfig) {
    try {
      const id: string = await saveConnection(config);
      const exists = s.savedConnections.some((c) => c.id === id);
      if (exists) {
        s.savedConnections = s.savedConnections.map((c) => (c.id === id ? { ...config, id } : c));
      } else {
        s.savedConnections = [...s.savedConnections, { ...config, id }];
      }
    } catch {
      // Non-fatal
    }
  }

  async function handleConnectionDelete(connectionId: string) {
    try {
      await deleteConnection(connectionId);
      s.savedConnections = s.savedConnections.filter((c) => c.id !== connectionId);
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
      let newTab: import('$lib/ipc/types').TabState;
      try {
        newTab = await createTab({ cols: 80, rows: 24 });
      } catch {
        s.connectionOpenError = true;
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
          s.connectionOpenError = true;
          return;
        }
      }
    } else {
      const activePaneId = getActiveTab()?.activePaneId;
      if (!activePaneId) return;
      try {
        await openSshConnection(activePaneId, connectionId);
      } catch {
        s.connectionOpenError = true;
        return;
      }
    }
    s.connectionManagerOpen = false;
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
      s.searchMatches = await searchPane(activePaneId, query);
      s.searchCurrentIdx = s.searchMatches.length > 0 ? 1 : 0;
    } catch {
      s.searchMatches = [];
      s.searchCurrentIdx = 0;
    }
  }

  function handleSearchNext() {
    if (s.searchMatches.length === 0) return;
    s.searchCurrentIdx = (s.searchCurrentIdx % s.searchMatches.length) + 1;
  }

  function handleSearchPrev() {
    if (s.searchMatches.length === 0) return;
    s.searchCurrentIdx = s.searchCurrentIdx <= 1 ? s.searchMatches.length : s.searchCurrentIdx - 1;
  }

  function handleSearchClose() {
    s.searchOpen = false;
    s.searchMatches = [];
    s.searchCurrentIdx = 0;
    s.activeViewportEl?.focus({ preventScroll: true });
  }

  // -------------------------------------------------------------------------
  // Context menu hint (FS-UX-002)
  // -------------------------------------------------------------------------

  async function handleContextMenuHintDismiss() {
    if (!s.contextMenuHintVisible) return;
    s.contextMenuHintVisible = false;
    s.contextMenuHintDismissed = true;
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
      s.searchOpen = true;
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('paste'))) {
      event.preventDefault();
      handleGlobalPaste();
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('preferences'))) {
      event.preventDefault();
      s.prefsOpen = true;
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
        s.requestedRenameTabId = sessionState.activeTabId;
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

  return {
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
  };
}
