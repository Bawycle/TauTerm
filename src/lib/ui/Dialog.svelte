<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Dialog — modal dialog built on Bits UI Dialog (v2 API).

  Renders an overlay + centred content panel with title, close button,
  scrollable body, and optional footer slot (for action buttons).

  Props:
    open     — whether the dialog is open (bindable via onclose)
    title    — dialog title (also used as sr-only description)
    size     — 'small' (420px) or 'medium' (560px)
    onclose  — called when the dialog should close
    children — body content snippet
    footer   — optional footer snippet (action buttons)

  Accessibility: role="dialog", aria-modal, aria-labelledby on Content.
  Security: title and children rendered via text interpolation / snippets — no {@html}.
-->
<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { X } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    open?: boolean;
    title: string;
    size?: 'small' | 'medium';
    onclose?: () => void;
    /**
     * Override the auto-focus behaviour when the dialog opens (DIV-UXD-012).
     * Receives the default open event; call `e.preventDefault()` to suppress
     * default focus, then manually focus the desired element.
     * Use this to focus Cancel in destructive dialogs.
     */
    onopenautoFocus?: (e: Event) => void;
    children: Snippet;
    footer?: Snippet;
  }

  let {
    open = $bindable(false),
    title,
    size = 'small',
    onclose,
    onopenautoFocus,
    children,
    footer,
  }: Props = $props();

  const contentWidth = $derived(size === 'medium' ? 'w-[560px]' : 'w-[420px]');
</script>

<Dialog.Root
  bind:open
  onOpenChange={(o) => {
    if (!o) onclose?.();
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-[49] bg-(--color-bg-overlay)/60" />

    <Dialog.Content
      class="fixed z-[50] top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2
             {contentWidth} max-w-[90vw]
             bg-(--color-bg-raised) border border-(--color-border) rounded-[4px]
             shadow-(--shadow-overlay) p-6"
      aria-modal="true"
      onOpenAutoFocus={onopenautoFocus}
    >
      <!-- Header: title + close button -->
      <div class="flex items-start justify-between mb-3">
        <Dialog.Title class="text-[16px] font-semibold text-(--color-text-primary) leading-snug">
          {title}
        </Dialog.Title>

        <Dialog.Close
          class="flex items-center justify-center w-[44px] h-[44px] text-(--color-text-secondary) hover:text-(--color-text-primary) hover:bg-(--color-hover-bg) rounded-[2px] -mr-3 -mt-1 flex-shrink-0"
          aria-label={m.dialog_close()}
        >
          <X size={16} aria-hidden="true" />
        </Dialog.Close>
      </div>

      <!-- sr-only description (accessibility) -->
      <Dialog.Description class="sr-only">{title}</Dialog.Description>

      <!-- Body content -->
      <div class="text-[14px] text-(--color-text-primary)">
        {@render children()}
      </div>

      <!-- Optional footer (action buttons) -->
      {#if footer}
        <div class="flex justify-end gap-2 mt-6">
          {@render footer()}
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
