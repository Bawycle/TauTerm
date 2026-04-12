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
    /** When provided, the invisible tab context menu trigger is positioned at these viewport coordinates. */
    anchorX?: number;
    anchorY?: number;
    /**
     * Optional keyboard shortcut hints to display right-aligned on menu items (UXD §7.8.1).
     * Keys match action names: 'copy', 'paste', 'search', 'splitH', 'splitV', 'closePane',
     * 'newTab', 'rename', 'closeTab'.
     */
    shortcuts?: Partial<Record<string, string>>;
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
    anchorX,
    anchorY,
    shortcuts = {},
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

  /**
   * When anchorX/anchorY are provided, the trigger is rendered at the given
   * viewport coordinates so that Bits UI can anchor the menu content to the
   * right-click position. Without this, the sr-only trigger is offscreen and
   * the menu would appear at the wrong location.
   */
  const triggerStyle = $derived(
    anchorX !== undefined && anchorY !== undefined
      ? `position: fixed; left: ${anchorX}px; top: ${anchorY}px; width: 0; height: 0; overflow: visible; clip: unset; white-space: unset;`
      : undefined,
  );

  // svelte-ignore state_referenced_locally -- intentional: local mutable copy needed
  // because onOpenChange mutates internalOpen internally; $derived would be read-only.
  // $effect keeps it in sync when the parent changes the `open` prop.
  let internalOpen = $state(open);
  $effect(() => {
    internalOpen = open;
  });

  const menuContentClass =
    'z-(--z-dropdown) min-w-[180px] max-w-[280px] bg-(--color-bg-raised) border border-(--color-border-overlay) rounded-(--radius-md) shadow-(--shadow-raised) py-1';

  const menuItemClass =
    'flex items-center justify-between gap-2 h-[44px] px-3 text-(--font-size-ui-base) text-(--color-text-primary) cursor-pointer select-none outline-none hover:bg-(--color-hover-bg) focus:bg-(--color-hover-bg) active:bg-(--color-active-bg) data-[disabled]:text-(--color-text-tertiary) data-[disabled]:pointer-events-none transition-[background-color,color,border-color] duration-(--duration-fast) ease-out';

  /** Left side of a menu item: icon + label. */
  const menuItemLabelClass = 'flex items-center gap-2 min-w-0';

  /** Right-aligned shortcut hint (UXD §7.8.1). */
  const menuItemShortcutClass =
    'ml-4 shrink-0 font-mono text-(--font-size-ui-xs) text-(--color-text-tertiary) leading-none';

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
      <ContextMenu.Content class={menuContentClass} preventScroll={false}>
        <!-- Copy -->
        <ContextMenu.Item
          class={menuItemClass}
          disabled={!hasSelection}
          onSelect={() => {
            if (hasSelection) oncopy?.();
          }}
        >
          <span class={menuItemLabelClass}>
            <Copy size={16} aria-hidden="true" />
            {m.action_copy()}
          </span>
          {#if shortcuts.copy}
            <span class={menuItemShortcutClass}>{shortcuts.copy}</span>
          {/if}
        </ContextMenu.Item>

        <!-- Paste -->
        <ContextMenu.Item class={menuItemClass} onSelect={() => onpaste?.()}>
          <span class={menuItemLabelClass}>
            <ClipboardPaste size={16} aria-hidden="true" />
            {m.action_paste()}
          </span>
          {#if shortcuts.paste}
            <span class={menuItemShortcutClass}>{shortcuts.paste}</span>
          {/if}
        </ContextMenu.Item>

        <ContextMenu.Separator class={separatorClass} />

        <!-- Search -->
        <ContextMenu.Item class={menuItemClass} onSelect={() => onsearch?.()}>
          <span class={menuItemLabelClass}>
            <Search size={16} aria-hidden="true" />
            {m.action_search()}
          </span>
          {#if shortcuts.search}
            <span class={menuItemShortcutClass}>{shortcuts.search}</span>
          {/if}
        </ContextMenu.Item>

        <ContextMenu.Separator class={separatorClass} />

        <!-- Split actions -->
        <ContextMenu.Item class={menuItemClass} onSelect={() => onsplitH?.()}>
          <span class={menuItemLabelClass}>
            <SplitSquareHorizontal size={16} aria-hidden="true" />
            {m.action_split_horizontal()}
          </span>
          {#if shortcuts.splitH}
            <span class={menuItemShortcutClass}>{shortcuts.splitH}</span>
          {/if}
        </ContextMenu.Item>
        <ContextMenu.Item class={menuItemClass} onSelect={() => onsplitV?.()}>
          <span class={menuItemLabelClass}>
            <SplitSquareVertical size={16} aria-hidden="true" />
            {m.action_split_vertical()}
          </span>
          {#if shortcuts.splitV}
            <span class={menuItemShortcutClass}>{shortcuts.splitV}</span>
          {/if}
        </ContextMenu.Item>

        {#if canClosePane}
          <ContextMenu.Separator class={separatorClass} />
          <ContextMenu.Item class={menuItemClass} onSelect={() => onclosepane?.()}>
            <span class={menuItemLabelClass}>
              <X size={16} aria-hidden="true" />
              {m.pane_close()}
            </span>
            {#if shortcuts.closePane}
              <span class={menuItemShortcutClass}>{shortcuts.closePane}</span>
            {/if}
          </ContextMenu.Item>
        {/if}
      </ContextMenu.Content>
    </ContextMenu.Portal>
  </ContextMenu.Root>
{:else}
  <!-- Tab context menu: DropdownMenu with controlled open state -->
  <DropdownMenu.Root
    open={internalOpen}
    onOpenChange={(o) => {
      internalOpen = o;
      if (!o) onclose?.();
    }}
  >
    <!-- Invisible trigger — parent controls open state via right-click handler.
         When anchorX/anchorY are provided the trigger is fixed-positioned at the
         pointer location so Bits UI anchors the menu content correctly. -->
    <DropdownMenu.Trigger
      class={triggerStyle ? undefined : 'sr-only'}
      style={triggerStyle}
      aria-label={m.tab_context_menu_aria_label()}
    />

    <DropdownMenu.Portal>
      <DropdownMenu.Content class={menuContentClass} align="start" sideOffset={4} preventScroll={false}>
        <!-- New Tab -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onnewtab?.()}>
          <span class={menuItemLabelClass}>
            <Plus size={16} aria-hidden="true" />
            {m.action_new_tab()}
          </span>
          {#if shortcuts.newTab}
            <span class={menuItemShortcutClass}>{shortcuts.newTab}</span>
          {/if}
        </DropdownMenu.Item>

        <DropdownMenu.Separator class={separatorClass} />

        <!-- Rename -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onrename?.()}>
          <span class={menuItemLabelClass}>
            <Pencil size={16} aria-hidden="true" />
            {m.action_rename()}
          </span>
          {#if shortcuts.rename}
            <span class={menuItemShortcutClass}>{shortcuts.rename}</span>
          {/if}
        </DropdownMenu.Item>

        <DropdownMenu.Separator class={separatorClass} />

        <!-- Split actions -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onsplitH?.()}>
          <span class={menuItemLabelClass}>
            <SplitSquareHorizontal size={16} aria-hidden="true" />
            {m.action_split_horizontal()}
          </span>
          {#if shortcuts.splitH}
            <span class={menuItemShortcutClass}>{shortcuts.splitH}</span>
          {/if}
        </DropdownMenu.Item>
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onsplitV?.()}>
          <span class={menuItemLabelClass}>
            <SplitSquareVertical size={16} aria-hidden="true" />
            {m.action_split_vertical()}
          </span>
          {#if shortcuts.splitV}
            <span class={menuItemShortcutClass}>{shortcuts.splitV}</span>
          {/if}
        </DropdownMenu.Item>

        <DropdownMenu.Separator class={separatorClass} />

        <!-- Close Tab -->
        <DropdownMenu.Item class={menuItemClass} onSelect={() => onclosetab?.()}>
          <span class={menuItemLabelClass}>
            <X size={16} aria-hidden="true" />
            {m.action_close_tab()}
          </span>
          {#if shortcuts.closeTab}
            <span class={menuItemShortcutClass}>{shortcuts.closeTab}</span>
          {/if}
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Portal>
  </DropdownMenu.Root>
{/if}
