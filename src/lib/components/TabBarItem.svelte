<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TabBarItem — renders a single tab in the tab bar.

  Handles: SSH badge, inline rename input, title display, activity indicator,
  close button, drag-and-drop events, keyboard navigation, and context menu trigger.

  Props flow:
    - tab, isActive, isDragging, isRenaming, renameValue — data (down)
    - rename callbacks, DnD callbacks, onTabClick/Close, onContextMenu — events (up)

  Security: tab title via text interpolation only — no {@html} (TUITC-SEC-010/011).
-->
<script lang="ts">
  import { X, Bell, CheckCircle, XCircle, Network, LayoutPanelLeft } from 'lucide-svelte';
  import type { TabState, PaneNotification } from '$lib/ipc/types';
  import * as m from '$lib/paraglide/messages';

  // Local ref for the rename input — focus managed here (no parent bind:this needed).
  let renameInputEl = $state<HTMLInputElement | null>(null);

  // Focus + select-all when rename mode activates for this tab.
  $effect(() => {
    if (isRenaming && renameInputEl !== null) {
      renameInputEl.focus();
      renameInputEl.select();
    }
  });

  interface Props {
    tab: TabState;
    index: number;
    isActive: boolean;
    isDragging: boolean;
    isRenaming: boolean;
    renameValue: string;
    notification: PaneNotification | null;
    title: string;
    isSSH: boolean;
    dropIndicatorIndex: number | null;
    dragTabId: string | null;
    onTabClick: (tabId: string) => void;
    onTabClose: (tabId: string) => void;
    onStartRename: (tabId: string, currentTitle: string) => void;
    onConfirmRename: (tabId: string) => void;
    onCancelRename: () => void;
    onRenameValueChange: (value: string) => void;
    onDragStart: (event: DragEvent, tabId: string) => void;
    onDragOver: (event: DragEvent, index: number) => void;
    onDragEnd: () => void;
    onDrop: (event: DragEvent, index: number) => void;
    onContextMenu: (event: MouseEvent, tabId: string) => void;
    onTabKeydown: (event: KeyboardEvent, tabId: string, title: string) => void;
    onRenameKeydown: (event: KeyboardEvent, tabId: string) => void;
    /** Whether the tab has ≥2 panes — controls the split indicator icon visibility. */
    isMultiPane?: boolean;
  }

  let {
    tab,
    index,
    isActive,
    isDragging,
    isRenaming,
    renameValue,
    notification,
    title,
    isSSH,
    dropIndicatorIndex,
    dragTabId,
    onTabClick,
    onTabClose,
    onStartRename,
    onConfirmRename,
    onCancelRename,
    onRenameValueChange,
    onDragStart,
    onDragOver,
    onDragEnd,
    onDrop,
    onContextMenu,
    onTabKeydown,
    onRenameKeydown,
    isMultiPane = false,
  }: Props = $props();
</script>

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
  id="tab-{tab.id}"
  role="tab"
  aria-selected={isActive}
  aria-controls="tab-panel-{tab.id}"
  tabindex={isActive ? 0 : -1}
  data-tab-id={tab.id}
  data-tab-index={index}
  draggable="true"
  onclick={() => {
    if (!isRenaming) onTabClick(tab.id);
  }}
  ondblclick={(e) => {
    e.preventDefault();
    onStartRename(tab.id, title);
  }}
  onmousedown={(e) => {
    // Middle-click (button 1) closes the tab (UXD §7.1.2)
    if (e.button === 1) {
      e.preventDefault();
      onTabClose(tab.id);
    }
  }}
  oncontextmenu={(e) => onContextMenu(e, tab.id)}
  onkeydown={(e) => onTabKeydown(e, tab.id, title)}
  ondragstart={(e) => onDragStart(e, tab.id)}
  ondragover={(e) => onDragOver(e, index)}
  ondrop={(e) => onDrop(e, index)}
  ondragend={onDragEnd}
>
  <!-- SSH badge (UXD §7.1.7) -->
  {#if isSSH}
    <span class="tab-bar__ssh-badge" aria-label={m.tab_bar_ssh_badge_aria_label()}>
      <Network size={12} aria-hidden="true" />
    </span>
  {/if}

  <!-- Split indicator (UXD §7.1.9) — visible when tab has ≥2 panes -->
  {#if isMultiPane && !isRenaming}
    <span class="tab-bar__split-indicator" aria-hidden="true">
      <LayoutPanelLeft size={14} />
    </span>
  {/if}

  <!-- Inline rename input (UXD §7.1.6) -->
  {#if isRenaming}
    <input
      bind:this={renameInputEl}
      class="tab-bar__rename-input"
      type="text"
      aria-label={m.tab_bar_rename_input_aria_label()}
      value={renameValue}
      oninput={(e) => onRenameValueChange((e.currentTarget as HTMLInputElement).value)}
      onkeydown={(e) => onRenameKeydown(e, tab.id)}
      onblur={() => onConfirmRename(tab.id)}
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
    >
      <X size={14} aria-hidden="true" />
    </button>
  {/if}
</div>
