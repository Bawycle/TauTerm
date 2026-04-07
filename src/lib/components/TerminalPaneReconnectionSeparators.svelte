<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPaneReconnectionSeparators — decorative overlay injected at each SSH
  reconnection event (FS-SSH-042, UXD §7.19).

  Not interactive; aria-hidden (purely decorative).

  Props:
    separators — array of Unix timestamps (ms) for each reconnection event
-->
<script lang="ts">
  import SshReconnectionSeparator from './SshReconnectionSeparator.svelte';

  interface Props {
    separators: number[];
  }

  let { separators }: Props = $props();
</script>

{#if separators.length > 0}
  <div class="terminal-pane__reconnection-separators" aria-hidden="true">
    {#each separators as ts (ts)}
      <SshReconnectionSeparator timestampMs={ts} />
    {/each}
  </div>
{/if}
