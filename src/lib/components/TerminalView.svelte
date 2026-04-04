<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalView — main terminal UI surface.
  Composes the TabBar, one or more TerminalPane instances per tab,
  and the StatusBar. Manages session state received from the backend.

  IPC sources:
    - invoke('get_session_state') on mount → SessionState snapshot
    - listen('session-state-changed') → SessionStateChangedEvent deltas
-->
<script lang="ts">
  import TabBar from './TabBar.svelte';
  import StatusBar from './StatusBar.svelte';
</script>

<div class="terminal-view">
  <TabBar />

  <div class="terminal-view__pane-area">
    <!-- TerminalPane instances are rendered here based on active tab layout -->
    <slot />
  </div>

  <StatusBar />
</div>

<style>
  .terminal-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--color-bg-base);
  }

  .terminal-view__pane-area {
    flex: 1;
    overflow: hidden;
    position: relative;
    background-color: var(--term-bg);
  }
</style>
