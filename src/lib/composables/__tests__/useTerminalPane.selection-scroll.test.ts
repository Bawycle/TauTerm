// SPDX-License-Identifier: MPL-2.0

/**
 * Regression tests for selection coordinate tracking across scroll changes.
 *
 * Selection coordinates are stored as buffer-absolute row indices so that
 * highlight positions follow the text when scrollOffset changes — the same
 * pattern used by search match highlighting.
 *
 * Covered:
 *   TPSEL-SCROLL-001 — selection partially exits viewport on scroll-up →
 *                       only visible buffer rows remain highlighted
 *   TPSEL-SCROLL-002 — selection fully exits viewport on scroll-to-live →
 *                       no cells highlighted
 *   TPSEL-SCROLL-003 — selection stable when scrollOffset unchanged
 *                       (live output regression guard)
 *   TPSEL-SCROLL-004 — selection at screen rows 1-2 stays stable across
 *                       same-offset full-redraw
 *
 * Coordinate note:
 *   jsdom's getBoundingClientRect returns { left:0, top:0, width:0, height:0 }.
 *   pixelToCell uses Math.max(1, rect.width/cols) so cw=ch=1 and rect offsets
 *   are 0. Therefore: screen_col = floor(clientX) and screen_row = floor(clientY),
 *   clamped to [0, cols-1] and [0, rows-1]. Pixel coords map 1-to-1 to cells.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { ScreenUpdateEvent } from '$lib/ipc/types';
import TerminalPane from '$lib/components/TerminalPane.svelte';
import {
  createListenerRegistry,
  createFireEvent,
  makeScreenUpdate,
  type ListenerFn,
  type ListenerRegistry,
} from './event-registry';

// ---------------------------------------------------------------------------
// jsdom polyfill: element.animate
// ---------------------------------------------------------------------------

if (typeof Element.prototype.animate === 'undefined') {
  Object.defineProperty(Element.prototype, 'animate', {
    value: function (
      _keyframes: unknown,
      _options?: unknown,
    ): {
      finished: Promise<void>;
      cancel(): void;
      finish(): void;
      addEventListener(): void;
      removeEventListener(): void;
      dispatchEvent(): boolean;
    } {
      return {
        finished: Promise.resolve(),
        cancel() {},
        finish() {},
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {
          return true;
        },
      };
    },
    writable: true,
    configurable: true,
  });
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PANE_ID = 'pane-sel-scroll';
const COLS = 80;
const ROWS = 24;

// ---------------------------------------------------------------------------
// Module-level state (reset in beforeEach)
// ---------------------------------------------------------------------------

let registry: ListenerRegistry = createListenerRegistry();
let fireEvent: <T>(eventName: string, payload: T) => void;
const instances: ReturnType<typeof mount>[] = [];

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  registry = createListenerRegistry();
  fireEvent = createFireEvent(registry);

  vi.spyOn(tauriEvent, 'listen').mockImplementation(
    async (eventName: string, handler: ListenerFn) => {
      if (!registry.has(eventName)) {
        registry.set(eventName, []);
      }
      registry.get(eventName)!.push(handler as ListenerFn);
      return () => {
        const handlers = registry.get(eventName);
        if (handlers) {
          const idx = handlers.indexOf(handler as ListenerFn);
          if (idx !== -1) handlers.splice(idx, 1);
        }
      };
    },
  );

  vi.spyOn(tauriCore, 'invoke').mockResolvedValue(undefined as unknown as never);

  vi.spyOn(globalThis, 'requestAnimationFrame').mockImplementation((cb) => {
    cb(performance.now());
    return 1;
  });
  vi.spyOn(globalThis, 'cancelAnimationFrame').mockImplementation(() => {});
});

afterEach(() => {
  instances.forEach((inst) => {
    try {
      unmount(inst);
    } catch {
      /* component may have thrown on mount */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  registry.clear();
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function mountPane(
  paneId = PANE_ID,
): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> }> {
  const container = document.createElement('div');
  document.body.appendChild(container);

  const instance = mount(TerminalPane, {
    target: container,
    props: { paneId, tabId: 'tab-1', active: true },
  });
  instances.push(instance);

  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();

  return { container, instance };
}

/**
 * Fires a full-redraw screen-update to establish scroll state, then drains
 * the microtask queue and flushes Svelte's update batch.
 */
function applyScrollState(scrollOffset: number, scrollbackLines: number): void {
  fireEvent<ScreenUpdateEvent>(
    'screen-update',
    makeScreenUpdate(PANE_ID, {
      isFullRedraw: true,
      scrollOffset,
      scrollbackLines,
      cols: COLS,
      rows: ROWS,
      cells: [],
    }),
  );
  flushSync();
}

/**
 * Simulates a drag selection from screenRow1/col1 to screenRow2/col2.
 * Because jsdom returns a zeroed bounding rect, pixelToCell maps pixel→cell
 * as col=clientX, row=clientY (clamped). Both coordinates must be integers in
 * [0, ROWS-1] / [0, COLS-1].
 *
 * Returns a drain+flush promise that must be awaited.
 */
async function selectScreenRange(
  container: HTMLElement,
  fromRow: number,
  fromCol: number,
  toRow: number,
  toCol: number,
): Promise<void> {
  const viewport = container.querySelector('.terminal-pane__viewport')!;

  viewport.dispatchEvent(
    new MouseEvent('mousedown', {
      bubbles: true,
      clientX: fromCol,
      clientY: fromRow,
      button: 0,
      detail: 1,
    }),
  );

  viewport.dispatchEvent(
    new MouseEvent('mouseup', {
      bubbles: true,
      clientX: toCol,
      clientY: toRow,
      button: 0,
      detail: 1,
    }),
  );

  // Drain microtasks: copySelectionToClipboard is async (awaits IPC invoke).
  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();
}

/**
 * Returns the set of (screenRow, col) pairs that are currently selected,
 * regardless of which selection variant class is applied:
 *   - terminal-pane__cell--selected        (active, not flashing)
 *   - terminal-pane__cell--selected-flash  (active, flashing — flash animation in progress)
 *   - terminal-pane__cell--selected-inactive (pane not active)
 * All three indicate `isSelected(row, col) === true`.
 */
function selectedCells(container: HTMLElement): Set<string> {
  const result = new Set<string>();
  const rows = container.querySelectorAll('.terminal-pane__row');
  rows.forEach((rowEl, rowIdx) => {
    const cells = rowEl.querySelectorAll('.terminal-pane__cell');
    cells.forEach((cellEl, colIdx) => {
      if (
        cellEl.classList.contains('terminal-pane__cell--selected') ||
        cellEl.classList.contains('terminal-pane__cell--selected-flash') ||
        cellEl.classList.contains('terminal-pane__cell--selected-inactive')
      ) {
        result.add(`${rowIdx}:${colIdx}`);
      }
    });
  });
  return result;
}

// ---------------------------------------------------------------------------
// TPSEL-SCROLL-001
// Selection partially exits viewport when scrolling upward
// ---------------------------------------------------------------------------

describe('TPSEL-SCROLL-001: selection partially exits viewport on scroll-up', () => {
  it('only buffer rows still in viewport remain highlighted after scroll', async () => {
    const { container } = await mountPane();

    // State: scrollOffset=5, scrollbackLines=10 → screenStart=5.
    // Screen rows 0-23 correspond to buffer rows 5-28.
    applyScrollState(5, 10);

    // Select screen rows 1-2 (buffer rows 6-7).
    await selectScreenRange(container, 1, 0, 2, COLS - 1);

    // Verify initial selection: rows 1 and 2 are highlighted.
    const beforeScroll = selectedCells(container);
    expect(beforeScroll.has('1:0')).toBe(true);
    expect(beforeScroll.has('2:0')).toBe(true);
    expect(beforeScroll.has('0:0')).toBe(false);
    expect(beforeScroll.has('3:0')).toBe(false);

    // Scroll up by 2: scrollOffset=3, scrollbackLines=10 → screenStart=7.
    // Buffer row 6 → screen row -1 (off-screen).
    // Buffer row 7 → screen row 0 (visible).
    applyScrollState(3, 10);

    const afterScroll = selectedCells(container);
    // Only buffer row 7 is visible (at screen row 0).
    expect(afterScroll.has('0:0')).toBe(true);
    expect(afterScroll.has('0:' + (COLS - 1))).toBe(true);
    // Buffer row 6 is off-screen — rows 1+ must not be selected.
    expect(afterScroll.has('1:0')).toBe(false);
    expect(afterScroll.has('2:0')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TPSEL-SCROLL-002
// Selection fully exits viewport when scrolling to live view
// ---------------------------------------------------------------------------

describe('TPSEL-SCROLL-002: selection fully exits viewport on scroll-to-live', () => {
  it('no cells highlighted after selection scrolls completely off screen', async () => {
    const { container } = await mountPane();

    // State: scrollOffset=5, scrollbackLines=10 → screenStart=5.
    applyScrollState(5, 10);

    // Select screen rows 1-2 (buffer rows 6-7).
    await selectScreenRange(container, 1, 0, 2, COLS - 1);

    // Sanity: selection is visible before scroll.
    expect(selectedCells(container).size).toBeGreaterThan(0);

    // Scroll to live view: scrollOffset=0, scrollbackLines=10 → screenStart=10.
    // Buffer rows 6-7 → screen rows -4, -3 → all off-screen.
    applyScrollState(0, 10);

    expect(selectedCells(container).size).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// TPSEL-SCROLL-003
// Selection stable when scrollOffset is unchanged (live output regression guard)
// ---------------------------------------------------------------------------

describe('TPSEL-SCROLL-003: selection stable with no scroll change', () => {
  it('selection at rows 1-2 remains after a same-offset screen-update', async () => {
    const { container } = await mountPane();

    // Live state: no scrollback.
    applyScrollState(0, 0);

    // Select screen rows 1-2 (buffer rows 1-2 since screenStart=0).
    await selectScreenRange(container, 1, 0, 2, COLS - 1);

    const beforeUpdate = selectedCells(container);
    expect(beforeUpdate.has('1:0')).toBe(true);
    expect(beforeUpdate.has('2:0')).toBe(true);

    // Simulate live PTY output arriving: a new full-redraw at same scroll position.
    applyScrollState(0, 0);

    const afterUpdate = selectedCells(container);
    expect(afterUpdate.has('1:0')).toBe(true);
    expect(afterUpdate.has('2:0')).toBe(true);
    expect(afterUpdate.has('0:0')).toBe(false);
    expect(afterUpdate.has('3:0')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TPSEL-SCROLL-004
// Selection count matches expected span after same-offset redraw
// ---------------------------------------------------------------------------

describe('TPSEL-SCROLL-004: selection cell count correct after same-offset redraw', () => {
  it('selecting rows 1-2 of an 80-col grid produces 2×80 selected cells', async () => {
    const { container } = await mountPane();

    applyScrollState(5, 10);

    // Select full rows 1 and 2 (col 0 → col COLS-1).
    await selectScreenRange(container, 1, 0, 2, COLS - 1);

    // Redraw with the same scroll state (no change).
    applyScrollState(5, 10);

    const cells = selectedCells(container);
    // Rows 1 and 2, all COLS columns each.
    expect(cells.size).toBe(2 * COLS);

    // No drift: row 0 and row 3 must be clean.
    for (let c = 0; c < COLS; c++) {
      expect(cells.has(`0:${c}`), `row 0 col ${c} must not be selected`).toBe(false);
      expect(cells.has(`3:${c}`), `row 3 col ${c} must not be selected`).toBe(false);
    }
  });
});
