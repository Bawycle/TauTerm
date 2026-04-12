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

import { onMount, onDestroy, untrack } from 'svelte';
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
// Module-level singletons
// ---------------------------------------------------------------------------

/**
 * Shared TextEncoder instance — allocated once at module load.
 * Avoids per-event allocation in sendMouseEvent/handleFocus/handleBlur
 * (sendMouseEvent can fire at 60+ fps in anyEvent mouse mode).
 */
const encoder = new TextEncoder();

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

  // P-OPT-1: rAF batching state for screen-update events.
  // Declared at composable scope so onDestroy can cancel a pending frame.
  //
  // rafScheduled guards scheduleRaf() idempotency and is set BEFORE calling
  // requestAnimationFrame(). This separates the "is a frame already pending?"
  // check from the frame handle (rafId), avoiding a race where a synchronous
  // rAF execution (e.g. in unit tests) clears rafId before the assignment
  // `rafId = requestAnimationFrame(...)` completes.
  const rafQueue: ScreenUpdateEvent[] = [];
  let rafId: number | null = null;
  let rafScheduled = false;

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

  // Selection coordinates are stored in buffer-absolute rows (0 = oldest scrollback
  // line, scrollbackLines = first live screen row). This makes selections stable
  // across scroll changes — the same pattern used by search match highlighting.
  //
  // Conversion: screenRow = bufferRow - (scrollbackLines - scrollOffset)
  //             bufferRow = screenRow + (scrollbackLines - scrollOffset)
  function screenToBufferRow(screenRow: number): number {
    return screenRow + (scrollbackLines - scrollOffset);
  }

  function bufferToScreenRow(bufferRow: number): number {
    return bufferRow - (scrollbackLines - scrollOffset);
  }

  // Returns a GetCellFn that accepts buffer-absolute row indices and maps them
  // to the current visible grid slice. Rows outside the visible window return ''.
  function makeBufferGetCell(): (r: number, c: number) => string {
    const screenStart = scrollbackLines - scrollOffset;
    return (r, c) => {
      const screenRow = r - screenStart;
      if (screenRow < 0 || screenRow >= rows) return '';
      return grid[screenRow * cols + c]?.content ?? '';
    };
  }

  // -------------------------------------------------------------------------
  // DOM refs (set by template)
  // -------------------------------------------------------------------------

  let viewportEl = $state<HTMLDivElement | undefined>();
  /** Hidden textarea — true keyboard focus/input sink. Receives GTK IM commits. */
  let inputEl = $state<HTMLTextAreaElement | undefined>();
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

  // FS-SEARCH-006: scroll to center the active search match in the viewport.
  $effect(() => {
    const idx = props.activeSearchMatchIndex();
    const matches = props.searchMatches();
    if (idx <= 0 || matches.length === 0) return;
    const match = matches[idx - 1];
    if (!match) return;
    // Read layout values without tracking as reactive deps to avoid feedback loop.
    // Guard: scrollbackLines=0 means the snapshot hasn't arrived yet — skip to
    // avoid a spurious scroll-to-bottom before the pane is fully initialized.
    const lines = untrack(() => scrollbackLines);
    if (lines === 0) return;
    const centerRow = Math.floor(untrack(() => rows) / 2);
    const targetOffset = lines - match.scrollbackRow + centerRow;
    scrollToOffset(targetOffset);
  });

  // -------------------------------------------------------------------------
  // Effects
  // -------------------------------------------------------------------------

  // Auto-focus when active — prefer inputEl (hidden textarea, receives GTK IM commits).
  // Falls back to viewportEl if textarea is not yet mounted.
  // Guard: skip when a modal dialog is open — stealing focus would freeze the dialog's
  // scrollable area by triggering FocusScope's focus-restoration loop.
  $effect(() => {
    if (props.active() && !document.querySelector('[role="dialog"][aria-modal="true"]')) {
      (inputEl ?? viewportEl)?.focus({ preventScroll: true });
    }
  });

  // Re-focus when the OS window gains native GTK focus. The $effect above fires
  // after Svelte's DOM commit, but the WebKitGTK window may not yet have GTK-level
  // focus at that point — focus() is then a no-op. This listener ensures the
  // terminal input is focused once the window manager actually delivers focus.
  // Also handles the user switching back from another application.
  $effect(() => {
    if (!props.active()) return;
    function onWindowFocus() {
      if (!document.querySelector('[role="dialog"][aria-modal="true"]')) {
        (inputEl ?? viewportEl)?.focus({ preventScroll: true });
      }
    }
    window.addEventListener('focus', onWindowFocus);
    return () => window.removeEventListener('focus', onWindowFocus);
  });

  // Dev-only frame timing (P12a + P-DIAG-1).
  // import.meta.env.DEV is replaced by `false` in production builds by Vite;
  // Rollup tree-shakes the entire body — zero production overhead.
  //
  // $effect runs after Svelte has committed all DOM mutations triggered by the
  // reactive writes in applyScreenUpdate() (the last of which is screenGeneration++).
  //
  // Measures emitted per frame:
  //   tauterm:frameRender  — full wall-clock: asu:start → render:end
  //   tauterm:applyOnly    — pure JS cost:    asu:start → apply:end (before screenGeneration++)
  //   tauterm:repaintTime  — Svelte reconcile + WebKitGTK repaint: apply:end → render:end
  //
  // try-catch: the initial $effect run fires before the first applyScreenUpdate()
  // call, so the start marks do not exist yet — measure would throw.
  $effect(() => {
    if (!(import.meta.env.DEV || import.meta.env.VITE_PERF_INSTRUMENTATION === '1')) return;
    const _gen = screenGeneration; // subscribe to every generation increment
    void _gen;
    try {
      performance.mark('tauterm:render:end');
      performance.measure('tauterm:frameRender', 'tauterm:asu:start', 'tauterm:render:end');
      performance.measure('tauterm:applyOnly', 'tauterm:asu:start', 'tauterm:apply:end');
      performance.measure('tauterm:repaintTime', 'tauterm:apply:end', 'tauterm:render:end');
    } catch {
      // Start marks not yet set (initial run before first screen update). No-op.
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
    if (import.meta.env.DEV || import.meta.env.VITE_PERF_INSTRUMENTATION === '1')
      performance.mark('tauterm:asu:start');
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
      // P12a: cell-level mutation. Write individual gridRows[r][c] instead of
      // replacing the entire row array. Svelte 5's deep proxy tracks element-level
      // writes, so only the spans for changed cells are re-evaluated by the reconciler
      // (instead of all 220+ cells in every dirty row).
      //
      // Pre-condition: gridRows[r] already exists with length === update.cols.
      // Guaranteed by the full-rebuild path above (isFullRedraw or row count change).
      // The backend sends isFullRedraw: true on all resize events.
      for (const cellUpdate of update.cells) {
        const r = cellUpdate.row;
        const c = cellUpdate.col;
        if (r >= 0 && r < update.rows && c >= 0 && c < update.cols) {
          gridRows[r][c] = grid[r * update.cols + c] ?? defaultCell();
        }
      }
    }

    // Sync local cols/rows from the event — canonical source of truth.
    if (cols !== update.cols || rows !== update.rows) {
      cols = update.cols;
      rows = update.rows;
      props.ondimensionschange()?.(update.cols, update.rows);
      // Invalidate selection: buffer-absolute row indices assume a fixed scrollback
      // layout. After a resize the backend reflows the scrollback, making old
      // buffer rows point to wrong content.
      selection.clearSelection();
      selectionRange = null;
      isSelecting = false;
      hasSelection = false;
    }

    // P-DIAG-1: mark end of pure-JS work (before Svelte schedules DOM mutations).
    // tauterm:applyOnly = asu:start → apply:end = applyUpdates + proxy writes.
    // tauterm:repaintTime = apply:end → render:end ≈ Svelte reconcile + WebKitGTK repaint.
    if (import.meta.env.DEV || import.meta.env.VITE_PERF_INSTRUMENTATION === '1')
      performance.mark('tauterm:apply:end');
    screenGeneration++;
  }

  onMount(async () => {
    // WP3b: Register screen-update listener BEFORE the snapshot IPC call so that
    // updates emitted during the fetch are buffered and replayed after the snapshot.
    const pendingUpdates: ScreenUpdateEvent[] = [];
    let buffering = true;

    // P-OPT-1: rAF batching — coalesce screen-update events arriving between two
    // browser paint frames into a single requestAnimationFrame callback. This
    // reduces WebKitGTK repaint count from N events/frame to 1.
    //
    // All events are applied in arrival order: each ScreenUpdateEvent carries the
    // delta since the previous emission (not a full snapshot). Skipping intermediate
    // events would lose cells not covered by later ones. Svelte 5 batches $state
    // mutations made in a synchronous call stack — N applyScreenUpdate() calls in
    // one rAF callback produce 1 DOM reconcile cycle.
    //
    // rafQueue, rafId and rafScheduled are declared at composable scope so
    // onDestroy can cancel a pending frame after the component is unmounted.
    // rafScheduled is set BEFORE calling requestAnimationFrame() to avoid a
    // re-entrance bug: if the rAF callback fires synchronously (e.g. in unit
    // tests with a synchronous rAF mock), rafId would be cleared inside the
    // callback before the assignment `rafId = requestAnimationFrame(...)` runs,
    // leaving rafId non-null and blocking future scheduleRaf() calls. The
    // separate rafScheduled boolean has no such race.
    function flushRafQueue(): void {
      rafScheduled = false;
      rafId = null;
      const batch = rafQueue.splice(0);
      for (const update of batch) {
        applyScreenUpdate(update);
      }
    }

    function scheduleRaf(): void {
      if (rafScheduled) return;
      rafScheduled = true;
      rafId = requestAnimationFrame(flushRafQueue);
    }

    unlistens.push(
      await onScreenUpdate((update) => {
        if (update.paneId !== props.paneId()) return;

        if (buffering) {
          pendingUpdates.push(update);
          return;
        }
        rafQueue.push(update);
        scheduleRaf();
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
        cursor = { ...cursor, shape: ev.shape };
        // cursor.blink = DECSET ?12 state, managed exclusively by screen-update (backend authority).
        // cursorBlinks(cursor.shape) reflects DECSCUSR intention and is combined in currentCursorBlinks.
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
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
      rafScheduled = false;
    }
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
    await sendBytes(encoder.encode(seq));
  }

  // mouseButtonCode is imported from useTerminalPane.handlers.ts — used directly.

  // -------------------------------------------------------------------------
  // Selection
  // -------------------------------------------------------------------------

  async function copySelectionToClipboard() {
    const sel = selection.getSelection();
    if (sel) {
      const text = selection.getSelectedText(makeBufferGetCell(), cols);
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
    // Restore keyboard focus to the hidden textarea. On Linux, the arboard
    // clipboard write can cause WebKitGTK to move focus to document.body.
    // This mirrors the refocusTerminal safety-net in useTerminalView.
    inputEl?.focus({ preventScroll: true });
  }

  function isSelected(rowIdx: number, col: number): boolean {
    if (!selectionRange) return false;
    const { start, end } = selectionRange;
    // Convert buffer-absolute selection bounds to screen-space for comparison
    // with the viewport loop index. Out-of-range cases are handled naturally:
    // if startScreen < 0, `rowIdx < startScreen` is always false (rowIdx ≥ 0)
    // and `rowIdx === startScreen` is never true, so the full top row is selected.
    // The symmetric argument applies when endScreen ≥ rows.
    const startScreen = bufferToScreenRow(start.row);
    const endScreen = bufferToScreenRow(end.row);
    if (rowIdx < startScreen || rowIdx > endScreen) return false;
    if (rowIdx === startScreen && col < start.col) return false;
    if (rowIdx === endScreen && col > end.col) return false;
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
      selection.selectLineAt(screenToBufferRow(cell.row), cols);
      await copySelectionToClipboard();
      return;
    }

    if (event.detail === 2) {
      isSelecting = false;
      selection.selectWordAt(
        cell.col,
        screenToBufferRow(cell.row),
        makeBufferGetCell(),
        cols,
        props.wordDelimiters(),
      );
      await copySelectionToClipboard();
      return;
    }

    isSelecting = true;
    selection.startSelection({ row: screenToBufferRow(cell.row), col: cell.col });
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
    const moveCell = pixelToCell(event);
    selection.extendSelection({ row: screenToBufferRow(moveCell.row), col: moveCell.col });
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
    const upCell = pixelToCell(event);
    selection.extendSelection({ row: screenToBufferRow(upCell.row), col: upCell.col });
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
      await sendBytes(encoder.encode('\x1b[I'));
    }
  }

  async function handleBlur() {
    if (focusEventsActive) {
      await sendBytes(encoder.encode('\x1b[O'));
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
    } finally {
      // Restore keyboard focus to the hidden textarea. On Linux, closing the
      // context menu (Bits UI) moves focus away from the terminal input sink.
      // Without this, the user must click the terminal before typing again.
      inputEl?.focus({ preventScroll: true });
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
    get inputEl() {
      return inputEl;
    },
    set inputEl(v: HTMLTextAreaElement | undefined) {
      inputEl = v;
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
