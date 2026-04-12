<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TabBar — horizontal tab strip at the top of the window.
  Displays all open tabs, active indicator, SSH badges,
  and notification indicators.

  The new-tab button is the last element of this component, outside the scrollable
  tabs container, fixed at the right of the scroll area (UXD §7.1.5, §7.1.1).

  Props:
    tabs        — list of TabState from session state
    activeTabId — ID of the currently active tab
    onTabClick  — callback when a tab is clicked
    onTabClose  — callback when a tab close button is clicked
    onNewTab    — callback forwarded to the tab context menu "New tab" item

  Features:
    - Inline rename: double-click or F2 on focused tab (FS-TAB-006)
    - Right-click context menu → Rename (FS-TAB-006, UXD §7.8.2)
    - Drag-and-drop reorder via HTML5 DnD (FS-TAB-005)
    - Horizontal scroll with ChevronLeft/ChevronRight when tabs overflow (UXD §6.2, §12.2)
    - Scroll arrow badges when hidden tabs have active notifications (UXD §7.1.3)

  Libraries:
    - lucide-svelte: Plus icon (new-tab button)
    - ContextMenu: tab context menu

  Security:
    - Tab titles via Svelte text interpolation only — no {@html} (TUITC-SEC-010)
-->
<script lang="ts">
  import { Plus } from 'lucide-svelte';
  import type { TabState, PaneNotification } from '$lib/ipc/types';
  import * as m from '$lib/paraglide/messages';
  import { getRootPane, resolveTabTitle } from '$lib/utils/tab-title';
  import { useTabBarScroll } from '$lib/composables/TabBar.scroll.svelte';
  import { useTabBarRename } from '$lib/composables/useTabBarRename.svelte';
  import { useTabBarDnd } from '$lib/composables/useTabBarDnd.svelte';
  import { useTabBarContextMenu } from '$lib/composables/useTabBarContextMenu.svelte';
  import TabBarItem from './TabBarItem.svelte';
  import TabBarScroll from './TabBarScroll.svelte';
  import TabBarContextMenu from './TabBarContextMenu.svelte';

  interface Props {
    tabs: TabState[];
    activeTabId: string;
    onTabClick: (tabId: string) => void;
    onTabClose: (tabId: string) => void;
    /** Forwarded to the tab context menu "New tab" item. */
    onNewTab: () => void;
    /**
     * Set to the tab ID to start rename mode on that tab programmatically
     * (e.g. from the F2 global shortcut in TerminalView).
     * TabBar reacts to changes and clears it after acting.
     */
    requestedRenameTabId?: string | null;
    onRenameHandled?: () => void;
    /** Called when inline rename exits (confirm or cancel) so focus can be restored. */
    onRenameComplete?: () => void;
    /** Called when Escape is pressed on a focused tab (not in rename mode) so focus can return to the terminal viewport. */
    onEscapeTabBar?: () => void;
  }

  const {
    tabs,
    activeTabId,
    onTabClick,
    onTabClose,
    onNewTab,
    requestedRenameTabId = null,
    onRenameHandled,
    onRenameComplete,
    onEscapeTabBar,
  }: Props = $props();

  // Sort tabs by order field
  const sortedTabs = $derived([...tabs].sort((a, b) => a.order - b.order));

  // Memoised tab count — used to detect add/remove without reacting to tab
  // content changes (e.g. active tab switch causes tabs = tabs.map(…) in the
  // parent, which creates a new array reference with the same length).
  const tabCount = $derived(tabs.length);

  // ── Helpers ──────────────────────────────────────────────────────────────────

  /** Display title: user label takes precedence over OSC process title (FS-TAB-006). */
  function tabDisplayTitle(tab: TabState): string {
    return resolveTabTitle(tab) ?? m.tab_untitled();
  }

  /** Active pane notification for this tab. */
  function tabNotification(tab: TabState): PaneNotification | null {
    return getRootPane(tab)?.notification ?? null;
  }

  /** Is the root pane an SSH session? */
  function isSSHTab(tab: TabState): boolean {
    return getRootPane(tab)?.sessionType === 'ssh';
  }

  /** Does the tab have ≥2 panes (split layout)? */
  function isMultiPaneTab(tab: TabState): boolean {
    return tab.layout.type === 'split';
  }

  // ── Inline rename state (FS-TAB-006) ────────────────────────────────────────
  const rename = useTabBarRename({
    requestedRenameTabId: () => requestedRenameTabId,
    onRenameHandled: () => onRenameHandled?.(),
    onRenameComplete: () => onRenameComplete?.(),
    getTabDisplayTitle: (tabId) => {
      const tab = sortedTabs.find((t) => t.id === tabId);
      return tab ? tabDisplayTitle(tab) : null;
    },
  });

  // ── DOM refs for scroll composable ──────────────────────────────────────────
  let tabsContainerEl = $state<HTMLDivElement | null>(null);
  let tabBarEl = $state<HTMLDivElement | null>(null);
  let newTabBtnEl = $state<HTMLButtonElement | null>(null);

  // ── Scroll composable ────────────────────────────────────────────────────────
  const scroll = useTabBarScroll({
    tabsContainerEl: () => tabsContainerEl,
    tabBarEl: () => tabBarEl,
    newTabBtnEl: () => newTabBtnEl,
    activeTabId: () => activeTabId,
    sortedTabs: () => sortedTabs,
    tabNotification,
    tabCount: () => tabCount,
  });

  // ── Drag-and-drop reorder state (FS-TAB-005) ────────────────────────────────
  const dnd = useTabBarDnd({ tabs: () => sortedTabs });

  // ── Tab context menu state (UXD §7.8.2) ─────────────────────────────────────
  const contextMenu = useTabBarContextMenu({
    onRenameRequest: (tabId, title) => rename.startRename(tabId, title),
  });

  /** Keyboard handler for tab items (TUITC-UX-111 to 113). */
  function handleTabKeydown(event: KeyboardEvent, tabId: string, title: string) {
    if (rename.renamingTabId === tabId) return;

    if (event.key === 'F2') {
      event.preventDefault();
      rename.startRename(tabId, title);
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
    } else if (event.key === 'Escape') {
      event.preventDefault();
      onEscapeTabBar?.();
    } else if (
      !event.isComposing &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey &&
      event.key.length === 1
    ) {
      // Printable character: return focus to terminal so the keystroke lands there.
      // The key itself is intentionally NOT forwarded — focus change alone is sufficient
      // (the user will retype). This mirrors GNOME Terminal / iTerm2 behaviour: the
      // tab list is a transient navigation surface, not a permanent focus owner.
      onEscapeTabBar?.();
    }
  }

  /** Keyboard handler for the rename input field. */
  function handleRenameKeydown(event: KeyboardEvent, tabId: string) {
    if (event.key === 'Enter') {
      event.preventDefault();
      rename.confirmRename(tabId);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      rename.cancelRename();
    }
  }
</script>

<div bind:this={tabBarEl} class="tab-bar" role="tablist" aria-label={m.tab_bar_tabs_aria_label()}>
  <TabBarScroll
    canScrollLeft={scroll.canScrollLeft}
    canScrollRight={false}
    leftBadge={scroll.leftBadge}
    rightBadge={null}
    onScrollLeft={() => scroll.scrollTabs('left')}
    onScrollRight={() => scroll.scrollTabs('right')}
  />

  <div
    bind:this={tabsContainerEl}
    class="tab-bar__tabs"
    ondragleave={dnd.handleDragLeave}
    onscroll={scroll.updateScrollState}
    role="presentation"
  >
    {#each sortedTabs as tab, index (tab.id)}
      {@const isActive = tab.id === activeTabId}
      {@const notification = tabNotification(tab)}
      {@const title = tabDisplayTitle(tab)}
      {@const isRenaming = rename.renamingTabId === tab.id}
      {@const isDragging = dnd.dragTabId === tab.id}

      <TabBarItem
        {tab}
        {index}
        {isActive}
        {isDragging}
        {isRenaming}
        renameValue={rename.renameValue}
        {notification}
        {title}
        isSSH={isSSHTab(tab)}
        isMultiPane={isMultiPaneTab(tab)}
        dropIndicatorIndex={dnd.dropIndicatorIndex}
        dragTabId={dnd.dragTabId}
        {onTabClick}
        {onTabClose}
        onStartRename={rename.startRename}
        onConfirmRename={rename.confirmRename}
        onCancelRename={rename.cancelRename}
        onRenameValueChange={(v) => {
          rename.renameValue = v;
        }}
        onDragStart={dnd.handleDragStart}
        onDragOver={dnd.handleDragOver}
        onDragEnd={dnd.handleDragEnd}
        onDrop={dnd.handleDrop}
        onContextMenu={contextMenu.handleTabContextMenu}
        onTabKeydown={handleTabKeydown}
        onRenameKeydown={handleRenameKeydown}
      />
    {/each}

    <!-- Drop indicator: shown after the last tab -->
    {#if dnd.dropIndicatorIndex === sortedTabs.length && dnd.dragTabId !== null}
      <div class="tab-bar__drop-indicator" aria-hidden="true"></div>
    {/if}
  </div>

  <TabBarScroll
    canScrollLeft={false}
    canScrollRight={scroll.canScrollRight}
    leftBadge={null}
    rightBadge={scroll.rightBadge}
    onScrollLeft={() => scroll.scrollTabs('left')}
    onScrollRight={() => scroll.scrollTabs('right')}
  />

  <!-- New tab button — outside the scrollable zone, right of the scroll area (UXD §7.1.1) -->
  <button
    bind:this={newTabBtnEl}
    class="tab-bar__new-tab"
    type="button"
    tabindex={-1}
    aria-label={m.tab_bar_new_tab()}
    title={m.tab_bar_new_tab_tooltip()}
    onmousedown={(e) => e.preventDefault()}
    onclick={onNewTab}
  >
    <Plus size={16} aria-hidden="true" />
  </button>

  <!-- Tab context menu (UXD §7.8.2) -->
  {#if contextMenu.contextMenuTabId !== null}
    {@const ctxTab = sortedTabs.find((t) => t.id === contextMenu.contextMenuTabId)}
    {#if ctxTab}
      {@const ctxTitle = tabDisplayTitle(ctxTab)}
      <TabBarContextMenu
        contextMenuTabId={contextMenu.contextMenuTabId}
        contextMenuX={contextMenu.contextMenuX}
        contextMenuY={contextMenu.contextMenuY}
        {onNewTab}
        onRename={() => contextMenu.handleContextMenuRename(ctxTab.id, ctxTitle)}
        onCloseTab={() => {
          contextMenu.handleContextMenuClose();
          onTabClose(ctxTab.id);
        }}
        onClose={contextMenu.handleContextMenuClose}
      />
    {/if}
  {/if}
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
    /* flex: 1 0 0 — fill the full available row width (row − SSH button).
     * SSH button is therefore always anchored at the right edge of the window.
     * .tab-bar__tabs uses flex: 0 1 auto so it sizes to its content — the "+"
     * button lands right after the tabs, and empty bar space sits between them
     * and the SSH button. overflow: hidden clips any transient overflows. */
    flex: 1 0 0;
    overflow: hidden;
    min-width: 0;
  }

  .tab-bar__tabs {
    display: flex;
    align-items: stretch;
    /* flex: 1 — fills available bar space so the "+" button lands at the far
     * right of the tab strip area.
     * max-width: max-content — caps growth at the actual tab content width so
     * the "+" button lands right after the last tab rather than at the far
     * right of the bar.
     * IMPORTANT: clientWidth on this element is NOT used for overflow detection
     * (see updateScrollState in TabBar.scroll.svelte.ts). WebKitGTK resolves
     * max-width: max-content (and flex-basis: max-content) on overflow:auto
     * elements to ~min-content, making clientWidth unreliable. */
    flex: 1;
    max-width: max-content;
    height: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    /* Hide native scrollbar — navigation is provided by ChevronLeft/Right buttons */
    scrollbar-width: none;
    -ms-overflow-style: none;
    scroll-behavior: smooth;
  }

  .tab-bar__tabs::-webkit-scrollbar {
    display: none;
  }

  /* Scroll arrow buttons (UXD §6.2, §12.2) */
  :global(.tab-bar__scroll-arrow) {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    width: 24px;
    height: 100%;
    flex-shrink: 0;
    border: none;
    background: var(--color-tab-bg);
    color: var(--color-tab-new-fg);
    cursor: pointer;
    padding: 0;
    z-index: var(--z-base);
    transition:
      color var(--duration-instant),
      background-color var(--duration-instant);
  }

  :global(.tab-bar__scroll-arrow:hover) {
    color: var(--color-tab-new-hover-fg);
    background-color: var(--color-hover-bg);
  }

  :global(.tab-bar__scroll-arrow:active) {
    background-color: var(--color-active-bg);
  }

  :global(.tab-bar__scroll-arrow:focus-visible) {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }

  /* Left arrow: subtle right border to separate from tabs area */
  :global(.tab-bar__scroll-arrow--left) {
    border-right: 1px solid var(--color-border-subtle);
  }

  /* Right arrow: subtle left border */
  :global(.tab-bar__scroll-arrow--right) {
    border-left: 1px solid var(--color-border-subtle);
  }

  /* Activity badge on scroll arrows (UXD §7.1.3, §12.2) */
  :global(.tab-bar__scroll-badge) {
    position: absolute;
    top: 6px;
    right: 4px;
    width: var(--size-scroll-arrow-badge, 4px);
    height: var(--size-scroll-arrow-badge, 4px);
    border-radius: var(--radius-full);
    pointer-events: none;
  }

  :global(.tab-bar__scroll-badge--output) {
    background-color: var(--color-indicator-output);
  }

  :global(.tab-bar__scroll-badge--bell) {
    background-color: var(--color-indicator-bell);
  }

  /* Tab item — TUITC-UX-010 to 016 */
  :global(.tab-bar__tab) {
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
    transition:
      background-color var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  :global(.tab-bar__tab[draggable='true']) {
    cursor: grab;
  }

  /* Dragging state: lifted appearance (UXD §8.4) */
  :global(.tab-bar__tab--dragging) {
    opacity: 0.9;
    box-shadow: var(--shadow-raised);
    cursor: grabbing;
  }

  :global(.tab-bar__tab:hover:not(.tab-bar__tab--active)) {
    background-color: var(--color-tab-hover-bg);
    color: var(--color-tab-hover-fg);
  }

  :global(.tab-bar__tab--active) {
    background-color: var(--color-tab-active-bg);
    color: var(--color-tab-active-fg);
    font-weight: var(--font-weight-semibold);
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  :global(.tab-bar__tab:focus-visible) {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }

  /* Title: ellipsis truncation — TUITC-UX-014 */
  :global(.tab-bar__tab-title) {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /* Inline rename input (UXD §7.1.6) */
  :global(.tab-bar__rename-input) {
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

  /* Split indicator (UXD §7.1.9) */
  :global(.tab-bar__split-indicator) {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    opacity: 0.5;
  }

  /* SSH badge */
  :global(.tab-bar__ssh-badge) {
    display: flex;
    align-items: center;
    color: var(--color-ssh-badge-fg);
    flex-shrink: 0;
  }

  /* Activity indicators — TUITC-UX-020 to 024 */
  :global(.tab-bar__activity) {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  :global(.tab-bar__activity-dot) {
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
  :global(.tab-bar__close) {
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
  :global(.tab-bar__tab--active .tab-bar__close) {
    opacity: 1;
  }

  /* Visible on hover (any tab) */
  :global(.tab-bar__tab:hover .tab-bar__close) {
    opacity: 1;
  }

  :global(.tab-bar__close:hover) {
    color: var(--color-tab-close-hover-fg);
    background-color: var(--color-hover-bg);
  }

  :global(.tab-bar__close:active) {
    background-color: var(--color-active-bg);
  }

  :global(.tab-bar__close:focus-visible) {
    opacity: 1;
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  /* New tab button — right of scroll area, outside .tab-bar__tabs (UXD §7.1.1) */
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

  /* Drop insertion indicator (UXD §8.4) — 2px vertical bar in accent color */
  :global(.tab-bar__drop-indicator) {
    width: 2px;
    height: 100%;
    background-color: var(--color-accent);
    flex-shrink: 0;
    align-self: stretch;
    pointer-events: none;
  }
</style>
