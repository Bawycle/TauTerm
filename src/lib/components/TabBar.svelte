<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TabBar — horizontal tab strip at the top of the window.
  Displays all open tabs, active indicator, SSH badges,
  notification indicators, and the new-tab button.

  Props:
    tabs        — list of TabState from session state
    activeTabId — ID of the currently active tab
    onTabClick  — callback when a tab is clicked
    onTabClose  — callback when a tab close button is clicked
    onNewTab    — callback when the new-tab button is clicked

  Features:
    - Inline rename: double-click or F2 on focused tab (FS-TAB-006)
    - Right-click context menu → Rename (FS-TAB-006, UXD §7.8.2)
    - Drag-and-drop reorder via HTML5 DnD (FS-TAB-005)

  Libraries:
    - lucide-svelte: X, Plus, Bell, CheckCircle, XCircle, Network icons
    - bits-ui Tooltip: new-tab tooltip with 300ms delay (UXD §7.1.5, §7.10)
    - ContextMenu: tab context menu

  Security:
    - Tab titles via Svelte text interpolation only — no {@html} (TUITC-SEC-010)
-->
<script lang="ts">
  import { Plus, X, Bell, CheckCircle, XCircle, Network } from 'lucide-svelte';
  import { Tooltip } from 'bits-ui';
  import { invoke } from '@tauri-apps/api/core';
  import type { TabState, PaneState, PaneNotification } from '$lib/ipc/types';
  import * as m from '$lib/paraglide/messages';
  import ContextMenu from './ContextMenu.svelte';

  interface Props {
    tabs: TabState[];
    activeTabId: string;
    onTabClick: (tabId: string) => void;
    onTabClose: (tabId: string) => void;
    onNewTab: () => void;
    /**
     * Set to the tab ID to start rename mode on that tab programmatically
     * (e.g. from the F2 global shortcut in TerminalView).
     * TabBar reacts to changes and clears it after acting.
     */
    requestedRenameTabId?: string | null;
    onRenameHandled?: () => void;
  }

  const {
    tabs,
    activeTabId,
    onTabClick,
    onTabClose,
    onNewTab,
    requestedRenameTabId = null,
    onRenameHandled,
  }: Props = $props();

  // Sort tabs by order field
  const sortedTabs = $derived([...tabs].sort((a, b) => a.order - b.order));

  // ── Inline rename state (FS-TAB-006) ────────────────────────────────────────
  // ID of the tab currently being renamed, or null.
  let renamingTabId = $state<string | null>(null);
  // Current value of the rename input.
  let renameValue = $state('');
  // Reference to the input element so we can focus it programmatically.
  let renameInputEl = $state<HTMLInputElement | null>(null);

  /** Enter rename mode for the given tab. */
  function startRename(tabId: string, currentTitle: string) {
    renamingTabId = tabId;
    renameValue = currentTitle;
    // Focus is applied via the bind:this + $effect below.
  }

  /** Confirm rename: send IPC, then exit rename mode. */
  async function confirmRename(tabId: string) {
    if (renamingTabId !== tabId) return;
    // Empty value → clear label and revert to OSC/process title.
    const label: string | null = renameValue.trim() === '' ? null : renameValue.trim();
    try {
      await invoke('rename_tab', { tabId, label });
    } catch {
      // IPC errors are non-fatal for the UI; the title will stay unchanged on next state update.
    }
    renamingTabId = null;
    renameValue = '';
  }

  /** Cancel rename without saving. */
  function cancelRename() {
    renamingTabId = null;
    renameValue = '';
  }

  // Focus the input whenever rename mode activates.
  $effect(() => {
    if (renamingTabId !== null && renameInputEl !== null) {
      renameInputEl.focus();
      renameInputEl.select();
    }
  });

  // React to an external rename request (e.g. F2 global shortcut from TerminalView).
  $effect(() => {
    if (requestedRenameTabId === null || requestedRenameTabId === undefined) return;
    const tab = sortedTabs.find((t) => t.id === requestedRenameTabId);
    if (!tab) return;
    startRename(tab.id, tabDisplayTitle(tab));
    onRenameHandled?.();
  });

  // ── Drag-and-drop reorder state (FS-TAB-005) ────────────────────────────────
  // ID of the tab currently being dragged.
  let dragTabId = $state<string | null>(null);
  // Index (in sortedTabs) where the drop indicator is shown — between tabs.
  // 0 = before first tab, N = after last tab.
  let dropIndicatorIndex = $state<number | null>(null);

  function handleDragStart(event: DragEvent, tabId: string) {
    dragTabId = tabId;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
      // Store the tab ID so we can identify the source on drop.
      event.dataTransfer.setData('text/plain', tabId);
    }
  }

  function handleDragOver(event: DragEvent, index: number) {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    dropIndicatorIndex = index;
  }

  function handleDragLeave(event: DragEvent) {
    // Only clear the indicator when leaving the tab bar entirely.
    const relatedTarget = event.relatedTarget as Node | null;
    const bar = (event.currentTarget as HTMLElement).closest('.tab-bar__tabs');
    if (bar && relatedTarget && bar.contains(relatedTarget)) return;
    dropIndicatorIndex = null;
  }

  async function handleDrop(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    const sourceId = event.dataTransfer?.getData('text/plain') ?? dragTabId;
    if (!sourceId) {
      resetDrag();
      return;
    }
    const sorted = sortedTabs;
    const sourceIdx = sorted.findIndex((t) => t.id === sourceId);
    if (sourceIdx === -1 || sourceIdx === targetIndex || sourceIdx + 1 === targetIndex) {
      resetDrag();
      return;
    }

    // Compute new_order for the Rust backend.
    // The tab will be inserted at position targetIndex in the sorted list
    // (after removing the source). We derive the desired order value from
    // neighbours in the remaining list.
    const remaining = sorted.filter((t) => t.id !== sourceId);
    let newOrder: number;
    if (targetIndex === 0) {
      // Drop before all tabs: use a value lower than the first remaining tab.
      newOrder = remaining.length > 0 ? remaining[0].order - 1 : 0;
    } else {
      // Drop after index (targetIndex - 1) in the remaining list.
      // Adjust targetIndex for the removed element.
      const insertAfter = sourceIdx < targetIndex ? targetIndex - 1 : targetIndex;
      const clampedInsert = Math.min(insertAfter, remaining.length - 1);
      if (clampedInsert < remaining.length - 1) {
        // Between two tabs: midpoint.
        newOrder = Math.floor(
          (remaining[clampedInsert].order + remaining[clampedInsert + 1].order) / 2,
        );
        // If midpoint equals one of the neighbours (integers too close), use neighbour + 1.
        if (newOrder === remaining[clampedInsert].order) {
          newOrder = remaining[clampedInsert].order + 1;
        }
      } else {
        // After last remaining tab.
        newOrder = remaining[clampedInsert].order + 1;
      }
    }

    try {
      await invoke('reorder_tab', { tabId: sourceId, newOrder });
    } catch {
      // Non-fatal; the backend is the source of truth.
    }
    resetDrag();
  }

  function handleDragEnd() {
    resetDrag();
  }

  function resetDrag() {
    dragTabId = null;
    dropIndicatorIndex = null;
  }

  // ── Tab context menu state (UXD §7.8.2) ─────────────────────────────────────
  // ID of the tab whose context menu is open, or null.
  let contextMenuTabId = $state<string | null>(null);
  // Position for the DropdownMenu content — used as a CSS anchor override.
  // Bits UI DropdownMenu doesn't natively support anchor-to-pointer positioning,
  // so we place an absolutely-positioned trigger at the pointer location.
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);

  function handleTabContextMenu(event: MouseEvent, tabId: string) {
    event.preventDefault();
    contextMenuX = event.clientX;
    contextMenuY = event.clientY;
    contextMenuTabId = tabId;
  }

  function handleContextMenuClose() {
    contextMenuTabId = null;
  }

  function handleContextMenuRename(tabId: string, title: string) {
    contextMenuTabId = null;
    startRename(tabId, title);
  }

  // ── Helpers ──────────────────────────────────────────────────────────────────

  /** Extract the root (first leaf) pane state from a tab's layout tree. */
  function getRootPane(tab: TabState): PaneState | null {
    let node = tab.layout;
    while (node.type === 'split') node = node.first;
    return node.type === 'leaf' ? node.state : null;
  }

  /** Display title: user label takes precedence over OSC process title (FS-TAB-006). */
  function tabDisplayTitle(tab: TabState): string {
    if (tab.label !== null) return tab.label;
    return getRootPane(tab)?.processTitle ?? 'Terminal';
  }

  /** Active pane notification for this tab. */
  function tabNotification(tab: TabState): PaneNotification | null {
    return getRootPane(tab)?.notification ?? null;
  }

  /** Is the root pane an SSH session? */
  function isSSHTab(tab: TabState): boolean {
    return getRootPane(tab)?.sessionType === 'ssh';
  }

  /** Keyboard handler for tab items (TUITC-UX-111 to 113). */
  function handleTabKeydown(event: KeyboardEvent, tabId: string, title: string) {
    if (renamingTabId === tabId) return; // Let the input handle keys.

    if (event.key === 'F2') {
      event.preventDefault();
      startRename(tabId, title);
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onTabClick(tabId);
    } else if (event.key === 'Delete') {
      event.preventDefault();
      onTabClose(tabId);
    } else if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
      event.preventDefault();
      const sorted = sortedTabs;
      const idx = sorted.findIndex((t) => t.id === tabId);
      if (idx === -1) return;
      const next =
        event.key === 'ArrowRight'
          ? sorted[(idx + 1) % sorted.length]
          : sorted[(idx - 1 + sorted.length) % sorted.length];
      const nextEl = document.querySelector<HTMLElement>(`[data-tab-id="${next.id}"]`);
      nextEl?.focus();
    }
  }

  /** Keyboard handler for the rename input field. */
  function handleRenameKeydown(event: KeyboardEvent, tabId: string) {
    if (event.key === 'Enter') {
      event.preventDefault();
      confirmRename(tabId);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      cancelRename();
    }
  }
</script>

<div class="tab-bar" role="tablist" aria-label={m.tab_bar_tabs_aria_label()}>
  <div class="tab-bar__tabs" ondragleave={handleDragLeave} role="presentation">
    {#each sortedTabs as tab, index (tab.id)}
      {@const isActive = tab.id === activeTabId}
      {@const notification = tabNotification(tab)}
      {@const title = tabDisplayTitle(tab)}
      {@const isRenaming = renamingTabId === tab.id}
      {@const isDragging = dragTabId === tab.id}

      <!-- Drop indicator: shown before this tab -->
      {#if dropIndicatorIndex === index && dragTabId !== null}
        <div class="tab-bar__drop-indicator" aria-hidden="true"></div>
      {/if}

      <!-- Tab item — TUITC-UX-010 to 016 -->
      <!-- svelte-ignore a11y_interactive_supports_focus -->
      <div
        class="tab-bar__tab"
        class:tab-bar__tab--active={isActive}
        class:tab-bar__tab--dragging={isDragging}
        role="tab"
        aria-selected={isActive}
        tabindex={isActive ? 0 : -1}
        data-tab-id={tab.id}
        draggable="true"
        onclick={() => {
          if (!isRenaming) onTabClick(tab.id);
        }}
        ondblclick={(e) => {
          e.preventDefault();
          startRename(tab.id, title);
        }}
        oncontextmenu={(e) => handleTabContextMenu(e, tab.id)}
        onkeydown={(e) => handleTabKeydown(e, tab.id, title)}
        ondragstart={(e) => handleDragStart(e, tab.id)}
        ondragover={(e) => handleDragOver(e, index)}
        ondrop={(e) => handleDrop(e, index)}
        ondragend={handleDragEnd}
      >
        <!-- SSH badge (UXD §7.1.7) -->
        {#if isSSHTab(tab)}
          <span class="tab-bar__ssh-badge" aria-label={m.tab_bar_ssh_badge_aria_label()}>
            <Network size={12} aria-hidden="true" />
          </span>
        {/if}

        <!-- Inline rename input (UXD §7.1.6) -->
        {#if isRenaming}
          <input
            bind:this={renameInputEl}
            class="tab-bar__rename-input"
            type="text"
            aria-label={m.tab_bar_rename_input_aria_label()}
            bind:value={renameValue}
            onkeydown={(e) => handleRenameKeydown(e, tab.id)}
            onblur={() => confirmRename(tab.id)}
            onclick={(e) => e.stopPropagation()}
          />
        {:else}
          <!-- Title: text interpolation — NEVER {@html} (TUITC-SEC-010/011) -->
          <span class="tab-bar__tab-title">{title}</span>
        {/if}

        <!-- Activity indicator (TUITC-UX-020 to 024) — only on inactive tabs -->
        {#if !isActive && notification && !isRenaming}
          <span class="tab-bar__activity" aria-hidden="true">
            {#if notification.type === 'backgroundOutput'}
              <span class="tab-bar__activity-dot"></span>
            {:else if notification.type === 'processExited'}
              {#if (notification as { type: 'processExited'; exitCode: number }).exitCode === 0}
                <CheckCircle size={14} class="activity-icon activity-icon--success" />
              {:else}
                <XCircle size={14} class="activity-icon activity-icon--error" />
              {/if}
            {:else if notification.type === 'bell'}
              <Bell size={14} class="activity-icon activity-icon--bell" />
            {/if}
          </span>
        {/if}

        <!-- Close button — TUITC-UX-030 to 034 (44×44 hit area) -->
        {#if !isRenaming}
          <button
            class="tab-bar__close"
            type="button"
            aria-label={m.tab_bar_close_tab()}
            tabindex={-1}
            onclick={(e) => {
              e.stopPropagation();
              onTabClose(tab.id);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                e.stopPropagation();
                onTabClose(tab.id);
              }
            }}
          >
            <X size={14} aria-hidden="true" />
          </button>
        {/if}
      </div>
    {/each}

    <!-- Drop indicator: shown after the last tab -->
    {#if dropIndicatorIndex === sortedTabs.length && dragTabId !== null}
      <div class="tab-bar__drop-indicator" aria-hidden="true"></div>
    {/if}
  </div>

  <!-- Tab context menu (UXD §7.8.2) — DropdownMenu anchored to pointer position.
       Rendered once; controlled by contextMenuTabId. The ContextMenu trigger is
       fixed-positioned at the right-click coordinates via anchorX/anchorY. -->
  {#if contextMenuTabId !== null}
    {@const ctxTab = sortedTabs.find((t) => t.id === contextMenuTabId)}
    {#if ctxTab}
      {@const ctxTitle = tabDisplayTitle(ctxTab)}
      <ContextMenu
        variant="tab"
        open={true}
        anchorX={contextMenuX}
        anchorY={contextMenuY}
        onclose={handleContextMenuClose}
        onnewtab={onNewTab}
        onrename={() => handleContextMenuRename(ctxTab.id, ctxTitle)}
        onclosetab={() => {
          handleContextMenuClose();
          onTabClose(ctxTab.id);
        }}
      />
    {/if}
  {/if}

  <!-- New tab button with Bits UI Tooltip (TUITC-UX-040 to 043, UXD §7.1.5) -->
  <Tooltip.Root delayDuration={300}>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          class="tab-bar__new-tab"
          type="button"
          aria-label={m.tab_bar_new_tab()}
          onclick={onNewTab}
        >
          <Plus size={14} aria-hidden="true" />
        </button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content class="tab-bar__tooltip">
      {m.tab_bar_new_tab_tooltip()}
    </Tooltip.Content>
  </Tooltip.Root>
</div>

<style>
  /* Tab bar container — TUITC-UX-001 to 004 */
  .tab-bar {
    display: flex;
    align-items: center;
    height: var(--size-tab-height);
    min-height: var(--size-tab-height);
    background-color: var(--color-tab-bg);
    border-bottom: 1px solid var(--color-border);
    overflow: hidden;
    flex-shrink: 0;
  }

  .tab-bar__tabs {
    display: flex;
    align-items: stretch;
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  /* Tab item — TUITC-UX-010 to 016 */
  .tab-bar__tab {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    height: 100%;
    min-width: 120px;
    max-width: 240px;
    padding: 0 var(--space-2) 0 var(--space-3);
    border: none;
    background: transparent;
    color: var(--color-tab-inactive-fg);
    font-size: var(--font-size-ui-base);
    font-family: var(--font-ui);
    font-weight: var(--font-weight-normal);
    cursor: pointer;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    flex-shrink: 0;
    outline: none;
    user-select: none;
    position: relative;
  }

  .tab-bar__tab[draggable='true'] {
    cursor: grab;
  }

  /* Dragging state: lifted appearance (UXD §8.4) */
  .tab-bar__tab--dragging {
    opacity: 0.9;
    box-shadow: var(--shadow-raised);
    cursor: grabbing;
  }

  .tab-bar__tab:hover:not(.tab-bar__tab--active) {
    background-color: var(--color-tab-hover-bg);
    color: var(--color-tab-hover-fg);
  }

  .tab-bar__tab--active {
    background-color: var(--color-tab-active-bg);
    color: var(--color-tab-active-fg);
    font-weight: var(--font-weight-semibold);
  }

  .tab-bar__tab:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }

  /* Title: ellipsis truncation — TUITC-UX-014 */
  .tab-bar__tab-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /* Inline rename input (UXD §7.1.6) */
  .tab-bar__rename-input {
    flex: 1;
    min-width: 0;
    height: calc(var(--size-tab-height) - var(--space-2) * 2);
    padding: 0 var(--space-1);
    background-color: var(--term-bg);
    border: 1px solid var(--color-focus-ring);
    border-radius: var(--radius-sm);
    color: var(--color-tab-active-fg);
    font-size: var(--font-size-ui-base);
    font-family: var(--font-ui);
    font-weight: var(--font-weight-normal);
    outline: none;
    cursor: text;
    user-select: text;
  }

  /* SSH badge */
  .tab-bar__ssh-badge {
    display: flex;
    align-items: center;
    color: var(--color-ssh-badge-fg);
    flex-shrink: 0;
  }

  /* Activity indicators — TUITC-UX-020 to 024 */
  .tab-bar__activity {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .tab-bar__activity-dot {
    display: inline-block;
    width: var(--size-badge, 6px);
    height: var(--size-badge, 6px);
    border-radius: 50%;
    background-color: var(--color-activity);
  }

  /* Activity icon color classes are set on lucide-svelte components via class prop */
  :global(.activity-icon--success) {
    color: var(--color-process-end);
  }
  :global(.activity-icon--error) {
    color: var(--color-error);
  }
  :global(.activity-icon--bell) {
    color: var(--color-bell);
  }

  /* Close button — TUITC-UX-030 to 034 */
  .tab-bar__close {
    display: flex;
    align-items: center;
    justify-content: center;
    /* 44×44 hit area via negative margin compensation */
    width: var(--size-target-min);
    height: var(--size-target-min);
    margin: 0 calc((var(--size-target-min) - var(--size-icon-sm, 14px)) / -2);
    border: none;
    background: transparent;
    color: var(--color-tab-close-fg);
    cursor: pointer;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    padding: 0;
    opacity: 0;
    transition:
      opacity var(--duration-instant),
      color var(--duration-instant),
      background-color var(--duration-instant);
  }

  /* Always visible on active tab */
  .tab-bar__tab--active .tab-bar__close {
    opacity: 1;
  }

  /* Visible on hover (any tab) */
  .tab-bar__tab:hover .tab-bar__close {
    opacity: 1;
  }

  .tab-bar__close:hover {
    color: var(--color-tab-close-hover-fg);
    background-color: var(--color-hover-bg);
  }

  .tab-bar__close:active {
    background-color: var(--color-active-bg);
  }

  .tab-bar__close:focus-visible {
    opacity: 1;
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  /* New tab button — TUITC-UX-040 to 043 */
  .tab-bar__new-tab {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--size-target-min);
    height: 100%;
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--color-tab-new-fg);
    cursor: pointer;
    padding: 0;
    transition:
      color var(--duration-instant),
      background-color var(--duration-instant);
  }

  .tab-bar__new-tab:hover {
    color: var(--color-tab-new-hover-fg);
    background-color: var(--color-hover-bg);
  }

  .tab-bar__new-tab:active {
    background-color: var(--color-active-bg);
  }

  .tab-bar__new-tab:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  /* Tooltip content (UXD §7.10) */
  :global(.tab-bar__tooltip) {
    background-color: var(--color-bg-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-ui-xs);
    color: var(--color-text-primary);
    box-shadow: var(--shadow-raised);
    z-index: var(--z-tooltip, 60);
    white-space: nowrap;
  }

  /* Drop insertion indicator (UXD §8.4) — 2px vertical bar in accent color */
  .tab-bar__drop-indicator {
    width: 2px;
    height: 100%;
    background-color: var(--color-accent);
    flex-shrink: 0;
    align-self: stretch;
    pointer-events: none;
  }
</style>
