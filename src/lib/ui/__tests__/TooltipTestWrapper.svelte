<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Test helper: wraps Tooltip in the required Tooltip.Provider context.
  Used exclusively in Tooltip.test.ts — not part of the production UI library.
-->
<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import TooltipComponent from '../Tooltip.svelte';
  import type { Snippet } from 'svelte';

  interface Props {
    content: string;
    delayDuration?: number;
    children: Snippet;
  }

  const { content, delayDuration = 0, children }: Props = $props();
</script>

<Tooltip.Provider>
  <TooltipComponent {content} {delayDuration}>
    {#snippet children()}
      {@render children()}
    {/snippet}
  </TooltipComponent>
</Tooltip.Provider>
