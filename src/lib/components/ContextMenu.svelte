<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ContextMenu — right-click context menu with two variants:
    - "terminal": Copy, Paste, Search, Split actions, Close Pane (FS-A11Y-006, UXD §7.8.1)
    - "tab": New Tab, Rename, Split actions, Close Tab (FS-TAB-006, UXD §7.8.2)

  Terminal variant uses Bits UI ContextMenu (native right-click trigger).
  Tab variant uses Bits UI DropdownMenu triggered programmatically from the parent
  via the open prop.

  Props:
    variant      — 'terminal' | 'tab'
    hasSelection — (terminal) whether text is currently selected (enables Copy)
    canClosePane — (terminal) whether more than one pane exists (shows Close Pane)
    open         — (tab) controlled open state
    onclose      — called when menu closes
    oncopy       — Copy item clicked
    onpaste      — Paste item clicked
    onsearch     — Search item clicked
    onsplitH     — Split Top/Bottom clicked
    onsplitV     — Split Left/Right clicked
    onclosepane  — Close Pane clicked
    onnewtab     — New Tab clicked
    onrename     — Rename clicked
    onclosetab   — Close Tab clicked

  Security: no {@html}, no clipboard read on render.
  Accessibility: role="menu" + role="menuitem" from Bits UI primitives.
-->
<script lang="ts">
  import { ContextMenu, DropdownMenu } from 'bits-ui';
  import {
    Copy,
    ClipboardPaste,
    Search,
    SplitSquareHorizontal,
    SplitSquareVertical,
    X,
    Plus,
    Pencil,
  } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    variant: 'terminal' | 'tab';
    hasSelection?: boolean;
    canClosePane?: boolean;
    open?: boolean;
    onclose?: () => void;
    oncopy?: () => void;
    onpaste?: () => void;
    onsearch?: () => void;
    onsplitH?: () => void;
    onsplitV?: () => void;
    onclosepane?: () => void;
    onnewtab?: () => void;
    onrename?: () => void;
    onclosetab?: () => void;
    children?: import('svelte').Snippet;
  }

  const {
    variant,
    hasSelection = false,
    canClosePane = true,
    open = false,
    onclose,
    oncopy,
    onpaste,
    onsearch,
    onsplitH,
    onsplitV,
    onclosepane,
    onnewtab,
    onrename,
    onclosetab,
    children,
  }: Props = $props();

  // svelte-ignore state_referenced_locally -- intentional: local mutable copy needed
  // because onOpenChange mutates internalOpen internally; $derived would be read-only.
  // $effect keeps it in sync when the parent changes the `open` prop.
  let internalOpen = $state(open);
  $effect(() => { internalOpen = open; });

  const menuContentClass =
    'z-[30] min-w-[180px] max-w-[280px] bg-(--color-bg-raised) border border-(--color-border) rounded-[4px] shadow-(--shadow-raised) py-1';

  const menuItemClass =
    'flex items-center gap-2 h-[44px] px-3 text-[13px] text-(--color-text-primary) cursor-pointer select-none outline-none hover:bg-(--color-hover-bg) focus:bg-(--color-hover-bg) active:bg-(--color-active-bg) data-[disabled]:text-(--color-text-tertiary) data-[disabled]:pointer-events-none';

  const separatorClass = 'my-1 mx-3 h-px bg-(--color-border)';
</script>

{#if variant === 'terminal'}
  <!-- ContextMenu: native right-click trigger wraps children slot -->
  <ContextMenu.Root>
    <ContextMenu.Trigger class="contents">
      {#if children}
        {@render children()}
      {/if}
    </ContextMenu.Trigger>

    <ContextMenu.Portal>
      <ContextMenu.Content class={menuContentClass}>
        <!-- Copy -->
        <ContextMenu.Item
          class={menuItemClass}
          disabled={!hasSelection}
          onSelect={() => { if (hasSelection) oncopy?.(); }}
        >
          <Copy size={16} aria-hidden="true" />
          {m.action_copy()}
        </ContextMenu.Item>

        <!-- Paste -->
        <ContextMenu.Item class={menuItemClass} onSelect={() => onpaste?.()}>
          <ClipboardPaste size={16} aria-hidden="true" />
          {m.action_paste()}
        </ContextMenu.Item>

        <ContextMenu.Separator class={separatorClass} />

        <!-- Search -->
        <ContextMenu.Item class={menuItemClass} onSelect={() => onsearch?.()}>
          <Search size={16} aria-hidden="true" />
          {m.action_search()}
        </ContextMenu.Item>

        <ContextMenu.Separator class={separatorClass} />

        <!-- Split actions -->
        <ContextMenu.Item class={menuItemClass} onSelect={() => onsplitH?.()}>
          <SplitSquareHorizontal size={16} aria-hidden="true" />
          {m.action_split_horizontal()}
        </ContextMenu.Item>
        <ContextMenu.Item class={menuItemClass} onSelect={() => onsplitV?.()}>
          <SplitSquareVertical size={16} aria-hidden="true" />
          {m.action_split_vertical()}
        </ContextMenu.Item>

        {#if canClosePane}
          <ContextMenu.Separator class={separatorClass} />
          <ContextMenu.Item class={menuItemClass} onSelect={() => onclosepane?.()}>
            <X size={16} aria-hidden="true" />
            {m.pane_close()}
          </ContextMenu.Item>
        {/if}
      </ContextMenu.Content>
    </ContextMenu.Portal>
  </ContextMenu.Root>

{:else}
  <!-- Tab context menu: DropdownMenu with controlled open state -->
  <DropdownMenu.Root
    open={internalOpen}
    onOpenChange={(o) => { internalOpen = o; if (!o) onclose?.(); }}
  >
    <!-- Invisible trigger — parent controls open state via right-click handler -->
    <DropdownMenu.Trigger
      class="sr-only"
      aria-label={m.tab_context_menu_aria_label()}
    />

    <DropdownMenu.Portal>
      <DropdownMenu.Content
        class={menuContentClass}
        align="start"
        sideOffset={4}
      >
        <!-- New Tab -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onnewtab?.()}>
          <Plus size={16} aria-hidden="true" />
          {m.action_new_tab()}
        </DropdownMenu.Item>

        <DropdownMenu.Separator class={separatorClass} />

        <!-- Rename -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onrename?.()}>
          <Pencil size={16} aria-hidden="true" />
          {m.action_rename()}
        </DropdownMenu.Item>

        <DropdownMenu.Separator class={separatorClass} />

        <!-- Split actions -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onsplitH?.()}>
          <SplitSquareHorizontal size={16} aria-hidden="true" />
          {m.action_split_horizontal()}
        </DropdownMenu.Item>
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onsplitV?.()}>
          <SplitSquareVertical size={16} aria-hidden="true" />
          {m.action_split_vertical()}
        </DropdownMenu.Item>

        <DropdownMenu.Separator class={separatorClass} />

        <!-- Close Tab -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onclosetab?.()}>
          <X size={16} aria-hidden="true" />
          {m.action_close_tab()}
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Portal>
  </DropdownMenu.Root>
{/if}
