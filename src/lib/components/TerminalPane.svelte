<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPane — an individual terminal pane bound to a PTY session.
  Renders the character cell grid, cursor, selection, and scrollbar.
  Handles keyboard input, mouse events, and resize observation.

  Props:
    paneId  — unique pane identifier (PaneId from IPC contract)
    active  — whether this pane currently has focus

  IPC sources:
    - listen('screen-update') → ScreenUpdateEvent
    - listen('scroll-position-changed') → ScrollPositionChangedEvent
    - listen('notification-changed') → NotificationChangedEvent
  IPC commands:
    - invoke('resize_pane') on viewport resize
    - invoke('send_input') on keyboard input
    - invoke('scroll_pane') on mouse wheel
    - invoke('copy_selection') on copy shortcut
    - invoke('paste_to_pane') on paste shortcut
-->
<script lang="ts">
  import type { PaneId } from '$lib/ipc/types';

  interface Props {
    paneId: PaneId;
    active: boolean;
  }

  const { paneId, active }: Props = $props();
</script>

<div
  class="terminal-pane"
  class:terminal-pane--active={active}
  data-pane-id={paneId}
  role="region"
  aria-label="Terminal pane"
>
  <!-- Character cell grid rendered here (canvas or DOM) -->
  <div class="terminal-pane__viewport">
    <!-- TODO: implement cell grid rendering -->
  </div>
</div>

<style>
  .terminal-pane {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--term-bg);
    border: 1px solid var(--color-pane-border-inactive);
  }

  .terminal-pane--active {
    border-color: var(--color-pane-border-active);
  }

  .terminal-pane__viewport {
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-family: var(--font-terminal);
    font-size: var(--font-size-terminal);
    line-height: var(--line-height-terminal);
    color: var(--term-fg);
  }
</style>
