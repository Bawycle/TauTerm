// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive session state — replica of the backend's SessionState.
 *
 * Single source of truth for all tab/pane topology in the frontend.
 * Updated by merging SessionStateChangedEvent deltas from the backend.
 *
 * Usage: import `sessionState` and read `.tabs` / `.activeTabId`.
 * Mutations go through the exported helper functions (applyDelta, setInitial).
 * Do NOT mutate the exported object directly from components.
 */

import type {
  SessionState,
  SessionStateChangedEvent,
  TabState,
  TabId,
  PaneId,
  PaneNode,
  PaneState,
} from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Reactive state — module-level singleton
// ---------------------------------------------------------------------------

export const sessionState = $state<{
  tabs: TabState[];
  activeTabId: TabId;
}>({
  tabs: [],
  activeTabId: '',
});

// ---------------------------------------------------------------------------
// Initialisation
// ---------------------------------------------------------------------------

/**
 * Populate the session state from a full snapshot returned by get_session_state.
 */
export function setInitialSession(state: SessionState): void {
  sessionState.tabs = state.tabs;
  sessionState.activeTabId = state.activeTabId;
}

// ---------------------------------------------------------------------------
// Delta merge — applies a SessionStateChangedEvent to the reactive state
// ---------------------------------------------------------------------------

/**
 * Apply a SessionStateChangedEvent delta to the reactive session state.
 * Mirrors the logic previously embedded in TerminalView.onMount's listen handler.
 *
 * The event is a discriminated union — switch on `change.type` for exhaustive
 * handling. TypeScript narrows each branch automatically.
 */
export function applySessionDelta(change: SessionStateChangedEvent): void {
  switch (change.type) {
    case 'tabCreated':
      sessionState.tabs = [...sessionState.tabs.filter((t) => t.id !== change.tab.id), change.tab];
      break;

    case 'tabClosed':
      sessionState.tabs = sessionState.tabs.filter((t) => t.id !== change.closedTabId);
      if (change.activeTabId !== undefined) {
        sessionState.activeTabId = change.activeTabId;
      } else if (sessionState.tabs.length === 0) {
        sessionState.activeTabId = '';
      }
      break;

    case 'tabReordered':
    case 'paneMetadataChanged':
    case 'activePaneChanged':
      sessionState.tabs = sessionState.tabs.map((t) => (t.id === change.tab.id ? change.tab : t));
      break;

    case 'activeTabChanged':
      sessionState.activeTabId = change.activeTabId;
      sessionState.tabs = sessionState.tabs.map((t) => (t.id === change.tab.id ? change.tab : t));
      break;
  }
}

// ---------------------------------------------------------------------------
// Optimistic updates — used when the frontend acts before the backend responds
// ---------------------------------------------------------------------------

/**
 * Remove a tab from local state immediately (optimistic close).
 * The backend confirms via tab-closed event; this prevents flicker.
 */
export function removeTab(tabId: TabId): void {
  sessionState.tabs = sessionState.tabs.filter((t) => t.id !== tabId);
  if (sessionState.activeTabId === tabId) {
    const remaining = sessionState.tabs;
    sessionState.activeTabId = remaining[remaining.length - 1]?.id ?? '';
  }
}

/**
 * Add a new tab and set it as active (optimistic new-tab).
 */
export function addTab(tab: TabState): void {
  sessionState.tabs = [...sessionState.tabs, tab];
  sessionState.activeTabId = tab.id;
}

/**
 * Update a tab in-place (optimistic pane split / close).
 */
export function updateTab(tab: TabState): void {
  sessionState.tabs = sessionState.tabs.map((t) => (t.id === tab.id ? tab : t));
}

/**
 * Set activeTabId without calling the backend (used for UI-only tab switching
 * when the invoke is fire-and-forget).
 */
export function setActiveTabId(tabId: TabId): void {
  sessionState.activeTabId = tabId;
}

// ---------------------------------------------------------------------------
// Read helpers — pure derivations from sessionState
// ---------------------------------------------------------------------------

/**
 * Returns the currently active TabState, or null if none.
 */
export function getActiveTab(): TabState | null {
  return sessionState.tabs.find((t) => t.id === sessionState.activeTabId) ?? null;
}

/**
 * Collect all leaf pane IDs (and their states) from a PaneNode tree.
 */
export function collectLeafPanes(node: PaneNode): { paneId: PaneId; state: PaneState }[] {
  if (node.type === 'leaf') return [{ paneId: node.paneId, state: node.state }];
  return [...collectLeafPanes(node.first), ...collectLeafPanes(node.second)];
}

/**
 * Returns leaf panes for the active tab, or [] if no active tab.
 */
export function getActivePanes(): { paneId: PaneId; state: PaneState }[] {
  const tab = getActiveTab();
  return tab ? collectLeafPanes(tab.layout) : [];
}

/**
 * Find the pane in a given direction relative to the active pane.
 * Uses flat leaf order from collectLeafPanes: left/up = prev, right/down = next.
 * Returns null if there is no neighbour.
 */
export function findNeighbourPaneId(direction: 'left' | 'right' | 'up' | 'down'): PaneId | null {
  const tab = getActiveTab();
  if (!tab) return null;
  const panes = collectLeafPanes(tab.layout);
  const currentIdx = panes.findIndex((p) => p.paneId === tab.activePaneId);
  if (currentIdx === -1 || panes.length <= 1) return null;
  if (direction === 'left' || direction === 'up') {
    return currentIdx > 0 ? panes[currentIdx - 1].paneId : null;
  }
  return currentIdx < panes.length - 1 ? panes[currentIdx + 1].paneId : null;
}
