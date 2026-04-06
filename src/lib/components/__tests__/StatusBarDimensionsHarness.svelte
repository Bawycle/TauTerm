<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  Test-only harness: wraps StatusBar to allow reactive prop updates from tests.

  In Svelte 5, $set() on a mounted component is removed. Tests that need to
  change props after mount must go through a Svelte component whose $state
  feeds the child's props reactively.

  Props passed to mount() seed the initial state. After mount, call
  setCols() / setRows() / setDimsVisible() on the instance to change values.

  NOT part of the production build.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import StatusBar from '../StatusBar.svelte';

  interface Props {
    cols?: number | null;
    rows?: number | null;
    dimsVisible?: boolean;
  }

  const props: Props = $props();

  let cols = $state<number | null>(untrack(() => props.cols ?? null));
  let rows = $state<number | null>(untrack(() => props.rows ?? null));
  let dimsVisible = $state<boolean>(untrack(() => props.dimsVisible ?? false));

  export function setCols(v: number | null) { cols = v; }
  export function setRows(v: number | null) { rows = v; }
  export function setDimsVisible(v: boolean) { dimsVisible = v; }
</script>

<StatusBar {cols} {rows} {dimsVisible} />
