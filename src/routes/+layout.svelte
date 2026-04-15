<!-- SPDX-License-Identifier: MPL-2.0 -->
<script lang="ts">
  import '../app.css';
  import { preferences } from '$lib/state/preferences.svelte';
  import { applyTheme } from '$lib/theming/apply-theme';
  import { Tooltip } from 'bits-ui';

  const { children } = $props();

  $effect(() => {
    applyTheme(
      preferences.value?.appearance?.themeName ?? 'umbra',
      preferences.value?.themes ?? [],
    );
  });

  $effect(() => {
    const fontFamily = preferences.value?.appearance?.fontFamily ?? '';
    const fontSize = preferences.value?.appearance?.fontSize ?? 14;
    const opacity = preferences.value?.appearance?.opacity ?? 1.0;
    // Override CSS token only when a non-empty value is set; otherwise the
    // @theme fallback ('JetBrains Mono', …) from app.css remains in effect.
    if (fontFamily) {
      document.documentElement.style.setProperty('--font-terminal', fontFamily);
    } else {
      document.documentElement.style.removeProperty('--font-terminal');
    }
    document.documentElement.style.setProperty('--font-size-terminal', `${fontSize}px`);
    document.documentElement.style.setProperty('--terminal-opacity', String(opacity));
  });
</script>

<!-- Tooltip.Provider is required by Bits UI v2 for all Tooltip.Root descendants.
     Placed here (layout) rather than in +page.svelte so that any future route
     automatically inherits the context. -->
<Tooltip.Provider>
  {@render children()}
</Tooltip.Provider>
