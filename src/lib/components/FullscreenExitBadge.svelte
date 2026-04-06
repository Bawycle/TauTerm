<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  FullscreenExitBadge — floating pill button shown in fullscreen mode when
  the tab bar is hidden. Lets the user exit fullscreen without needing to
  recall the tab bar or remember F11.

  Props:
    tabBarVisible — whether the tab bar is currently shown (default: true).
                    The badge only appears when the tab bar is hidden.
    onToggle      — callback invoked when the user activates the button.

  Accessibility:
    - role="button" / tabindex="0" — keyboard accessible
    - aria-label from Paraglide (m.exit_fullscreen)
    - Tooltip with keyboard shortcut hint
    - prefers-reduced-motion: instant opacity change, no transition
-->
<script lang="ts">
  import { Minimize2 } from 'lucide-svelte';
  import Tooltip from '$lib/ui/Tooltip.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    tabBarVisible?: boolean;
    onToggle?: () => void;
  }

  const { tabBarVisible = true, onToggle }: Props = $props();

  function handleClick() {
    onToggle?.();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onToggle?.();
    }
  }
</script>

{#if !tabBarVisible}
  <Tooltip content={m.exit_fullscreen_tooltip()} delayDuration={300}>
    <button
      class="fullscreen-exit-badge"
      type="button"
      tabindex="0"
      aria-label={m.exit_fullscreen()}
      data-testid="fullscreen-exit-badge"
      onclick={handleClick}
      onkeydown={handleKeydown}
    >
      <Minimize2 size={16} aria-hidden="true" />
    </button>
  </Tooltip>
{/if}

<style>
  .fullscreen-exit-badge {
    position: fixed;
    top: var(--space-3);
    right: var(--space-3);
    z-index: var(--z-fullscreen-chrome);

    display: flex;
    align-items: center;
    justify-content: center;

    /* Pill shape — minimum 33×33px touch target (badge is small by design) */
    min-width: 33px;
    min-height: 33px;
    padding: var(--space-1) var(--space-2);
    gap: var(--space-1);

    background-color: var(--color-bg-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full, 9999px);
    box-shadow: var(--shadow-raised);

    color: var(--color-text-secondary);
    cursor: pointer;
    opacity: 0.7;

    transition: opacity var(--duration-fast) var(--ease-in);
  }

  @media (prefers-reduced-motion: reduce) {
    .fullscreen-exit-badge {
      transition: none;
    }
  }

  .fullscreen-exit-badge:hover,
  .fullscreen-exit-badge:focus-visible {
    opacity: 1;
    color: var(--color-text-primary);
  }

  .fullscreen-exit-badge:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }
</style>
