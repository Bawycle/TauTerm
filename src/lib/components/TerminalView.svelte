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
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import TabBar from './TabBar.svelte';
  import StatusBar from './StatusBar.svelte';
  import TerminalPane from './TerminalPane.svelte';
  import SearchOverlay from './SearchOverlay.svelte';
  import PreferencesPanel from './PreferencesPanel.svelte';
  import SshHostKeyDialog from './SshHostKeyDialog.svelte';
  import SshCredentialDialog from './SshCredentialDialog.svelte';
  import ConnectionManager from './ConnectionManager.svelte';
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import { Network } from 'lucide-svelte';
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

  // FS-KBD-003: F2 rename — ID to trigger rename on TabBar, cleared after TabBar acts.
  let requestedRenameTabId = $state<string | null>(null);

  // Tâche #13: ConnectionManager panel toggle.
  let connectionManagerOpen = $state(false);
  let savedConnections = $state<SshConnectionConfig[]>([]);

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

    // Fetch preferences for PreferencesPanel
    try {
      preferences = await invoke('get_preferences');
    } catch {
      // Non-fatal — panel will use defaults
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
    try {
      if (target === 'tab') {
        // Open a new tab, then start an SSH connection on its first pane.
        const newTab: TabState = await invoke('create_tab', { config: { cols: 80, rows: 24 } });
        tabs = [...tabs, newTab];
        activeTabId = newTab.id;
        // The layout has exactly one leaf pane after tab creation.
        const panes = collectLeafPanes(newTab.layout);
        if (panes.length > 0) {
          await invoke('open_ssh_connection', { paneId: panes[0].paneId, connectionId });
        }
      } else {
        // Open in active pane.
        const activePaneId = activeTab?.activePaneId;
        if (!activePaneId) return;
        await invoke('open_ssh_connection', { paneId: activePaneId, connectionId });
      }
      connectionManagerOpen = false;
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Application shortcuts (FS-KBD-001/003, TUITC-FN-047)
  // These are intercepted here at the TerminalView level and never forwarded to PTY
  // -------------------------------------------------------------------------

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.shiftKey) {
      switch (event.key) {
        case 'T':
        case 't':
          event.preventDefault();
          handleNewTab();
          break;
        case 'W':
        case 'w':
          event.preventDefault();
          if (activeTabId) handleTabClose(activeTabId);
          break;
        case 'F':
        case 'f':
          event.preventDefault();
          searchOpen = true;
          break;
        case 'V':
        case 'v':
          // Item 2: Ctrl+Shift+V → paste via IPC (FS-CLIP-007)
          event.preventDefault();
          handleGlobalPaste();
          break;
        case 'D':
        case 'd':
          // FS-KBD-003: split pane horizontally
          event.preventDefault();
          handleSplitPane('horizontal');
          break;
        case 'E':
        case 'e':
          // FS-KBD-003: split pane vertically
          event.preventDefault();
          handleSplitPane('vertical');
          break;
        case 'Q':
        case 'q': {
          // FS-KBD-003: close active pane (with confirmation if process active)
          event.preventDefault();
          const activePaneId = activeTab?.activePaneId;
          if (activePaneId) handlePaneClose(activePaneId);
          break;
        }
        case 'ArrowLeft':
          event.preventDefault();
          handleNavigatePane('left');
          break;
        case 'ArrowRight':
          event.preventDefault();
          handleNavigatePane('right');
          break;
        case 'ArrowUp':
          event.preventDefault();
          handleNavigatePane('up');
          break;
        case 'ArrowDown':
          event.preventDefault();
          handleNavigatePane('down');
          break;
      }
    }
    // Ctrl+Tab — next tab (FS-KBD-003)
    if (event.ctrlKey && !event.shiftKey && event.key === 'Tab') {
      event.preventDefault();
      handleSwitchTab(1);
    }
    // Ctrl+Shift+Tab — previous tab (FS-KBD-003)
    if (event.ctrlKey && event.shiftKey && event.key === 'Tab') {
      event.preventDefault();
      handleSwitchTab(-1);
    }
    // Ctrl+, — open preferences (FS-KBD-003)
    if (event.ctrlKey && !event.shiftKey && event.key === ',') {
      event.preventDefault();
      prefsOpen = true;
    }
    // F2 — rename active tab (FS-KBD-003)
    if (!event.ctrlKey && !event.shiftKey && !event.altKey && event.key === 'F2') {
      event.preventDefault();
      if (activeTabId) {
        requestedRenameTabId = activeTabId;
      }
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
  // Preferences
  // -------------------------------------------------------------------------

  async function handlePreferencesUpdate(patch: PreferencesPatch) {
    try {
      preferences = await invoke<Preferences>('update_preferences', { patch });
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
    <!-- SSH connections toggle button (Tâche #13) -->
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

  <!-- Pane area: render all leaf panes of the active tab -->
  <div class="terminal-view__pane-area">
    {#if activeTab && activePanes.length > 0}
      <!-- Simple single-pane layout — multi-pane split layout deferred to split-tree -->
      {#if activePanes.length === 1}
        <TerminalPane
          paneId={activePanes[0].paneId}
          tabId={activeTab.id}
          active={true}
          sshState={sshStates.get(activePanes[0].paneId) ?? null}
          wordDelimiters={preferences?.terminal.wordDelimiters}
        />
      {:else}
        <!-- Multi-pane: simple flex layout (full split-tree rendering deferred) -->
        <div class="terminal-view__split-container">
          {#each activePanes as { paneId } (paneId)}
            <TerminalPane
              {paneId}
              tabId={activeTab.id}
              active={paneId === activeTab.activePaneId}
              sshState={sshStates.get(paneId) ?? null}
              wordDelimiters={preferences?.terminal.wordDelimiters}
            />
          {/each}
        </div>
      {/if}
    {:else}
      <div class="terminal-view__empty">
        <p>{m.terminal_view_empty()}</p>
      </div>
    {/if}
  </div>

  <!-- Status bar: reflects active pane state -->
  <StatusBar {activePaneState} />

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

  <!-- FS-PTY-008: Close confirmation dialog -->
  <Dialog
    open={pendingClose !== null}
    title={m.close_confirm_title()}
    size="small"
    onclose={handleCloseCancel}
  >
    {#snippet children()}
      <p class="text-[14px] text-(--color-text-secondary) leading-relaxed">
        {m.close_confirm_body()}
      </p>
    {/snippet}
    {#snippet footer()}
      <Button variant="ghost" onclick={handleCloseCancel}>{m.action_cancel()}</Button>
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
      }}
    />
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

  .terminal-view__split-container {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
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
</style>
