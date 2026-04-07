<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPanePasteDialog — confirmation dialog for multi-line paste without
  bracketed paste mode active (FS-CLIP-009).

  Displayed when tp.pasteConfirmOpen is true. Offers Cancel / Paste actions and
  a "Don't ask again" toggle.

  Props:
    tp                          — composable instance from useTerminalPane
    confirmMultilinePaste       — current preference value (controls toggle initial state)
    ondisableConfirmMultilinePaste — callback when user checks "Don't ask again"
-->
<script lang="ts">
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import Toggle from '$lib/ui/Toggle.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { useTerminalPane } from '$lib/composables/useTerminalPane.svelte';

  interface Props {
    tp: ReturnType<typeof useTerminalPane>;
    confirmMultilinePaste: boolean;
    ondisableConfirmMultilinePaste?: () => void;
  }

  let { tp, confirmMultilinePaste, ondisableConfirmMultilinePaste }: Props = $props();
</script>

<!-- FS-CLIP-009: Multiline paste confirmation dialog -->
<Dialog
  open={tp.pasteConfirmOpen}
  title={m.paste_confirm_title()}
  size="small"
  onclose={tp.handlePasteCancel}
>
  {#snippet children()}
    <p class="text-[14px] text-(--color-text-secondary) leading-relaxed">
      {m.paste_confirm_body({ lines: tp.pasteConfirmText.split('\n').length })}
    </p>
    <div class="mt-4">
      <Toggle
        checked={!confirmMultilinePaste}
        label={m.paste_confirm_dont_ask()}
        onchange={(v) => {
          if (v) ondisableConfirmMultilinePaste?.();
        }}
      />
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" onclick={tp.handlePasteCancel}>{m.action_cancel()}</Button>
    <Button variant="primary" onclick={tp.handlePasteConfirm}>{m.paste_confirm_action()}</Button>
  {/snippet}
</Dialog>
