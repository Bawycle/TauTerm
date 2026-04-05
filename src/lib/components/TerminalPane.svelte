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
    TabId,
    ScreenSnapshot,
    ScreenUpdateEvent,
    ScrollPositionChangedEvent,
    ModeStateChangedEvent,
    CursorState,
    SshLifecycleState,
  } from '$lib/ipc/types';
  import { buildGridFromSnapshot, applyUpdates, type CellStyle } from '$lib/terminal/screen.js';
  import { keyEventToVtSequence } from '$lib/terminal/keyboard.js';
  import { SelectionManager } from '$lib/terminal/selection.js';
  import { cursorShape, cursorBlinks } from '$lib/terminal/color.js';
  import { pasteToBytes } from '$lib/terminal/paste.js';
  import ProcessTerminatedPane from './ProcessTerminatedPane.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import ScrollToBottomButton from './ScrollToBottomButton.svelte';
  import Dialog from '$lib/ui/Dialog.svelte';
  import Button from '$lib/ui/Button.svelte';
  import Toggle from '$lib/ui/Toggle.svelte';
  import * as m from '$lib/paraglide/messages';

  interface Props {
    paneId: PaneId;
    tabId: TabId;
    active: boolean;
    /** Set to true when the PTY process has exited (from session-state-changed event). */
    terminated?: boolean;
    exitCode?: number;
    signalName?: string;
    /** Whether there is more than one pane (controls Close Pane visibility). */
    canClosePane?: boolean;
    /** SSH lifecycle state for Disconnected reconnect UI. */
    sshState?: SshLifecycleState | null;
    /**
     * Characters treated as word delimiters for double-click word selection.
     * Mirrors the Rust backend default (TerminalPrefs.wordDelimiters).
     */
    wordDelimiters?: string;
    /**
     * Whether to show a confirmation dialog when pasting multi-line text
     * without bracketed paste active (FS-CLIP-009).
     */
    confirmMultilinePaste?: boolean;
    onrestart?: () => void;
    onclosepane?: () => void;
    onsearch?: () => void;
    onsplitH?: () => void;
    onsplitV?: () => void;
    /** Called when the user checks "Don't ask again" in the paste confirmation dialog. */
    ondisableConfirmMultilinePaste?: () => void;
  }

  const {
    paneId,
    tabId: _tabId,
    active,
    terminated = false,
    exitCode = 0,
    signalName,
    canClosePane = true,
    sshState = null,
    wordDelimiters = ' \t|"\'`&()*,;<=>[]{}~',
    confirmMultilinePaste = true,
    onrestart,
    onclosepane,
    onsearch,
    onsplitH,
    onsplitV,
    ondisableConfirmMultilinePaste,
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
  let deckpam = $state(false);
  let mouseReporting = $state<'none' | 'x10' | 'normal' | 'buttonEvent' | 'anyEvent'>('none');
  let mouseEncoding = $state<'x10' | 'sgr' | 'urxvt'>('x10');
  let focusEventsActive = $state(false);
  let bracketedPasteActive = $state(false);

  let cursorVisible = $state(true);
  let blinkTimer: ReturnType<typeof setInterval> | null = null;

  const selection = new SelectionManager();
  let selectionRange = $state(selection.getSelection());
  let isSelecting = $state(false);

  let scrollbarVisible = $state(false);
  let scrollbarFadeTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollbarDragging = $state(false);
  let scrollbarHover = $state(false);
  let scrollbarDragStartY = 0;
  let scrollbarDragStartOffset = 0;

  let viewportEl: HTMLDivElement | undefined = $state();
  let scrollbarEl: HTMLDivElement | undefined = $state();

  // Context menu state
  let hasSelection = $state(false);

  // FS-CLIP-009: Multiline paste confirmation dialog state.
  let pasteConfirmOpen = $state(false);
  let pasteConfirmText = $state('');

  let screenGeneration = $state(0);

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
  const showScrollbar = $derived(
    scrollbarVisible || scrollOffset > 0 || scrollbarHover || scrollbarDragging,
  );

  const scrollbarThumbHeightPct = $derived(
    scrollbackLines > 0
      ? Math.max((32 / (rows * 16 || 400)) * 100, (rows / (rows + scrollbackLines)) * 100)
      : 0,
  );

  const scrollbarThumbTopPct = $derived(
    scrollbackLines > 0 && scrollOffset > 0
      ? ((scrollbackLines - scrollOffset) / (scrollbackLines + rows)) * 100
      : scrollOffset === 0
        ? 100 - scrollbarThumbHeightPct
        : 0,
  );

  // Build rendered grid rows
  const gridRows = $derived(
    Array.from({ length: rows }, (_, r) =>
      Array.from({ length: cols }, (_, c) => {
        const cell = grid[r * cols + c];
        return (
          cell ??
          ({
            content: ' ',
            fg: undefined,
            bg: undefined,
            width: 1,
            bold: false,
            dim: false,
            italic: false,
            underline: 0,
            blink: false,
            inverse: false,
            hidden: false,
            strikethrough: false,
            underlineColor: undefined,
          } satisfies CellStyle)
        );
      }),
    ),
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
      screenGeneration++;
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
          scrollbarFadeTimer = setTimeout(() => {
            scrollbarVisible = false;
          }, 1500);
        }
      },
    );

    unlistenModeState = await listen<ModeStateChangedEvent>('mode-state-changed', (event) => {
      const mode = event.payload;
      if (mode.paneId !== paneId) return;
      decckm = mode.decckm;
      deckpam = mode.deckpam;
      mouseReporting = mode.mouseReporting;
      mouseEncoding = mode.mouseEncoding;
      focusEventsActive = mode.focusEvents;
      bracketedPasteActive = mode.bracketedPaste;
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
    if (blinkTimer) {
      clearInterval(blinkTimer);
      blinkTimer = null;
    }
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
      await invoke('resize_pane', {
        paneId,
        cols: newCols,
        rows: newRows,
        pixelWidth,
        pixelHeight,
      });
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
    // Let the IME composition pipeline finish before we consume the event
    if (event.isComposing) return;

    const sequence = keyEventToVtSequence(event, decckm, deckpam);
    if (sequence !== null) {
      event.preventDefault();
      sendBytes(sequence);
    }
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
    } catch {
      /* non-fatal */
    }
  }

  // -------------------------------------------------------------------------
  // Mouse wheel (TUITC-FN-052, FS-VT-085)
  // -------------------------------------------------------------------------

  async function handleWheel(event: WheelEvent) {
    event.preventDefault();
    // FS-VT-085: Shift+Wheel always scrolls TauTerm's scrollback
    if (!event.shiftKey && mouseReporting !== 'none') {
      const button = event.deltaY < 0 ? 64 : 65;
      const cell = pixelToCell(event);
      await sendMouseEvent(button, cell.col, cell.row, event, false);
      return;
    }
    const newOffset = Math.max(0, scrollOffset + (event.deltaY > 0 ? -3 : 3));
    try {
      await invoke('scroll_pane', { paneId, offset: newOffset });
    } catch {
      /* non-fatal */
    }
  }

  // -------------------------------------------------------------------------
  // Mouse reporting (FS-VT-080 to 086)
  // -------------------------------------------------------------------------

  async function sendMouseEvent(
    button: number,
    col: number,
    row: number,
    event: MouseEvent,
    release: boolean,
  ) {
    const modBits = (event.shiftKey ? 4 : 0) | (event.metaKey ? 8 : 0) | (event.ctrlKey ? 16 : 0);
    const cb = button | modBits;
    const cx = col + 1;
    const cy = row + 1;
    let seq: string;
    if (mouseEncoding === 'sgr') {
      const suffix = release ? 'm' : 'M';
      seq = `\x1b[<${cb};${cx};${cy}${suffix}`;
    } else {
      const clamp = (n: number) => Math.min(n + 32, 255);
      seq = `\x1b[M${String.fromCharCode(clamp(cb), clamp(cx), clamp(cy))}`;
    }
    await sendBytes(new TextEncoder().encode(seq));
  }

  function mouseButtonCode(event: MouseEvent): number {
    switch (event.button) {
      case 0:
        return 0;
      case 1:
        return 1;
      case 2:
        return 2;
      default:
        return 3;
    }
  }

  // -------------------------------------------------------------------------
  // Selection (TUITC-FN-060/061/062, FS-CLIP-002, FS-CLIP-003)
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

  /** Copy the current selection to clipboard and update hasSelection. */
  async function copySelectionToClipboard() {
    const sel = selection.getSelection();
    if (sel) {
      const text = selection.getSelectedText((r, c) => grid[r * cols + c]?.content ?? '', cols);
      hasSelection = text.length > 0;
      if (hasSelection) {
        try {
          await invoke('copy_to_clipboard', { text });
        } catch {
          /* non-fatal */
        }
      }
    } else {
      hasSelection = false;
    }
    selectionRange = sel;
  }

  async function handleMousedown(event: MouseEvent) {
    // set_active_pane on click
    if (!active) {
      try {
        await invoke('set_active_pane', { paneId });
      } catch {
        /* non-fatal */
      }
    }
    if (event.button !== 0) return;
    // FS-VT-082/083: mouse reporting active + not Shift → send to PTY, skip selection
    if (mouseReporting !== 'none' && !event.shiftKey) {
      const cell = pixelToCell(event);
      await sendMouseEvent(mouseButtonCode(event), cell.col, cell.row, event, false);
      return;
    }

    const cell = pixelToCell(event);

    // Triple-click (detail >= 3): select full line (FS-CLIP-003)
    if (event.detail >= 3) {
      isSelecting = false;
      selection.selectLineAt(cell.row, cols);
      await copySelectionToClipboard();
      return;
    }

    // Double-click (detail === 2): select word (FS-CLIP-002)
    if (event.detail === 2) {
      isSelecting = false;
      selection.selectWordAt(
        cell.col,
        cell.row,
        (r, c) => grid[r * cols + c]?.content ?? '',
        cols,
        wordDelimiters,
      );
      await copySelectionToClipboard();
      return;
    }

    // Single click: start drag selection
    isSelecting = true;
    selection.startSelection(cell);
    selectionRange = selection.getSelection();
  }

  async function handleMousemove(event: MouseEvent) {
    if (
      (mouseReporting === 'buttonEvent' && event.buttons !== 0) ||
      mouseReporting === 'anyEvent'
    ) {
      if (!event.shiftKey) {
        const cell = pixelToCell(event);
        const motionBtn = event.buttons !== 0 ? 32 + mouseButtonCode(event) : 35;
        await sendMouseEvent(motionBtn, cell.col, cell.row, event, false);
        return;
      }
    }
    if (!isSelecting) return;
    selection.extendSelection(pixelToCell(event));
    selectionRange = selection.getSelection();
  }

  async function handleMouseup(event: MouseEvent) {
    if (mouseReporting !== 'none' && mouseReporting !== 'x10' && !event.shiftKey) {
      const cell = pixelToCell(event);
      const releaseBtn = mouseEncoding === 'sgr' ? mouseButtonCode(event) : 3;
      await sendMouseEvent(releaseBtn, cell.col, cell.row, event, true);
      return;
    }
    if (!isSelecting) return;
    isSelecting = false;
    selection.extendSelection(pixelToCell(event));
    await copySelectionToClipboard();
  }

  // -------------------------------------------------------------------------
  // Focus events (FS-VT-084, DECSET 1004)
  // -------------------------------------------------------------------------

  async function handleFocus() {
    if (!active) return;
    if (focusEventsActive) {
      await sendBytes(new TextEncoder().encode('\x1b[I'));
    }
  }

  async function handleBlur() {
    if (focusEventsActive) {
      await sendBytes(new TextEncoder().encode('\x1b[O'));
    }
  }

  // -------------------------------------------------------------------------
  // SSH reconnect (FS-SSH-040/041)
  // -------------------------------------------------------------------------

  async function handleReconnect() {
    try {
      await invoke('reconnect_ssh', { paneId });
    } catch {
      /* non-fatal */
    }
  }

  async function handleContextMenuCopy() {
    if (!selectionRange) return;
    await copySelectionToClipboard();
  }

  async function handleContextMenuPaste() {
    try {
      const text: string = await invoke('get_clipboard');
      if (text) {
        await pasteText(text);
      }
    } catch {
      /* non-fatal */
    }
  }

  /**
   * Paste text with bracketed paste support (FS-CLIP-008, SEC-BLK-012/014).
   * Delegates to pasteToBytes() which handles wrapping and sanitization.
   * When bracketed paste is inactive and text contains newlines, shows a
   * confirmation dialog first (FS-CLIP-009), unless disabled via preference.
   */
  async function pasteText(text: string) {
    const hasNewlines = text.includes('\n');
    if (!bracketedPasteActive && hasNewlines && confirmMultilinePaste) {
      pasteConfirmText = text;
      pasteConfirmOpen = true;
      return;
    }
    const encoded = pasteToBytes(text, bracketedPasteActive);
    if (encoded) await sendBytes(encoded);
  }

  /** Confirmed paste from the FS-CLIP-009 dialog. */
  async function handlePasteConfirm() {
    const text = pasteConfirmText;
    pasteConfirmOpen = false;
    pasteConfirmText = '';
    const encoded = pasteToBytes(text, bracketedPasteActive);
    if (encoded) await sendBytes(encoded);
  }

  function handlePasteCancel() {
    pasteConfirmOpen = false;
    pasteConfirmText = '';
  }

  // -------------------------------------------------------------------------
  // Scrollbar interaction (FS-SB-007)
  // -------------------------------------------------------------------------

  /**
   * Convert a pointer Y position on the scrollbar track to a scroll offset.
   * The scrollbar thumb represents the visible viewport within total content height.
   */
  function scrollbarYToOffset(clientY: number): number {
    if (!scrollbarEl) return scrollOffset;
    const rect = scrollbarEl.getBoundingClientRect();
    const fraction = Math.max(0, Math.min(1, (clientY - rect.top) / rect.height));
    // fraction 0 = top of scrollback, fraction 1 = bottom (offset 0)
    // scrollbackLines = total lines above current view
    const totalLines = rows + scrollbackLines;
    const targetLine = Math.round(fraction * totalLines);
    // Convert to offset: offset is lines above visible area (0 = at bottom)
    return Math.max(0, Math.min(scrollbackLines, scrollbackLines - targetLine + rows));
  }

  async function scrollToOffset(offset: number) {
    const clamped = Math.max(0, Math.min(scrollbackLines, offset));
    try {
      await invoke('scroll_pane', { paneId, offset: clamped });
    } catch {
      /* non-fatal */
    }
  }

  function handleScrollbarPointerdown(event: PointerEvent) {
    event.preventDefault();
    event.stopPropagation();
    // Capture pointer to receive events outside the element
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);

    const thumbEl = (event.currentTarget as HTMLElement).querySelector(
      '.terminal-pane__scrollbar-thumb',
    ) as HTMLElement | null;
    const thumbRect = thumbEl?.getBoundingClientRect();

    // Determine if click landed on the thumb or on the track
    if (thumbRect && event.clientY >= thumbRect.top && event.clientY <= thumbRect.bottom) {
      // Click on thumb: start drag
      scrollbarDragging = true;
      scrollbarDragStartY = event.clientY;
      scrollbarDragStartOffset = scrollOffset;
    } else {
      // Click on track: jump to position
      const newOffset = scrollbarYToOffset(event.clientY);
      scrollToOffset(newOffset);
    }
  }

  function handleScrollbarPointermove(event: PointerEvent) {
    if (!scrollbarDragging) return;
    event.preventDefault();
    const deltaY = event.clientY - scrollbarDragStartY;
    if (!scrollbarEl) return;
    const trackHeight = scrollbarEl.getBoundingClientRect().height;
    const totalLines = rows + scrollbackLines;
    // Delta in lines: proportional to track height
    const deltaLines = Math.round((deltaY / trackHeight) * totalLines);
    // Dragging down = scrolling toward bottom = reducing offset
    const newOffset = Math.max(0, Math.min(scrollbackLines, scrollbarDragStartOffset - deltaLines));
    scrollToOffset(newOffset);
  }

  function handleScrollbarPointerup(event: PointerEvent) {
    if (!scrollbarDragging) return;
    event.preventDefault();
    scrollbarDragging = false;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  }

  function handleScrollbarWheel(event: WheelEvent) {
    event.preventDefault();
    event.stopPropagation();
    const newOffset = Math.max(0, scrollOffset + (event.deltaY > 0 ? -3 : 3));
    scrollToOffset(newOffset);
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
  data-active={active ? 'true' : undefined}
  role="region"
  aria-label={m.terminal_pane_aria_label()}
>
  <!-- ContextMenu wraps the viewport so right-click opens it -->
  <ContextMenu
    variant="terminal"
    {hasSelection}
    {canClosePane}
    oncopy={handleContextMenuCopy}
    onpaste={handleContextMenuPaste}
    {onsearch}
    {onsplitH}
    {onsplitV}
    {onclosepane}
  >
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      bind:this={viewportEl}
      class="terminal-pane__viewport terminal-grid"
      data-screen-generation={screenGeneration}
      tabindex={active ? 0 : -1}
      role="textbox"
      aria-multiline="true"
      aria-label={m.terminal_output_aria_label()}
      aria-readonly="false"
      onkeydown={handleKeydown}
      onmousedown={handleMousedown}
      onmousemove={handleMousemove}
      onmouseup={handleMouseup}
      onwheel={handleWheel}
      onfocus={handleFocus}
      onblur={handleBlur}
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
                style={cellStyle(cell)}>{cell.content === '' ? '\u00a0' : cell.content}</span
              >
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

    <!-- Scrollbar overlay — interactive (FS-SB-007, TUITC-UX-070 to 073) -->
    {#if showScrollbar && scrollbackLines > 0}
      <div
        bind:this={scrollbarEl}
        class="terminal-pane__scrollbar"
        class:terminal-pane__scrollbar--dragging={scrollbarDragging}
        aria-hidden="true"
        onpointerdown={handleScrollbarPointerdown}
        onpointermove={handleScrollbarPointermove}
        onpointerup={handleScrollbarPointerup}
        onpointerleave={handleScrollbarPointerup}
        onwheel={handleScrollbarWheel}
        onmouseenter={() => {
          scrollbarHover = true;
        }}
        onmouseleave={() => {
          scrollbarHover = false;
        }}
      >
        <div
          class="terminal-pane__scrollbar-thumb"
          class:terminal-pane__scrollbar-thumb--hover={scrollbarHover || scrollbarDragging}
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
    <ProcessTerminatedPane {exitCode} {signalName} {onrestart} onclose={onclosepane} />
  {/if}

  <!-- SSH disconnected banner — shown when SSH connection drops (FS-SSH-040/041) -->
  {#if sshState?.type === 'disconnected'}
    <div class="terminal-pane__ssh-disconnected" role="status" aria-live="polite">
      <span class="terminal-pane__ssh-disconnected-label"
        >{m.ssh_banner_disconnected({ reason: '' })}</span
      >
      <button class="terminal-pane__ssh-reconnect-btn" type="button" onclick={handleReconnect}
        >{m.ssh_reconnect()}</button
      >
    </div>
  {/if}
</div>

<!-- FS-CLIP-009: Multiline paste confirmation dialog -->
<Dialog
  open={pasteConfirmOpen}
  title={m.paste_confirm_title()}
  size="small"
  onclose={handlePasteCancel}
>
  {#snippet children()}
    <p class="text-[14px] text-(--color-text-secondary) leading-relaxed">
      {m.paste_confirm_body({ lines: pasteConfirmText.split('\n').length })}
    </p>
    <div class="mt-4">
      <Toggle
        checked={!confirmMultilinePaste}
        label={m.paste_confirm_dont_ask()}
        onchange={(v) => {
          if (v) ondisableConfirmMultilinePaste?.();
        }}
      />
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" onclick={handlePasteCancel}>{m.action_cancel()}</Button>
    <Button variant="primary" onclick={handlePasteConfirm}>{m.paste_confirm_action()}</Button>
  {/snippet}
</Dialog>

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

  /* Scrollbar overlay (FS-SB-007, TUITC-UX-070 to 073) */
  .terminal-pane__scrollbar {
    position: absolute;
    top: 0;
    right: 0;
    width: var(--size-scrollbar-width, 8px);
    height: 100%;
    z-index: var(--z-scrollbar, 15);
    cursor: pointer;
    transition: opacity var(--duration-slow, 300ms);
  }

  .terminal-pane__scrollbar--dragging {
    cursor: grabbing;
  }

  .terminal-pane__scrollbar-thumb {
    position: absolute;
    right: 0;
    width: 100%;
    min-height: 32px;
    background-color: var(--color-scrollbar-thumb);
    border-radius: var(--radius-full, 9999px);
    transition: background-color var(--duration-fast, 80ms);
    cursor: grab;
  }

  .terminal-pane__scrollbar--dragging .terminal-pane__scrollbar-thumb {
    cursor: grabbing;
  }

  .terminal-pane__scrollbar-thumb--hover {
    background-color: var(--color-scrollbar-thumb-hover);
  }

  @media (prefers-reduced-motion: reduce) {
    .terminal-pane__scrollbar {
      transition: none;
    }
    .terminal-pane__scrollbar-thumb {
      transition: none;
    }
  }

  /* SSH disconnected banner (FS-SSH-040/041) */
  .terminal-pane__ssh-disconnected {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-3, 0.75rem);
    padding: var(--spacing-2, 0.5rem) var(--spacing-4, 1rem);
    background-color: var(--color-surface-overlay, rgba(0, 0, 0, 0.85));
    border-top: 1px solid var(--color-border-subtle, rgba(255, 255, 255, 0.1));
    z-index: var(--z-overlay, 50);
  }

  .terminal-pane__ssh-disconnected-label {
    color: var(--color-text-muted, #a0a0a0);
    font-size: var(--font-size-sm, 0.875rem);
  }

  .terminal-pane__ssh-reconnect-btn {
    padding: var(--spacing-1, 0.25rem) var(--spacing-3, 0.75rem);
    background-color: var(--color-accent, #6e9fd8);
    color: var(--color-on-accent, #000);
    border: none;
    border-radius: var(--radius-sm, 4px);
    font-size: var(--font-size-sm, 0.875rem);
    cursor: pointer;
    min-height: 44px;
    min-width: 44px;
  }

  .terminal-pane__ssh-reconnect-btn:hover {
    background-color: var(--color-accent-hover, #5a8bc4);
  }

  .terminal-pane__ssh-reconnect-btn:focus-visible {
    outline: 2px solid var(--color-focus-ring, #6e9fd8);
    outline-offset: 2px;
  }
</style>
