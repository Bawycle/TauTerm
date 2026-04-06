<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalView — root terminal UI container.

  Manages session state: fetches initial snapshot on mount via get_session_state,
  then applies incremental updates from session-state-changed events.
  Composes TabBar, TerminalPane instances for the active tab, and StatusBar.

  IPC sources:
    - invoke('get_session_state')    on mount → SessionState full snapshot
    - listen('session-state-changed') → SessionStateChangedEvent deltas
    - listen('ssh-state-changed')    → SshStateChangedEvent
    - listen('host-key-prompt')      → HostKeyPromptEvent (TOFU dialog)
    - listen('credential-prompt')    → CredentialPromptEvent (password dialog)
    - listen('notification-changed') → NotificationChangedEvent (tab badges)
  IPC commands (delegated to children or handled here):
    - invoke('create_tab')           on new-tab button click
    - invoke('close_tab')            on tab close button click
    - invoke('set_active_pane')      on pane click
    - invoke('accept_host_key')      on TOFU accept
    - invoke('reject_host_key')      on TOFU reject
    - invoke('provide_credentials')  on password submit
    - invoke('close_ssh_connection') on credential cancel
    - invoke('get_clipboard')        on Ctrl+Shift+V paste

  Application shortcuts intercepted here (FS-KBD-001/003):
    - Ctrl+Shift+T → new tab
    - Ctrl+Shift+W → close active tab (with close confirmation if process active)
    - Ctrl+Shift+V → paste from clipboard (bracketed paste handled by TerminalPane)
    - Ctrl+Shift+D → split pane horizontally
    - Ctrl+Shift+E → split pane vertically
    - Ctrl+Shift+Q → close active pane (with close confirmation if process active)
    - Ctrl+Shift+ArrowLeft/Right/Up/Down → navigate between panes
    - Ctrl+Tab / Ctrl+Shift+Tab → switch to next/prev tab
    - F2 → rename active tab

  Security:
    - No {@html} anywhere in this component
    - TOFU dialog displays config.host, never server-provided data (SEC-BLK-004)
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import TabBar from './TabBar.svelte';
  import StatusBar from './StatusBar.svelte';
  import SplitPane from './SplitPane.svelte';
  import SearchOverlay from './SearchOverlay.svelte';
  import PreferencesPanel from './PreferencesPanel.svelte';
  import SshHostKeyDialog from './SshHostKeyDialog.svelte';
  import SshCredentialDialog from './SshCredentialDialog.svelte';
  import ConnectionManager from './ConnectionManager.svelte';
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import { Network, MousePointerClick } from 'lucide-svelte';
  import { applyLocaleChange } from '$lib/state/locale.svelte';
  import { applyPreferencesUpdate } from '$lib/preferences/applyUpdate';
  import type {
    Preferences,
    PreferencesPatch,
    SearchQuery,
    SearchMatch,
    SshConnectionConfig,
  } from '$lib/ipc/types';
  import type {
    SessionState,
    SessionStateChangedEvent,
    TabState,
    PaneState,
    PaneId,
    PaneNode,
    SshStateChangedEvent,
    SshLifecycleState,
    HostKeyPromptEvent,
    CredentialPromptEvent,
    NotificationChangedEvent,
    ModeStateChangedEvent,
  } from '$lib/ipc/types';
  import { pasteToBytes } from '$lib/terminal/paste.js';
  import * as m from '$lib/paraglide/messages';

  // -------------------------------------------------------------------------
  // Default preferences — used as fallback when get_preferences fails (Risk 3).
  // Values mirror the Rust defaults in preferences.rs.
  // -------------------------------------------------------------------------
  const DEFAULT_PREFERENCES: Preferences = {
    appearance: {
      fontFamily: 'monospace',
      fontSize: 13,
      cursorStyle: 'block',
      cursorBlinkMs: 530,
      themeName: 'umbra',
      opacity: 1.0,
      language: 'en',
      contextMenuHintShown: false,
    },
    terminal: {
      scrollbackLines: 10000,
      allowOsc52Write: false,
      wordDelimiters: ' ,;:.{}[]()"`|\\/',
      bellType: 'visual',
      confirmMultilinePaste: true,
    },
    keyboard: { bindings: {} },
    connections: [],
    themes: [],
  };

  // -------------------------------------------------------------------------
  // State
  // -------------------------------------------------------------------------

  let tabs = $state<TabState[]>([]);
  let activeTabId = $state<string>('');

  // SearchOverlay state
  let searchOpen = $state(false);
  let searchMatches = $state<SearchMatch[]>([]);
  let searchCurrentIdx = $state(0);

  // PreferencesPanel state
  let prefsOpen = $state(false);
  let preferences = $state<Preferences | undefined>(undefined);

  /**
   * Line height of the active theme (FS-THEME-010).
   * Resolved from the user-defined theme whose name matches `preferences.appearance.themeName`.
   * `undefined` when the active theme is the built-in "umbra" or has no lineHeight override.
   */
  const activeThemeLineHeight = $derived(
    preferences?.themes.find((t) => t.name === preferences?.appearance.themeName)?.lineHeight,
  );

  // SSH auth dialog state (item 6)
  let hostKeyPrompt = $state<HostKeyPromptEvent | null>(null);
  let credentialPrompt = $state<CredentialPromptEvent | null>(null);

  // Per-pane SSH lifecycle states (item 7)
  let sshStates = $state<Map<PaneId, SshLifecycleState>>(new Map());

  // Per-tab notification state (item 8): tabId → has-unread
  let tabNotifications = $state<Map<string, boolean>>(new Map());

  // Per-pane bracketed paste mode (for Ctrl+Shift+V paste, item 2)
  let bracketedPasteByPane = $state<Map<PaneId, boolean>>(new Map());

  // FS-PTY-008: track panes whose process has terminated (processExited notification received).
  // A pane absent from this set is considered to have a running process.
  let terminatedPanes = $state<Set<PaneId>>(new Set());

  // FS-PTY-008: close confirmation dialog state.
  // Stores the pending action (tab close or pane close) awaiting user confirmation.
  type PendingClose = { kind: 'tab'; tabId: string } | { kind: 'pane'; paneId: PaneId };
  let pendingClose = $state<PendingClose | null>(null);
  // DIV-UXD-012: ref to the Cancel button in the close confirmation dialog for initial focus.
  let closeConfirmCancelBtn = $state<HTMLButtonElement | undefined>(undefined);

  // FS-KBD-003: F2 rename — ID to trigger rename on TabBar, cleared after TabBar acts.
  let requestedRenameTabId = $state<string | null>(null);

  // DIV-UXD-008: active pane terminal dimensions for StatusBar display.
  let activePaneCols = $state<number | null>(null);
  let activePaneRows = $state<number | null>(null);
  // Transient visibility: shown on resize, hidden after 2s of inactivity.
  let dimsVisible = $state(false);
  let dimsHideTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Depend on both dimensions so any resize re-triggers the timer.
    const _c = activePaneCols;
    const _r = activePaneRows;
    if (_c === null || _r === null) return;
    // Show immediately.
    dimsVisible = true;
    // Reset the hide timer on every resize event.
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

  // Tâche #13: ConnectionManager panel toggle.
  let connectionManagerOpen = $state(false);
  let savedConnections = $state<SshConnectionConfig[]>([]);
  // Error flag: set when open_ssh_connection or create_tab fails during handleConnectionOpen.
  // Displayed as a transient banner; auto-clears when ConnectionManager is reopened.
  let connectionOpenError = $state(false);

  // FS-UX-002: First-launch context menu hint.
  // Visible when preferences are loaded and hint has not been shown yet.
  // Latched: once dismissed, stays dismissed even if preferences are refreshed.
  // A 2s delay avoids showing the hint while the app is still settling on first launch.
  let contextMenuHintVisible = $state(false);
  let contextMenuHintDismissed = $state(false);
  let contextMenuHintTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (
      preferences !== undefined &&
      !preferences.appearance.contextMenuHintShown &&
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

  let unlistenSessionState: (() => void) | null = null;
  let unlistenSshState: (() => void) | null = null;
  let unlistenHostKeyPrompt: (() => void) | null = null;
  let unlistenCredentialPrompt: (() => void) | null = null;
  let unlistenNotificationChanged: (() => void) | null = null;
  let unlistenModeState: (() => void) | null = null;

  // -------------------------------------------------------------------------
  // Derived
  // -------------------------------------------------------------------------

  const activeTab = $derived(tabs.find((t) => t.id === activeTabId) ?? null);

  /** Collect all leaf pane IDs from the active tab's layout tree. */
  function collectLeafPanes(node: PaneNode): { paneId: string; state: PaneState }[] {
    if (node.type === 'leaf') return [{ paneId: node.paneId, state: node.state }];
    return [...collectLeafPanes(node.first), ...collectLeafPanes(node.second)];
  }

  /**
   * Find the neighbour pane in a given direction for the active pane.
   * Uses the flat leaf order from collectLeafPanes:
   * - left/up = previous in list, right/down = next in list.
   * Returns null if there is no neighbour.
   */
  function findNeighbourPaneId(direction: 'left' | 'right' | 'up' | 'down'): PaneId | null {
    if (!activeTab) return null;
    const panes = activePanes;
    const currentIdx = panes.findIndex((p) => p.paneId === activeTab.activePaneId);
    if (currentIdx === -1 || panes.length <= 1) return null;
    if (direction === 'left' || direction === 'up') {
      return currentIdx > 0 ? panes[currentIdx - 1].paneId : null;
    }
    return currentIdx < panes.length - 1 ? panes[currentIdx + 1].paneId : null;
  }

  const activePanes = $derived(activeTab ? collectLeafPanes(activeTab.layout) : []);

  const activePaneState = $derived(
    activeTab
      ? (activePanes.find((p) => p.paneId === activeTab.activePaneId)?.state ?? null)
      : null,
  );

  // -------------------------------------------------------------------------
  // Mount / destroy
  // -------------------------------------------------------------------------

  onMount(async () => {
    // Fetch full session snapshot
    try {
      const state: SessionState = await invoke('get_session_state');
      tabs = state.tabs;
      activeTabId = state.activeTabId;
    } catch {
      // Backend not ready — will be populated by first session-state-changed event
    }

    // Fetch preferences for PreferencesPanel.
    // On IPC failure, fall back to DEFAULT_PREFERENCES so all $derived values
    // remain defined and the UI remains functional (FS-PREF-003 graceful degradation).
    try {
      preferences = await invoke('get_preferences');
    } catch {
      preferences = DEFAULT_PREFERENCES;
    }

    // Tâche #13: fetch saved SSH connections for ConnectionManager
    try {
      savedConnections = await invoke<SshConnectionConfig[]>('get_connections');
    } catch {
      // Non-fatal
    }

    // Listen for topology changes
    unlistenSessionState = await listen<SessionStateChangedEvent>(
      'session-state-changed',
      (event) => {
        const change = event.payload;
        switch (change.changeType) {
          case 'tab-created':
            if (change.tab) {
              tabs = [...tabs.filter((t) => t.id !== change.tab!.id), change.tab];
            }
            break;

          case 'tab-closed': {
            // Determine which tab was closed: prefer explicit closedTabId,
            // otherwise find the tab that is no longer referenced.
            const closedId =
              change.closedTabId ??
              tabs.find((t) => t.id !== change.activeTabId && t.id !== activeTabId)?.id ??
              activeTabId; // last resort fallback — old behavior
            tabs = tabs.filter((t) => t.id !== closedId);
            if (change.activeTabId !== undefined) {
              activeTabId = change.activeTabId;
            } else if (activeTabId === closedId && tabs.length > 0) {
              activeTabId = tabs[tabs.length - 1].id;
            } else if (tabs.length === 0) {
              activeTabId = '';
            }
            break;
          }

          case 'tab-reordered':
          case 'pane-metadata-changed':
          case 'active-pane-changed':
            if (change.tab) {
              tabs = tabs.map((t) => (t.id === change.tab!.id ? change.tab! : t));
            }
            break;

          case 'active-tab-changed':
            if (change.activeTabId !== undefined) activeTabId = change.activeTabId;
            if (change.tab) {
              tabs = tabs.map((t) => (t.id === change.tab!.id ? change.tab! : t));
            }
            break;
        }
      },
    );

    // Item 6: SSH auth dialogs
    unlistenHostKeyPrompt = await listen<HostKeyPromptEvent>('host-key-prompt', (event) => {
      hostKeyPrompt = event.payload;
    });

    unlistenCredentialPrompt = await listen<CredentialPromptEvent>('credential-prompt', (event) => {
      credentialPrompt = event.payload;
    });

    // Item 7: SSH lifecycle state per pane
    unlistenSshState = await listen<SshStateChangedEvent>('ssh-state-changed', (event) => {
      const ev = event.payload;
      const next = new Map(sshStates);
      next.set(ev.paneId, ev.state);
      sshStates = next;
    });

    // Item 8: Tab activity notifications
    unlistenNotificationChanged = await listen<NotificationChangedEvent>(
      'notification-changed',
      (event) => {
        updatePaneNotification(event.payload);
      },
    );

    // Item 2: Track bracketed paste mode per pane for Ctrl+Shift+V
    unlistenModeState = await listen<ModeStateChangedEvent>('mode-state-changed', (event) => {
      const mode = event.payload;
      const next = new Map(bracketedPasteByPane);
      next.set(mode.paneId, mode.bracketedPaste);
      bracketedPasteByPane = next;
    });
  });

  onDestroy(() => {
    unlistenSessionState?.();
    unlistenSshState?.();
    unlistenHostKeyPrompt?.();
    unlistenCredentialPrompt?.();
    unlistenNotificationChanged?.();
    unlistenModeState?.();
  });

  // -------------------------------------------------------------------------
  // Item 8: notification tracking
  // -------------------------------------------------------------------------

  function updatePaneNotification(ev: NotificationChangedEvent) {
    const next = new Map(tabNotifications);
    if (ev.notification !== null) {
      next.set(ev.tabId, true);
    } else {
      // Clear only if this tab no longer has any notification
      next.delete(ev.tabId);
    }
    tabNotifications = next;

    // FS-PTY-008: track process-terminated state per pane.
    if (ev.notification?.type === 'processExited') {
      const nextTerminated = new Set(terminatedPanes);
      nextTerminated.add(ev.paneId);
      terminatedPanes = nextTerminated;
    } else if (ev.notification === null) {
      // Notification cleared — pane may have been restarted; clear terminated flag.
      const nextTerminated = new Set(terminatedPanes);
      nextTerminated.delete(ev.paneId);
      terminatedPanes = nextTerminated;
    }
  }

  /** Returns true if the given pane has a running process (not terminated). */
  function isPaneProcessActive(paneId: PaneId): boolean {
    return !terminatedPanes.has(paneId);
  }

  /** Returns true if any pane in the given tab's layout has a running process. */
  function isTabProcessActive(tab: TabState): boolean {
    const panes = collectLeafPanes(tab.layout);
    return panes.some((p) => isPaneProcessActive(p.paneId));
  }

  // -------------------------------------------------------------------------
  // Tab management (callbacks from TabBar)
  // -------------------------------------------------------------------------

  async function handleTabClick(tabId: string) {
    // Clear notification badge when user switches to that tab
    const next = new Map(tabNotifications);
    next.delete(tabId);
    tabNotifications = next;

    activeTabId = tabId;

    try {
      await invoke('set_active_tab', { tabId });
    } catch {
      // Non-fatal — frontend state is already updated
    }
  }

  async function handleTabClose(tabId: string) {
    const tab = tabs.find((t) => t.id === tabId);
    if (tab && isTabProcessActive(tab)) {
      // FS-PTY-008: show confirmation dialog before closing a tab with an active process.
      pendingClose = { kind: 'tab', tabId };
      return;
    }
    await doCloseTab(tabId);
  }

  async function doCloseTab(tabId: string) {
    try {
      await invoke('close_tab', { tabId });
      tabs = tabs.filter((t) => t.id !== tabId);
      if (activeTabId === tabId) {
        const remaining = tabs.filter((t) => t.id !== tabId);
        activeTabId = remaining[remaining.length - 1]?.id ?? '';
      }
      // FS-TAB-008: closing the last tab closes the window.
      if (tabs.length === 0) {
        await getCurrentWindow().close();
      }
    } catch {
      // Tab close failed — backend may have already handled it
    }
  }

  async function handleNewTab() {
    try {
      // First tab (no existing tabs) gets a login shell so ~/.bash_profile /
      // ~/.zprofile are sourced (FS-PTY-013). Subsequent tabs use a normal
      // interactive shell.
      const login = tabs.length === 0;
      const newTab: TabState = await invoke('create_tab', {
        config: { cols: 80, rows: 24, login },
      });
      tabs = [...tabs, newTab];
      activeTabId = newTab.id;
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // FS-PTY-008: close confirmation dialog handlers
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

  // -------------------------------------------------------------------------
  // FS-KBD-003: pane close and split actions
  // -------------------------------------------------------------------------

  async function handlePaneClose(paneId: PaneId) {
    if (isPaneProcessActive(paneId)) {
      pendingClose = { kind: 'pane', paneId };
      return;
    }
    await doClosePane(paneId);
  }

  async function doClosePane(paneId: PaneId) {
    try {
      const updatedTab: TabState | null = await invoke('close_pane', { paneId });
      if (updatedTab === null) {
        // All panes removed — the tab itself was removed by the backend.
        tabs = tabs.filter((t) => t.id !== activeTabId);
        if (tabs.length > 0) {
          activeTabId = tabs[tabs.length - 1].id;
        } else {
          activeTabId = '';
        }
      } else {
        tabs = tabs.map((t) => (t.id === updatedTab.id ? updatedTab : t));
      }
    } catch {
      // Non-fatal
    }
  }

  async function handleSplitPane(direction: 'horizontal' | 'vertical') {
    const activePaneId = activeTab?.activePaneId;
    if (!activePaneId) return;
    try {
      const updatedTab: TabState = await invoke('split_pane', { paneId: activePaneId, direction });
      tabs = tabs.map((t) => (t.id === updatedTab.id ? updatedTab : t));
    } catch {
      // Non-fatal
    }
  }

  async function handleNavigatePane(direction: 'left' | 'right' | 'up' | 'down') {
    const targetPaneId = findNeighbourPaneId(direction);
    if (!targetPaneId) return;
    try {
      await invoke('set_active_pane', { paneId: targetPaneId });
    } catch {
      // Non-fatal
    }
  }

  function handleSwitchTab(delta: 1 | -1) {
    if (tabs.length <= 1) return;
    const sorted = [...tabs].sort((a, b) => a.order - b.order);
    const idx = sorted.findIndex((t) => t.id === activeTabId);
    if (idx === -1) return;
    const next = sorted[(idx + delta + sorted.length) % sorted.length];
    handleTabClick(next.id);
  }

  // -------------------------------------------------------------------------
  // Tâche #13: ConnectionManager IPC handlers
  // -------------------------------------------------------------------------

  async function handleConnectionSave(config: SshConnectionConfig) {
    try {
      const id: string = await invoke('save_connection', { config });
      // Update local list — if new, id is returned; if update, same id.
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
      await invoke('delete_connection', { connectionId });
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
      // Two-step operation: create_tab then open_ssh_connection.
      // If the SSH step fails, roll back the orphan tab to keep state consistent (FS-SSH-032).
      let newTab: TabState;
      try {
        newTab = await invoke('create_tab', { config: { cols: 80, rows: 24 } });
      } catch {
        connectionOpenError = true;
        return;
      }
      tabs = [...tabs, newTab];
      activeTabId = newTab.id;
      const panes = collectLeafPanes(newTab.layout);
      if (panes.length > 0) {
        try {
          await invoke('open_ssh_connection', { paneId: panes[0].paneId, connectionId });
        } catch {
          // Roll back the orphan tab created above.
          try {
            await invoke('close_tab', { tabId: newTab.id });
          } catch {
            // close_tab also failed — backend may have already cleaned up.
          }
          tabs = tabs.filter((t) => t.id !== newTab.id);
          if (activeTabId === newTab.id) {
            activeTabId = tabs[tabs.length - 1]?.id ?? '';
          }
          connectionOpenError = true;
          return;
        }
      }
    } else {
      // Open in active pane.
      const activePaneId = activeTab?.activePaneId;
      if (!activePaneId) return;
      try {
        await invoke('open_ssh_connection', { paneId: activePaneId, connectionId });
      } catch {
        connectionOpenError = true;
        return;
      }
    }
    connectionManagerOpen = false;
  }

  // -------------------------------------------------------------------------
  // Application shortcuts (FS-KBD-001/003, FS-KBD-002, TUITC-FN-047)
  // These are intercepted here at the TerminalView level and never forwarded to PTY.
  //
  // FS-KBD-002: shortcuts are resolved from preferences.keyboard.bindings first,
  // with hardcoded defaults as fallback when bindings are absent.
  // -------------------------------------------------------------------------

  /**
   * Default shortcut strings — mirrors PreferencesPanel.defaultShortcuts.
   * Used as fallback when preferences.keyboard.bindings does not override an action.
   */
  const defaultShortcuts: Record<string, string> = {
    new_tab: 'Ctrl+Shift+T',
    close_tab: 'Ctrl+Shift+W',
    paste: 'Ctrl+Shift+V',
    search: 'Ctrl+Shift+F',
    preferences: 'Ctrl+,',
    next_tab: 'Ctrl+Tab',
    prev_tab: 'Ctrl+Shift+Tab',
    rename_tab: 'F2',
    // Pane shortcuts (DIV-FS-003)
    split_pane_h: 'Ctrl+Shift+D',
    split_pane_v: 'Ctrl+Shift+E',
    close_pane: 'Ctrl+Shift+Q',
    navigate_pane_left: 'Ctrl+Shift+ArrowLeft',
    navigate_pane_right: 'Ctrl+Shift+ArrowRight',
    navigate_pane_up: 'Ctrl+Shift+ArrowUp',
    navigate_pane_down: 'Ctrl+Shift+ArrowDown',
  };

  /**
   * Resolve the effective shortcut string for a given action ID,
   * preferring user bindings from preferences over the hardcoded defaults.
   */
  function effectiveShortcut(actionId: string): string {
    return preferences?.keyboard?.bindings?.[actionId] ?? defaultShortcuts[actionId] ?? '';
  }

  /**
   * Test whether a KeyboardEvent matches a shortcut string of the form
   * "Ctrl+Shift+T", "Ctrl+,", "F2", etc.
   * Modifier matching is order-independent; key comparison is case-insensitive
   * for single-character keys (handles Shift state differences).
   */
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

    // For single-char keys compare case-insensitively (Shift can alter event.key case).
    const eventKey = event.key;
    if (requiredKey.length === 1) {
      return eventKey.toLowerCase() === requiredKey.toLowerCase();
    }
    return eventKey === requiredKey;
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    // FS-KBD-002: check user-configurable shortcuts first, then fixed pane shortcuts.

    if (matchesShortcut(event, effectiveShortcut('new_tab'))) {
      event.preventDefault();
      handleNewTab();
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('close_tab'))) {
      event.preventDefault();
      if (activeTabId) handleTabClose(activeTabId);
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('search'))) {
      event.preventDefault();
      searchOpen = true;
      return;
    }
    if (matchesShortcut(event, effectiveShortcut('paste'))) {
      // FS-CLIP-007: Ctrl+Shift+V → paste via IPC
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
      if (activeTabId) {
        requestedRenameTabId = activeTabId;
      }
      return;
    }

    // FS-KBD-003 / DIV-FS-003: pane actions — resolved from preferences.keyboard.bindings.
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
      const activePaneId = activeTab?.activePaneId;
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
  // Item 2: Ctrl+Shift+V global paste
  // Reads clipboard and dispatches to active pane's pasteText via sendInput.
  // Bracketed paste wrapping is handled in TerminalPane.pasteText() — here we
  // use the get_clipboard command and send through send_input so the pane's
  // bracketedPasteActive state is respected.
  // -------------------------------------------------------------------------

  async function handleGlobalPaste() {
    const activePaneId = activeTab?.activePaneId;
    if (!activePaneId) return;
    try {
      const text: string = await invoke('get_clipboard');
      if (!text) return;
      // Use bracketed paste mode tracked from mode-state-changed events (item 2)
      const isBracketed = bracketedPasteByPane.get(activePaneId) ?? false;
      const encoded = pasteToBytes(text, isBracketed);
      if (!encoded) return;
      await invoke('send_input', { paneId: activePaneId, data: Array.from(encoded) });
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Item 6: SSH auth handlers
  // -------------------------------------------------------------------------

  async function handleAcceptHostKey() {
    const prompt = hostKeyPrompt;
    if (!prompt) return;
    hostKeyPrompt = null;
    try {
      await invoke('accept_host_key', { paneId: prompt.paneId });
    } catch {
      /* non-fatal */
    }
  }

  async function handleRejectHostKey() {
    const prompt = hostKeyPrompt;
    if (!prompt) return;
    hostKeyPrompt = null;
    try {
      await invoke('reject_host_key', { paneId: prompt.paneId });
    } catch {
      /* non-fatal */
    }
  }

  async function handleProvideCredentials(password: string) {
    const prompt = credentialPrompt;
    if (!prompt) return;
    credentialPrompt = null;
    try {
      await invoke('provide_credentials', { paneId: prompt.paneId, password });
    } catch {
      /* non-fatal */
    }
  }

  async function handleCancelCredentials() {
    const prompt = credentialPrompt;
    if (!prompt) return;
    credentialPrompt = null;
    try {
      await invoke('close_ssh_connection', { paneId: prompt.paneId });
    } catch {
      /* non-fatal */
    }
  }

  // -------------------------------------------------------------------------
  // Search
  // -------------------------------------------------------------------------

  async function handleSearch(query: SearchQuery) {
    const activePaneId = activeTab?.activePaneId;
    if (!activePaneId) return;
    try {
      searchMatches = await invoke<SearchMatch[]>('search_pane', { paneId: activePaneId, query });
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
  // FS-UX-002: Context menu hint dismissal
  // -------------------------------------------------------------------------

  async function handleContextMenuHintDismiss() {
    if (!contextMenuHintVisible) return;
    contextMenuHintVisible = false;
    contextMenuHintDismissed = true;
    try {
      await invoke('mark_context_menu_used');
      // Sync local preferences snapshot so PreferencesPanel sees the updated flag
      // without requiring a round-trip to get_preferences (FS-UX-002 / FS-PREF-003).
      if (preferences !== undefined) {
        preferences = {
          ...preferences,
          appearance: { ...preferences.appearance, contextMenuHintShown: true },
        };
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
      preferences = await applyPreferencesUpdate(patch, invoke, applyLocaleChange);
    } catch {
      // Non-fatal
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="terminal-view"
  role="application"
  aria-label={m.terminal_view_aria_label()}
  onkeydown={handleGlobalKeydown}
>
  <!-- Tab bar: renders tabs from session state -->
  <div class="terminal-view__tab-row">
    <TabBar
      {tabs}
      {activeTabId}
      onTabClick={handleTabClick}
      onTabClose={handleTabClose}
      onNewTab={handleNewTab}
      {requestedRenameTabId}
      onRenameHandled={() => {
        requestedRenameTabId = null;
      }}
    />
    <!-- SSH connections toggle button (FS-SSH-031, UXD §7.1.8) -->
    <button
      class="terminal-view__ssh-btn"
      class:terminal-view__ssh-btn--active={connectionManagerOpen}
      type="button"
      onclick={() => {
        connectionManagerOpen = !connectionManagerOpen;
      }}
      aria-label={connectionManagerOpen
        ? m.ssh_connections_panel_close()
        : m.ssh_connections_panel_open()}
      aria-pressed={connectionManagerOpen}
      title={m.ssh_connections_panel_toggle()}
    >
      <Network size={16} aria-hidden="true" />
    </button>
  </div>

  <!-- Pane area: render the full split-tree layout for the active tab -->
  <!-- FS-UX-002: contextmenu bubbles up from TerminalPane to dismiss the first-launch hint -->
  <div class="terminal-view__pane-area" role="region" oncontextmenu={handleContextMenuHintDismiss}>
    {#key activeTabId}
      {#if activeTab && activePanes.length > 0}
        <SplitPane
          node={activeTab.layout}
          tabId={activeTab.id}
          activePaneId={activeTab.activePaneId}
          {sshStates}
          {terminatedPanes}
          wordDelimiters={preferences?.terminal.wordDelimiters}
          confirmMultilinePaste={preferences?.terminal.confirmMultilinePaste ?? true}
          cursorBlinkMs={preferences?.appearance.cursorBlinkMs}
          bellType={preferences?.terminal.bellType}
          lineHeight={activeThemeLineHeight}
          {searchMatches}
          activeSearchMatchIndex={searchCurrentIdx}
          canClosePane={activePanes.length > 1}
          onpaneclick={async (paneId) => {
            try {
              await invoke('set_active_pane', { paneId });
            } catch {
              /* non-fatal */
            }
          }}
          onclosepane={handlePaneClose}
          onsearch={() => {
            searchOpen = true;
          }}
          onsplith={() => handleSplitPane('horizontal')}
          onsplitv={() => handleSplitPane('vertical')}
          ondisableConfirmMultilinePaste={() =>
            handlePreferencesUpdate({ terminal: { confirmMultilinePaste: false } })}
          ondimensionschange={(paneId, c, r) => {
            if (paneId === activeTab?.activePaneId) {
              activePaneCols = c;
              activePaneRows = r;
            }
          }}
        />
      {:else}
        <div class="terminal-view__empty">
          <p>{m.terminal_view_empty()}</p>
        </div>
      {/if}
    {/key}
  </div>

  <!-- FS-UX-002: First-launch context menu hint — non-blocking, bottom-right corner -->
  {#if contextMenuHintVisible}
    <div class="terminal-view__context-hint" aria-hidden="true" transition:fade={{ duration: 300 }}>
      <MousePointerClick size={14} aria-hidden="true" />
      <span>{m.context_menu_hint()}</span>
    </div>
  {/if}

  <!-- Status bar: reflects active pane state (DIV-UXD-008) -->
  <StatusBar
    {activePaneState}
    cols={activePaneCols}
    rows={activePaneRows}
    {dimsVisible}
    onsettings={() => {
      prefsOpen = true;
    }}
  />

  <!-- SearchOverlay: positioned relative to pane area (FS-SEARCH-007, UXD §7.4) -->
  {#if activePanes.length > 0}
    <div class="terminal-view__search-container">
      <SearchOverlay
        bind:open={searchOpen}
        matchCount={searchMatches.length}
        currentMatch={searchCurrentIdx}
        onsearch={handleSearch}
        onnext={handleSearchNext}
        onprev={handleSearchPrev}
        onclose={handleSearchClose}
      />
    </div>
  {/if}

  <!-- PreferencesPanel: modal dialog (FS-PREF-005, UXD §7.6) -->
  <PreferencesPanel
    bind:open={prefsOpen}
    {preferences}
    onclose={() => {
      prefsOpen = false;
    }}
    onupdate={handlePreferencesUpdate}
  />

  <!-- SSH TOFU host key dialog (FS-SSH-011, SEC-BLK-004) -->
  <SshHostKeyDialog
    open={hostKeyPrompt !== null}
    host={hostKeyPrompt?.host ?? ''}
    keyType={hostKeyPrompt?.keyType ?? ''}
    fingerprint={hostKeyPrompt?.fingerprint ?? ''}
    isChanged={hostKeyPrompt?.isChanged ?? false}
    onaccept={handleAcceptHostKey}
    onreject={handleRejectHostKey}
    onclose={handleRejectHostKey}
  />

  <!-- SSH credential prompt dialog (FS-SSH-012) -->
  <SshCredentialDialog
    open={credentialPrompt !== null}
    host={credentialPrompt?.host ?? ''}
    username={credentialPrompt?.username ?? ''}
    prompt={credentialPrompt?.prompt}
    onsubmit={handleProvideCredentials}
    oncancel={handleCancelCredentials}
    onclose={handleCancelCredentials}
  />

  <!-- FS-PTY-008: Close confirmation dialog — DIV-UXD-012: initial focus on Cancel -->
  <Dialog
    open={pendingClose !== null}
    title={m.close_confirm_title()}
    size="small"
    onclose={handleCloseCancel}
    onopenautoFocus={(e) => {
      e.preventDefault();
      // Focus Cancel — the safe default, preventing accidental destructive action (DIV-UXD-012).
      closeConfirmCancelBtn?.focus();
    }}
  >
    {#snippet children()}
      <p class="text-[14px] text-(--color-text-secondary) leading-relaxed">
        {m.close_confirm_body()}
      </p>
    {/snippet}
    {#snippet footer()}
      <Button variant="ghost" bind:buttonRef={closeConfirmCancelBtn} onclick={handleCloseCancel}
        >{m.action_cancel()}</Button
      >
      <Button variant="destructive" onclick={handleCloseConfirm}>{m.close_confirm_action()}</Button>
    {/snippet}
  </Dialog>

  <!-- Tâche #13: SSH connections slide-in panel -->
  {#if connectionManagerOpen}
    <ConnectionManager
      standalone={true}
      connections={savedConnections}
      onsave={handleConnectionSave}
      ondelete={handleConnectionDelete}
      onopen={handleConnectionOpen}
      onclose={() => {
        connectionManagerOpen = false;
        connectionOpenError = false;
      }}
    />
  {/if}

  <!-- FS-SSH-032: connection open error banner — shown when create_tab or open_ssh_connection fails -->
  {#if connectionOpenError}
    <div
      class="terminal-view__connection-error"
      role="alert"
      aria-live="assertive"
      transition:fade={{ duration: 200 }}
    >
      <span>{m.error_connection_failed()}</span>
      <button
        type="button"
        class="terminal-view__connection-error-close"
        onclick={() => {
          connectionOpenError = false;
        }}
        aria-label={m.action_close()}
      >{m.action_close()}</button>
    </div>
  {/if}
</div>

<style>
  .terminal-view {
    display: flex;
    flex-direction: column;
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--color-bg-base);
  }

  .terminal-view__tab-row {
    display: flex;
    align-items: stretch;
    flex-shrink: 0;
  }

  .terminal-view__ssh-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--size-target-min, 44px);
    min-width: var(--size-target-min, 44px);
    height: var(--size-tab-height, 44px);
    border: none;
    border-left: 1px solid var(--color-border);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-tab-bg);
    color: var(--color-text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    transition:
      color var(--duration-instant),
      background-color var(--duration-instant);
  }

  .terminal-view__ssh-btn:hover {
    color: var(--color-text-primary);
    background-color: var(--color-hover-bg);
  }

  .terminal-view__ssh-btn--active {
    color: var(--color-accent);
    background-color: var(--color-tab-active-bg);
  }

  .terminal-view__ssh-btn:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }

  .terminal-view__pane-area {
    flex: 1;
    overflow: hidden;
    position: relative;
    background-color: var(--term-bg);
  }

  .terminal-view__search-container {
    position: absolute;
    top: 44px; /* below tab bar */
    right: 0;
    left: 0;
    bottom: 28px; /* above status bar */
    pointer-events: none;
    z-index: 20;
  }

  /* Allow SearchOverlay itself to receive pointer events */
  :global(.terminal-view__search-container > *) {
    pointer-events: auto;
  }

  .terminal-view__empty {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    color: var(--color-text-tertiary);
    font-size: var(--font-size-ui-base);
    font-family: var(--font-ui);
  }

  /* FS-SSH-032: SSH connection open error banner */
  .terminal-view__connection-error {
    position: absolute;
    bottom: calc(var(--size-status-bar-height) + var(--space-4));
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background-color: var(--color-error-bg);
    border: 1px solid var(--color-error-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-raised);
    color: var(--color-error-fg);
    font-size: var(--font-size-ui-sm);
    font-family: var(--font-ui);
    z-index: var(--z-overlay);
    white-space: nowrap;
    pointer-events: auto;
  }

  .terminal-view__connection-error-close {
    background: none;
    border: none;
    padding: 0 var(--space-1);
    color: inherit;
    cursor: pointer;
    font-size: var(--font-size-ui-sm);
    font-family: var(--font-ui);
    opacity: 0.8;
    min-width: 44px;
    min-height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .terminal-view__connection-error-close:hover {
    opacity: 1;
  }

  .terminal-view__connection-error-close:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }

  /* FS-UX-002: First-launch context menu hint (UXD §7.13) */
  .terminal-view__context-hint {
    position: absolute;
    bottom: calc(var(--size-status-bar-height) + var(--space-4));
    right: var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background-color: var(--color-bg-raised);
    border: 1px solid var(--color-border-subtle);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-raised);
    color: var(--color-text-secondary);
    font-size: var(--font-size-ui-sm);
    font-family: var(--font-ui);
    pointer-events: none;
    z-index: var(--z-overlay);
    white-space: nowrap;
  }
</style>
