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
    - Ctrl+Shift+W → close active tab
    - Ctrl+Shift+V → paste from clipboard (bracketed paste handled by TerminalPane)

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
  import type { Preferences, PreferencesPatch, SearchQuery, SearchMatch } from '$lib/ipc/types';
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
    // TODO: confirmation dialog via Bits UI Dialog if running process (FS-PTY-008)
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
      }
    }
    // Ctrl+, — open preferences (FS-KBD-003)
    if (event.ctrlKey && !event.shiftKey && event.key === ',') {
      event.preventDefault();
      prefsOpen = true;
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
  <TabBar
    {tabs}
    {activeTabId}
    onTabClick={handleTabClick}
    onTabClose={handleTabClose}
    onNewTab={handleNewTab}
  />

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
