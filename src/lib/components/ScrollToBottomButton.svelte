<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ScrollToBottomButton — passive scroll indicator button.

  Displayed as an absolute-positioned pill in the bottom-right corner of the
  terminal viewport when the user has scrolled up into scrollback history.
  Clicking returns the viewport to the live bottom of the buffer.

  Props:
    onclick — callback invoked when the button is activated (click or Enter/Space)

  Accessibility:
    role="button", tabindex="0", aria-label from i18n key scroll_to_bottom
    Keyboard: Enter / Space activate the onclick callback
-->
<script lang="ts">
  import { ArrowDown } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    onclick: () => void;
  }

  const { onclick }: Props = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onclick();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="scroll-to-bottom-btn"
  role="button"
  tabindex="0"
  aria-label={m.scroll_to_bottom()}
  {onclick}
  onkeydown={handleKeydown}
>
  <ArrowDown size="var(--size-icon-md)" aria-hidden="true" />
</div>

<style>
  .scroll-to-bottom-btn {
    position: absolute;
    bottom: var(--space-3);
    right: var(--space-3);
    z-index: var(--z-scrollbar);

    display: flex;
    align-items: center;
    justify-content: center;

    /* Pill shape */
    border-radius: var(--radius-full);
    border: 1px solid var(--color-border);

    /* Minimum size: 33×33px (auxiliary control) */
    min-width: 33px;
    min-height: 33px;
    padding: var(--space-1);

    /* Colors — idle state */
    background-color: var(--color-bg-raised);
    color: var(--color-icon-default);
    box-shadow: var(--shadow-raised);

    cursor: pointer;
    user-select: none;

    /* Opacity controlled by parent's transition:fade directive */
    opacity: 1;
  }

  .scroll-to-bottom-btn:hover {
    background-color: var(--color-hover-bg);
    color: var(--color-icon-active);
  }

  .scroll-to-bottom-btn:active {
    background-color: var(--color-active-bg);
    box-shadow: none;
  }

  .scroll-to-bottom-btn:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
    box-shadow:
      0 0 0 2px var(--color-focus-ring-offset),
      var(--shadow-raised);
  }

  @media (prefers-reduced-motion: reduce) {
    .scroll-to-bottom-btn {
      transition: none;
    }
  }
</style>
