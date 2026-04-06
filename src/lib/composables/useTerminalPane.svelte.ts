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
 *   - Cursor blink timer
 *   - Border pulse state (inactive pane activity indicators)
 *   - Selection flash state (copy feedback)
 *   - Bell flash state
 *   - Paste confirmation dialog state
 *   - Scrollbar interaction state
 *   - Resize debounce
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
  resizePane,
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
import type {
  PaneId,
  CursorState,
  BellType,
  SearchMatch,
  NotificationChangedEvent,
} from '$lib/ipc/types';
import type { CellStyle } from '$lib/terminal/screen.js';

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
  // Cursor blink
  // -------------------------------------------------------------------------

  let cursorVisible = $state(true);
  let blinkTimer: ReturnType<typeof setInterval> | null = null;

  // -------------------------------------------------------------------------
  // Selection
  // -------------------------------------------------------------------------

  const selection = new SelectionManager();
  let selectionRange = $state(selection.getSelection());
  let isSelecting = $state(false);
  let hasSelection = $state(false);

  // -------------------------------------------------------------------------
  // Scrollbar interaction
  // -------------------------------------------------------------------------

  let scrollbarVisible = $state(false);
  let scrollbarFadeTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollbarDragging = $state(false);
  let scrollbarHover = $state(false);
  let scrollbarDragStartY = 0;
  let scrollbarDragStartOffset = 0;

  // -------------------------------------------------------------------------
  // Visual states
  // -------------------------------------------------------------------------

  let bellFlashing = $state(false);
  let bellFlashTimer: ReturnType<typeof setTimeout> | null = null;

  type BorderPulse = 'output' | 'bell' | 'exit' | null;
  let borderPulse = $state<BorderPulse>(null);
  let borderPulseTimer: ReturnType<typeof setTimeout> | null = null;

  let selectionFlashing = $state(false);
  let selectionFlashTimer: ReturnType<typeof setTimeout> | null = null;

  // -------------------------------------------------------------------------
  // Paste confirmation dialog
  // -------------------------------------------------------------------------

  let pasteConfirmOpen = $state(false);
  let pasteConfirmText = $state('');

  // -------------------------------------------------------------------------
  // DOM refs (set by template)
  // -------------------------------------------------------------------------

  let viewportEl = $state<HTMLDivElement | undefined>();
  let scrollbarEl = $state<HTMLDivElement | undefined>();

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
            hyperlink: undefined,
          } satisfies CellStyle)
        );
      }),
    ),
  );

  const searchMatchSet = $derived.by(() => {
    const set = new Set<string>();
    if (props.searchMatches().length === 0) return set;
    const screenStart = scrollbackLines - scrollOffset;
    for (let i = 0; i < props.searchMatches().length; i++) {
      const match = props.searchMatches()[i];
      const screenRow = match.scrollbackRow - screenStart;
      if (screenRow < 0 || screenRow >= rows) continue;
      for (let c = match.colStart; c < match.colEnd; c++) {
        set.add(`${screenRow}:${c}`);
      }
    }
    return set;
  });

  const activeSearchMatchSet = $derived.by(() => {
    const set = new Set<string>();
    if (props.searchMatches().length === 0 || props.activeSearchMatchIndex() <= 0) return set;
    const match = props.searchMatches()[props.activeSearchMatchIndex() - 1];
    if (!match) return set;
    const screenStart = scrollbackLines - scrollOffset;
    const screenRow = match.scrollbackRow - screenStart;
    if (screenRow < 0 || screenRow >= rows) return set;
    for (let c = match.colStart; c < match.colEnd; c++) {
      set.add(`${screenRow}:${c}`);
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

  // Cursor blink timer — restarts when cursorBlinkMs changes
  $effect(() => {
    const interval = props.cursorBlinkMs();
    stopCursorBlink();
    blinkTimer = setInterval(() => {
      cursorVisible = currentCursorBlinks ? !cursorVisible : true;
    }, interval);
    return () => {
      stopCursorBlink();
    };
  });

  // Clear border pulse when pane becomes active
  $effect(() => {
    if (props.active() && borderPulse !== null) {
      if (borderPulseTimer) clearTimeout(borderPulseTimer);
      borderPulseTimer = null;
      borderPulse = null;
    }
  });

  // -------------------------------------------------------------------------
  // Resize debounce
  // -------------------------------------------------------------------------

  let resizeObserver: ResizeObserver | null = null;
  let resizeDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  function scheduleSendResize() {
    if (resizeDebounceTimer) clearTimeout(resizeDebounceTimer);
    props.ondimensionschange()?.(cols, rows);
    resizeDebounceTimer = setTimeout(sendResize, 50);
  }

  async function sendResize() {
    if (!viewportEl) return;
    const rect = viewportEl.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;

    const testSpan = viewportEl.querySelector('.terminal-pane__cell') as HTMLElement | null;
    const cellW = testSpan ? testSpan.offsetWidth : Math.max(1, rect.width / cols);
    const cellH = testSpan ? testSpan.offsetHeight : Math.max(1, rect.height / rows);

    const newCols = Math.max(1, Math.floor(rect.width / cellW));
    const newRows = Math.max(1, Math.floor(rect.height / cellH));
    const pixelWidth = Math.max(1, Math.floor(rect.width));
    const pixelHeight = Math.max(1, Math.floor(rect.height));

    props.ondimensionschange()?.(newCols, newRows);
    try {
      await resizePane(props.paneId(), newCols, newRows, pixelWidth, pixelHeight);
      cols = newCols;
      rows = newRows;
    } catch {
      // Non-fatal
    }
  }

  // -------------------------------------------------------------------------
  // Mount / destroy
  // -------------------------------------------------------------------------

  let unlistens: Array<() => void> = [];

  onMount(async () => {
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
      // Backend not ready — populated by first screen-update event
    }

    unlistens.push(
      await onScreenUpdate((update) => {
        if (update.paneId !== props.paneId()) return;
        applyUpdates(grid, update.cells, cols);
        cursor = update.cursor;
        if (typeof update.scrollbackLines === 'number') {
          scrollbackLines = update.scrollbackLines;
        }
        grid = grid.slice();
        screenGeneration++;
      }),
    );

    unlistens.push(
      await onScrollPositionChanged((pos) => {
        if (pos.paneId !== props.paneId()) return;
        scrollOffset = pos.offset;
        scrollbackLines = pos.scrollbackLines;
        scrollbarVisible = true;
        if (scrollbarFadeTimer) clearTimeout(scrollbarFadeTimer);
        if (scrollOffset === 0) {
          scrollbarFadeTimer = setTimeout(() => {
            scrollbarVisible = false;
          }, 1500);
        }
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
        startCursorBlink();
      }),
    );

    unlistens.push(
      await onBellTriggered((ev) => {
        if (ev.paneId !== props.paneId()) return;
        handleBell();
      }),
    );

    unlistens.push(
      await onNotificationChanged((ev) => {
        if (ev.paneId !== props.paneId()) return;
        if (props.active()) return;
        handleNotificationForBorderPulse(ev);
      }),
    );

    if (viewportEl) {
      resizeObserver = new ResizeObserver(() => scheduleSendResize());
      resizeObserver.observe(viewportEl);
    }
  });

  onDestroy(() => {
    for (const unlisten of unlistens) unlisten();
    unlistens = [];
    stopCursorBlink();
    if (bellFlashTimer) clearTimeout(bellFlashTimer);
    if (borderPulseTimer) clearTimeout(borderPulseTimer);
    if (selectionFlashTimer) clearTimeout(selectionFlashTimer);
    resizeObserver?.disconnect();
    if (resizeDebounceTimer) clearTimeout(resizeDebounceTimer);
    if (scrollbarFadeTimer) clearTimeout(scrollbarFadeTimer);
  });

  // -------------------------------------------------------------------------
  // Cursor blink helpers
  // -------------------------------------------------------------------------

  function startCursorBlink() {
    stopCursorBlink();
    blinkTimer = setInterval(() => {
      cursorVisible = currentCursorBlinks ? !cursorVisible : true;
    }, props.cursorBlinkMs());
  }

  function stopCursorBlink() {
    if (blinkTimer) {
      clearInterval(blinkTimer);
      blinkTimer = null;
    }
    cursorVisible = true;
  }

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
          triggerSelectionFlash();
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

  async function handleWheel(event: WheelEvent) {
    event.preventDefault();
    if (!event.shiftKey && mouseReporting !== 'none') {
      const button = event.deltaY < 0 ? 64 : 65;
      const cell = pixelToCell(event);
      await sendMouseEvent(button, cell.col, cell.row, event, false);
      return;
    }
    const newOffset = Math.max(0, scrollOffset + (event.deltaY > 0 ? -3 : 3));
    try {
      await scrollPane(props.paneId(), newOffset);
    } catch {
      /* non-fatal */
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
      pasteConfirmText = text;
      pasteConfirmOpen = true;
      return;
    }
    const encoded = pasteToBytes(text, bracketedPasteActive);
    if (encoded) await sendBytes(encoded);
  }

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
  // Scrollbar interaction
  // -------------------------------------------------------------------------

  function scrollbarYToOffset(clientY: number): number {
    if (!scrollbarEl) return scrollOffset;
    const rect = scrollbarEl.getBoundingClientRect();
    const fraction = Math.max(0, Math.min(1, (clientY - rect.top) / rect.height));
    const totalLines = rows + scrollbackLines;
    const targetLine = Math.round(fraction * totalLines);
    return Math.max(0, Math.min(scrollbackLines, scrollbackLines - targetLine + rows));
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
      scrollbarDragging = true;
      scrollbarDragStartY = event.clientY;
      scrollbarDragStartOffset = scrollOffset;
    } else {
      scrollToOffset(scrollbarYToOffset(event.clientY));
    }
  }

  function handleScrollbarPointermove(event: PointerEvent) {
    if (!scrollbarDragging) return;
    event.preventDefault();
    const deltaY = event.clientY - scrollbarDragStartY;
    if (!scrollbarEl) return;
    const trackHeight = scrollbarEl.getBoundingClientRect().height;
    const totalLines = rows + scrollbackLines;
    const deltaLines = Math.round((deltaY / trackHeight) * totalLines);
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
  // Bell handler
  // -------------------------------------------------------------------------

  function handleBell() {
    if (props.bellType() === 'none') return;

    if (props.bellType() === 'visual' || props.bellType() === 'both') {
      if (bellFlashTimer) clearTimeout(bellFlashTimer);
      bellFlashing = true;
      bellFlashTimer = setTimeout(() => {
        bellFlashing = false;
        bellFlashTimer = null;
      }, 80);
    }

    if (props.bellType() === 'audio' || props.bellType() === 'both') {
      try {
        const ctx = new AudioContext();
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = 'sine';
        osc.frequency.value = 440;
        gain.gain.setValueAtTime(0.3, ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.08);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start(ctx.currentTime);
        osc.stop(ctx.currentTime + 0.08);
        osc.onended = () => ctx.close();
      } catch {
        // AudioContext unavailable in test environments — non-fatal.
      }
    }
  }

  // -------------------------------------------------------------------------
  // Border pulse
  // -------------------------------------------------------------------------

  function triggerBorderPulse(type: BorderPulse, durationMs: number) {
    if (borderPulse === 'exit' && type !== 'exit') return;
    if (borderPulseTimer) clearTimeout(borderPulseTimer);
    borderPulse = type;
    borderPulseTimer = setTimeout(() => {
      borderPulse = null;
      borderPulseTimer = null;
    }, durationMs);
  }

  function handleNotificationForBorderPulse(ev: NotificationChangedEvent) {
    if (ev.notification === null) {
      if (borderPulse !== 'exit') {
        clearTimeout(borderPulseTimer ?? undefined);
        borderPulseTimer = null;
        borderPulse = null;
      }
      return;
    }
    switch (ev.notification.type) {
      case 'backgroundOutput':
        triggerBorderPulse('output', 800);
        break;
      case 'bell':
        triggerBorderPulse('bell', 800);
        break;
      case 'processExited':
        triggerBorderPulse('exit', 1500);
        break;
    }
  }

  // -------------------------------------------------------------------------
  // Selection flash
  // -------------------------------------------------------------------------

  function triggerSelectionFlash() {
    if (selectionFlashTimer) clearTimeout(selectionFlashTimer);
    selectionFlashing = true;
    selectionFlashTimer = setTimeout(() => {
      selectionFlashing = false;
      selectionFlashTimer = null;
    }, 80);
  }

  // -------------------------------------------------------------------------
  // Cell rendering helper
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
      return cursorVisible;
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
      return selectionFlashing;
    },

    // Scrollbar
    get scrollbarVisible() {
      return scrollbarVisible;
    },
    get scrollbarDragging() {
      return scrollbarDragging;
    },
    get scrollbarHover() {
      return scrollbarHover;
    },
    set scrollbarHover(v: boolean) {
      scrollbarHover = v;
    },

    // Visual states
    get bellFlashing() {
      return bellFlashing;
    },
    get borderPulse() {
      return borderPulse;
    },

    // Paste dialog
    get pasteConfirmOpen() {
      return pasteConfirmOpen;
    },
    get pasteConfirmText() {
      return pasteConfirmText;
    },

    // Methods
    sendBytes,
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
