<!-- SPDX-License-Identifier: MPL-2.0 -->
<script lang="ts">
  interface Props {
    /** Resolved title for this pane. Already i18n-resolved by caller. */
    title: string;
    /** Whether this pane is the active (focused) one. */
    isActive: boolean;
  }

  const { title, isActive }: Props = $props();
</script>

<div class="pane-title-bar" class:pane-title-bar--active={isActive} aria-hidden="true">
  <span class="pane-title-bar__title">{title}</span>
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
</style>
