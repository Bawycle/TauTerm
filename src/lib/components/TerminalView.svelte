<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalView — root terminal UI container.

  Manages session state: fetches initial snapshot on mount via get_session_state,
  then applies incremental updates from session-state-changed events.
  Composes TabBar, TerminalPane instances for the active tab, and StatusBar.

  IPC sources:
    - invoke('get_session_state')    on mount → SessionState full snapshot
    - listen('session-state-changed') → SessionStateChangedEvent deltas
  IPC commands (delegated to children or handled here):
    - invoke('create_tab')           on new-tab button click
    - invoke('close_tab')            on tab close button click
    - invoke('set_active_pane')      on pane click

  Application shortcuts intercepted here (FS-KBD-001/003):
    - Ctrl+Shift+T → new tab
    - Ctrl+Shift+W → close active tab

  Security:
    - No {@html} anywhere in this component
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
  import type { Preferences, PreferencesPatch, SearchQuery, SearchMatch } from '$lib/ipc/types';
  import type {
    SessionState,
    SessionStateChangedEvent,
    TabState,
    PaneState,
    PaneNode,
  } from '$lib/ipc/types';

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

  let unlistenSessionState: (() => void) | null = null;

  // -------------------------------------------------------------------------
  // Derived
  // -------------------------------------------------------------------------

  const activeTab = $derived(tabs.find((t) => t.id === activeTabId) ?? null);

  /** Collect all leaf pane IDs from the active tab's layout tree. */
  function collectLeafPanes(node: PaneNode): { paneId: string; state: PaneState }[] {
    if (node.type === 'leaf') return [{ paneId: node.paneId, state: node.state }];
    return [...collectLeafPanes(node.first), ...collectLeafPanes(node.second)];
  }

  const activePanes = $derived(
    activeTab ? collectLeafPanes(activeTab.layout) : []
  );

  const activePaneState = $derived(
    activeTab
      ? activePanes.find((p) => p.paneId === activeTab.activePaneId)?.state ?? null
      : null
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

          case 'tab-closed':
            if (change.activeTabId !== undefined) {
              tabs = tabs.filter((t) => t.id !== activeTabId);
              activeTabId = change.activeTabId ?? '';
            }
            break;

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
      }
    );
  });

  onDestroy(() => {
    unlistenSessionState?.();
  });

  // -------------------------------------------------------------------------
  // Tab management (callbacks from TabBar)
  // -------------------------------------------------------------------------

  async function handleTabClick(tabId: string) {
    // Tab switching is frontend-local — update activeTabId, then inform backend
    activeTabId = tabId;
    // set_active_pane will be called when user clicks a pane within the tab;
    // for now, just switch display.
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
      const newTab: TabState = await invoke('create_tab', {
        config: { cols: 80, rows: 24 },
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
      }
    }
    // Ctrl+, — open preferences (FS-KBD-003)
    if (event.ctrlKey && !event.shiftKey && event.key === ',') {
      event.preventDefault();
      prefsOpen = true;
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
  aria-label="TauTerm terminal"
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
          active={true}
        />
      {:else}
        <!-- Multi-pane: simple flex layout (full split-tree rendering deferred) -->
        <div class="terminal-view__split-container">
          {#each activePanes as { paneId } (paneId)}
            <TerminalPane
              {paneId}
              active={paneId === activeTab.activePaneId}
            />
          {/each}
        </div>
      {/if}
    {:else}
      <div class="terminal-view__empty">
        <p>No terminal sessions. Press Ctrl+Shift+T or click + to open one.</p>
      </div>
    {/if}
  </div>

  <!-- Status bar: reflects active pane state -->
  <StatusBar activePaneState={activePaneState} />

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
    onclose={() => { prefsOpen = false; }}
    onupdate={handlePreferencesUpdate}
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
