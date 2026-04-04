<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  StatusBar — single-line status strip at the bottom of the window.
  Displays the active pane's session type, SSH connection state,
  CWD, process title, and notification indicators.

  IPC sources:
    - Reads active pane state from session state (SessionState / PaneState)
    - listen('ssh-state-changed') → SshStateChangedEvent
-->
<script lang="ts">
  // i18n: import * as m from '$lib/paraglide/messages';
  // (available once Paraglide compilation runs)
</script>

<div class="status-bar" role="status" aria-live="polite">
  <div class="status-bar__left">
    <!-- Session type indicator and SSH badge rendered here -->
    <span class="status-bar__session-type">
      <!-- TODO: render session type from active pane state -->
    </span>
  </div>

  <div class="status-bar__center">
    <!-- CWD / process title rendered here -->
  </div>

  <div class="status-bar__right">
    <!-- Notification indicators rendered here -->
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
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .status-bar__right {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
  }

  .status-bar__session-type {
    color: var(--color-text-tertiary);
    font-size: var(--font-size-ui-2xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
</style>
