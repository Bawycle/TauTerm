// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView composable — orchestrator.
 *
 * Delegates to:
 *   - useTerminalView.core.svelte.ts   → reactive state, effects, IPC listeners
 *   - useTerminalView.session-handlers.svelte.ts → tab/pane/close-confirm handlers
 *   - useTerminalView.io-handlers.svelte.ts      → SSH, connections, paste, search,
 *                                                   shortcuts, keyboard handler, prefs
 *
 * Public interface (return value) is identical to the original monolithic file.
 * All IPC commands are invoked via $lib/ipc/commands.
 *
 * Separation of concerns:
 *   - This file: wiring; returns the API consumed by TerminalView.svelte
 *   - TerminalView.svelte: template markup and DOM event binding
 */

import {
  sessionState,
  getActiveTab,
  getActivePanes,
  collectLeafPanes,
} from '$lib/state/session.svelte';
import {
  sshStates,
  hostKeyPrompt,
  credentialPrompt,
  passphrasePrompt,
} from '$lib/state/ssh.svelte';
import { tabNotifications } from '$lib/state/notifications.svelte';
import { preferences } from '$lib/state/preferences.svelte';
import { fullscreenState, setFullscreen } from '$lib/state/fullscreen.svelte';
import { toggleFullscreen as ipcToggleFullscreen } from '$lib/ipc/commands';
import { createViewState } from './useTerminalView.core.svelte';
import { createSessionHandlers } from './useTerminalView.session-handlers.svelte';
import { createIoHandlers } from './useTerminalView.io-handlers.svelte';

// ---------------------------------------------------------------------------
// Re-export state references so TerminalView.svelte can bind to them directly
// ---------------------------------------------------------------------------

export {
  sessionState,
  sshStates,
  hostKeyPrompt,
  credentialPrompt,
  passphrasePrompt,
  tabNotifications,
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
  // Wire up session handlers first (doClosePane is needed by createViewState).
  // We use a forward-reference pattern: createViewState receives a proxy that
  // delegates to the real doClosePane once session handlers are created.
  let doClosePaneRef: ((paneId: string) => Promise<void>) | null = null;
  const doClosePaneProxy = (paneId: string) => doClosePaneRef!(paneId);

  // Core state + effects + IPC listeners (must be called synchronously here so
  // Svelte 5 runes / onMount / onDestroy run in the component's reactive scope).
  const s = createViewState(doClosePaneProxy);

  // Session handlers (tab/pane/close-confirm/dims) — no runes, pure closures.
  const session = createSessionHandlers(s);
  doClosePaneRef = session.doClosePane;

  // Toggle-fullscreen helper (used by both io-handlers and the return object).
  async function handleToggleFullscreen(): Promise<void> {
    try {
      const result = await ipcToggleFullscreen();
      setFullscreen(result.isFullscreen);
    } catch {
      /* non-fatal */
    }
  }

  // IO handlers (SSH, connections, paste, search, keyboard, prefs) — no runes.
  const io = createIoHandlers(
    s,
    session.handleSwitchTab,
    session.handleNewTab,
    session.handleTabClose,
    session.handleSplitPane,
    session.handlePaneClose,
    session.handleNavigatePane,
    handleToggleFullscreen,
  );

  // -------------------------------------------------------------------------
  // Return all state and handlers needed by TerminalView.svelte
  // -------------------------------------------------------------------------

  return {
    // State (read-only from the template)
    get searchOpen() {
      return s.searchOpen;
    },
    set searchOpen(v: boolean) {
      s.searchOpen = v;
    },
    get searchMatches() {
      return s.searchMatches;
    },
    get searchCurrentIdx() {
      return s.searchCurrentIdx;
    },
    get prefsOpen() {
      return s.prefsOpen;
    },
    set prefsOpen(v: boolean) {
      s.prefsOpen = v;
    },
    get activePaneCols() {
      return s.activePaneCols;
    },
    get activePaneRows() {
      return s.activePaneRows;
    },
    get dimsVisible() {
      return s.dimsVisible;
    },
    get connectionManagerOpen() {
      return s.connectionManagerOpen;
    },
    set connectionManagerOpen(v: boolean) {
      s.connectionManagerOpen = v;
    },
    get savedConnections() {
      return s.savedConnections;
    },
    get connectionOpenError() {
      return s.connectionOpenError;
    },
    set connectionOpenError(v: boolean) {
      s.connectionOpenError = v;
    },
    get contextMenuHintVisible() {
      return s.contextMenuHintVisible;
    },
    get pendingClose() {
      return s.pendingClose;
    },
    get closeConfirmCancelBtn() {
      return s.closeConfirmCancelBtn;
    },
    set closeConfirmCancelBtn(v: HTMLButtonElement | undefined) {
      s.closeConfirmCancelBtn = v;
    },
    get pendingWindowClose() {
      return s.pendingWindowClose;
    },
    get windowCloseConfirmCancelBtn() {
      return s.windowCloseConfirmCancelBtn;
    },
    set windowCloseConfirmCancelBtn(v: HTMLButtonElement | undefined) {
      s.windowCloseConfirmCancelBtn = v;
    },
    get requestedRenameTabId() {
      return s.requestedRenameTabId;
    },
    set requestedRenameTabId(v: string | null) {
      s.requestedRenameTabId = v;
    },
    get activeThemeLineHeight() {
      return s.activeThemeLineHeight;
    },
    get isFullscreen() {
      return s.isFullscreen;
    },

    // Focus management
    get activeViewportEl() {
      return s.activeViewportEl;
    },
    set activeViewportEl(v: HTMLElement | null) {
      s.activeViewportEl = v;
    },

    // Handlers — session
    handleTabClick: session.handleTabClick,
    handleTabClose: session.handleTabClose,
    handleNewTab: session.handleNewTab,
    handleCloseConfirm: session.handleCloseConfirm,
    handleCloseCancel: session.handleCloseCancel,
    handleWindowCloseConfirm: session.handleWindowCloseConfirm,
    handleWindowCloseCancel: session.handleWindowCloseCancel,
    handlePaneClose: session.handlePaneClose,
    handleSplitPane: session.handleSplitPane,
    handleNavigatePane: session.handleNavigatePane,
    handleSwitchTab: session.handleSwitchTab,
    handleDimensionsChange: session.handleDimensionsChange,
    handlePaneRename: session.handlePaneRename,

    // Handlers — IO
    handleConnectionSave: io.handleConnectionSave,
    handleConnectionDelete: io.handleConnectionDelete,
    handleConnectionOpen: io.handleConnectionOpen,
    handleAcceptHostKey: io.handleAcceptHostKey,
    handleRejectHostKey: io.handleRejectHostKey,
    handleProvideCredentials: io.handleProvideCredentials,
    handleCancelCredentials: io.handleCancelCredentials,
    handleProvidePassphrase: io.handleProvidePassphrase,
    handleCancelPassphrase: io.handleCancelPassphrase,
    handleGlobalPaste: io.handleGlobalPaste,
    handleSearch: io.handleSearch,
    handleSearchNext: io.handleSearchNext,
    handleSearchPrev: io.handleSearchPrev,
    handleSearchClose: io.handleSearchClose,
    handleContextMenuHintDismiss: io.handleContextMenuHintDismiss,
    handlePreferencesUpdate: io.handlePreferencesUpdate,
    handleGlobalKeydown: io.handleGlobalKeydown,
    handleToggleFullscreen,
  };
}
