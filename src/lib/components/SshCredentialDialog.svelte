<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SshCredentialDialog — password/credential prompt for SSH auth (FS-SSH-012).

  Shown when the backend emits `credential-prompt`. Collects username and
  password (or other keyboard-interactive responses) and invokes
  `provide_credentials`.

  Props:
    open     — whether dialog is open
    host     — the configured host (for display only)
    username — pre-filled username from the saved connection
    prompt   — optional keyboard-interactive server prompt text
    onsubmit — called with the entered credentials
    oncancel — called when user cancels (causes connection abort)
    onclose  — called when dialog closes
-->
<script lang="ts">
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    open: boolean;
    host: string;
    username: string;
    prompt?: string;
    onsubmit?: (password: string) => void;
    oncancel?: () => void;
    onclose?: () => void;
  }

  const { open, host, username, prompt, onsubmit, oncancel, onclose }: Props = $props();

  let password = $state('');

  function handleSubmit() {
    onsubmit?.(password);
    password = '';
    onclose?.();
  }

  function handleCancel() {
    password = '';
    oncancel?.();
    onclose?.();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSubmit();
    }
  }
</script>

<Dialog {open} title={m.ssh_state_authenticating({ host })} size="small" onclose={handleCancel}>
  {#snippet children()}
    <div class="ssh-credential-dialog">
      <p class="ssh-credential-dialog__intro">
        {prompt ?? m.ssh_credential_password_for({ username, host })}
      </p>

      <div class="ssh-credential-dialog__field">
        <label class="ssh-credential-dialog__label" for="ssh-credential-username">
          {m.connection_field_user()}
        </label>
        <input
          id="ssh-credential-username"
          class="ssh-credential-dialog__input"
          type="text"
          value={username}
          readonly
        />
      </div>

      <div class="ssh-credential-dialog__field">
        <label class="ssh-credential-dialog__label" for="ssh-credential-password">
          {m.connection_field_password()}
        </label>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          id="ssh-credential-password"
          class="ssh-credential-dialog__input"
          type="password"
          bind:value={password}
          autofocus
          onkeydown={handleKeydown}
        />
      </div>
    </div>
  {/snippet}

  {#snippet footer()}
    <Button variant="ghost" onclick={handleCancel}>{m.action_cancel()}</Button>
    <Button variant="primary" onclick={handleSubmit} disabled={!password}>{m.action_ok()}</Button>
  {/snippet}
</Dialog>

<style>
  .ssh-credential-dialog {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .ssh-credential-dialog__intro {
    color: var(--color-text-secondary);
    font-size: var(--font-size-ui-sm);
  }

  .ssh-credential-dialog__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .ssh-credential-dialog__label {
    display: block;
    font-size: var(--font-size-ui-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-secondary);
    font-family: var(--font-ui);
  }

  .ssh-credential-dialog__input {
    width: 100%;
    height: 44px;
    padding: 0 var(--space-3);
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-primary);
    background-color: var(--term-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-ui);
    outline: none;
  }

  .ssh-credential-dialog__input:focus-visible {
    border-color: var(--color-focus-ring);
    box-shadow: 0 0 0 2px var(--color-focus-ring);
  }
</style>
