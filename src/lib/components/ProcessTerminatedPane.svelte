<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  ProcessTerminatedPane — bottom banner shown when the shell process exits.

  Appears overlaid at the bottom of a TerminalPane. Terminal scrollback
  content remains visible behind this banner (FS-PTY-005, FS-PTY-006).

  Props:
    exitCode   — the integer exit code of the terminated process
    signalName — optional signal name (e.g. "SIGKILL") for signal-killed processes
    onrestart  — called when the user clicks "Restart"
    onclose    — called when the user clicks "Close"

  Security: exitCode rendered as number (text interpolation), no {@html}.
  Accessibility: icon is decorative (aria-hidden), meaning carried by text.
-->
<script lang="ts">
  import { CheckCircle, XCircle } from 'lucide-svelte';
  import Button from '$lib/ui/Button.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    exitCode: number;
    signalName?: string;
    onrestart?: () => void;
    onclose?: () => void;
  }

  const { exitCode, signalName, onrestart, onclose }: Props = $props();

  const isSuccess = $derived(exitCode === 0);
</script>

<div
  class="process-terminated-pane"
  role="status"
  aria-live="polite"
>
  <!-- Exit status text (left side) -->
  <div class="process-terminated-pane__status">
    {#if isSuccess}
      <CheckCircle
        size={16}
        class="process-terminated-pane__icon process-terminated-pane__icon--success"
        aria-hidden="true"
      />
      <span class="process-terminated-pane__primary">
        {m.notification_process_exit_success()}
      </span>
    {:else}
      <XCircle
        size={16}
        class="process-terminated-pane__icon process-terminated-pane__icon--error"
        aria-hidden="true"
      />
      <div class="process-terminated-pane__exit-info">
        <span class="process-terminated-pane__primary">
          {m.notification_process_exit_failure({ code: exitCode })}
        </span>
        {#if signalName}
          <span class="process-terminated-pane__secondary">
            {signalName}
          </span>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Action buttons (right side) -->
  <div class="process-terminated-pane__actions">
    <Button variant="primary" onclick={onrestart}>
      {m.action_restart()}
    </Button>
    <Button variant="ghost" onclick={onclose}>
      {m.action_close()}
    </Button>
  </div>
</div>

<style>
  .process-terminated-pane {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    min-height: 44px;
    padding: var(--space-3, 12px);
    background-color: var(--color-bg-surface);
    border-top: 1px solid var(--color-border);
    gap: var(--space-3, 12px);
    font-family: var(--font-ui);
    font-size: var(--font-size-ui-base);
  }

  .process-terminated-pane__status {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2, 8px);
    flex: 1;
    min-width: 0;
  }

  .process-terminated-pane__icon {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .process-terminated-pane__icon--success {
    color: var(--color-success);
  }

  .process-terminated-pane__icon--error {
    color: var(--color-error);
  }

  .process-terminated-pane__exit-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .process-terminated-pane__primary {
    color: var(--color-text-primary);
    font-size: var(--font-size-ui-base);
  }

  .process-terminated-pane__secondary {
    color: var(--color-text-secondary);
    font-size: var(--font-size-ui-sm);
  }

  .process-terminated-pane__actions {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    flex-shrink: 0;
  }
</style>
