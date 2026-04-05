<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SshHostKeyDialog — TOFU host key verification dialog (FS-SSH-011).

  Shows on first connection or key change. Displays the host (from config,
  never from server data — SEC-BLK-004), fingerprint, key type, and a
  warning when the key has changed.

  Props:
    open        — whether dialog is open
    host        — the configured host value (SEC-BLK-004: from config.host, never raw server data)
    keyType     — key algorithm (e.g. "ED25519")
    fingerprint — SHA-256 fingerprint
    isChanged   — true if this is a key change (potential MITM)
    onaccept    — called when user accepts the key
    onreject    — called when user rejects the key
    onclose     — called when dialog closes
-->
<script lang="ts">
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    open: boolean;
    /** Configured host value — SEC-BLK-004: NEVER use server-provided data here. */
    host: string;
    keyType: string;
    fingerprint: string;
    isChanged: boolean;
    onaccept?: () => void;
    onreject?: () => void;
    onclose?: () => void;
  }

  const { open, host, keyType, fingerprint, isChanged, onaccept, onreject, onclose }: Props =
    $props();

  function handleAccept() {
    onaccept?.();
    onclose?.();
  }

  function handleReject() {
    onreject?.();
    onclose?.();
  }
</script>

<Dialog
  {open}
  title={isChanged ? m.ssh_host_key_dialog_title_changed() : m.ssh_host_key_dialog_title_verify()}
  size="medium"
  {onclose}
>
  {#snippet children()}
    <div class="ssh-host-key-dialog">
      {#if isChanged}
        <!-- Key change: prominent warning (FS-SSH-011) -->
        <div class="ssh-host-key-dialog__warning" role="alert">
          <strong>{m.ssh_host_key_dialog_warning_title({ host })}</strong>
          <p>
            {m.ssh_host_key_dialog_warning_body()}
          </p>
        </div>
      {:else}
        <!-- First-time TOFU (FS-SSH-011) -->
        <p class="ssh-host-key-dialog__intro">
          {m.ssh_host_key_dialog_intro({ host })}
        </p>
      {/if}

      <dl class="ssh-host-key-dialog__details">
        <div class="ssh-host-key-dialog__detail-row">
          <dt>{m.ssh_host_key_dialog_label_host()}</dt>
          <!-- SEC-BLK-004: host is from config, text interpolation only -->
          <dd>{host}</dd>
        </div>
        <div class="ssh-host-key-dialog__detail-row">
          <dt>{m.ssh_host_key_dialog_label_key_type()}</dt>
          <dd>{keyType}</dd>
        </div>
        <div class="ssh-host-key-dialog__detail-row">
          <dt>{m.ssh_host_key_dialog_label_fingerprint()}</dt>
          <dd class="ssh-host-key-dialog__fingerprint">{fingerprint}</dd>
        </div>
      </dl>
    </div>
  {/snippet}

  {#snippet footer()}
    <!-- FS-SSH-011: default action is Reject — Accept requires deliberate non-default action -->
    <Button variant="ghost" onclick={handleReject}>{m.action_cancel()}</Button>
    {#if isChanged}
      <!-- Key change: Accept must be non-default, visually less prominent -->
      <Button variant="ghost" onclick={handleAccept}>{m.ssh_host_key_dialog_accept_anyway()}</Button
      >
    {:else}
      <Button variant="primary" onclick={handleAccept}>{m.ssh_host_key_dialog_accept()}</Button>
    {/if}
  {/snippet}
</Dialog>

<style>
  .ssh-host-key-dialog {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .ssh-host-key-dialog__warning {
    padding: var(--space-3);
    background-color: var(--color-error-bg, rgba(239, 68, 68, 0.1));
    border: 1px solid var(--color-error);
    border-radius: var(--radius-sm);
    color: var(--color-error);
    font-size: var(--font-size-ui-sm);
  }

  .ssh-host-key-dialog__warning p {
    margin-top: var(--space-1);
    color: var(--color-text-primary);
  }

  .ssh-host-key-dialog__intro {
    color: var(--color-text-secondary);
    font-size: var(--font-size-ui-sm);
    line-height: 1.5;
  }

  .ssh-host-key-dialog__details {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-3);
    background-color: var(--color-bg-surface);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-ui-sm);
  }

  .ssh-host-key-dialog__detail-row {
    display: flex;
    gap: var(--space-3);
  }

  .ssh-host-key-dialog__detail-row dt {
    flex-shrink: 0;
    min-width: 120px;
    color: var(--color-text-secondary);
    font-weight: var(--font-weight-semibold);
  }

  .ssh-host-key-dialog__detail-row dd {
    color: var(--color-text-primary);
    word-break: break-all;
    margin: 0;
  }

  .ssh-host-key-dialog__fingerprint {
    font-family: var(--font-terminal);
    font-size: var(--font-size-ui-xs);
  }
</style>
