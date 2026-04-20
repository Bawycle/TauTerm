// SPDX-License-Identifier: MPL-2.0

/**
 * useTerminalView — tab/pane session handlers, close confirmation handlers,
 * and dimensions callback.
 *
 * All handlers close over the ViewState bag returned by createViewState().
 * No runes are declared here — state lives entirely in core.svelte.ts.
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  createTab,
  closeTab,
  splitPane,
  closePane,
  setActivePane,
  setActiveTab,
  hasForegroundProcess,
  setPaneLabel,
} from '$lib/ipc';
import {
  sessionState,
  removeTab,
  addTab,
  updateTab,
  setActiveTabId,
  getActiveTab,
  collectLeafPanes,
  findNeighbourPaneId,
} from '$lib/state/session.svelte';
import { clearTabNotification } from '$lib/state/notifications.svelte';
import type { TabState, PaneId, TabId } from '$lib/ipc';
import type { ViewState } from './useTerminalView.core.svelte';

export function createSessionHandlers(s: ViewState) {
  // -------------------------------------------------------------------------
  // Internal helpers (shared between tab and pane handlers)
  // -------------------------------------------------------------------------

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
        if (ownerTab) {
          removeTab(ownerTab.id);
          if (sessionState.tabs.length === 0) {
            await getCurrentWindow().destroy();
          }
        }
      } else {
        updateTab(updatedTab);
      }
    } catch {
      // Non-fatal
    }
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
      s.pendingClose = { kind: 'tab', tabId };
      return;
    }
    await doCloseTab(tabId);
  }

  async function handleNewTab() {
    try {
      const login = sessionState.tabs.length === 0;
      const activeTab = getActiveTab();
      const sourcePaneId = activeTab?.activePaneId;
      const newTab: TabState = await createTab({
        label: null,
        cols: 80,
        rows: 24,
        login,
        sourcePaneId,
      });
      addTab(newTab);
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Close confirmation dialog
  // -------------------------------------------------------------------------

  async function handleCloseConfirm() {
    const pending = s.pendingClose;
    s.pendingClose = null;
    if (!pending) return;
    if (pending.kind === 'tab') {
      await doCloseTab(pending.tabId);
    } else {
      await doClosePane(pending.paneId);
    }
  }

  function handleCloseCancel() {
    s.pendingClose = null;
  }

  async function handleWindowCloseConfirm() {
    s.pendingWindowClose = null;
    // destroy() forces close without re-emitting CloseRequested.
    await getCurrentWindow().destroy();
  }

  function handleWindowCloseCancel() {
    s.pendingWindowClose = null;
  }

  // -------------------------------------------------------------------------
  // Pane actions
  // -------------------------------------------------------------------------

  async function handlePaneClose(paneId: PaneId) {
    // FS-PTY-008: only show dialog when a non-shell foreground process is active.
    // .catch(() => false): IPC error → treat as no foreground process (fail-open, allows close)
    const hasForeground = await hasForegroundProcess(paneId).catch(() => false);
    if (hasForeground) {
      s.pendingClose = { kind: 'pane', paneId };
      return;
    }
    await doClosePane(paneId);
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
  // Pane rename
  // -------------------------------------------------------------------------

  async function handlePaneRename(paneId: PaneId, label: string | null) {
    try {
      const updatedTab = await setPaneLabel(paneId, label);
      updateTab(updatedTab);
    } catch {
      // Non-fatal: title stays unchanged
    }
  }

  // -------------------------------------------------------------------------
  // Dimensions callback from TerminalPane
  // -------------------------------------------------------------------------

  function handleDimensionsChange(paneId: PaneId, c: number, r: number) {
    const activeTab = getActiveTab();
    if (paneId === activeTab?.activePaneId) {
      s.activePaneCols = c;
      s.activePaneRows = r;
    }
  }

  return {
    doClosePane,
    doCloseTab,
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
    handleDimensionsChange,
    handlePaneRename,
  };
}
