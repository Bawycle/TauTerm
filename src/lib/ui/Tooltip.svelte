<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Tooltip — wrapper around Bits UI Tooltip with TauTerm design tokens.

  The trigger must use the child snippet pattern (bits-ui v2 API) to
  forward Tooltip.Trigger props to the actual interactive element.

  Props:
    content        — tooltip text content
    delayDuration  — hover delay in ms (default 300)
    children       — the trigger element(s) to wrap

  Security: content is rendered as text only — no {@html}.
-->
<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import type { Snippet } from 'svelte';

  interface Props {
    content: string;
    delayDuration?: number;
    children: Snippet;
  }

  const { content, delayDuration = 300, children }: Props = $props();
</script>

<Tooltip.Root {delayDuration}>
  <Tooltip.Trigger>
    {#snippet child({ props })}
      <span {...props} style="display: contents;">
        {@render children()}
      </span>
    {/snippet}
  </Tooltip.Trigger>

  <Tooltip.Content
    class="z-(--z-tooltip) px-2 py-1 text-(--font-size-ui-xs) text-(--color-text-primary) bg-(--color-bg-raised) border border-(--color-border-overlay) rounded-(--radius-md) shadow-(--shadow-raised) max-w-[240px] pointer-events-none"
    sideOffset={4}
  >
    {content}
  </Tooltip.Content>
</Tooltip.Root>
