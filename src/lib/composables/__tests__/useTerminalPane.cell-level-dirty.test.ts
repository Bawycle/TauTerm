// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for P12a cell-level dirty tracking in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-DIRTY-001 — differential update writes only the targeted cell
 *   TPSC-DIRTY-002 — wide character + continuation cell via differential update
 *   TPSC-DIRTY-003 — multiple cells in the same row, other rows unchanged
 *   TPSC-DIRTY-004 — out-of-bounds cells in differential update are ignored
 *   TPSC-DIRTY-005 — full-redraw path still rebuilds all rows (non-regression)
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
    value: function (): {
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
// Test constants
// ---------------------------------------------------------------------------

const PANE_ID = 'pane-dirty-test';

// Minimal CellAttrsDto with all optional fields at defaults (post-P-IPC1).
const DEFAULT_ATTRS = {};

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
// Helper: mount TerminalPane and drain the onMount async chain
// ---------------------------------------------------------------------------

async function mountPane(
  paneId = PANE_ID,
): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> }> {
  const container = document.createElement('div');
  document.body.appendChild(container);

  const instance = mount(TerminalPane, {
    target: container,
    props: { paneId, tabId: 'tab-dirty', active: true },
  });
  instances.push(instance);

  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();

  return { container, instance };
}

// ---------------------------------------------------------------------------
// Helper: establish a small grid via isFullRedraw, blank content.
// Returns the container after the initial setup flush.
// ---------------------------------------------------------------------------

function initGrid(
  container: HTMLElement,
  cols: number,
  rows: number,
): void {
  const cells = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      cells.push({ row: r, col: c, content: ' ', width: 1, attrs: DEFAULT_ATTRS });
    }
  }
  fireEvent<ScreenUpdateEvent>(
    'screen-update',
    makeScreenUpdate(PANE_ID, { cols, rows, isFullRedraw: true, cells }),
  );
  flushSync();
  void container; // container is used for assertions by callers
}

// ---------------------------------------------------------------------------
// Helper: get text content of a cell span at [row][col].
// Returns null if the row or cell does not exist in the DOM.
// ---------------------------------------------------------------------------

function getCellText(container: HTMLElement, rowIdx: number, colIdx: number): string | null {
  const rows = container.querySelectorAll('.terminal-pane__row');
  if (!rows[rowIdx]) return null;
  const cells = rows[rowIdx].querySelectorAll('.terminal-pane__cell');
  if (!cells[colIdx]) return null;
  return cells[colIdx].textContent ?? null;
}

// ---------------------------------------------------------------------------
// TPSC-DIRTY-001: differential update — only the targeted cell changes
// ---------------------------------------------------------------------------

describe('TPSC-DIRTY-001: differential update writes only the targeted cell', () => {
  it('cell [1][2] gets new content; all other cells remain blank', async () => {
    const { container } = await mountPane();

    // Establish a 4-col × 3-row grid, all blank.
    initGrid(container, 4, 3);

    // Differential update: only cell [1][2] changes to 'X'.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 1, col: 2, content: 'X', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );
    flushSync();

    // Changed cell contains 'X'.
    expect(getCellText(container, 1, 2)).toBe('X');

    // All other cells remain at their initial value ('\u00a0' — the &nbsp;
    // substitution for empty/blank content in TerminalPaneViewport).
    const rows = container.querySelectorAll('.terminal-pane__row');
    expect(rows.length).toBe(3);

    for (let r = 0; r < 3; r++) {
      const rowCells = rows[r].querySelectorAll('.terminal-pane__cell');
      for (let c = 0; c < 4; c++) {
        if (r === 1 && c === 2) continue; // the changed cell
        // Width=1 cells are rendered. initGrid uses content: ' ' (space) which
        // the template renders as ' ' (not '\u00a0' — that substitution only
        // applies to content === '').
        expect(rowCells[c]?.textContent).toBe(' ');
      }
    }
  });
});

// ---------------------------------------------------------------------------
// TPSC-DIRTY-002: wide character + continuation cell in differential update
// ---------------------------------------------------------------------------

describe('TPSC-DIRTY-002: wide character + continuation cell via differential update', () => {
  it('width=2 span is rendered; width=0 continuation slot is not rendered', async () => {
    const { container } = await mountPane();

    // Establish a 4-col × 2-row grid, all blank (width=1).
    initGrid(container, 4, 2);

    // Differential update: CJK wide character at [0][0] (width=2) and its
    // continuation slot at [0][1] (width=0, empty content).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 2,
        isFullRedraw: false,
        cells: [
          { row: 0, col: 0, content: '中', width: 2, attrs: DEFAULT_ATTRS },
          { row: 0, col: 1, content: '', width: 0, attrs: DEFAULT_ATTRS },
        ],
      }),
    );
    flushSync();

    const row0 = container.querySelectorAll('.terminal-pane__row')[0];
    const visibleCells = row0.querySelectorAll('.terminal-pane__cell');

    // Row 0 has 4 columns: [0]=wide(2), [1]=continuation(0, hidden), [2]=normal, [3]=normal.
    // Only width≠0 cells are rendered via {#if cell.width !== 0}.
    // After the update: [0]=wide 中, [1]=hidden(0), [2]=blank, [3]=blank → 3 visible spans.
    expect(visibleCells.length).toBe(3);

    // First visible span: wide character
    expect(visibleCells[0].textContent).toBe('中');
    expect(visibleCells[0].classList.contains('terminal-pane__cell--wide')).toBe(true);

    // Width=0 continuation slot: not in the DOM (filtered by {#if}).
    // Remaining two visible spans are the original blank normal-width cells
    // (content: ' ' from initGrid → renders as ' ', not '\u00a0').
    expect(visibleCells[1].textContent).toBe(' ');
    expect(visibleCells[2].textContent).toBe(' ');
  });
});

// ---------------------------------------------------------------------------
// TPSC-DIRTY-003: multiple cells in the same row; other rows unchanged
// ---------------------------------------------------------------------------

describe('TPSC-DIRTY-003: multiple cells in same row; other rows unchanged', () => {
  it('only the two targeted cells in row 0 change; row 1 is untouched', async () => {
    const { container } = await mountPane();

    // Establish a 5-col × 2-row grid, all set to 'A'.
    const cells = [];
    for (let r = 0; r < 2; r++) {
      for (let c = 0; c < 5; c++) {
        cells.push({ row: r, col: c, content: 'A', width: 1, attrs: DEFAULT_ATTRS });
      }
    }
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, { cols: 5, rows: 2, isFullRedraw: true, cells }),
    );
    flushSync();

    // Differential: change [0][1] → 'B' and [0][3] → 'C'.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 5,
        rows: 2,
        isFullRedraw: false,
        cells: [
          { row: 0, col: 1, content: 'B', width: 1, attrs: DEFAULT_ATTRS },
          { row: 0, col: 3, content: 'C', width: 1, attrs: DEFAULT_ATTRS },
        ],
      }),
    );
    flushSync();

    // Row 0: A B A C A
    expect(getCellText(container, 0, 0)).toBe('A');
    expect(getCellText(container, 0, 1)).toBe('B');
    expect(getCellText(container, 0, 2)).toBe('A');
    expect(getCellText(container, 0, 3)).toBe('C');
    expect(getCellText(container, 0, 4)).toBe('A');

    // Row 1: all 'A' — untouched.
    for (let c = 0; c < 5; c++) {
      expect(getCellText(container, 1, c)).toBe('A');
    }
  });
});

// ---------------------------------------------------------------------------
// TPSC-DIRTY-004: out-of-bounds cells in differential update are ignored
// ---------------------------------------------------------------------------

describe('TPSC-DIRTY-004: out-of-bounds cells in differential update are ignored', () => {
  it('valid cell is written; row/col out of bounds do not throw and are not rendered', async () => {
    const { container } = await mountPane();

    // Establish a 4-col × 3-row grid, all blank.
    initGrid(container, 4, 3);

    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, {
          cols: 4,
          rows: 3,
          isFullRedraw: false,
          cells: [
            { row: 2, col: 3, content: 'X', width: 1, attrs: DEFAULT_ATTRS }, // valid (last cell)
            { row: 5, col: 0, content: 'Y', width: 1, attrs: DEFAULT_ATTRS }, // row oob
            { row: 0, col: 9, content: 'Z', width: 1, attrs: DEFAULT_ATTRS }, // col oob
          ],
        }),
      );
      flushSync();
    }).not.toThrow();

    // Valid cell written.
    expect(getCellText(container, 2, 3)).toBe('X');

    // Grid has exactly 3 rows — no phantom rows created by oob writes.
    const rows = container.querySelectorAll('.terminal-pane__row');
    expect(rows.length).toBe(3);

    // No cell contains 'Y' or 'Z'.
    const allText = Array.from(container.querySelectorAll('.terminal-pane__cell'))
      .map((el) => el.textContent ?? '')
      .join('');
    expect(allText).not.toContain('Y');
    expect(allText).not.toContain('Z');
  });
});

// ---------------------------------------------------------------------------
// TPSC-DIRTY-005: full-redraw path still rebuilds all rows (non-regression)
// ---------------------------------------------------------------------------

describe('TPSC-DIRTY-005: full-redraw path rebuilds all rows correctly', () => {
  it('isFullRedraw: true replaces entire grid with new content', async () => {
    const { container } = await mountPane();

    // Establish a 3-col × 2-row grid with 'A'.
    const initialCells = [];
    for (let r = 0; r < 2; r++) {
      for (let c = 0; c < 3; c++) {
        initialCells.push({ row: r, col: c, content: 'A', width: 1, attrs: DEFAULT_ATTRS });
      }
    }
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, { cols: 3, rows: 2, isFullRedraw: true, cells: initialCells }),
    );
    flushSync();

    // Full-redraw with new content: all cells → 'B'.
    const newCells = [];
    for (let r = 0; r < 2; r++) {
      for (let c = 0; c < 3; c++) {
        newCells.push({ row: r, col: c, content: 'B', width: 1, attrs: DEFAULT_ATTRS });
      }
    }
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, { cols: 3, rows: 2, isFullRedraw: true, cells: newCells }),
    );
    flushSync();

    // All cells show 'B'.
    for (let r = 0; r < 2; r++) {
      for (let c = 0; c < 3; c++) {
        expect(getCellText(container, r, c)).toBe('B');
      }
    }
  });
});
