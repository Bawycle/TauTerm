<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SshConnectingOverlay — full-pane overlay shown while an SSH session is
  in `connecting` or `authenticating` state (UXD §7.5.2).

  Covers the pane with a centred status indicator so the user knows the
  connection is in progress. Non-interactive (pointer-events: none) so it
  does not block mouse events if the terminal renders underneath.

  Props:
    state  — 'connecting' | 'authenticating'

  Accessibility:
    - role="status", aria-live="polite" for non-intrusive announcement.
    - Icon is decorative (aria-hidden).
    - Animations are disabled when prefers-reduced-motion: reduce is set.
-->
<script lang="ts">
  import { Network } from 'lucide-svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    state: 'connecting' | 'authenticating';
  }

  const { state }: Props = $props();

  const label = $derived(
    state === 'authenticating' ? m.ssh_overlay_authenticating() : m.ssh_overlay_connecting(),
  );
</script>

<div class="ssh-connecting-overlay" role="status" aria-live="polite">
  <span
    class="ssh-connecting-overlay__icon"
    class:ssh-connecting-overlay__icon--spin={state === 'connecting'}
    class:ssh-connecting-overlay__icon--pulse={state === 'authenticating'}
    aria-hidden="true"
  >
    <Network size={20} />
  </span>
  <span class="ssh-connecting-overlay__label">{label}</span>
</div>

<style>
  .ssh-connecting-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    pointer-events: none;
    color: var(--color-text-muted);
    background-color: var(--color-bg-overlay);
    z-index: var(--z-overlay);
  }

  .ssh-connecting-overlay__icon {
    display: flex;
    color: var(--color-accent);
  }

  .ssh-connecting-overlay__label {
    font-size: var(--font-size-ui-sm);
    font-family: var(--font-ui);
  }

  /* connecting: continuous rotation */
  @keyframes ssh-spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* authenticating: opacity pulse */
  @keyframes ssh-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  .ssh-connecting-overlay__icon--spin {
    animation: ssh-spin 1s linear infinite;
  }

  .ssh-connecting-overlay__icon--pulse {
    animation: ssh-pulse 600ms ease-in-out infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .ssh-connecting-overlay__icon--spin,
    .ssh-connecting-overlay__icon--pulse {
      animation: none;
    }
  }
</style>
