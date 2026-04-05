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

  Libraries:
    - lucide-svelte: X, Plus, Bell, CheckCircle, XCircle, Network icons
    - bits-ui Tooltip: new-tab tooltip with 300ms delay (UXD §7.1.5, §7.10)

  Security:
    - Tab titles via Svelte text interpolation only — no {@html} (TUITC-SEC-010)
-->
<script lang="ts">
  import { Plus, X, Bell, CheckCircle, XCircle, Network } from 'lucide-svelte';
  import { Tooltip } from 'bits-ui';
  import type { TabState, PaneState, PaneNotification } from '$lib/ipc/types';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    tabs: TabState[];
    activeTabId: string;
    onTabClick: (tabId: string) => void;
    onTabClose: (tabId: string) => void;
    onNewTab: () => void;
  }

  const { tabs, activeTabId, onTabClick, onTabClose, onNewTab }: Props = $props();

  // Sort tabs by order field
  const sortedTabs = $derived([...tabs].sort((a, b) => a.order - b.order));

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
  function handleTabKeydown(event: KeyboardEvent, tabId: string) {
    if (event.key === 'Enter' || event.key === ' ') {
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
</script>

<div class="tab-bar" role="tablist" aria-label="Terminal tabs">
  <div class="tab-bar__tabs">
    {#each sortedTabs as tab (tab.id)}
      {@const isActive = tab.id === activeTabId}
      {@const notification = tabNotification(tab)}
      {@const title = tabDisplayTitle(tab)}

      <!-- Tab item — TUITC-UX-010 to 016 -->
      <!-- svelte-ignore a11y_interactive_supports_focus -->
      <div
        class="tab-bar__tab"
        class:tab-bar__tab--active={isActive}
        role="tab"
        aria-selected={isActive}
        tabindex={isActive ? 0 : -1}
        data-tab-id={tab.id}
        onclick={() => onTabClick(tab.id)}
        onkeydown={(e) => handleTabKeydown(e, tab.id)}
      >
        <!-- SSH badge (UXD §7.1.7) -->
        {#if isSSHTab(tab)}
          <span class="tab-bar__ssh-badge" aria-label="SSH session">
            <Network size={12} aria-hidden="true" />
          </span>
        {/if}

        <!-- Title: text interpolation — NEVER {@html} (TUITC-SEC-010/011) -->
        <span class="tab-bar__tab-title">{title}</span>

        <!-- Activity indicator (TUITC-UX-020 to 024) — only on inactive tabs -->
        {#if !isActive && notification}
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
      </div>
    {/each}
  </div>

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
</style>
