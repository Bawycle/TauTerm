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
  import StatusBar from '../StatusBar.svelte';

  interface Props {
    cols?: number | null;
    rows?: number | null;
    dimsVisible?: boolean;
  }

  const { cols: initCols = null, rows: initRows = null, dimsVisible: initDimsVisible = false }: Props = $props();

  let cols = $state<number | null>(initCols);
  let rows = $state<number | null>(initRows);
  let dimsVisible = $state<boolean>(initDimsVisible);

  export function setCols(v: number | null) { cols = v; }
  export function setRows(v: number | null) { rows = v; }
  export function setDimsVisible(v: boolean) { dimsVisible = v; }
</script>

<StatusBar {cols} {rows} {dimsVisible} />
