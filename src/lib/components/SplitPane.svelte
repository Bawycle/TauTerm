<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  SplitPane — recursive component that renders a PaneNode tree.

  - Leaf node   → renders a TerminalPane
  - Split node  → renders two SplitPane children with a draggable divider

  The divider allows the user to resize the split ratio interactively.
  The ratio is maintained as local frontend state (override of the ratio
  received from the backend snapshot); the TerminalPane ResizeObserver
  already handles PTY resize when its container changes size.

  Props:
    node           — the PaneNode to render (leaf or split)
    tabId          — parent tab identifier (forwarded to TerminalPane)
    activePaneId   — currently active pane (controls `active` on TerminalPane)
    sshStates      — Map<PaneId, SshLifecycleState> forwarded to TerminalPane
    terminatedPanes — Set<PaneId> of panes whose process exited
    wordDelimiters — forwarded to TerminalPane
    canClosePane   — whether there are multiple panes (controls close visibility)
    onpaneclick    — called with paneId when user clicks a pane
    onclosepane    — called with paneId when user requests pane close
    onsearch       — called when user opens search from pane context menu
    onsplith       — called when user requests horizontal split from context menu
    onsplitv       — called when user requests vertical split from context menu
-->
<script lang="ts">
  import type { PaneNode, PaneId, TabId, SshLifecycleState } from '$lib/ipc/types';
  import TerminalPane from './TerminalPane.svelte';
  import SplitPane from './SplitPane.svelte';

  interface Props {
    node: PaneNode;
    tabId: TabId;
    activePaneId: PaneId;
    sshStates: Map<PaneId, SshLifecycleState>;
    terminatedPanes: Set<PaneId>;
    wordDelimiters?: string;
    canClosePane?: boolean;
    onpaneclick?: (paneId: PaneId) => void;
    onclosepane?: (paneId: PaneId) => void;
    onsearch?: (paneId: PaneId) => void;
    onsplith?: (paneId: PaneId) => void;
    onsplitv?: (paneId: PaneId) => void;
  }

  const {
    node,
    tabId,
    activePaneId,
    sshStates,
    terminatedPanes,
    wordDelimiters,
    canClosePane = true,
    onpaneclick,
    onclosepane,
    onsearch,
    onsplith,
    onsplitv,
  }: Props = $props();

  // ---------------------------------------------------------------------------
  // Split ratio — local override so dragging updates immediately without
  // waiting for a backend round-trip.
  //
  // `backendRatio` tracks the ratio from the backend via $derived (reactive).
  // `dragRatio` holds the in-progress drag value (null when not dragging).
  // `ratio` resolves to whichever is current.
  // ---------------------------------------------------------------------------

  /** Ratio from the backend snapshot — re-derived whenever `node` prop changes. */
  const backendRatio = $derived(node.type === 'split' ? node.ratio : 0.5);

  /** Ratio overridden by an active drag gesture; null means "use backendRatio". */
  let dragRatio = $state<number | null>(null);

  /** Effective ratio used for layout rendering. */
  const ratio = $derived(dragRatio ?? backendRatio);

  // ---------------------------------------------------------------------------
  // Drag state
  // ---------------------------------------------------------------------------

  let containerEl = $state<HTMLElement | undefined>(undefined);

  /** Clamp ratio to leave at least 5% on each side so neither pane vanishes. */
  function clampRatio(r: number): number {
    return Math.max(0.05, Math.min(0.95, r));
  }

  function handlePointerDown(e: PointerEvent) {
    e.preventDefault();
    // Start drag — initialise dragRatio from current effective ratio.
    dragRatio = ratio;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (dragRatio === null || !containerEl || node.type !== 'split') return;
    const rect = containerEl.getBoundingClientRect();
    if (node.direction === 'horizontal') {
      // Horizontal split: first pane left, second pane right → ratio is width fraction.
      dragRatio = clampRatio((e.clientX - rect.left) / rect.width);
    } else {
      // Vertical split: first pane top, second pane bottom → ratio is height fraction.
      dragRatio = clampRatio((e.clientY - rect.top) / rect.height);
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (dragRatio === null) return;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    // Keep the final drag value — dragRatio stays set so layout holds the
    // user's chosen ratio until the next backend snapshot updates backendRatio.
  }
</script>

{#if node.type === 'leaf'}
  <!--
    Leaf node: render the terminal pane directly.
    The outer div fills its flex slot and contains the pane.
  -->
  <div class="split-pane__leaf" role="none" onclick={() => onpaneclick?.(node.paneId)}>
    <TerminalPane
      paneId={node.paneId}
      {tabId}
      active={node.paneId === activePaneId}
      terminated={terminatedPanes.has(node.paneId)}
      {canClosePane}
      sshState={sshStates.get(node.paneId) ?? null}
      {wordDelimiters}
      onclosepane={() => onclosepane?.(node.paneId)}
      onsearch={() => onsearch?.(node.paneId)}
      onsplitH={() => onsplith?.(node.paneId)}
      onsplitV={() => onsplitv?.(node.paneId)}
    />
  </div>
{:else}
  <!--
    Split node: two children side-by-side (horizontal) or stacked (vertical),
    separated by a draggable divider.
  -->
  <div
    bind:this={containerEl}
    class="split-pane__container"
    class:split-pane__container--horizontal={node.direction === 'horizontal'}
    class:split-pane__container--vertical={node.direction === 'vertical'}
  >
    <!-- First child -->
    <div
      class="split-pane__child"
      style={node.direction === 'horizontal'
        ? `width: ${ratio * 100}%; height: 100%;`
        : `width: 100%; height: ${ratio * 100}%;`}
    >
      <SplitPane
        node={node.first}
        {tabId}
        {activePaneId}
        {sshStates}
        {terminatedPanes}
        {wordDelimiters}
        {canClosePane}
        {onpaneclick}
        {onclosepane}
        {onsearch}
        {onsplith}
        {onsplitv}
      />
    </div>

    <!-- Draggable divider -->
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div
      class="split-pane__divider"
      class:split-pane__divider--horizontal={node.direction === 'horizontal'}
      class:split-pane__divider--vertical={node.direction === 'vertical'}
      class:split-pane__divider--dragging={dragRatio !== null}
      role="separator"
      aria-orientation={node.direction === 'horizontal' ? 'vertical' : 'horizontal'}
      onpointerdown={handlePointerDown}
      onpointermove={handlePointerMove}
      onpointerup={handlePointerUp}
    ></div>

    <!-- Second child -->
    <div class="split-pane__child split-pane__child--second">
      <SplitPane
        node={node.second}
        {tabId}
        {activePaneId}
        {sshStates}
        {terminatedPanes}
        {wordDelimiters}
        {canClosePane}
        {onpaneclick}
        {onclosepane}
        {onsearch}
        {onsplith}
        {onsplitv}
      />
    </div>
  </div>
{/if}

<style>
  /* Leaf: fills its flex slot entirely. */
  .split-pane__leaf {
    width: 100%;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* Container for a split node — flex row (horizontal) or column (vertical). */
  .split-pane__container {
    width: 100%;
    height: 100%;
    overflow: hidden;
    display: flex;
  }

  .split-pane__container--horizontal {
    flex-direction: row;
  }

  .split-pane__container--vertical {
    flex-direction: column;
  }

  /*
   * Each child takes the size set by inline style (ratio-based).
   * The second child fills the remaining space via flex: 1 so that rounding
   * errors in the percentage calculation never create a gap.
   */
  .split-pane__child {
    overflow: hidden;
    flex-shrink: 0;
  }

  .split-pane__child--second {
    flex: 1;
  }

  /* Divider — 5px hit area with a 1px visible line centred inside it. */
  .split-pane__divider {
    flex-shrink: 0;
    position: relative;
    background-color: var(--color-divider);
    transition: background-color var(--duration-instant, 80ms);
    z-index: 1;
    /* The 1px border provides the visual line; the full width/height gives the
       pointer hit area. */
  }

  .split-pane__divider:hover,
  .split-pane__divider--dragging {
    background-color: var(--color-divider-active);
  }

  .split-pane__divider--horizontal {
    width: 5px;
    height: 100%;
    cursor: col-resize;
  }

  .split-pane__divider--vertical {
    width: 100%;
    height: 5px;
    cursor: row-resize;
  }
</style>
