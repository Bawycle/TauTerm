<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  TerminalPane — an individual terminal pane bound to a PTY session.

  Renders the character cell grid (DOM-based, rows of spans), cursor overlay,
  selection highlighting, and scrollbar. Handles keyboard input, mouse events,
  and resize observation.

  Props:
    paneId  — unique pane identifier (PaneId from IPC contract)
    active  — whether this pane currently has focus

  IPC sources:
    - invoke('get_pane_screen_snapshot') on mount → initial screen state
    - listen('screen-update')            → ScreenUpdateEvent (incremental updates)
    - listen('scroll-position-changed')  → ScrollPositionChangedEvent
    - listen('mode-state-changed')       → ModeStateChangedEvent (DECCKM)
  IPC commands:
    - invoke('resize_pane')       on viewport resize (debounced, TUITC-FN-072/073)
    - invoke('send_input')        on keyboard input
    - invoke('scroll_pane')       on mouse wheel
    - invoke('copy_to_clipboard') on selection complete
    - invoke('scroll_to_bottom')  on new output when at bottom

  Security:
    - No {@html} — all cell content uses Svelte text interpolation (textContent path)
    - Input is encoded as byte array (Uint8Array → number[]), never raw string
    - Resize dimensions clamped to minimum 1 (TUITC-SEC-050)
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type {
    PaneId,
    ScreenSnapshot,
    ScreenUpdateEvent,
    ScrollPositionChangedEvent,
    ModeStateChangedEvent,
    CursorState,
  } from '$lib/ipc/types';
  import {
    buildGridFromSnapshot,
    applyUpdates,
    type CellStyle,
  } from '$lib/terminal/screen.js';
  import { keyEventToVtSequence } from '$lib/terminal/keyboard.js';
  import { SelectionManager } from '$lib/terminal/selection.js';
  import { cursorShape, cursorBlinks } from '$lib/terminal/color.js';
  import ProcessTerminatedPane from './ProcessTerminatedPane.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import ScrollToBottomButton from './ScrollToBottomButton.svelte';

  interface Props {
    paneId: PaneId;
    active: boolean;
    /** Set to true when the PTY process has exited (from session-state-changed event). */
    terminated?: boolean;
    exitCode?: number;
    signalName?: string;
    /** Whether there is more than one pane (controls Close Pane visibility). */
    canClosePane?: boolean;
    onrestart?: () => void;
    onclosepane?: () => void;
    onsearch?: () => void;
    onsplitH?: () => void;
    onsplitV?: () => void;
  }

  const {
    paneId,
    active,
    terminated = false,
    exitCode = 0,
    signalName,
    canClosePane = true,
    onrestart,
    onclosepane,
    onsearch,
    onsplitH,
    onsplitV,
  }: Props = $props();

  // -------------------------------------------------------------------------
  // Reactive state
  // -------------------------------------------------------------------------

  let cols = $state(80);
  let rows = $state(24);
  let grid = $state<CellStyle[]>([]);
  let cursor = $state<CursorState>({ row: 0, col: 0, visible: true, shape: 0, blink: true });
  let scrollOffset = $state(0);
  let scrollbackLines = $state(0);
  let decckm = $state(false);

  let cursorVisible = $state(true);
  let blinkTimer: ReturnType<typeof setInterval> | null = null;

  const selection = new SelectionManager();
  let selectionRange = $state(selection.getSelection());
  let isSelecting = $state(false);

  let scrollbarVisible = $state(false);
  let scrollbarFadeTimer: ReturnType<typeof setTimeout> | null = null;

  let viewportEl: HTMLDivElement | undefined = $state();

  // Context menu state
  let contextMenuOpen = $state(false);
  let hasSelection = $state(false);

  let unlistenScreenUpdate: (() => void) | null = null;
  let unlistenScrollPos: (() => void) | null = null;
  let unlistenModeState: (() => void) | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  // -------------------------------------------------------------------------
  // Derived
  // -------------------------------------------------------------------------

  const currentCursorShape = $derived(cursorShape(cursor.shape));
  const currentCursorBlinks = $derived(cursor.blink && cursorBlinks(cursor.shape));
  const showScrollbar = $derived(scrollbarVisible || scrollOffset > 0);

  const scrollbarThumbHeightPct = $derived(
    scrollbackLines > 0
      ? Math.max(32 / (rows * 16 || 400) * 100, (rows / (rows + scrollbackLines)) * 100)
      : 0
  );

  const scrollbarThumbTopPct = $derived(
    scrollbackLines > 0 && scrollOffset > 0
      ? ((scrollbackLines - scrollOffset) / (scrollbackLines + rows)) * 100
      : scrollOffset === 0 ? 100 - scrollbarThumbHeightPct : 0
  );

  // Build rendered grid rows
  const gridRows = $derived(
    Array.from({ length: rows }, (_, r) =>
      Array.from({ length: cols }, (_, c) => {
        const cell = grid[r * cols + c];
        return cell ?? ({
          content: ' ', fg: undefined, bg: undefined, width: 1,
          bold: false, dim: false, italic: false, underline: 0,
          blink: false, inverse: false, hidden: false, strikethrough: false,
          underlineColor: undefined,
        } satisfies CellStyle);
      })
    )
  );

  // -------------------------------------------------------------------------
  // Mount / destroy
  // -------------------------------------------------------------------------

  onMount(async () => {
    try {
      const snapshot: ScreenSnapshot = await invoke('get_pane_screen_snapshot', { paneId });
      cols = snapshot.cols;
      rows = snapshot.rows;
      grid = buildGridFromSnapshot(snapshot.cells, snapshot.rows, snapshot.cols);
      cursor = {
        row: snapshot.cursorRow,
        col: snapshot.cursorCol,
        visible: snapshot.cursorVisible,
        shape: snapshot.cursorShape,
        blink: cursorBlinks(snapshot.cursorShape),
      };
      scrollOffset = snapshot.scrollOffset;
      scrollbackLines = snapshot.scrollbackLines;
    } catch {
      // Backend not ready — grid populated by first screen-update event
    }

    unlistenScreenUpdate = await listen<ScreenUpdateEvent>('screen-update', (event) => {
      const update = event.payload;
      if (update.paneId !== paneId) return;
      applyUpdates(grid, update.cells, cols);
      cursor = update.cursor;
      // Keep scrollbackLines in sync so the scrollbar stays accurate
      if (typeof update.scrollbackLines === 'number') {
        scrollbackLines = update.scrollbackLines;
      }
      // Trigger Svelte reactivity on the grid array
      grid = grid.slice();
    });

    unlistenScrollPos = await listen<ScrollPositionChangedEvent>(
      'scroll-position-changed',
      (event) => {
        const pos = event.payload;
        if (pos.paneId !== paneId) return;
        scrollOffset = pos.offset;
        scrollbackLines = pos.scrollbackLines;
        scrollbarVisible = true;
        if (scrollbarFadeTimer) clearTimeout(scrollbarFadeTimer);
        if (scrollOffset === 0) {
          scrollbarFadeTimer = setTimeout(() => { scrollbarVisible = false; }, 1500);
        }
      }
    );

    unlistenModeState = await listen<ModeStateChangedEvent>('mode-state-changed', (event) => {
      const mode = event.payload;
      if (mode.paneId !== paneId) return;
      decckm = mode.decckm;
    });

    startCursorBlink();

    if (viewportEl) {
      resizeObserver = new ResizeObserver(() => scheduleSendResize());
      resizeObserver.observe(viewportEl);
    }
  });

  onDestroy(() => {
    unlistenScreenUpdate?.();
    unlistenScrollPos?.();
    unlistenModeState?.();
    stopCursorBlink();
    resizeObserver?.disconnect();
    if (resizeDebounceTimer) clearTimeout(resizeDebounceTimer);
    if (scrollbarFadeTimer) clearTimeout(scrollbarFadeTimer);
  });

  // -------------------------------------------------------------------------
  // Cursor blink (FS-VT-032, TUITC-FN-004)
  // -------------------------------------------------------------------------

  function startCursorBlink() {
    stopCursorBlink();
    blinkTimer = setInterval(() => {
      cursorVisible = currentCursorBlinks ? !cursorVisible : true;
    }, 530);
  }

  function stopCursorBlink() {
    if (blinkTimer) { clearInterval(blinkTimer); blinkTimer = null; }
    cursorVisible = true;
  }

  // -------------------------------------------------------------------------
  // Resize (TUITC-FN-072/073, TUITC-SEC-050)
  // -------------------------------------------------------------------------

  function scheduleSendResize() {
    if (resizeDebounceTimer) clearTimeout(resizeDebounceTimer);
    resizeDebounceTimer = setTimeout(sendResize, 50);
  }

  async function sendResize() {
    if (!viewportEl) return;
    const rect = viewportEl.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;

    // Measure cell size from a real character span if available; fallback to estimate
    const testSpan = viewportEl.querySelector('.terminal-pane__cell') as HTMLElement | null;
    const cellW = testSpan ? testSpan.offsetWidth : Math.max(1, rect.width / cols);
    const cellH = testSpan ? testSpan.offsetHeight : Math.max(1, rect.height / rows);

    // TUITC-SEC-050: clamp to minimum 1
    const newCols = Math.max(1, Math.floor(rect.width / cellW));
    const newRows = Math.max(1, Math.floor(rect.height / cellH));
    const pixelWidth = Math.max(1, Math.floor(rect.width));
    const pixelHeight = Math.max(1, Math.floor(rect.height));

    try {
      await invoke('resize_pane', { paneId, cols: newCols, rows: newRows, pixelWidth, pixelHeight });
      cols = newCols;
      rows = newRows;
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Keyboard input (TUITC-FN-040 to 048)
  // -------------------------------------------------------------------------

  function handleKeydown(event: KeyboardEvent) {
    // Application shortcuts (Ctrl+Shift+*) are handled at the TerminalView level
    // Do NOT intercept them here — let them bubble
    if (event.ctrlKey && event.shiftKey) return;
    if (event.ctrlKey && event.key === ',') return; // Ctrl+, = preferences

    const sequence = keyEventToVtSequence(event, decckm);
    if (sequence !== null) {
      event.preventDefault();
      sendBytes(sequence);
    }
  }

  function handleInput(event: Event) {
    const inputEvent = event as InputEvent;
    if (inputEvent.isComposing) return;
    const data = inputEvent.data;
    if (data) sendBytes(new TextEncoder().encode(data));
  }

  async function sendBytes(bytes: Uint8Array) {
    const data = Array.from(bytes);
    try {
      await invoke('send_input', { paneId, data });
    } catch {
      // PTY may have closed
    }
  }

  // -------------------------------------------------------------------------
  // Scroll-to-bottom button handler
  // -------------------------------------------------------------------------

  async function handleScrollToBottom() {
    try {
      await invoke('scroll_pane', { paneId, offset: 0 });
    } catch { /* non-fatal */ }
  }

  // -------------------------------------------------------------------------
  // Mouse wheel (TUITC-FN-052)
  // -------------------------------------------------------------------------

  async function handleWheel(event: WheelEvent) {
    event.preventDefault();
    const newOffset = Math.max(0, scrollOffset + (event.deltaY > 0 ? -3 : 3));
    try {
      await invoke('scroll_pane', { paneId, offset: newOffset });
    } catch { /* non-fatal */ }
  }

  // -------------------------------------------------------------------------
  // Selection (TUITC-FN-060/061/062)
  // -------------------------------------------------------------------------

  function pixelToCell(event: MouseEvent): { row: number; col: number } {
    if (!viewportEl) return { row: 0, col: 0 };
    const rect = viewportEl.getBoundingClientRect();
    const cw = Math.max(1, rect.width / cols);
    const ch = Math.max(1, rect.height / rows);
    return {
      col: Math.max(0, Math.min(cols - 1, Math.floor((event.clientX - rect.left) / cw))),
      row: Math.max(0, Math.min(rows - 1, Math.floor((event.clientY - rect.top) / ch))),
    };
  }

  function handleMousedown(event: MouseEvent) {
    if (event.button !== 0) return;
    isSelecting = true;
    selection.startSelection(pixelToCell(event));
    selectionRange = selection.getSelection();
  }

  function handleMousemove(event: MouseEvent) {
    if (!isSelecting) return;
    selection.extendSelection(pixelToCell(event));
    selectionRange = selection.getSelection();
  }

  async function handleMouseup(event: MouseEvent) {
    if (!isSelecting) return;
    isSelecting = false;
    selection.extendSelection(pixelToCell(event));
    selectionRange = selection.getSelection();
    if (selectionRange) {
      const text = selection.getSelectedText((r, c) => grid[r * cols + c]?.content ?? '', cols);
      hasSelection = text.length > 0;
      if (hasSelection) {
        try { await invoke('copy_to_clipboard', { text }); } catch { /* non-fatal */ }
      }
    } else {
      hasSelection = false;
    }
  }

  async function handleContextMenuCopy() {
    if (!selectionRange) return;
    const text = selection.getSelectedText((r, c) => grid[r * cols + c]?.content ?? '', cols);
    if (text) {
      try { await invoke('copy_to_clipboard', { text }); } catch { /* non-fatal */ }
    }
  }

  async function handleContextMenuPaste() {
    try {
      const text: string = await invoke('get_clipboard');
      if (text) {
        const data = Array.from(new TextEncoder().encode(text));
        await invoke('send_input', { paneId, data });
      }
    } catch { /* non-fatal */ }
  }

  // -------------------------------------------------------------------------
  // Cell rendering helpers
  // -------------------------------------------------------------------------

  function cellStyle(cell: CellStyle): string {
    const parts: string[] = [];
    const fg = cell.inverse ? cell.bg : cell.fg;
    const bg = cell.inverse ? cell.fg : cell.bg;
    if (fg) parts.push(`color:${fg}`);
    if (bg) parts.push(`background-color:${bg}`);
    if (cell.bold) parts.push('font-weight:bold');
    if (cell.italic) parts.push('font-style:italic');
    if (cell.dim) parts.push('opacity:0.5');
    if (cell.hidden) parts.push('color:transparent');
    const dec: string[] = [];
    if (cell.underline > 0) dec.push('underline');
    if (cell.strikethrough) dec.push('line-through');
    if (dec.length) parts.push(`text-decoration:${dec.join(' ')}`);
    return parts.join(';');
  }

  function isSelected(row: number, col: number): boolean {
    if (!selectionRange) return false;
    const { start, end } = selectionRange;
    if (row < start.row || row > end.row) return false;
    if (row === start.row && col < start.col) return false;
    if (row === end.row && col > end.col) return false;
    return true;
  }
</script>

<div
  class="terminal-pane"
  class:terminal-pane--active={active}
  data-pane-id={paneId}
  role="region"
  aria-label="Terminal pane"
>
  <!-- ContextMenu wraps the viewport so right-click opens it -->
  <ContextMenu
    variant="terminal"
    {hasSelection}
    {canClosePane}
    oncopy={handleContextMenuCopy}
    onpaste={handleContextMenuPaste}
    onsearch={onsearch}
    onsplitH={onsplitH}
    onsplitV={onsplitV}
    onclosepane={onclosepane}
  >
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={viewportEl}
    class="terminal-pane__viewport terminal-grid"
    tabindex={active ? 0 : -1}
    role="textbox"
    aria-multiline="true"
    aria-label="Terminal output"
    aria-readonly="false"
    onkeydown={handleKeydown}
    oninput={handleInput}
    onmousedown={handleMousedown}
    onmousemove={handleMousemove}
    onmouseup={handleMouseup}
    onwheel={handleWheel}
  >
    <!-- Cell grid: rows × cells — SECURITY: text via interpolation, never {@html} -->
    {#each gridRows as row, rowIdx}
      <div class="terminal-pane__row">
        {#each row as cell, colIdx}
          {#if cell.width !== 0}
            <span
              class="terminal-pane__cell"
              class:terminal-pane__cell--wide={cell.width === 2}
              class:terminal-pane__cell--selected={isSelected(rowIdx, colIdx) && active}
              class:terminal-pane__cell--selected-inactive={isSelected(rowIdx, colIdx) && !active}
              style={cellStyle(cell)}
            >{cell.content === '' ? '\u00a0' : cell.content}</span>
          {/if}
        {/each}
      </div>
    {/each}

    <!-- Cursor overlay (TUITC-FN-001 to 006, TUITC-UX-050 to 053) -->
    {#if cursor.visible && (cursorVisible || !currentCursorBlinks)}
      <div
        class="terminal-pane__cursor"
        class:terminal-pane__cursor--block={currentCursorShape === 'block'}
        class:terminal-pane__cursor--underline={currentCursorShape === 'underline'}
        class:terminal-pane__cursor--bar={currentCursorShape === 'bar'}
        class:terminal-pane__cursor--unfocused={!active}
        style:top="{cursor.row}lh"
        style:left="{cursor.col}ch"
        aria-hidden="true"
      ></div>
    {/if}
  </div>

  <!-- Scrollbar overlay — no layout shift (TUITC-UX-070 to 073) -->
  {#if showScrollbar && scrollbackLines > 0}
    <div class="terminal-pane__scrollbar" aria-hidden="true">
      <div
        class="terminal-pane__scrollbar-thumb"
        style:height="{scrollbarThumbHeightPct}%"
        style:top="{scrollbarThumbTopPct}%"
      ></div>
    </div>
  {/if}

  <!-- Scroll-to-bottom button — shown when scrolled up into scrollback history -->
  {#if scrollOffset > 0}
    <ScrollToBottomButton onclick={handleScrollToBottom} />
  {/if}
  </ContextMenu>

  <!-- ProcessTerminatedPane banner — shown when PTY process exits (FS-PTY-005/006) -->
  {#if terminated}
    <ProcessTerminatedPane
      {exitCode}
      {signalName}
      onrestart={onrestart}
      onclose={onclosepane}
    />
  {/if}
</div>

<style>
  .terminal-pane {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--term-bg);
    border: 1px solid var(--color-pane-border-inactive);
  }

  .terminal-pane--active {
    border-color: var(--color-pane-border-active);
  }

  .terminal-pane__viewport {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-family: var(--font-terminal);
    font-size: var(--font-size-terminal);
    line-height: var(--line-height-terminal);
    color: var(--term-fg);
    background-color: var(--term-bg);
    white-space: pre;
    outline: none;
    cursor: text;
    user-select: none;
  }

  .terminal-pane__row {
    display: flex;
    flex-wrap: nowrap;
    height: 1lh;
    min-height: 1lh;
  }

  .terminal-pane__cell {
    display: inline-block;
    width: 1ch;
    min-width: 1ch;
    height: 1lh;
    overflow: hidden;
    white-space: pre;
    flex-shrink: 0;
  }

  .terminal-pane__cell--wide {
    width: 2ch;
    min-width: 2ch;
  }

  /* Selection colors (TUITC-UX-060/061) */
  .terminal-pane__cell--selected {
    background-color: var(--term-selection-bg) !important;
  }

  .terminal-pane__cell--selected-inactive {
    background-color: var(--term-selection-bg-inactive) !important;
  }

  /* Cursor (TUITC-UX-050 to 053) */
  .terminal-pane__cursor {
    position: absolute;
    pointer-events: none;
    z-index: var(--z-cursor, 10);
  }

  .terminal-pane__cursor--block {
    width: 1ch;
    height: 1lh;
    background-color: var(--term-cursor-bg);
    mix-blend-mode: difference;
  }

  .terminal-pane__cursor--underline {
    width: 1ch;
    height: var(--size-cursor-underline-height, 2px);
    top: calc(attr(style top) + 1lh - var(--size-cursor-underline-height, 2px)) !important;
    background-color: var(--term-cursor-bg);
  }

  .terminal-pane__cursor--bar {
    width: var(--size-cursor-bar-width, 2px);
    height: 1lh;
    background-color: var(--term-cursor-bg);
  }

  /* Unfocused: hollow outline per FS-VT-034, UXD §7.3.1 */
  .terminal-pane__cursor--unfocused {
    background-color: transparent !important;
    border: 1px solid var(--term-cursor-unfocused);
    mix-blend-mode: normal;
  }

  /* Scrollbar overlay (TUITC-UX-070 to 073) */
  .terminal-pane__scrollbar {
    position: absolute;
    top: 0;
    right: 0;
    width: var(--size-scrollbar-width, 8px);
    height: 100%;
    z-index: var(--z-scrollbar, 15);
    pointer-events: none;
    transition: opacity var(--duration-slow, 300ms);
  }

  .terminal-pane__scrollbar-thumb {
    position: absolute;
    right: 0;
    width: 100%;
    min-height: 32px;
    background-color: var(--color-scrollbar-thumb);
    border-radius: var(--radius-full, 9999px);
  }

  @media (prefers-reduced-motion: reduce) {
    .terminal-pane__scrollbar {
      transition: none;
    }
  }
</style>
