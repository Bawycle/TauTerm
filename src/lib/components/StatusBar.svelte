<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  StatusBar — single-line status strip at the bottom of the window.

  Displays the active pane's session type, SSH connection state (text + icon),
  process title, and CWD.

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
  import { Network, WifiOff, XCircle } from 'lucide-svelte';
  import type { PaneState, SshLifecycleState } from '$lib/ipc/types';

  interface Props {
    activePaneState?: PaneState | null;
    /** Hostname of the SSH connection (needed for "user@host" text). */
    sshHost?: string;
    /** Username for the SSH connection. */
    sshUser?: string;
  }

  const { activePaneState = null, sshHost, sshUser }: Props = $props();

  // -------------------------------------------------------------------------
  // SSH indicator logic (TUITC-UX-090 to 092, UXD §7.5.1)
  // -------------------------------------------------------------------------

  const isSsh = $derived(
    activePaneState?.sessionType === 'ssh' && activePaneState?.sshState !== null
  );

  const sshState = $derived(activePaneState?.sshState ?? null);

  const sshStatusText = $derived((): string | null => {
    if (!isSsh || !sshState) return null;
    switch (sshState.type) {
      case 'connecting':
        return sshHost ? `Connecting to ${sshHost}...` : 'Connecting...';
      case 'authenticating':
        return 'Authenticating...';
      case 'connected':
        return sshUser && sshHost ? `${sshUser}@${sshHost}` : 'Connected';
      case 'disconnected':
        return 'Disconnected';
      case 'closed':
        return 'Closed';
      default:
        return null;
    }
  });

  // Icon selection per SSH state
  type IconName = 'network' | 'wifi-off' | 'x-circle' | null;
  const sshIconName = $derived((): IconName => {
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
  const sshIconClass = $derived((): string => {
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
  const sshIconAnimClass = $derived((): string => {
    if (!sshState) return '';
    if (sshState.type === 'connecting') return 'status-bar__ssh-icon--rotating';
    if (sshState.type === 'authenticating') return 'status-bar__ssh-icon--pulsing';
    return '';
  });

  // Process title for center display
  const processTitle = $derived(activePaneState?.processTitle ?? '');
  const cwd = $derived(activePaneState?.cwd ?? '');
</script>

<div class="status-bar" role="status" aria-live="polite">
  <!-- Left: session type + SSH indicator -->
  <div class="status-bar__left">
    {#if activePaneState?.sessionType === 'local'}
      <!-- No indicator for local sessions (absence = local per UXD §7.5.1) -->
      <span class="status-bar__session-label">LOCAL</span>
    {/if}

    {#if isSsh && sshState}
      <!-- SSH connection indicator (TUITC-UX-090 to 092) -->
      <span
        class="status-bar__ssh"
        class:status-bar__ssh--disconnected={sshState.type === 'disconnected'}
      >
        <!-- Icon: Network / WifiOff / XCircle -->
        <span class="status-bar__ssh-icon {sshIconClass()} {sshIconAnimClass()}">
          {#if sshIconName() === 'network'}
            <Network size={14} aria-hidden="true" />
          {:else if sshIconName() === 'wifi-off'}
            <WifiOff size={14} aria-hidden="true" />
          {:else if sshIconName() === 'x-circle'}
            <XCircle size={14} aria-hidden="true" />
          {/if}
        </span>

        <!-- SSH status text — text interpolation only -->
        {#if sshStatusText()}
          <span class="status-bar__ssh-text">{sshStatusText()}</span>
        {/if}
      </span>
    {/if}
  </div>

  <!-- Center: process title -->
  <div class="status-bar__center">
    {#if processTitle}
      <span class="status-bar__process-title">{processTitle}</span>
    {/if}
  </div>

  <!-- Right: CWD (truncated) -->
  <div class="status-bar__right">
    {#if cwd}
      <span class="status-bar__cwd" title={cwd}>{cwd}</span>
    {/if}
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    height: var(--size-status-bar-height);
    min-height: var(--size-status-bar-height);
    background-color: var(--color-bg-surface);
    border-top: 1px solid var(--color-border);
    padding: 0 var(--space-2);
    flex-shrink: 0;
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-secondary);
    overflow: hidden;
    gap: var(--space-2);
  }

  .status-bar__left {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
  }

  .status-bar__center {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    min-width: 0;
  }

  .status-bar__right {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
    max-width: 30%;
    overflow: hidden;
  }

  .status-bar__session-label {
    color: var(--color-text-tertiary);
    font-size: var(--font-size-ui-2xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-family: var(--font-mono-ui);
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
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.4; }
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

  .status-bar__process-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-secondary);
  }

  .status-bar__cwd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-tertiary);
    font-size: var(--font-size-ui-xs);
    font-family: var(--font-mono-ui);
  }
</style>
