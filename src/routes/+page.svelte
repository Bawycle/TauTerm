<!-- SPDX-License-Identifier: MPL-2.0 -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { Tooltip } from 'bits-ui';
  import TerminalView from '$lib/components/TerminalView.svelte';
  import { initLocale } from '$lib/state/locale.svelte';

  onMount(async () => {
    // Restore the persisted locale from the backend (FS-I18N-005).
    await initLocale();
  });
</script>

<!-- Tooltip.Provider is required by bits-ui v2 for all Tooltip.Root descendants -->
<Tooltip.Provider>
  <div class="app-shell">
    <TerminalView />
  </div>
</Tooltip.Provider>

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
