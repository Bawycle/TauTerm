<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  StatusBar — single-line status strip at the bottom of the window.

  Layout (UXD §6.4):
    Left  : shell name (processTitle) + current working directory, truncated with ellipsis.
    Right : SSH connection status indicator (if applicable).

  Elements deferred to DIV-UXD-008 (not implemented here):
    - Settings button
    - Terminal dimensions (cols×rows)

  Props:
    activePaneState — PaneState of the currently active pane (null if no session)
    sshHost         — hostname of the SSH connection (for "user@host" display)
    sshUser         — username of the SSH connection

  Libraries:
    - lucide-svelte: Network, WifiOff, XCircle icons (TUITC-UX-090 to 092, UXD §7.5.1)

  Security:
    - All text via Svelte text interpolation — no {@html}
-->
<script lang="ts">
  import { Network, WifiOff, XCircle, Settings } from 'lucide-svelte';
  import { fade } from 'svelte/transition';
  import { cubicIn } from 'svelte/easing';
  import type { PaneState, SshLifecycleState } from '$lib/ipc/types';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    activePaneState?: PaneState | null;
    /** Hostname of the SSH connection (needed for "user@host" text). */
    sshHost?: string;
    /** Username for the SSH connection. */
    sshUser?: string;
    /** Terminal grid columns (for dimensions display). */
    cols?: number | null;
    /** Terminal grid rows (for dimensions display). */
    rows?: number | null;
    /** Whether the dimensions label is currently visible (transient: shown on resize, fades after 2s). */
    dimsVisible?: boolean;
    /** Called when the Settings button is clicked (DIV-UXD-008). */
    onsettings?: () => void;
  }

  const {
    activePaneState = null,
    sshHost,
    sshUser,
    cols = null,
    rows = null,
    dimsVisible = false,
    onsettings,
  }: Props = $props();

  // -------------------------------------------------------------------------
  // SSH indicator logic (TUITC-UX-090 to 092, UXD §7.5.1)
  // -------------------------------------------------------------------------

  const isSsh = $derived(
    activePaneState?.sessionType === 'ssh' && activePaneState?.sshState !== null,
  );

  const sshState = $derived(activePaneState?.sshState ?? null);

  const sshStatusText = $derived.by((): string | null => {
    if (!isSsh || !sshState) return null;
    switch (sshState.type) {
      case 'connecting':
        return sshHost ? m.ssh_state_connecting({ host: sshHost }) : m.status_bar_connecting();
      case 'authenticating':
        return sshHost ? m.ssh_state_authenticating({ host: sshHost }) : m.status_bar_connecting();
      case 'connected':
        return sshUser && sshHost
          ? m.ssh_state_connected({ user: sshUser, host: sshHost })
          : m.status_bar_connected();
      case 'disconnected':
        return m.status_bar_disconnected();
      case 'closed':
        return m.ssh_state_closed();
      default:
        return null;
    }
  });

  // Icon selection per SSH state
  type IconName = 'network' | 'wifi-off' | 'x-circle' | null;
  const sshIconName: IconName = $derived.by(() => {
    if (!isSsh || !sshState) return null;
    switch (sshState.type) {
      case 'connecting':
      case 'authenticating':
      case 'connected':
        return 'network';
      case 'disconnected':
        return 'wifi-off';
      case 'closed':
        return 'x-circle';
      default:
        return null;
    }
  });

  // CSS class for icon color per SSH state (UXD §7.5.1)
  const sshIconClass: string = $derived.by(() => {
    if (!sshState) return '';
    switch (sshState.type) {
      case 'connecting':
      case 'authenticating':
        return 'status-bar__ssh-icon--connecting';
      case 'connected':
        return 'status-bar__ssh-icon--connected';
      case 'disconnected':
        return 'status-bar__ssh-icon--disconnected';
      case 'closed':
        return 'status-bar__ssh-icon--closed';
      default:
        return '';
    }
  });

  // Animation class for connecting/authenticating states
  const sshIconAnimClass: string = $derived.by(() => {
    if (!sshState) return '';
    if (sshState.type === 'connecting') return 'status-bar__ssh-icon--rotating';
    if (sshState.type === 'authenticating') return 'status-bar__ssh-icon--pulsing';
    return '';
  });

  // Left zone: shell name (processTitle) and CWD
  const processTitle = $derived(activePaneState?.processTitle ?? '');
  const cwd = $derived(activePaneState?.cwd ?? '');
</script>

<div class="status-bar" role="status" aria-live="polite">
  <!-- Left: shell name + CWD (UXD §6.4) -->
  <div class="status-bar__left">
    {#if processTitle}
      <span class="status-bar__shell-name">{processTitle}</span>
    {/if}
    {#if cwd}
      <span class="status-bar__cwd" title={cwd}>{cwd}</span>
    {/if}
  </div>

  <!-- Right: terminal dimensions + SSH indicator + Settings button (DIV-UXD-008) -->
  <div class="status-bar__right">
    <!-- Terminal dimensions: cols×rows — transient, visible during resize only (DIV-UXD-008) -->
    {#if cols !== null && rows !== null && dimsVisible}
      <span
        in:fade={{ duration: 0 }}
        out:fade={{ duration: 300, easing: cubicIn }}
        class="status-bar__dimensions"
        aria-label={m.status_bar_dimensions_aria({ cols, rows })}
      >
        {cols}×{rows}
      </span>
    {/if}

    {#if isSsh && sshState}
      <!-- SSH connection indicator (TUITC-UX-090 to 092) -->
      <span
        class="status-bar__ssh"
        class:status-bar__ssh--disconnected={sshState.type === 'disconnected'}
      >
        <!-- Icon: Network / WifiOff / XCircle -->
        <span class="status-bar__ssh-icon {sshIconClass} {sshIconAnimClass}">
          {#if sshIconName === 'network'}
            <Network size={14} aria-hidden="true" />
          {:else if sshIconName === 'wifi-off'}
            <WifiOff size={14} aria-hidden="true" />
          {:else if sshIconName === 'x-circle'}
            <XCircle size={14} aria-hidden="true" />
          {/if}
        </span>

        <!-- SSH status text — text interpolation only -->
        {#if sshStatusText}
          <span class="status-bar__ssh-text">{sshStatusText}</span>
        {/if}
      </span>
    {/if}

    <!-- Settings button (DIV-UXD-008) -->
    <button
      type="button"
      class="status-bar__settings-btn"
      onclick={() => onsettings?.()}
      aria-label={m.status_bar_settings()}
      title={m.status_bar_settings()}
    >
      <Settings size={14} aria-hidden="true" />
    </button>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    height: var(--size-status-bar-height);
    min-height: var(--size-status-bar-height);
    background-color: var(--color-bg-base);
    border-top: 1px solid var(--color-border);
    padding: 0 var(--space-2);
    flex-shrink: 0;
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-secondary);
    overflow: hidden;
    gap: var(--space-2);
  }

  /* Left: shell name + CWD — takes all available space, truncates with ellipsis */
  .status-bar__left {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  /* Right: SSH indicator — shrinks to fit, never grows */
  .status-bar__right {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
  }

  /* Shell name (processTitle): monospace, truncated */
  .status-bar__shell-name {
    font-family: var(--font-mono-ui);
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 0;
    max-width: 30%;
  }

  /* CWD: monospace, truncated, takes remaining space */
  .status-bar__cwd {
    font-family: var(--font-mono-ui);
    font-size: var(--font-size-ui-xs);
    color: var(--color-text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  /* SSH indicator (TUITC-UX-090 to 092, UXD §7.5.1) */
  .status-bar__ssh {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .status-bar__ssh-icon {
    display: flex;
    align-items: center;
  }

  /* Color variants per SSH state (UXD §7.5.1) */
  .status-bar__ssh-icon--connecting,
  .status-bar__ssh-icon--connecting :global(svg),
  .status-bar__ssh-icon--pulsing,
  .status-bar__ssh-icon--pulsing :global(svg) {
    color: var(--color-ssh-connecting-fg);
  }

  .status-bar__ssh-icon--connected,
  .status-bar__ssh-icon--connected :global(svg) {
    color: var(--color-ssh-badge-fg);
  }

  .status-bar__ssh-icon--disconnected,
  .status-bar__ssh-icon--disconnected :global(svg),
  .status-bar__ssh--disconnected {
    color: var(--color-ssh-disconnected-fg);
  }

  .status-bar__ssh-icon--closed,
  .status-bar__ssh-icon--closed :global(svg) {
    color: var(--color-text-muted);
  }

  /* Animations */
  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  .status-bar__ssh-icon--rotating :global(svg) {
    animation: spin 1.2s linear infinite;
  }

  .status-bar__ssh-icon--pulsing :global(svg) {
    animation: pulse 1.5s ease-in-out infinite;
  }

  /* Disable animations for reduced motion */
  @media (prefers-reduced-motion: reduce) {
    .status-bar__ssh-icon--rotating :global(svg),
    .status-bar__ssh-icon--pulsing :global(svg) {
      animation: none;
    }
  }

  .status-bar__ssh-text {
    font-size: var(--font-size-ui-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
  }

  /* Terminal dimensions: cols×rows — transient (DIV-UXD-008)
     Hidden at rest (opacity 0, visibility hidden); space is always reserved so
     the layout does not shift when the element appears or disappears. */
  .status-bar__dimensions {
    font-family: var(--font-mono-ui);
    font-size: var(--font-size-ui-xs);
    color: var(--color-text-tertiary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* Disable opacity transition for users who prefer reduced motion.
     The element still appears/disappears; it just does so without animation. */
  /* Settings ghost button (DIV-UXD-008) */
  .status-bar__settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 2px;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    flex-shrink: 0;
    transition:
      color var(--duration-instant),
      background-color var(--duration-instant);
    padding: 0;
  }

  .status-bar__settings-btn:hover {
    color: var(--color-text-primary);
    background-color: var(--color-hover-bg);
  }

  .status-bar__settings-btn:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -1px;
  }
</style>
