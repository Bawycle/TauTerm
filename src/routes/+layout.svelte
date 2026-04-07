<!-- SPDX-License-Identifier: MPL-2.0 -->
<script lang="ts">
  import '../app.css';
  import { preferences } from '$lib/state/preferences.svelte';
  import { applyTheme } from '$lib/theming/apply-theme';
  import { Tooltip } from 'bits-ui';

  const { children } = $props();

  $effect(() => {
    applyTheme(
      preferences.value?.appearance.themeName ?? 'umbra',
      preferences.value?.themes ?? [],
    );
  });
</script>

<!-- Tooltip.Provider is required by Bits UI v2 for all Tooltip.Root descendants.
     Placed here (layout) rather than in +page.svelte so that any future route
     automatically inherits the context. -->
<Tooltip.Provider>
  {@render children()}
</Tooltip.Provider>
