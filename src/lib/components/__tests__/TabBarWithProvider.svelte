<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Test-only wrapper: renders TabBar inside a Tooltip.Provider context.
  Required because TabBar uses <Tooltip.Root> from Bits UI v2, which needs
  an ancestor <Tooltip.Provider> to initialise its context.

  This component is NOT part of the production build.
  It is used exclusively by TabBar.provider.test.ts.
-->
<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import TabBar from '../TabBar.svelte';
  import type { TabState } from '$lib/ipc/types';

  interface Props {
    tabs?: TabState[];
    activeTabId?: string;
    onTabClick?: (tabId: string) => void;
    onTabClose?: (tabId: string) => void;
    onNewTab?: () => void;
  }

  const {
    tabs = [],
    activeTabId = '',
    onTabClick = () => {},
    onTabClose = () => {},
    onNewTab = () => {},
  }: Props = $props();
</script>

<Tooltip.Provider>
  <TabBar
    {tabs}
    {activeTabId}
    {onTabClick}
    {onTabClose}
    {onNewTab}
  />
</Tooltip.Provider>
