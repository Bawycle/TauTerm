<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Test-only harness: wraps TerminalPane to allow reactive changes to the
  `active` prop after mount.

  In Svelte 5, $set() on a mounted component is not available. Tests that need
  to toggle `active` after mount must go through a Svelte component whose
  $state feeds the child's props reactively.

  Usage:
    const instance = mount(TerminalPaneActiveHarness, { target, props: { paneId, tabId, active: true, cursorBlinkMs: 533 } });
    // later:
    (instance as HarnessInstance).setActive(false);
    flushSync();

  NOT part of the production build.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import TerminalPane from '$lib/components/TerminalPane.svelte';

  interface Props {
    paneId: string;
    tabId: string;
    active?: boolean;
    cursorBlinkMs?: number;
  }

  const props: Props = $props();

  let active = $state<boolean>(untrack(() => props.active ?? true));

  export function setActive(v: boolean) {
    active = v;
  }
</script>

<TerminalPane
  paneId={props.paneId}
  tabId={props.tabId}
  {active}
  cursorBlinkMs={props.cursorBlinkMs ?? 533}
/>
