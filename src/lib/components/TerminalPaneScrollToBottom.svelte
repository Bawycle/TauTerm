<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPaneScrollToBottom — button shown when scrolled up into scrollback history.

  Fades in/out with a 150ms transition. Clicking scrolls back to the live viewport.

  Props:
    scrollOffset — current scroll offset (> 0 means scrolled up into scrollback)
    onclick      — handler to scroll back to bottom
-->
<script lang="ts">
  import { fade } from 'svelte/transition';
  import ScrollToBottomButton from './ScrollToBottomButton.svelte';

  interface Props {
    scrollOffset: number;
    onclick: () => void;
  }

  let { scrollOffset, onclick }: Props = $props();
</script>

{#if scrollOffset > 0}
  <div transition:fade={{ duration: 150 }}>
    <ScrollToBottomButton {onclick} />
  </div>
{/if}
