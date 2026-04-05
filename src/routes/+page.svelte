<!-- SPDX-License-Identifier: MPL-2.0 -->
<script lang="ts">
  import { onMount } from 'svelte';
  import TerminalView from '$lib/components/TerminalView.svelte';
  import { initLocale } from '$lib/state/locale.svelte';

  onMount(async () => {
    // Restore the persisted locale from the backend (FS-I18N-005).
    await initLocale();
  });
</script>

<div class="app-shell">
  <TerminalView />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background-color: var(--color-bg-base);
  }
</style>
