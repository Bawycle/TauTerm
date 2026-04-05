<!-- SPDX-License-Identifier: MPL-2.0 -->
<script lang="ts">
  import '../app.css';
  import { Tooltip } from 'bits-ui';

  const { children } = $props();
</script>

<!-- Tooltip.Provider is required by Bits UI v2 for all Tooltip.Root descendants.
     Placed here (layout) rather than in +page.svelte so that any future route
     automatically inherits the context. -->
<Tooltip.Provider>
  {@render children()}
</Tooltip.Provider>
