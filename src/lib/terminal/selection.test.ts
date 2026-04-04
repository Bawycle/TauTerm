// SPDX-License-Identifier: MPL-2.0

import { describe, it, expect, beforeEach } from 'vitest';
import { SelectionManager } from './selection.js';

/** Build a fake screen buffer of `rows × cols` cells. */
function makeGrid(
  rows: number,
  cols: number,
  content: string[][],
): (r: number, c: number) => string {
  return (r: number, c: number) => content[r]?.[c] ?? '';
}

/** Build a grid where each cell contains a single character from a flat string. */
function gridFromLines(lines: string[], cols: number): (r: number, c: number) => string {
  const padded = lines.map((l) => l.padEnd(cols, ' ').split(''));
  return (r: number, c: number) => padded[r]?.[c] ?? '';
}

// ---------------------------------------------------------------------------
// TEST-CLIP-001 — Start / extend / clear selection
// ---------------------------------------------------------------------------
describe('TEST-CLIP-001: start, extend, and clear selection', () => {
  it('initial state has no selection', () => {
    const sm = new SelectionManager();
    expect(sm.getSelection()).toBeNull();
  });

  it('start selection at same cell → no selection (zero-length)', () => {
    const sm = new SelectionManager();
    sm.startSelection({ row: 0, col: 0 });
    expect(sm.getSelection()).toBeNull();
  });

  it('start then extend → selection returned', () => {
    const sm = new SelectionManager();
    sm.startSelection({ row: 0, col: 2 });
    sm.extendSelection({ row: 0, col: 8 });
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start).toEqual({ row: 0, col: 2 });
    expect(sel!.end).toEqual({ row: 0, col: 8 });
  });

  it('clear selection removes it', () => {
    const sm = new SelectionManager();
    sm.startSelection({ row: 0, col: 0 });
    sm.extendSelection({ row: 0, col: 5 });
    sm.clearSelection();
    expect(sm.getSelection()).toBeNull();
  });

  it('extend with no prior start is a no-op', () => {
    const sm = new SelectionManager();
    sm.extendSelection({ row: 0, col: 5 });
    expect(sm.getSelection()).toBeNull();
  });

  it('starting a new selection replaces the old one', () => {
    const sm = new SelectionManager();
    sm.startSelection({ row: 0, col: 0 });
    sm.extendSelection({ row: 0, col: 10 });
    sm.startSelection({ row: 2, col: 3 });
    sm.extendSelection({ row: 2, col: 7 });
    const sel = sm.getSelection()!;
    expect(sel.start.row).toBe(2);
    expect(sel.start.col).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// TEST-CLIP-002 — Get selected text, single line
// ---------------------------------------------------------------------------
describe('TEST-CLIP-002: getSelectedText — single line', () => {
  const COLS = 20;

  it('selects exact character range on a single row', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['Hello, World!       '], COLS);
    sm.startSelection({ row: 0, col: 0 });
    sm.extendSelection({ row: 0, col: 12 });
    expect(sm.getSelectedText(getCell, COLS)).toBe('Hello, World!');
  });

  it('trailing spaces are trimmed', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['abc      '], COLS);
    sm.startSelection({ row: 0, col: 0 });
    sm.extendSelection({ row: 0, col: 8 });
    expect(sm.getSelectedText(getCell, COLS)).toBe('abc');
  });

  it('no selection returns empty string', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['abc'], COLS);
    expect(sm.getSelectedText(getCell, COLS)).toBe('');
  });
});

// ---------------------------------------------------------------------------
// TEST-CLIP-003 — Get selected text, multi-line
// ---------------------------------------------------------------------------
describe('TEST-CLIP-003: getSelectedText — multi-line', () => {
  const COLS = 20;

  it('two-row selection produces two lines joined with \\n', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['Hello               ', 'World               '], COLS);
    sm.startSelection({ row: 0, col: 0 });
    sm.extendSelection({ row: 1, col: 4 });
    const text = sm.getSelectedText(getCell, COLS);
    expect(text).toBe('Hello\nWorld');
  });

  it('three-row selection: middle rows are full-width', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(
      ['line one            ', 'line two            ', 'line three          '],
      COLS,
    );
    sm.startSelection({ row: 0, col: 5 });
    sm.extendSelection({ row: 2, col: 9 });
    const text = sm.getSelectedText(getCell, COLS);
    // Row 0: from col 5 → 'one'
    // Row 1: full row → 'line two'
    // Row 2: up to col 9 → 'line three'
    expect(text).toBe('one\nline two\nline three');
  });
});

// ---------------------------------------------------------------------------
// TEST-CLIP-003b — Reversed selection (dragging up/left) normalizes correctly
// ---------------------------------------------------------------------------
describe('TEST-CLIP-003b: reversed selection normalizes correctly', () => {
  const COLS = 20;

  it('dragging from right to left on same row', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['Hello, World!       '], COLS);
    // Start at col 12, extend back to col 0
    sm.startSelection({ row: 0, col: 12 });
    sm.extendSelection({ row: 0, col: 0 });
    const sel = sm.getSelection()!;
    expect(sel.start.col).toBe(0);
    expect(sel.end.col).toBe(12);
    expect(sm.getSelectedText(getCell, COLS)).toBe('Hello, World!');
  });

  it('dragging from lower row to upper row', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['first               ', 'second              '], COLS);
    sm.startSelection({ row: 1, col: 5 });
    sm.extendSelection({ row: 0, col: 0 });
    const sel = sm.getSelection()!;
    expect(sel.start.row).toBe(0);
    expect(sel.end.row).toBe(1);
    const text = sm.getSelectedText(getCell, COLS);
    expect(text).toBe('first\nsecond');
  });
});

// ---------------------------------------------------------------------------
// Wide character handling — continuation cells (empty string) are skipped
// ---------------------------------------------------------------------------
describe('wide character handling', () => {
  it('skips continuation cells (empty string) in text extraction', () => {
    const sm = new SelectionManager();
    // Simulate: row 0 = [A, W, '', B] where W is a wide char occupying cols 1–2
    const cells: string[][] = [['A', 'W', '', 'B']];
    const getCell = makeGrid(1, 4, cells);
    sm.startSelection({ row: 0, col: 0 });
    sm.extendSelection({ row: 0, col: 3 });
    // Should extract 'AWB' — the empty continuation cell is skipped
    expect(sm.getSelectedText(getCell, 4)).toBe('AWB');
  });
});
