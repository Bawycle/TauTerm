<!-- SPDX-License-Identifier: MPL-2.0 -->
<script lang="ts">
  let isRenaming = $state(false);
  let renameValue = $state('');
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  interface Props {
    title: string;
    isActive: boolean;
    onrename?: (label: string | null) => void;
  }
  const { title, isActive, onrename }: Props = $props();

  $effect(() => {
    if (isRenaming && inputEl) {
      inputEl.focus();
      inputEl.select();
    }
  });

  function startRename() {
    renameValue = title;
    isRenaming = true;
  }

  function confirmRename() {
    if (!isRenaming) return;
    isRenaming = false;
    const label: string | null = renameValue.trim() === '' ? null : renameValue.trim();
    onrename?.(label);
  }

  function cancelRename() {
    isRenaming = false;
    renameValue = '';
  }
</script>

<div
  class="pane-title-bar"
  class:pane-title-bar--active={isActive}
  aria-hidden="true"
  ondblclick={startRename}
>
  {#if isRenaming}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      bind:this={inputEl}
      class="pane-title-bar__input"
      type="text"
      bind:value={renameValue}
      onblur={confirmRename}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          confirmRename();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          cancelRename();
        }
      }}
    />
  {:else}
    <span class="pane-title-bar__title">{title}</span>
  {/if}
</div>

<style>
  .pane-title-bar {
    height: var(--size-pane-title-bar-height);
    width: 100%;
    background-color: var(--color-bg-surface);
    border-bottom: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    padding: 0 var(--space-2);
    flex-shrink: 0;
    opacity: 0.6;
    font-family: var(--font-ui);
    font-size: var(--font-size-ui-xs);
    font-weight: var(--font-weight-normal);
    color: var(--color-text-primary);
    overflow: hidden;
    cursor: default;
  }

  .pane-title-bar--active {
    opacity: 1;
    font-weight: var(--font-weight-medium);
  }

  .pane-title-bar__title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .pane-title-bar__input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: inherit;
    font: inherit;
    padding: 0;
  }
</style>
