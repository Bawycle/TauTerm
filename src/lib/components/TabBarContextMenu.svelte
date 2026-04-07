<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TabBarContextMenu — wrapper around ContextMenu for tab actions (UXD §7.8.2).

  Rendered only when contextMenuTabId is non-null. The ContextMenu is shown
  anchored at the pointer coordinates (anchorX / anchorY).

  Props:
    contextMenuTabId  — ID of the tab whose menu is open, or null
    contextMenuX      — pointer X coordinate (clientX)
    contextMenuY      — pointer Y coordinate (clientY)
    onNewTab          — forwarded from parent TabBar
    onRename          — callback: close menu + start rename
    onCloseTab        — callback: close menu + close tab
    onClose           — callback: menu dismissed
-->
<script lang="ts">
  import ContextMenu from './ContextMenu.svelte';

  interface Props {
    contextMenuTabId: string | null;
    contextMenuX: number;
    contextMenuY: number;
    onNewTab: () => void;
    onRename: () => void;
    onCloseTab: () => void;
    onClose: () => void;
  }

  let {
    contextMenuTabId,
    contextMenuX,
    contextMenuY,
    onNewTab,
    onRename,
    onCloseTab,
    onClose,
  }: Props = $props();
</script>

{#if contextMenuTabId !== null}
  <ContextMenu
    variant="tab"
    open={true}
    anchorX={contextMenuX}
    anchorY={contextMenuY}
    onclose={onClose}
    onnewtab={onNewTab}
    onrename={onRename}
    onclosetab={onCloseTab}
  />
{/if}
