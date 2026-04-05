<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Test-only wrapper: renders TerminalView inside a Tooltip.Provider context.

  TerminalView → TabBar → Tooltip.Root requires an ancestor Tooltip.Provider.
  Without this wrapper, mounting TerminalView in unit tests throws
  "Context 'Tooltip.Provider' not found" (the same bug as TBTC-CTX-002).

  This component is NOT part of the production build.
  It is used exclusively by TerminalView.integration.test.ts.
-->
<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import TerminalView from '../TerminalView.svelte';
</script>

<Tooltip.Provider>
  <TerminalView />
</Tooltip.Provider>
