<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SshPassphraseDialog — passphrase prompt for SSH encrypted private key auth (FS-SSH-019a).

  Shown when the backend emits `passphrase-prompt`. Collects the passphrase for
  an encrypted private key and invokes `provide_passphrase`.

  Props:
    open                — whether dialog is open
    keyPathLabel        — filename of the private key (never a full path)
    failed              — previous attempt was rejected (show error message)
    isKeychainAvailable — OS keychain is available (show save checkbox)
    onsubmit            — called with (passphrase, saveInKeychain)
    oncancel            — called when user cancels (causes connection abort)
    onclose             — called when dialog closes
-->
<script lang="ts">
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    open: boolean;
    keyPathLabel: string;
    failed?: boolean;
    isKeychainAvailable?: boolean;
    onsubmit?: (passphrase: string, saveInKeychain: boolean) => void;
    oncancel?: () => void;
    onclose?: () => void;
  }

  const {
    open,
    keyPathLabel,
    failed = false,
    isKeychainAvailable = false,
    onsubmit,
    oncancel,
    onclose,
  }: Props = $props();

  let passphrase = $state('');
  let saveInKeychain = $state(false);

  function handleSubmit() {
    onsubmit?.(passphrase, saveInKeychain);
    passphrase = '';
    saveInKeychain = false;
    // Do NOT call onclose here: the parent controls `open` by clearing the
    // passphrase-prompt state when onsubmit fires. Calling onclose would
    // cascade into handleCancel → oncancel, signalling a cancellation after
    // a successful submit.
  }

  function handleCancel() {
    passphrase = '';
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

<Dialog {open} title={m.ssh_passphrase_dialog_title()} size="small" onclose={handleCancel}>
  {#snippet children()}
    <div class="ssh-passphrase-dialog">
      <p class="ssh-passphrase-dialog__intro">
        {m.ssh_passphrase_intro({ key: keyPathLabel })}
      </p>

      {#if failed}
        <p class="ssh-passphrase-dialog__error">{m.ssh_passphrase_auth_failed()}</p>
      {/if}

      <div class="ssh-passphrase-dialog__field">
        <label class="ssh-passphrase-dialog__label" for="ssh-passphrase-input"
          >{m.ssh_passphrase_field_label()}</label
        >
        <!-- svelte-ignore a11y_autofocus -->
        <input
          id="ssh-passphrase-input"
          class="ssh-passphrase-dialog__input"
          type="password"
          bind:value={passphrase}
          autofocus
          onkeydown={handleKeydown}
        />
      </div>

      {#if isKeychainAvailable}
        <div class="ssh-passphrase-dialog__field ssh-passphrase-dialog__field--checkbox">
          <label class="ssh-passphrase-dialog__checkbox-label">
            <input type="checkbox" bind:checked={saveInKeychain} />
            {m.ssh_passphrase_save_in_keychain()}
          </label>
        </div>
      {/if}
    </div>
  {/snippet}

  {#snippet footer()}
    <Button variant="ghost" onclick={handleCancel}>{m.action_cancel()}</Button>
    <Button variant="primary" onclick={handleSubmit} disabled={!passphrase}
      >{m.action_unlock()}</Button
    >
  {/snippet}
</Dialog>

<style>
  .ssh-passphrase-dialog {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .ssh-passphrase-dialog__intro {
    color: var(--color-text-secondary);
    font-size: var(--font-size-ui-sm);
  }

  .ssh-passphrase-dialog__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .ssh-passphrase-dialog__label {
    display: block;
    font-size: var(--font-size-ui-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-secondary);
    font-family: var(--font-ui);
  }

  .ssh-passphrase-dialog__input {
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

  .ssh-passphrase-dialog__input:focus-visible {
    border-color: var(--color-focus-ring);
    box-shadow: 0 0 0 2px var(--color-focus-ring);
  }

  .ssh-passphrase-dialog__error {
    color: var(--color-error);
    font-size: var(--font-size-ui-sm);
  }

  .ssh-passphrase-dialog__field--checkbox {
    flex-direction: row;
    align-items: center;
  }

  .ssh-passphrase-dialog__checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-ui-sm);
    color: var(--color-text-secondary);
    font-family: var(--font-ui);
    cursor: pointer;
  }
</style>
