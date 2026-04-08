<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPaneBanners — overlay banners for SSH deprecated algorithm, process
  terminated, and SSH disconnected states.

  Rendered conditionally based on props; the parent (TerminalPane) owns the state
  and passes it down. Callbacks flow up via props.

  Props:
    deprecatedAlgorithm  — algorithm name if a warning was issued, or null
    terminated           — whether the PTY process has exited
    exitCode             — PTY exit code
    signalName           — signal name if the process was killed by a signal
    sshState             — SSH lifecycle state (for disconnected banner)
    canClosePane         — controls Close Pane visibility in ProcessTerminatedPane
    onDismissAlgorithm   — dismiss the deprecated-algorithm banner
    onrestart            — restart the process
    onclosepane          — close the pane
    onReconnect          — reconnect the SSH session
-->
<script lang="ts">
  import ProcessTerminatedPane from './ProcessTerminatedPane.svelte';
  import SshConnectingOverlay from './SshConnectingOverlay.svelte';
  import SshDeprecatedAlgorithmBanner from './SshDeprecatedAlgorithmBanner.svelte';
  import * as m from '$lib/paraglide/messages';
  import type { SshLifecycleState } from '$lib/ipc/types';

  interface Props {
    deprecatedAlgorithm: string | null;
    terminated: boolean;
    exitCode: number;
    signalName?: string;
    sshState: SshLifecycleState | null;
    canClosePane: boolean;
    onDismissAlgorithm: () => void;
    onrestart?: () => void;
    onclosepane?: () => void;
    onReconnect: () => void;
  }

  let {
    deprecatedAlgorithm,
    terminated,
    exitCode,
    signalName,
    sshState,
    canClosePane,
    onDismissAlgorithm,
    onrestart,
    onclosepane,
    onReconnect,
  }: Props = $props();
</script>

<!-- Deprecated SSH algorithm banner — displaces terminal content downward (UXD §7.21) -->
{#if deprecatedAlgorithm !== null}
  <SshDeprecatedAlgorithmBanner algorithm={deprecatedAlgorithm} ondismiss={onDismissAlgorithm} />
{/if}

<!-- ProcessTerminatedPane banner — shown when PTY process exits (FS-PTY-005/006) -->
{#if terminated}
  <ProcessTerminatedPane {exitCode} {signalName} {onrestart} onclose={onclosepane} />
{/if}

<!-- SSH connecting overlay — shown while establishing connection (UXD §7.5.2) -->
{#if sshState?.type === 'connecting' || sshState?.type === 'authenticating'}
  <SshConnectingOverlay state={sshState.type} />
{/if}

<!-- SSH disconnected banner — shown when SSH connection drops (FS-SSH-040/041) -->
{#if sshState?.type === 'disconnected'}
  <div class="terminal-pane__ssh-disconnected" role="status" aria-live="polite">
    <div class="terminal-pane__ssh-disconnected-content">
      <span class="terminal-pane__ssh-disconnected-label">
        {m.ssh_banner_disconnected_title()}
      </span>
      {#if sshState.reason}
        <span class="terminal-pane__ssh-disconnected-reason">
          {sshState.reason}
        </span>
      {/if}
    </div>
    <button class="terminal-pane__ssh-reconnect-btn" type="button" onclick={onReconnect}>
      {m.ssh_reconnect()}
    </button>
  </div>
{/if}

<style>
  /* SSH disconnected banner (FS-SSH-040/041) */
  .terminal-pane__ssh-disconnected {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    background-color: var(--color-error-bg);
    border-top: 1px solid var(--color-error);
    z-index: var(--z-overlay);
  }

  .terminal-pane__ssh-disconnected-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .terminal-pane__ssh-disconnected-label {
    color: var(--color-error-text);
    font-size: var(--font-size-ui-sm);
  }

  .terminal-pane__ssh-disconnected-reason {
    color: var(--color-text-secondary);
    font-size: var(--font-size-ui-sm);
    /* TODO: --font-mono does not exist; using --font-mono-ui (closest available token) */
    font-family: var(--font-mono-ui);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    max-width: 60ch;
  }

  .terminal-pane__ssh-reconnect-btn {
    padding: var(--space-1) var(--space-3);
    background-color: var(--color-accent);
    color: var(--term-fg);
    border: none;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-ui-sm);
    cursor: pointer;
    min-height: var(--size-target-min);
    min-width: var(--size-target-min);
  }

  .terminal-pane__ssh-reconnect-btn:hover {
    background-color: var(--color-accent);
    filter: brightness(1.15);
  }

  .terminal-pane__ssh-reconnect-btn:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }
</style>
