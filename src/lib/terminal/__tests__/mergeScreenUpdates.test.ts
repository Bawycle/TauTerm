// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for mergeScreenUpdates (P-HT-1 rAF coalescing).
 *
 * Covered:
 *   MERGE-001 — empty batch → throw
 *   MERGE-002 — single event → returned as-is (same reference)
 *   MERGE-003 — two partials on distinct cells → both present in result
 *   MERGE-004 — two partials on same cell → last-write wins
 *   MERGE-005 — full-redraw in the middle → events before it are discarded
 *   MERGE-006 — multiple full-redraws → only last full-redraw and successors survive
 *   MERGE-007 — full-redraw followed by partials → isFullRedraw=true AND partial cells present
 *   MERGE-008 — scalars taken from last event
 *   MERGE-009 — cursor-only event (cells: []) → cursor from last, cells from earlier events
 *   MERGE-010 — heterogeneous scrollOffsets → no cross-group merge (returns array)
 *   MERGE-PBT-001 — N identical copies merge to equivalent of single copy
 *   MERGE-PBT-002 — if any event has isFullRedraw=true, result does too
 */

import { describe, it, expect } from 'vitest';
import { mergeScreenUpdates } from '$lib/terminal/screen';
import type { ScreenUpdateEvent, CellUpdate, CursorState } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Factory helpers
// ---------------------------------------------------------------------------

function makeCursor(overrides: Partial<CursorState> = {}): CursorState {
  return { row: 0, col: 0, visible: true, shape: 0, blink: true, ...overrides };
}

function makeCell(row: number, col: number, content = 'X'): CellUpdate {
  return {
    row,
    col,
    content,
    width: 1,
    attrs: {},
  };
}

function makeEvent(overrides: Partial<ScreenUpdateEvent> = {}): ScreenUpdateEvent {
  return {
    paneId: 'pane-test',
    cells: [],
    cursor: makeCursor(),
    scrollbackLines: 0,
    isFullRedraw: false,
    scrollOffset: 0,
    cols: 80,
    rows: 24,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// MERGE-001: empty batch → throw
// ---------------------------------------------------------------------------

describe('MERGE-001: empty batch throws', () => {
  it('throws an error when called with an empty array', () => {
    expect(() => mergeScreenUpdates([])).toThrow('batch must be non-empty');
  });
});

// ---------------------------------------------------------------------------
// MERGE-002: single event → returned as-is (same reference)
// ---------------------------------------------------------------------------

describe('MERGE-002: single event returned as-is', () => {
  it('returns the exact same object reference for a single-element batch', () => {
    const event = makeEvent({ cells: [makeCell(0, 0, 'A')] });
    const result = mergeScreenUpdates([event]);
    expect(result).toBe(event);
  });
});

// ---------------------------------------------------------------------------
// MERGE-003: two partials on distinct cells → both present
// ---------------------------------------------------------------------------

describe('MERGE-003: two partials on distinct cells', () => {
  it('merged result contains both cells', () => {
    const e1 = makeEvent({ cells: [makeCell(0, 0, 'A')] });
    const e2 = makeEvent({ cells: [makeCell(1, 1, 'B')] });
    const result = mergeScreenUpdates([e1, e2]);
    // Homogeneous batch → single event
    expect(Array.isArray(result)).toBe(false);
    const merged = result as ScreenUpdateEvent;
    expect(merged.cells).toHaveLength(2);
    const contents = new Set(merged.cells.map((c) => c.content));
    expect(contents).toContain('A');
    expect(contents).toContain('B');
  });
});

// ---------------------------------------------------------------------------
// MERGE-004: two partials on same cell → last-write wins
// ---------------------------------------------------------------------------

describe('MERGE-004: same cell — last-write wins', () => {
  it('only the last cell update for a given position survives', () => {
    const e1 = makeEvent({ cells: [makeCell(0, 0, 'A')] });
    const e2 = makeEvent({ cells: [makeCell(0, 0, 'B')] });
    const result = mergeScreenUpdates([e1, e2]) as ScreenUpdateEvent;
    expect(result.cells).toHaveLength(1);
    expect(result.cells[0].content).toBe('B');
  });
});

// ---------------------------------------------------------------------------
// MERGE-005: full-redraw in the middle → events before discarded
// ---------------------------------------------------------------------------

describe('MERGE-005: full-redraw discards preceding events', () => {
  it('cells from events before the full-redraw are not in the result', () => {
    const e1 = makeEvent({ cells: [makeCell(0, 0, 'A')] });
    const e2 = makeEvent({ isFullRedraw: true, cells: [makeCell(1, 1, 'B')] });
    const e3 = makeEvent({ cells: [makeCell(2, 2, 'C')] });
    const result = mergeScreenUpdates([e1, e2, e3]) as ScreenUpdateEvent;
    const contents = result.cells.map((c) => c.content);
    expect(contents).not.toContain('A');
    expect(contents).toContain('B');
    expect(contents).toContain('C');
  });
});

// ---------------------------------------------------------------------------
// MERGE-006: multiple full-redraws → only last full-redraw + successors
// ---------------------------------------------------------------------------

describe('MERGE-006: multiple full-redraws — only last survives', () => {
  it('events before the last full-redraw are discarded', () => {
    const e1 = makeEvent({ isFullRedraw: true, cells: [makeCell(0, 0, 'A')] });
    const e2 = makeEvent({ cells: [makeCell(1, 0, 'B')] });
    const e3 = makeEvent({ isFullRedraw: true, cells: [makeCell(2, 0, 'C')] });
    const e4 = makeEvent({ cells: [makeCell(3, 0, 'D')] });
    const result = mergeScreenUpdates([e1, e2, e3, e4]) as ScreenUpdateEvent;
    const contents = result.cells.map((c) => c.content);
    expect(contents).not.toContain('A');
    expect(contents).not.toContain('B');
    expect(contents).toContain('C');
    expect(contents).toContain('D');
  });
});

// ---------------------------------------------------------------------------
// MERGE-007: full-redraw + partials → isFullRedraw=true AND partial cells
// ---------------------------------------------------------------------------

describe('MERGE-007: full-redraw followed by partials', () => {
  it('result has isFullRedraw=true and contains cells from both', () => {
    const e1 = makeEvent({ isFullRedraw: true, cells: [makeCell(0, 0, 'F')] });
    const e2 = makeEvent({ cells: [makeCell(1, 1, 'P')] });
    const result = mergeScreenUpdates([e1, e2]) as ScreenUpdateEvent;
    expect(result.isFullRedraw).toBe(true);
    const contents = result.cells.map((c) => c.content);
    expect(contents).toContain('F');
    expect(contents).toContain('P');
  });
});

// ---------------------------------------------------------------------------
// MERGE-008: scalars from last event
// ---------------------------------------------------------------------------

describe('MERGE-008: scalars taken from last event', () => {
  it('cursor, scrollbackLines, scrollOffset, cols, rows from the last event', () => {
    const cursor1 = makeCursor({ row: 0, col: 0 });
    const cursor2 = makeCursor({ row: 5, col: 10 });
    const e1 = makeEvent({
      cursor: cursor1,
      scrollbackLines: 100,
      scrollOffset: 0,
      cols: 80,
      rows: 24,
    });
    const e2 = makeEvent({
      cursor: cursor2,
      scrollbackLines: 200,
      scrollOffset: 0,
      cols: 120,
      rows: 40,
    });
    const result = mergeScreenUpdates([e1, e2]) as ScreenUpdateEvent;
    expect(result.cursor).toEqual(cursor2);
    expect(result.scrollbackLines).toBe(200);
    expect(result.cols).toBe(120);
    expect(result.rows).toBe(40);
  });
});

// ---------------------------------------------------------------------------
// MERGE-009: cursor-only event (cells: []) → cursor from last, cells preserved
// ---------------------------------------------------------------------------

describe('MERGE-009: cursor-only event preserves earlier cells', () => {
  it('cells from earlier events are present; cursor is from the last', () => {
    const cursor1 = makeCursor({ row: 0, col: 0 });
    const cursor2 = makeCursor({ row: 3, col: 7 });
    const e1 = makeEvent({ cursor: cursor1, cells: [makeCell(0, 0, 'A')] });
    const e2 = makeEvent({ cursor: cursor2, cells: [] }); // cursor-only
    const result = mergeScreenUpdates([e1, e2]) as ScreenUpdateEvent;
    expect(result.cursor).toEqual(cursor2);
    expect(result.cells).toHaveLength(1);
    expect(result.cells[0].content).toBe('A');
  });
});

// ---------------------------------------------------------------------------
// MERGE-010: heterogeneous scrollOffsets → returns array
// ---------------------------------------------------------------------------

describe('MERGE-010: heterogeneous scrollOffsets — no cross-group merge', () => {
  it('returns an array of merged groups when scrollOffsets differ', () => {
    const e1 = makeEvent({ scrollOffset: 0, cells: [makeCell(0, 0, 'A')] });
    const e2 = makeEvent({ scrollOffset: 0, cells: [makeCell(0, 1, 'B')] });
    const e3 = makeEvent({ scrollOffset: 5, cells: [makeCell(0, 0, 'C')] });
    const result = mergeScreenUpdates([e1, e2, e3]);
    expect(Array.isArray(result)).toBe(true);
    const arr = result as ScreenUpdateEvent[];
    expect(arr).toHaveLength(2);
    // First group: scrollOffset=0, cells A and B
    expect(arr[0].scrollOffset).toBe(0);
    expect(arr[0].cells).toHaveLength(2);
    // Second group: scrollOffset=5, cell C
    expect(arr[1].scrollOffset).toBe(5);
    expect(arr[1].cells).toHaveLength(1);
    expect(arr[1].cells[0].content).toBe('C');
  });
});

// ---------------------------------------------------------------------------
// MERGE-PBT-001: N identical copies ≡ 1 copy (manual randomized)
// ---------------------------------------------------------------------------

describe('MERGE-PBT-001: N identical copies merge equivalently to 1 copy', () => {
  it('merging N identical events produces equivalent cells and scalars', () => {
    // Run 20 randomized iterations.
    for (let iter = 0; iter < 20; iter++) {
      const n = 2 + Math.floor(Math.random() * 10); // 2–11 copies
      const cols = 10 + Math.floor(Math.random() * 100);
      const rows = 5 + Math.floor(Math.random() * 50);
      const numCells = 1 + Math.floor(Math.random() * 5);
      const cells: CellUpdate[] = [];
      for (let i = 0; i < numCells; i++) {
        cells.push(makeCell(Math.floor(Math.random() * rows), Math.floor(Math.random() * cols)));
      }
      const original = makeEvent({ cols, rows, cells, scrollbackLines: iter * 10 });
      const batch = Array.from({ length: n }, () => ({ ...original, cells: [...original.cells] }));
      const result = mergeScreenUpdates(batch) as ScreenUpdateEvent;
      // Scalars must match the original.
      expect(result.cols).toBe(original.cols);
      expect(result.rows).toBe(original.rows);
      expect(result.scrollbackLines).toBe(original.scrollbackLines);
      expect(result.cursor).toEqual(original.cursor);
      // Deduplicated cell count should be ≤ numCells (equal if all positions unique).
      expect(result.cells.length).toBeLessThanOrEqual(numCells);
      // Every cell in result must come from original.
      for (const c of result.cells) {
        expect(original.cells.some((oc) => oc.row === c.row && oc.col === c.col)).toBe(true);
      }
    }
  });
});

// ---------------------------------------------------------------------------
// MERGE-PBT-002: isFullRedraw propagation (manual randomized)
// ---------------------------------------------------------------------------

describe('MERGE-PBT-002: isFullRedraw propagates if any event has it', () => {
  it('result.isFullRedraw is true when any event in the batch is full-redraw', () => {
    for (let iter = 0; iter < 20; iter++) {
      const n = 2 + Math.floor(Math.random() * 8);
      const batch: ScreenUpdateEvent[] = [];
      let anyFullRedraw = false;
      for (let i = 0; i < n; i++) {
        const isFull = Math.random() < 0.3;
        if (isFull) anyFullRedraw = true;
        batch.push(makeEvent({ isFullRedraw: isFull }));
      }
      const result = mergeScreenUpdates(batch) as ScreenUpdateEvent;
      if (anyFullRedraw) {
        expect(result.isFullRedraw).toBe(true);
      }
      // If no full-redraw at all, result.isFullRedraw should be false.
      if (!anyFullRedraw) {
        expect(result.isFullRedraw).toBe(false);
      }
    }
  });
});
