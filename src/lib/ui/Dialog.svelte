<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Dialog — modal dialog built on Bits UI Dialog / AlertDialog (v2 API).

  Renders an overlay + centred content panel with title, close button,
  scrollable body, and optional footer slot (for action buttons).

  Props:
    open     — whether the dialog is open (bindable via onclose)
    title    — dialog title (also used as sr-only description)
    size     — 'small' (420px) or 'medium' (560px)
    variant  — 'dialog' (default) or 'alertdialog' (destructive confirmations)
    onclose  — called when the dialog should close
    children — body content snippet
    footer   — optional footer snippet (action buttons)

  Accessibility:
    - variant="dialog"      → role="dialog", aria-modal, aria-labelledby
    - variant="alertdialog" → role="alertdialog", aria-modal, aria-labelledby
      Use alertdialog for destructive or irreversible confirmations where the
      user must respond before continuing (ARIA 1.2 §6.21).
  Security: title and children rendered via text interpolation / snippets — no {@html}.
-->
<script lang="ts">
  import { Dialog, AlertDialog } from 'bits-ui';
  import { X } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    open?: boolean;
    title: string;
    size?: 'small' | 'medium';
    variant?: 'dialog' | 'alertdialog';
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
    variant = 'dialog',
    onclose,
    onopenautoFocus,
    children,
    footer,
  }: Props = $props();

  const contentWidth = $derived(size === 'medium' ? 'w-[560px]' : 'w-[420px]');

  const contentClass = $derived(
    `fixed z-(--z-modal) top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 ${contentWidth} max-w-[90vw] bg-(--color-bg-raised) border border-(--color-border-overlay) rounded-(--radius-md) shadow-(--shadow-overlay) p-6`,
  );
</script>

{#if variant === 'alertdialog'}
  <AlertDialog.Root
    bind:open
    onOpenChange={(o) => {
      if (!o) onclose?.();
    }}
  >
    <AlertDialog.Portal>
      <AlertDialog.Overlay
        class="fixed inset-0 z-(--z-modal-backdrop) bg-(--color-bg-overlay)/60"
      />

      <AlertDialog.Content
        class={contentClass}
        aria-modal="true"
        preventScroll={false}
        onOpenAutoFocus={onopenautoFocus}
        onCloseAutoFocus={(e) => {
          // Prevent Bits UI from restoring focus to the trigger element.
          // The focusin safety net in useTerminalView.core recaptures
          // focus to the terminal textarea once the dialog is fully removed.
          e.preventDefault();
        }}
      >
        <!-- Header: title + close button -->
        <div class="flex items-start justify-between mb-3">
          <AlertDialog.Title
            class="text-(--font-size-ui-lg) font-semibold text-(--color-text-primary) leading-snug"
          >
            {title}
          </AlertDialog.Title>

          <AlertDialog.Cancel
            class="flex items-center justify-center w-[44px] h-[44px] text-(--color-text-secondary) hover:text-(--color-text-primary) hover:bg-(--color-hover-bg) rounded-(--radius-sm) -mr-3 -mt-1 flex-shrink-0"
            aria-label={m.dialog_close()}
          >
            <X size={16} aria-hidden="true" />
          </AlertDialog.Cancel>
        </div>

        <!-- sr-only description (accessibility) -->
        <AlertDialog.Description class="sr-only">{title}</AlertDialog.Description>

        <!-- Body content -->
        <div class="text-(--font-size-ui-md) text-(--color-text-primary)">
          {@render children()}
        </div>

        <!-- Optional footer (action buttons) -->
        {#if footer}
          <div class="flex justify-end gap-2 mt-6">
            {@render footer()}
          </div>
        {/if}
      </AlertDialog.Content>
    </AlertDialog.Portal>
  </AlertDialog.Root>
{:else}
  <Dialog.Root
    bind:open
    onOpenChange={(o) => {
      if (!o) onclose?.();
    }}
  >
    <Dialog.Portal>
      <Dialog.Overlay class="fixed inset-0 z-(--z-modal-backdrop) bg-(--color-bg-overlay)/60" />

      <Dialog.Content
        class={contentClass}
        aria-modal="true"
        preventScroll={false}
        onOpenAutoFocus={onopenautoFocus}
        onCloseAutoFocus={(e) => {
          e.preventDefault();
        }}
      >
        <!-- Header: title + close button -->
        <div class="flex items-start justify-between mb-3">
          <Dialog.Title
            class="text-(--font-size-ui-lg) font-semibold text-(--color-text-primary) leading-snug"
          >
            {title}
          </Dialog.Title>

          <Dialog.Close
            class="flex items-center justify-center w-[44px] h-[44px] text-(--color-text-secondary) hover:text-(--color-text-primary) hover:bg-(--color-hover-bg) rounded-(--radius-sm) -mr-3 -mt-1 flex-shrink-0"
            aria-label={m.dialog_close()}
          >
            <X size={16} aria-hidden="true" />
          </Dialog.Close>
        </div>

        <!-- sr-only description (accessibility) -->
        <Dialog.Description class="sr-only">{title}</Dialog.Description>

        <!-- Body content -->
        <div class="text-(--font-size-ui-md) text-(--color-text-primary)">
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
{/if}
