<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TabBarScroll — left and right scroll arrow buttons for the tab bar.

  Visible only when tabs overflow in the respective direction (UXD §6.2, §12.2).
  Includes optional notification badge dots for hidden tabs.

  Props:
    canScrollLeft   — show left arrow
    canScrollRight  — show right arrow
    leftBadge       — 'bell' | 'output' | null
    rightBadge      — 'bell' | 'output' | null
    onScrollLeft    — callback when left arrow clicked
    onScrollRight   — callback when right arrow clicked
-->
<script lang="ts">
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    canScrollLeft: boolean;
    canScrollRight: boolean;
    leftBadge: 'bell' | 'output' | null;
    rightBadge: 'bell' | 'output' | null;
    onScrollLeft: () => void;
    onScrollRight: () => void;
  }

  let { canScrollLeft, canScrollRight, leftBadge, rightBadge, onScrollLeft, onScrollRight }: Props =
    $props();
</script>

<!-- Left scroll arrow — visible only when tabs overflow left (UXD §6.2, §12.2) -->
{#if canScrollLeft}
  <button
    class="tab-bar__scroll-arrow tab-bar__scroll-arrow--left"
    type="button"
    aria-label={m.tab_bar_scroll_left()}
    tabindex={-1}
    onclick={onScrollLeft}
  >
    <ChevronLeft size={14} aria-hidden="true" />
    {#if leftBadge !== null}
      <span
        class="tab-bar__scroll-badge"
        class:tab-bar__scroll-badge--bell={leftBadge === 'bell'}
        class:tab-bar__scroll-badge--output={leftBadge === 'output'}
        aria-hidden="true"
      ></span>
    {/if}
  </button>
{/if}

<!-- Right scroll arrow — visible only when tabs overflow right (UXD §6.2, §12.2) -->
{#if canScrollRight}
  <button
    class="tab-bar__scroll-arrow tab-bar__scroll-arrow--right"
    type="button"
    aria-label={m.tab_bar_scroll_right()}
    tabindex={-1}
    onclick={onScrollRight}
  >
    <ChevronRight size={14} aria-hidden="true" />
    {#if rightBadge !== null}
      <span
        class="tab-bar__scroll-badge"
        class:tab-bar__scroll-badge--bell={rightBadge === 'bell'}
        class:tab-bar__scroll-badge--output={rightBadge === 'output'}
        aria-hidden="true"
      ></span>
    {/if}
  </button>
{/if}
