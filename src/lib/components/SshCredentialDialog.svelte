<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SshCredentialDialog — password/credential prompt for SSH auth (FS-SSH-012).

  Shown when the backend emits `credential-prompt`. Collects username and
  password (or other keyboard-interactive responses) and invokes
  `provide_credentials`.

  Props:
    open               — whether dialog is open
    host               — the configured host (for display only)
    username           — pre-filled username from the saved connection
    prompt             — optional keyboard-interactive server prompt text
    failed             — previous attempt was rejected (show error message)
    isKeychainAvailable — OS keychain is available (show save checkbox)
    onsubmit           — called with the entered credentials and saveInKeychain flag
    oncancel           — called when user cancels (causes connection abort)
    onclose            — called when dialog closes
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
    failed?: boolean;
    isKeychainAvailable?: boolean;
    onsubmit?: (password: string, saveInKeychain: boolean) => void;
    oncancel?: () => void;
    onclose?: () => void;
  }

  const {
    open,
    host,
    username,
    prompt,
    failed = false,
    isKeychainAvailable = false,
    onsubmit,
    oncancel,
    onclose,
  }: Props = $props();

  let password = $state('');
  let saveInKeychain = $state(false);

  function handleSubmit() {
    onsubmit?.(password, saveInKeychain);
    password = '';
    saveInKeychain = false;
    // Do NOT call onclose here: the parent controls `open` by clearing the
    // credential-prompt state when onsubmit fires. Calling onclose would
    // cascade into handleCancel → oncancel, signalling a cancellation after
    // a successful submit.
  }

  function handleCancel() {
    password = '';
    saveInKeychain = false;
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

      {#if failed}
        <p class="ssh-credential-dialog__error">
          {m.ssh_credential_auth_failed()}
        </p>
      {/if}

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

      {#if isKeychainAvailable}
        <div class="ssh-credential-dialog__field ssh-credential-dialog__field--checkbox">
          <label class="ssh-credential-dialog__checkbox-label">
            <input type="checkbox" bind:checked={saveInKeychain} />
            {m.ssh_credential_save_in_keychain()}
          </label>
        </div>
      {/if}
    </div>
  {/snippet}

  {#snippet footer()}
    <Button variant="ghost" onclick={handleCancel}>{m.action_cancel()}</Button>
    <Button variant="primary" onclick={handleSubmit} disabled={!password}>{m.action_connect()}</Button>
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

  .ssh-credential-dialog__error {
    color: var(--color-error);
    font-size: var(--font-size-ui-sm);
  }

  .ssh-credential-dialog__field--checkbox {
    flex-direction: row;
    align-items: center;
  }

  .ssh-credential-dialog__checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-secondary);
    font-family: var(--font-ui);
    cursor: pointer;
  }
</style>
