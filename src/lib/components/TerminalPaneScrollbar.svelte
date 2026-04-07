<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPaneScrollbar — interactive scrollbar overlay for the terminal viewport.

  Shown when the pane has scrollback content (tp.scrollbackLines > 0) and
  the scrollbar visibility flag is active (tp.showScrollbar).

  The scrollbar fades in/out via svelte/transition:fade (FS-SB-007, TUITC-UX-070 to 073).

  Props:
    tp — composable instance from useTerminalPane
-->
<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { useTerminalPane } from '$lib/composables/useTerminalPane.svelte';

  interface Props {
    tp: ReturnType<typeof useTerminalPane>;
  }

  let { tp }: Props = $props();
</script>

{#if tp.showScrollbar && tp.scrollbackLines > 0}
  <div
    bind:this={tp.scrollbarEl}
    class="terminal-pane__scrollbar"
    transition:fade={{ duration: 300 }}
    class:terminal-pane__scrollbar--dragging={tp.scrollbarDragging}
    aria-hidden="true"
    onpointerdown={tp.handleScrollbarPointerdown}
    onpointermove={tp.handleScrollbarPointermove}
    onpointerup={tp.handleScrollbarPointerup}
    onpointerleave={tp.handleScrollbarPointerup}
    onwheel={tp.handleScrollbarWheel}
    onmouseenter={() => {
      tp.scrollbarHover = true;
    }}
    onmouseleave={() => {
      tp.scrollbarHover = false;
    }}
  >
    <div
      class="terminal-pane__scrollbar-thumb"
      class:terminal-pane__scrollbar-thumb--hover={tp.scrollbarHover || tp.scrollbarDragging}
      style:height="{tp.scrollbarThumbHeightPct}%"
      style:top="{tp.scrollbarThumbTopPct}%"
    ></div>
  </div>
{/if}
