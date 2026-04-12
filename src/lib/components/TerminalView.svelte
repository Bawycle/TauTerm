<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalView — root terminal UI container.

  Manages session state: fetches initial snapshot on mount via get_session_state,
  then applies incremental updates from session-state-changed events.
  Composes TabBar, TerminalPane instances for the active tab, and StatusBar.

  Reactive logic is extracted to TerminalView.svelte.ts (composable).
  This file contains only template markup and DOM event binding (§11.2).

  Security:
    - No {@html} anywhere in this component
    - TOFU dialog displays config.host, never server-provided data (SEC-BLK-004)
-->
<script lang="ts">
  import { fade } from 'svelte/transition';
  import TabBar from './TabBar.svelte';
  import StatusBar from './StatusBar.svelte';
  import SplitPane from './SplitPane.svelte';
  import SearchOverlay from './SearchOverlay.svelte';
  import PreferencesPanel from './PreferencesPanel.svelte';
  import SshHostKeyDialog from './SshHostKeyDialog.svelte';
  import SshCredentialDialog from './SshCredentialDialog.svelte';
  import SshPassphraseDialog from './SshPassphraseDialog.svelte';
  import ConnectionManager from './ConnectionManager.svelte';
  import FullscreenExitBadge from './FullscreenExitBadge.svelte';
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import { Network, MousePointerClick, Maximize2, Minimize2 } from 'lucide-svelte';
  import { useTerminalView } from '$lib/composables/useTerminalView.svelte';
  import { sessionState, getActiveTab, getActivePanes } from '$lib/state/session.svelte';
  import { hostKeyPrompt, credentialPrompt, passphrasePrompt } from '$lib/state/ssh.svelte';
  import { preferences } from '$lib/state/preferences.svelte';
  import { fullscreenState } from '$lib/state/fullscreen.svelte';
  import { setActivePane } from '$lib/ipc/commands';
  import * as m from '$lib/paraglide/messages';
  import { resolveTabTitle } from '$lib/utils/tab-title';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const tv = useTerminalView();

  // Respect prefers-reduced-motion: set fade duration to 0 when the user
  // has requested reduced motion. Evaluated once per component instance —
  // matchMedia is synchronous; no reactive wrapper needed since the media
  // query result does not change during a component's lifetime in this app.
  // The typeof guard also covers jsdom where matchMedia may be absent.
  const _reducedMotion =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const fadeDurationHint = _reducedMotion ? 0 : 300;
  const fadeDurationShort = _reducedMotion ? 0 : 200;

  // OS window title: "{tab-title} — TauTerm" (FS-TAB-010, UXD §7.1.10)
  // Reactive: updates on active tab change and active pane change (tab-title
  // already derives from activePaneId, so no separate pane dependency needed).
  $effect(() => {
    const tab = getActiveTab();
    const title = tab ? (resolveTabTitle(tab) ?? m.pane_title_fallback()) : m.pane_title_fallback();
    getCurrentWindow().setTitle(`${title} \u2014 TauTerm`);
  });

  // Derived from shared session state
  const activeTab = $derived(getActiveTab());
  const activePanes = $derived(getActivePanes());
  const activePaneState = $derived(
    activeTab
      ? (activePanes.find((p) => p.paneId === activeTab.activePaneId)?.state ?? null)
      : null,
  );

  // ---------------------------------------------------------------------------
  // Fullscreen auto-hide: tab bar and status bar fade out after 1.5s in
  // fullscreen mode, and reappear when the user hovers the top/bottom edge.
  // ---------------------------------------------------------------------------

  let tabBarVisible = $state(true);
  let statusBarVisible = $state(true);
  let tabBarHideTimer: ReturnType<typeof setTimeout> | null = null;
  let statusBarHideTimer: ReturnType<typeof setTimeout> | null = null;

  function recallTabBar() {
    tabBarVisible = true;
    if (tabBarHideTimer) clearTimeout(tabBarHideTimer);
    tabBarHideTimer = setTimeout(() => {
      if (fullscreenState.value) tabBarVisible = false;
      tabBarHideTimer = null;
    }, 1500);
  }

  function recallStatusBar() {
    statusBarVisible = true;
    if (statusBarHideTimer) clearTimeout(statusBarHideTimer);
    statusBarHideTimer = setTimeout(() => {
      if (fullscreenState.value) statusBarVisible = false;
      statusBarHideTimer = null;
    }, 1500);
  }

  // When entering/exiting fullscreen, reset visibility state.
  $effect(() => {
    if (!fullscreenState.value) {
      // Exiting fullscreen: restore bars immediately, cancel pending timers.
      tabBarVisible = true;
      statusBarVisible = true;
      if (tabBarHideTimer) {
        clearTimeout(tabBarHideTimer);
        tabBarHideTimer = null;
      }
      if (statusBarHideTimer) {
        clearTimeout(statusBarHideTimer);
        statusBarHideTimer = null;
      }
    } else {
      // Entering fullscreen: start the auto-hide sequence.
      recallTabBar();
      recallStatusBar();
    }
  });
</script>

<svelte:window onkeydown={tv.handleGlobalKeydown} />

<div class="terminal-view" role="application" aria-label={m.terminal_view_aria_label()}>
  <!-- Accessible aria-live region for fullscreen state announcements (WCAG 2.1 AA) -->
  <div aria-live="polite" class="sr-only" aria-atomic="true">
    {#if tv.isFullscreen}{m.fullscreen_entered()}{/if}
  </div>

  <!-- Tab bar: renders tabs from session state -->
  <div
    class="terminal-view__tab-row"
    class:terminal-view__tab-row--hidden={fullscreenState.value && !tabBarVisible}
  >
    <TabBar
      tabs={sessionState.tabs}
      activeTabId={sessionState.activeTabId}
      onTabClick={tv.handleTabClick}
      onTabClose={tv.handleTabClose}
      onNewTab={tv.handleNewTab}
      requestedRenameTabId={tv.requestedRenameTabId}
      onRenameHandled={() => {
        tv.requestedRenameTabId = null;
      }}
      onRenameComplete={() => {
        if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
        tv.activeViewportEl?.focus({ preventScroll: true });
      }}
      onEscapeTabBar={() => {
        if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
        tv.activeViewportEl?.focus({ preventScroll: true });
      }}
    />
    <!-- SSH connections toggle button (FS-SSH-031, UXD §7.1.8) -->
    <button
      class="terminal-view__ssh-btn"
      class:terminal-view__ssh-btn--active={tv.connectionManagerOpen}
      type="button"
      tabindex={-1}
      onclick={() => {
        tv.connectionManagerOpen = !tv.connectionManagerOpen;
      }}
      onmousedown={(e) => e.preventDefault()}
      aria-label={tv.connectionManagerOpen
        ? m.ssh_connections_panel_close()
        : m.ssh_connections_panel_open()}
      aria-pressed={tv.connectionManagerOpen}
      title={m.ssh_connections_panel_toggle()}
    >
      <Network size={16} aria-hidden="true" />
    </button>
    <!-- Fullscreen toggle button (FS-FULL-004) -->
    <button
      class="terminal-view__ssh-btn"
      type="button"
      tabindex={-1}
      onclick={tv.handleToggleFullscreen}
      aria-label={tv.isFullscreen ? m.exit_fullscreen() : m.enter_fullscreen()}
      aria-pressed={tv.isFullscreen}
      title={tv.isFullscreen ? m.exit_fullscreen() : m.enter_fullscreen()}
      data-testid="fullscreen-toggle-btn"
      onmousedown={(e) => e.preventDefault()}
    >
      {#if tv.isFullscreen}
        <Minimize2 size={16} aria-hidden="true" />
      {:else}
        <Maximize2 size={16} aria-hidden="true" />
      {/if}
    </button>
  </div>

  <!-- Pane area: render the full split-tree layout for the active tab -->
  <!-- FS-UX-002: contextmenu bubbles up from TerminalPane to dismiss the first-launch hint -->
  <div class="terminal-view__pane-area" role="none" oncontextmenu={tv.handleContextMenuHintDismiss}>
    {#key sessionState.activeTabId}
      {#if activeTab && activePanes.length > 0}
        <div
          id="tab-panel-{activeTab.id}"
          role="tabpanel"
          aria-labelledby="tab-{activeTab.id}"
          class="terminal-view__tab-panel"
        >
          <SplitPane
            node={activeTab.layout}
            tabId={activeTab.id}
            activePaneId={activeTab.activePaneId}
            wordDelimiters={preferences.value?.terminal.wordDelimiters}
            confirmMultilinePaste={preferences.value?.terminal.confirmMultilinePaste ?? true}
            cursorBlinkMs={preferences.value?.appearance.cursorBlinkMs}
            bellType={preferences.value?.terminal.bellType}
            fontFamily={preferences.value?.appearance.fontFamily}
            fontSize={preferences.value?.appearance.fontSize}
            hideCursorWhileTyping={preferences.value?.appearance.hideCursorWhileTyping ?? true}
            lineHeight={tv.activeThemeLineHeight}
            searchMatches={tv.searchMatches}
            activeSearchMatchIndex={tv.searchCurrentIdx}
            canClosePane={activePanes.length > 1}
            showPaneTitleBar={preferences.value?.appearance?.showPaneTitleBar ?? true}
            onpaneclick={async (paneId) => {
              try {
                await setActivePane(paneId);
              } catch {
                /* non-fatal */
              }
            }}
            onclosepane={tv.handlePaneClose}
            onsearch={() => {
              tv.searchOpen = true;
            }}
            onsplith={() => tv.handleSplitPane('horizontal')}
            onsplitv={() => tv.handleSplitPane('vertical')}
            ondisableConfirmMultilinePaste={() =>
              tv.handlePreferencesUpdate({ terminal: { confirmMultilinePaste: false } })}
            ondimensionschange={(paneId, c, r) => tv.handleDimensionsChange(paneId, c, r)}
            onrenamepane={tv.handlePaneRename}
            onviewportactive={(el) => {
              if (el !== null) {
                tv.activeViewportEl = el;
              }
              // null ignoré — une nouvelle registration par le pane actif prend la main ;
              // document.contains() dans le focus guard gère les refs périmées.
            }}
          />
        </div>
      {:else}
        <div class="terminal-view__empty">
          <p>{m.terminal_view_empty()}</p>
        </div>
      {/if}
    {/key}
  </div>

  <!-- FS-UX-002: First-launch context menu hint — non-blocking, bottom-right corner -->
  {#if tv.contextMenuHintVisible}
    <div
      class="terminal-view__context-hint"
      aria-hidden="true"
      transition:fade={{ duration: fadeDurationHint }}
    >
      <MousePointerClick size={14} aria-hidden="true" />
      <span>{m.context_menu_hint()}</span>
    </div>
  {/if}

  <!-- Status bar: reflects active pane state (DIV-UXD-008) -->
  <div
    class="terminal-view__status-row"
    class:terminal-view__status-row--hidden={fullscreenState.value && !statusBarVisible}
  >
    <StatusBar
      {activePaneState}
      cols={tv.activePaneCols}
      rows={tv.activePaneRows}
      dimsVisible={tv.dimsVisible}
      onsettings={() => {
        tv.prefsOpen = true;
      }}
    />
  </div>

  <!-- SearchOverlay: positioned relative to pane area (FS-SEARCH-007, UXD §7.4) -->
  {#if activePanes.length > 0}
    <div class="terminal-view__search-container">
      <SearchOverlay
        bind:open={tv.searchOpen}
        matchCount={tv.searchMatches.length}
        currentMatch={tv.searchCurrentIdx}
        onsearch={tv.handleSearch}
        onnext={tv.handleSearchNext}
        onprev={tv.handleSearchPrev}
        onclose={tv.handleSearchClose}
      />
    </div>
  {/if}

  <!-- PreferencesPanel: modal dialog (FS-PREF-005, UXD §7.6) -->
  <PreferencesPanel
    bind:open={tv.prefsOpen}
    preferences={preferences.value}
    onclose={() => {
      tv.prefsOpen = false;
    }}
    onCloseAutoFocus={() => {
      // Bits UI FocusScope may still be active at the instant this fires.
      // Calling focus() synchronously can be undone by FocusScope teardown.
      // Defer to next frame — by then the dialog is fully removed from DOM
      // and nothing will steal focus back.
      requestAnimationFrame(() => {
        tv.activeViewportEl?.focus({ preventScroll: true });
      });
    }}
    onupdate={tv.handlePreferencesUpdate}
  />

  <!-- SSH TOFU host key dialog (FS-SSH-011, SEC-BLK-004) -->
  <SshHostKeyDialog
    open={hostKeyPrompt.value !== null}
    host={hostKeyPrompt.value?.host ?? ''}
    keyType={hostKeyPrompt.value?.keyType ?? ''}
    fingerprint={hostKeyPrompt.value?.fingerprint ?? ''}
    isChanged={hostKeyPrompt.value?.isChanged ?? false}
    onaccept={tv.handleAcceptHostKey}
    onreject={tv.handleRejectHostKey}
    onclose={tv.handleRejectHostKey}
  />

  <!-- SSH credential prompt dialog (FS-SSH-012) -->
  <SshCredentialDialog
    open={credentialPrompt.value !== null}
    host={credentialPrompt.value?.host ?? ''}
    username={credentialPrompt.value?.username ?? ''}
    prompt={credentialPrompt.value?.prompt}
    failed={credentialPrompt.value?.failed ?? false}
    isKeychainAvailable={credentialPrompt.value?.isKeychainAvailable ?? false}
    onsubmit={tv.handleProvideCredentials}
    oncancel={tv.handleCancelCredentials}
    onclose={tv.handleCancelCredentials}
  />

  <!-- SSH passphrase prompt dialog (FS-SSH-019a) -->
  <SshPassphraseDialog
    open={passphrasePrompt.value !== null}
    keyPathLabel={passphrasePrompt.value?.keyPathLabel ?? ''}
    failed={passphrasePrompt.value?.failed ?? false}
    isKeychainAvailable={passphrasePrompt.value?.isKeychainAvailable ?? false}
    onsubmit={tv.handleProvidePassphrase}
    oncancel={tv.handleCancelPassphrase}
    onclose={tv.handleCancelPassphrase}
  />

  <!-- FS-PTY-008: Close confirmation dialog — DIV-UXD-012: initial focus on Cancel -->
  <Dialog
    open={tv.pendingClose !== null}
    title={m.close_confirm_title()}
    size="small"
    onclose={tv.handleCloseCancel}
    onopenautoFocus={(e) => {
      e.preventDefault();
      tv.closeConfirmCancelBtn?.focus();
    }}
  >
    {#snippet children()}
      <p class="text-[14px] text-(--color-text-secondary) leading-relaxed">
        {m.close_confirm_body()}
      </p>
    {/snippet}
    {#snippet footer()}
      <Button
        variant="ghost"
        bind:buttonRef={tv.closeConfirmCancelBtn}
        onclick={tv.handleCloseCancel}
        data-testid="close-confirm-cancel">{m.action_cancel()}</Button
      >
      <Button
        variant="destructive"
        onclick={tv.handleCloseConfirm}
        data-testid="close-confirm-action">{m.close_confirm_action()}</Button
      >
    {/snippet}
  </Dialog>

  <!-- FS-PTY-008: Window close confirmation dialog (WM close button) -->
  <Dialog
    open={tv.pendingWindowClose !== null}
    title={m.window_close_confirm_title()}
    size="small"
    onclose={tv.handleWindowCloseCancel}
    onopenautoFocus={(e) => {
      e.preventDefault();
      tv.windowCloseConfirmCancelBtn?.focus();
    }}
  >
    {#snippet children()}
      <p class="text-[14px] text-(--color-text-secondary) leading-relaxed">
        {m.window_close_confirm_body({ count: tv.pendingWindowClose?.paneCount ?? 0 })}
      </p>
    {/snippet}
    {#snippet footer()}
      <Button
        variant="ghost"
        bind:buttonRef={tv.windowCloseConfirmCancelBtn}
        onclick={tv.handleWindowCloseCancel}
        data-testid="window-close-confirm-cancel">{m.action_cancel()}</Button
      >
      <Button
        variant="destructive"
        onclick={tv.handleWindowCloseConfirm}
        data-testid="window-close-confirm-action">{m.close_confirm_action()}</Button
      >
    {/snippet}
  </Dialog>

  <!-- SSH connections slide-in panel -->
  {#if tv.connectionManagerOpen}
    <ConnectionManager
      standalone={true}
      connections={tv.savedConnections}
      onsave={tv.handleConnectionSave}
      ondelete={tv.handleConnectionDelete}
      onopen={tv.handleConnectionOpen}
      onclose={() => {
        tv.connectionManagerOpen = false;
        tv.connectionOpenError = false;
        // Focus restoration is handled by the focusin safety net in
        // useTerminalView.core once focus lands on document.body.
        // Defer to next frame to avoid racing with Bits UI FocusScope teardown.
        requestAnimationFrame(() => {
          if (
            document.activeElement === document.body &&
            !document.querySelector('[role="dialog"][aria-modal="true"]')
          ) {
            tv.activeViewportEl?.focus({ preventScroll: true });
          }
        });
      }}
    />
  {/if}

  <!-- Fullscreen: hover zones to recall hidden bars, and exit badge -->
  {#if fullscreenState.value}
    <!-- Top hover zone: recalls the tab bar -->
    <div
      class="terminal-view__fullscreen-hover-top"
      onmouseenter={recallTabBar}
      aria-hidden="true"
    ></div>
    <!-- Bottom hover zone: recalls the status bar -->
    <div
      class="terminal-view__fullscreen-hover-bottom"
      onmouseenter={recallStatusBar}
      aria-hidden="true"
    ></div>
    <!-- Exit badge: visible only when tab bar is hidden -->
    <FullscreenExitBadge {tabBarVisible} onToggle={tv.handleToggleFullscreen} />
  {/if}

  <!-- FS-SSH-032: connection open error banner -->
  {#if tv.connectionOpenError}
    <div
      class="terminal-view__connection-error"
      role="alert"
      aria-live="assertive"
      transition:fade={{ duration: fadeDurationShort }}
    >
      <span>{m.error_connection_failed()}</span>
      <button
        type="button"
        class="terminal-view__connection-error-close"
        onclick={() => {
          tv.connectionOpenError = false;
        }}
        aria-label={m.action_close()}>{m.action_close()}</button
      >
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
    transition: opacity var(--duration-fast) var(--ease-in);
  }

  @media (prefers-reduced-motion: reduce) {
    .terminal-view__tab-row {
      transition: none;
    }
  }

  .terminal-view__tab-row--hidden {
    opacity: 0;
    pointer-events: none;
  }

  .terminal-view__status-row {
    flex-shrink: 0;
    transition: opacity var(--duration-fast) var(--ease-in);
  }

  @media (prefers-reduced-motion: reduce) {
    .terminal-view__status-row {
      transition: none;
    }
  }

  .terminal-view__status-row--hidden {
    opacity: 0;
    pointer-events: none;
  }

  /* Fullscreen hover zones — 4px strips at top/bottom edges */
  .terminal-view__fullscreen-hover-top,
  .terminal-view__fullscreen-hover-bottom {
    position: fixed;
    left: 0;
    right: 0;
    height: 4px;
    z-index: var(--z-fullscreen-chrome);
  }

  .terminal-view__fullscreen-hover-top {
    top: 0;
  }

  .terminal-view__fullscreen-hover-bottom {
    bottom: 0;
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
    background-color: color-mix(
      in srgb,
      var(--term-bg) calc(var(--terminal-opacity, 1) * 100%),
      transparent
    );
  }

  .terminal-view__tab-panel {
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
