// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalPane composable — reactive logic extracted from TerminalPane.svelte.
 *
 * Manages:
 *   - IPC event subscription lifecycle (screen-update, scroll, mode state,
 *     cursor style, bell, notification)
 *   - ScreenGrid state (cell grid, cursor)
 *   - Scroll state (offset, scrollback lines, scrollbar derived values)
 *   - Terminal mode state (DECCKM, mouse reporting, bracketed paste, etc.)
 *   - Cursor blink timer (via useCursorBlink)
 *   - Border pulse state (via useVisualFx)
 *   - Selection flash state (via useVisualFx)
 *   - Bell flash state (via useVisualFx)
 *   - Paste confirmation dialog state (via usePasteDialog)
 *   - Scrollbar interaction state (via useScrollbarState)
 *   - Resize debounce (via useTerminalResize)
 *
 * Returns a reactive object used by TerminalPane.svelte.
 * The template and DOM event handlers remain in the .svelte file.
 *
 * Props are passed in at call time so the composable can filter events by paneId.
 */

import { onMount, onDestroy } from 'svelte';
import {
  onScreenUpdate,
  onScrollPositionChanged,
  onModeStateChanged,
  onCursorStyleChanged,
  onBellTriggered,
  onNotificationChanged,
} from '$lib/ipc/events';
import {
  getPaneScreenSnapshot,
  sendInput,
  scrollPane,
  copyToClipboard,
  openUrl,
  setActivePane,
  getClipboard,
  reconnectSsh,
} from '$lib/ipc/commands';
import { buildGridFromSnapshot, applyUpdates } from '$lib/terminal/screen.js';
import { cursorShape, cursorBlinks } from '$lib/terminal/color.js';
import { SelectionManager } from '$lib/terminal/selection.js';
import { pasteToBytes } from '$lib/terminal/paste.js';
import type { PaneId, CursorState, BellType, SearchMatch, ScreenUpdateEvent } from '$lib/ipc/types';
import type { CellStyle } from '$lib/terminal/screen.js';
import {
  defaultCell,
  buildFullGridRows,
  pixelToCell as pixelToCellPure,
  mouseButtonCode,
  cellStyle,
} from './useTerminalPane.handlers.js';
import {
  scrollbarThumbHeightPct as calcScrollbarThumbHeightPct,
  scrollbarThumbTopPct as calcScrollbarThumbTopPct,
  scrollbarYToOffset as scrollbarYToOffsetPure,
} from './useTerminalPane.scrollbar.js';
import { useCursorBlink } from './useTerminalPane.cursor-blink.svelte.js';
import { useVisualFx } from './useTerminalPane.visual-fx.svelte.js';
import { useScrollbarState } from './useTerminalPane.scrollbar-state.svelte.js';
import { usePasteDialog } from './useTerminalPane.paste-dialog.svelte.js';
import { useTerminalResize } from './useTerminalPane.resize.svelte.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Props passed as getter functions to preserve Svelte 5 reactivity across
 * the composable boundary. Each getter re-reads the current prop value so
 * $derived and $effect inside the composable stay up-to-date.
 */
export interface TerminalPaneComposableProps {
  paneId: () => PaneId;
  active: () => boolean;
  wordDelimiters: () => string;
  confirmMultilinePaste: () => boolean;
  cursorBlinkMs: () => number;
  bellType: () => BellType;
  searchMatches: () => SearchMatch[];
  activeSearchMatchIndex: () => number;
  ondimensionschange: () => ((cols: number, rows: number) => void) | undefined;
  ondisableConfirmMultilinePaste: () => (() => void) | undefined;
  /** CSS font-family for terminal text — used by Canvas cell measurement (F8). */
  fontFamily?: () => string | undefined;
  /** Font size in pixels — used by Canvas cell measurement (F8). */
  fontSize?: () => number | undefined;
  /** Line height multiplier — used by Canvas cell measurement (F8). */
  lineHeight?: () => number | undefined;
}

// ---------------------------------------------------------------------------
// Composable
// ---------------------------------------------------------------------------

export function useTerminalPane(props: TerminalPaneComposableProps) {
  // -------------------------------------------------------------------------
  // Terminal grid and cursor state
  // -------------------------------------------------------------------------

  let cols = $state(80);
  let rows = $state(24);
  let grid = $state<CellStyle[]>([]);
  let cursor = $state<CursorState>({ row: 0, col: 0, visible: true, shape: 0, blink: true });
  let scrollOffset = $state(0);
  let scrollbackLines = $state(0);
  let screenGeneration = $state(0);

  // rAF coalescing state for wheel scroll events.
  let pendingScrollOffset: number | null = null;
  let scrollRafId: ReturnType<typeof requestAnimationFrame> | null = null;

  // -------------------------------------------------------------------------
  // Terminal mode state
  // -------------------------------------------------------------------------

  let decckm = $state(false);
  let deckpam = $state(false);
  let mouseReporting = $state<'none' | 'x10' | 'normal' | 'buttonEvent' | 'anyEvent'>('none');
  let mouseEncoding = $state<'x10' | 'sgr' | 'urxvt'>('x10');
  let focusEventsActive = $state(false);
  let bracketedPasteActive = $state(false);

  // -------------------------------------------------------------------------
  // Selection
  // -------------------------------------------------------------------------

  const selection = new SelectionManager();
  let selectionRange = $state(selection.getSelection());
  let isSelecting = $state(false);
  let hasSelection = $state(false);

  // -------------------------------------------------------------------------
  // DOM refs (set by template)
  // -------------------------------------------------------------------------

  let viewportEl = $state<HTMLDivElement | undefined>();
  let scrollbarEl = $state<HTMLDivElement | undefined>();

  // -------------------------------------------------------------------------
  // Sub-composables
  // -------------------------------------------------------------------------

  const currentCursorShape = $derived(cursorShape(cursor.shape));
  const currentCursorBlinks = $derived(
    props.active() && cursor.blink && cursorBlinks(cursor.shape),
  );

  const cursorBlink = useCursorBlink({
    cursorBlinkMs: props.cursorBlinkMs,
    currentCursorBlinks: () => currentCursorBlinks,
  });

  const visualFx = useVisualFx({
    bellType: props.bellType,
    isActive: props.active,
  });

  const scrollbarState = useScrollbarState();

  const pasteDialog = usePasteDialog();

  const resize = useTerminalResize({
    paneId: props.paneId,
    viewportEl: () => viewportEl,
    getCols: () => cols,
    getRows: () => rows,
    ondimensionschange: props.ondimensionschange,
    fontFamily: props.fontFamily,
    fontSize: props.fontSize,
    lineHeight: props.lineHeight,
  });

  // -------------------------------------------------------------------------
  // Derived
  // -------------------------------------------------------------------------

  const showScrollbar = $derived(
    scrollbarState.scrollbarVisible ||
      scrollOffset > 0 ||
      scrollbarState.scrollbarHover ||
      scrollbarState.scrollbarDragging,
  );

  const scrollbarThumbHeightPct = $derived(calcScrollbarThumbHeightPct(rows, scrollbackLines));

  const scrollbarThumbTopPct = $derived(
    calcScrollbarThumbTopPct(rows, scrollbackLines, scrollOffset, scrollbarThumbHeightPct),
  );

  // WP3c: gridRows as $state updated differentially, not $derived rebuilding the entire 2D array.
  // Svelte 5 Proxy arrays: gridRows[r] = newRow invalidates only consumers reading gridRows[r],
  // not the full {#each} block. Full rebuild only on full_redraw or dimension change.
  let gridRows = $state<CellStyle[][]>([]);

  // defaultCell and buildFullGridRows are imported from useTerminalPane.handlers.ts.
  // Local wrappers bind the `grid` closure so callers don't need to pass it.
  function buildFullGridRowsBound(r: number, c: number): CellStyle[][] {
    return buildFullGridRows(r, c, grid);
  }

  const searchMatchSet = $derived.by(() => {
    const set = new Set<number>();
    if (props.searchMatches().length === 0) return set;
    const screenStart = scrollbackLines - scrollOffset;
    for (let i = 0; i < props.searchMatches().length; i++) {
      const match = props.searchMatches()[i];
      const screenRow = match.scrollbackRow - screenStart;
      if (screenRow < 0 || screenRow >= rows) continue;
      for (let c = match.colStart; c < match.colEnd; c++) {
        set.add(screenRow * cols + c);
      }
    }
    return set;
  });

  const activeSearchMatchSet = $derived.by(() => {
    const set = new Set<number>();
    if (props.searchMatches().length === 0 || props.activeSearchMatchIndex() <= 0) return set;
    const match = props.searchMatches()[props.activeSearchMatchIndex() - 1];
    if (!match) return set;
    const screenStart = scrollbackLines - scrollOffset;
    const screenRow = match.scrollbackRow - screenStart;
    if (screenRow < 0 || screenRow >= rows) return set;
    for (let c = match.colStart; c < match.colEnd; c++) {
      set.add(screenRow * cols + c);
    }
    return set;
  });

  // -------------------------------------------------------------------------
  // Effects
  // -------------------------------------------------------------------------

  // Auto-focus when active
  $effect(() => {
    if (props.active() && !false /* not terminated — caller checks */) {
      viewportEl?.focus({ preventScroll: true });
    }
  });

  // -------------------------------------------------------------------------
  // Mount / destroy
  // -------------------------------------------------------------------------

  let unlistens: Array<() => void> = [];

  // WP3c: Apply a screen update to the flat grid and update gridRows differentially.
  // Full rebuild on full_redraw or dimension mismatch; row-level rebuild otherwise.
  // The event's cols/rows are authoritative — they reflect the grid dimensions at
  // the time the backend produced this event, eliminating stride mismatch races.
  function applyScreenUpdate(update: ScreenUpdateEvent): void {
    const expectedGridSize = update.rows * update.cols;

    // FS-SB-009: live PTY event while locally scrolled → freeze viewport, only update scrollback count.
    // A live PTY event has scrollOffset === 0 and is NOT a full-redraw triggered by scroll_pane.
    const isLivePtyEvent = update.scrollOffset === 0 && !update.isFullRedraw;
    if (isLivePtyEvent && scrollOffset > 0) {
      if (typeof update.scrollbackLines === 'number') {
        scrollbackLines = update.scrollbackLines;
      }
      return; // gridRows unchanged, screenGeneration not incremented
    }

    // Sync local scroll state from the event payload.
    // For live PTY events while not scrolled: update.scrollOffset === 0 → correct.
    // For scroll-triggered viewports: update.scrollOffset === k → sets local state.
    scrollOffset = update.scrollOffset;

    // Ensure the flat grid matches the event dimensions before applying updates.
    // A terminal resize changes rows×cols, making the old grid too small: cells
    // beyond grid.length would be silently dropped by applyUpdates (oob skip),
    // and buildFullGridRows would read undefined → defaultCell() for those positions.
    // Recreate the grid when dimensions change so every cell index is valid.
    if (grid.length !== expectedGridSize) {
      grid = Array.from({ length: expectedGridSize }, () => defaultCell());
    }

    applyUpdates(grid, update.cells, update.cols);
    cursor = update.cursor;
    if (typeof update.scrollbackLines === 'number') {
      scrollbackLines = update.scrollbackLines;
    }

    if (update.isFullRedraw || gridRows.length !== update.rows) {
      // Full rebuild: dimension change or explicit full repaint.
      gridRows = buildFullGridRowsBound(update.rows, update.cols);
    } else {
      // Differential: rebuild only rows that have changed cells.
      const dirtyRows = new Set(update.cells.map((c) => c.row));
      for (const r of dirtyRows) {
        if (r >= 0 && r < update.rows) {
          gridRows[r] = Array.from(
            { length: update.cols },
            (_, c) => grid[r * update.cols + c] ?? defaultCell(),
          );
        }
      }
    }

    // Sync local cols/rows from the event — canonical source of truth.
    if (cols !== update.cols || rows !== update.rows) {
      cols = update.cols;
      rows = update.rows;
      props.ondimensionschange()?.(update.cols, update.rows);
    }

    screenGeneration++;
  }

  onMount(async () => {
    // WP3b: Register screen-update listener BEFORE the snapshot IPC call so that
    // updates emitted during the fetch are buffered and replayed after the snapshot.
    const pendingUpdates: ScreenUpdateEvent[] = [];
    let buffering = true;

    unlistens.push(
      await onScreenUpdate((update) => {
        if (update.paneId !== props.paneId()) return;

        if (buffering) {
          pendingUpdates.push(update);
          return;
        }
        applyScreenUpdate(update);
      }),
    );

    // Fetch the initial screen snapshot.
    try {
      const snapshot = await getPaneScreenSnapshot(props.paneId());

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
      // Backend not ready — populated by first screen-update event.
    }

    // Replay updates buffered during the snapshot fetch.
    buffering = false;
    for (const update of pendingUpdates) {
      applyScreenUpdate(update);
    }

    // Initialize gridRows from the snapshot (+ any replayed updates).
    if (gridRows.length !== rows) {
      gridRows = buildFullGridRowsBound(rows, cols);
    }

    unlistens.push(
      await onScrollPositionChanged((pos) => {
        if (pos.paneId !== props.paneId()) return;
        scrollOffset = pos.offset;
        scrollbackLines = pos.scrollbackLines;
        scrollbarState.showScrollbar(pos.offset === 0);
      }),
    );

    unlistens.push(
      await onModeStateChanged((mode) => {
        if (mode.paneId !== props.paneId()) return;
        decckm = mode.decckm;
        deckpam = mode.deckpam;
        mouseReporting = mode.mouseReporting;
        mouseEncoding = mode.mouseEncoding;
        focusEventsActive = mode.focusEvents;
        bracketedPasteActive = mode.bracketedPaste;
      }),
    );

    unlistens.push(
      await onCursorStyleChanged((ev) => {
        if (ev.paneId !== props.paneId()) return;
        cursor = { ...cursor, shape: ev.shape, blink: cursorBlinks(ev.shape) };
        // The blink $effect re-runs automatically when cursor.blink changes,
        // restarting the cycle from scratch with the new mode.
      }),
    );

    unlistens.push(
      await onBellTriggered((ev) => {
        if (ev.paneId !== props.paneId()) return;
        visualFx.handleBell();
      }),
    );

    unlistens.push(
      await onNotificationChanged((ev) => {
        if (ev.paneId !== props.paneId()) return;
        if (props.active()) return;
        visualFx.handleNotificationForBorderPulse(ev);
      }),
    );

    if (viewportEl) {
      resize.startObserving(viewportEl);
    }
  });

  onDestroy(() => {
    for (const unlisten of unlistens) unlisten();
    unlistens = [];
    visualFx.cleanup();
    resize.cleanup();
    scrollbarState.cleanup();
    if (scrollRafId !== null) {
      cancelAnimationFrame(scrollRafId);
      scrollRafId = null;
    }
  });

  // -------------------------------------------------------------------------
  // Keyboard input
  // -------------------------------------------------------------------------

  async function sendBytes(bytes: Uint8Array) {
    const data = Array.from(bytes);
    try {
      await sendInput(props.paneId(), data);
    } catch {
      // PTY may have closed
    }
  }

  // -------------------------------------------------------------------------
  // Scroll-to-bottom
  // -------------------------------------------------------------------------

  async function handleScrollToBottom() {
    try {
      await scrollPane(props.paneId(), 0);
    } catch {
      /* non-fatal */
    }
  }

  // -------------------------------------------------------------------------
  // Mouse helpers
  // -------------------------------------------------------------------------

  // pixelToCell is imported from useTerminalPane.handlers.ts.
  // This wrapper binds the composable's viewportEl, cols, and rows.
  function pixelToCell(event: MouseEvent): { row: number; col: number } {
    return pixelToCellPure(event, viewportEl, cols, rows);
  }

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

  // mouseButtonCode is imported from useTerminalPane.handlers.ts — used directly.

  // -------------------------------------------------------------------------
  // Selection
  // -------------------------------------------------------------------------

  async function copySelectionToClipboard() {
    const sel = selection.getSelection();
    if (sel) {
      const text = selection.getSelectedText((r, c) => grid[r * cols + c]?.content ?? '', cols);
      hasSelection = text.length > 0;
      if (hasSelection) {
        try {
          await copyToClipboard(text);
          visualFx.triggerSelectionFlash();
        } catch {
          /* non-fatal */
        }
      }
    } else {
      hasSelection = false;
    }
    selectionRange = sel;
  }

  function isSelected(row: number, col: number): boolean {
    if (!selectionRange) return false;
    const { start, end } = selectionRange;
    if (row < start.row || row > end.row) return false;
    if (row === start.row && col < start.col) return false;
    if (row === end.row && col > end.col) return false;
    return true;
  }

  // -------------------------------------------------------------------------
  // Mouse event handlers (to be bound in template)
  // -------------------------------------------------------------------------

  async function handleMousedown(event: MouseEvent) {
    if (!props.active()) {
      try {
        await setActivePane(props.paneId());
      } catch {
        /* non-fatal */
      }
    }
    if (event.button !== 0) return;

    const cell = pixelToCell(event);

    if (event.ctrlKey && event.detail === 1) {
      const hyperlink = grid[cell.row * cols + cell.col]?.hyperlink;
      if (hyperlink) {
        try {
          await openUrl(hyperlink, props.paneId());
        } catch {
          /* non-fatal */
        }
        return;
      }
    }

    if (mouseReporting !== 'none' && !event.shiftKey) {
      await sendMouseEvent(mouseButtonCode(event), cell.col, cell.row, event, false);
      return;
    }

    if (event.detail >= 3) {
      isSelecting = false;
      selection.selectLineAt(cell.row, cols);
      await copySelectionToClipboard();
      return;
    }

    if (event.detail === 2) {
      isSelecting = false;
      selection.selectWordAt(
        cell.col,
        cell.row,
        (r, c) => grid[r * cols + c]?.content ?? '',
        cols,
        props.wordDelimiters(),
      );
      await copySelectionToClipboard();
      return;
    }

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

  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    if (!event.shiftKey && mouseReporting !== 'none') {
      const button = event.deltaY < 0 ? 64 : 65;
      const cell = pixelToCell(event);
      sendMouseEvent(button, cell.col, cell.row, event, false);
      return;
    }
    const delta = event.deltaY > 0 ? -3 : 3;
    // Accumulate on pendingScrollOffset (not stale scrollOffset) so rapid
    // events increment correctly: 3, 6, 9, … instead of all landing on 3.
    const base = pendingScrollOffset ?? scrollOffset;
    pendingScrollOffset = Math.max(0, base + delta);

    if (scrollRafId === null) {
      scrollRafId = requestAnimationFrame(() => {
        scrollRafId = null;
        const target = pendingScrollOffset;
        pendingScrollOffset = null;
        if (target !== null) {
          // Speculative update: visual feedback is immediate.
          // applyScreenUpdate will correct it if the backend clamps.
          scrollOffset = target;
          scrollPane(props.paneId(), target).catch(() => {});
        }
      });
    }
  }

  // -------------------------------------------------------------------------
  // Focus events
  // -------------------------------------------------------------------------

  async function handleFocus() {
    if (!props.active()) return;
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
  // Context menu helpers
  // -------------------------------------------------------------------------

  async function handleContextMenuCopy() {
    if (!selectionRange) return;
    await copySelectionToClipboard();
  }

  async function handleContextMenuPaste() {
    try {
      const text: string = await getClipboard();
      if (text) {
        await pasteText(text);
      }
    } catch {
      /* non-fatal */
    }
  }

  // -------------------------------------------------------------------------
  // Paste with multiline confirmation
  // -------------------------------------------------------------------------

  async function pasteText(text: string) {
    const hasNewlines = text.includes('\n');
    if (!bracketedPasteActive && hasNewlines && props.confirmMultilinePaste()) {
      pasteDialog.openPasteConfirm(text);
      return;
    }
    const encoded = pasteToBytes(text, bracketedPasteActive);
    if (encoded) await sendBytes(encoded);
  }

  async function handlePasteConfirm() {
    const text = pasteDialog.pasteConfirmText;
    pasteDialog.closePasteConfirm();
    const encoded = pasteToBytes(text, bracketedPasteActive);
    if (encoded) await sendBytes(encoded);
  }

  function handlePasteCancel() {
    pasteDialog.closePasteConfirm();
  }

  // -------------------------------------------------------------------------
  // Scrollbar interaction
  // -------------------------------------------------------------------------

  // scrollbarYToOffset is imported from useTerminalPane.scrollbar.ts.
  // This wrapper binds the composable's scrollbarEl, scrollbackLines, rows, scrollOffset.
  function scrollbarYToOffset(clientY: number): number {
    return scrollbarYToOffsetPure(clientY, scrollbarEl, scrollbackLines, rows, scrollOffset);
  }

  async function scrollToOffset(offset: number) {
    const clamped = Math.max(0, Math.min(scrollbackLines, offset));
    try {
      await scrollPane(props.paneId(), clamped);
    } catch {
      /* non-fatal */
    }
  }

  function handleScrollbarPointerdown(event: PointerEvent) {
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);

    const thumbEl = (event.currentTarget as HTMLElement).querySelector(
      '.terminal-pane__scrollbar-thumb',
    ) as HTMLElement | null;
    const thumbRect = thumbEl?.getBoundingClientRect();

    if (thumbRect && event.clientY >= thumbRect.top && event.clientY <= thumbRect.bottom) {
      scrollbarState.startDrag(event.clientY, scrollOffset);
    } else {
      scrollToOffset(scrollbarYToOffset(event.clientY));
    }
  }

  function handleScrollbarPointermove(event: PointerEvent) {
    if (!scrollbarState.scrollbarDragging) return;
    event.preventDefault();
    const deltaY = event.clientY - scrollbarState.getDragStartY();
    if (!scrollbarEl) return;
    const trackHeight = scrollbarEl.getBoundingClientRect().height;
    const totalLines = rows + scrollbackLines;
    const deltaLines = Math.round((deltaY / trackHeight) * totalLines);
    const newOffset = Math.max(
      0,
      Math.min(scrollbackLines, scrollbarState.getDragStartOffset() - deltaLines),
    );
    scrollToOffset(newOffset);
  }

  function handleScrollbarPointerup(event: PointerEvent) {
    if (!scrollbarState.scrollbarDragging) return;
    event.preventDefault();
    scrollbarState.endDrag();
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  }

  function handleScrollbarWheel(event: WheelEvent) {
    event.preventDefault();
    event.stopPropagation();
    const newOffset = Math.max(0, scrollOffset + (event.deltaY > 0 ? -3 : 3));
    scrollToOffset(newOffset);
  }

  // -------------------------------------------------------------------------
  // SSH reconnect
  // -------------------------------------------------------------------------

  async function handleReconnect() {
    try {
      await reconnectSsh(props.paneId());
    } catch {
      /* non-fatal */
    }
  }

  // -------------------------------------------------------------------------
  // Return all state and handlers for TerminalPane.svelte
  // -------------------------------------------------------------------------

  return {
    // DOM refs (settable from template)
    get viewportEl() {
      return viewportEl;
    },
    set viewportEl(v: HTMLDivElement | undefined) {
      viewportEl = v;
    },
    get scrollbarEl() {
      return scrollbarEl;
    },
    set scrollbarEl(v: HTMLDivElement | undefined) {
      scrollbarEl = v;
    },

    // Terminal state
    get cols() {
      return cols;
    },
    get rows() {
      return rows;
    },
    get cursor() {
      return cursor;
    },
    get cursorVisible() {
      return cursorBlink.cursorVisible;
    },
    get scrollOffset() {
      return scrollOffset;
    },
    get scrollbackLines() {
      return scrollbackLines;
    },
    get screenGeneration() {
      return screenGeneration;
    },
    get decckm() {
      return decckm;
    },
    get deckpam() {
      return deckpam;
    },
    get mouseReporting() {
      return mouseReporting;
    },
    get mouseEncoding() {
      return mouseEncoding;
    },
    get focusEventsActive() {
      return focusEventsActive;
    },
    get bracketedPasteActive() {
      return bracketedPasteActive;
    },

    // Derived
    get gridRows() {
      return gridRows;
    },
    get searchMatchSet() {
      return searchMatchSet;
    },
    get activeSearchMatchSet() {
      return activeSearchMatchSet;
    },
    get currentCursorShape() {
      return currentCursorShape;
    },
    get currentCursorBlinks() {
      return currentCursorBlinks;
    },
    get showScrollbar() {
      return showScrollbar;
    },
    get scrollbarThumbHeightPct() {
      return scrollbarThumbHeightPct;
    },
    get scrollbarThumbTopPct() {
      return scrollbarThumbTopPct;
    },

    // Selection
    get selectionRange() {
      return selectionRange;
    },
    get isSelecting() {
      return isSelecting;
    },
    get hasSelection() {
      return hasSelection;
    },
    get selectionFlashing() {
      return visualFx.selectionFlashing;
    },

    // Scrollbar
    get scrollbarVisible() {
      return scrollbarState.scrollbarVisible;
    },
    get scrollbarDragging() {
      return scrollbarState.scrollbarDragging;
    },
    get scrollbarHover() {
      return scrollbarState.scrollbarHover;
    },
    set scrollbarHover(v: boolean) {
      scrollbarState.scrollbarHover = v;
    },

    // Visual states
    get bellFlashing() {
      return visualFx.bellFlashing;
    },
    get borderPulse() {
      return visualFx.borderPulse;
    },

    // Paste dialog
    get pasteConfirmOpen() {
      return pasteDialog.pasteConfirmOpen;
    },
    get pasteConfirmText() {
      return pasteDialog.pasteConfirmText;
    },

    // Methods
    sendBytes,
    stopCursorBlink: cursorBlink.stopCursorBlink,
    restartCursorBlink: cursorBlink.restartCursorBlink,
    pasteText,
    handleScrollToBottom,
    handleMousedown,
    handleMousemove,
    handleMouseup,
    handleWheel,
    handleFocus,
    handleBlur,
    handleContextMenuCopy,
    handleContextMenuPaste,
    handlePasteConfirm,
    handlePasteCancel,
    handleScrollbarPointerdown,
    handleScrollbarPointermove,
    handleScrollbarPointerup,
    handleScrollbarWheel,
    handleReconnect,
    isSelected,
    cellStyle,
  };
}
