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

// ---------------------------------------------------------------------------
// TEST-CLIP-004 — selectWordAt (FS-CLIP-002)
// ---------------------------------------------------------------------------
describe('TEST-CLIP-004: selectWordAt', () => {
  const COLS = 20;
  const DELIMITERS = ' \t|"\'`&()*,;<=>[]{}~';

  it('selects the whole word when clicking on a middle character', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['hello world         '], COLS);
    sm.selectWordAt(2, 0, getCell, COLS, DELIMITERS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start).toEqual({ row: 0, col: 0 });
    expect(sel!.end).toEqual({ row: 0, col: 4 });
    expect(sm.getSelectedText(getCell, COLS)).toBe('hello');
  });

  it('selects the whole word when clicking on the first character', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['hello world         '], COLS);
    sm.selectWordAt(6, 0, getCell, COLS, DELIMITERS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start).toEqual({ row: 0, col: 6 });
    expect(sel!.end).toEqual({ row: 0, col: 10 });
    expect(sm.getSelectedText(getCell, COLS)).toBe('world');
  });

  it('selects the whole word when clicking on the last character', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['hello world         '], COLS);
    sm.selectWordAt(10, 0, getCell, COLS, DELIMITERS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start).toEqual({ row: 0, col: 6 });
    expect(sel!.end).toEqual({ row: 0, col: 10 });
    expect(sm.getSelectedText(getCell, COLS)).toBe('world');
  });

  it('clicking on a delimiter selects only that delimiter cell', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['hello world         '], COLS);
    // col 5 is the space between 'hello' and 'world'
    sm.selectWordAt(5, 0, getCell, COLS, DELIMITERS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    // The delimiter cell is selected as a single-cell range
    expect(sel!.start.col).toBe(5);
    expect(sel!.end.col).toBe(5);
  });

  it('uses custom delimiters passed as argument', () => {
    const sm = new SelectionManager();
    // With '.' as a delimiter, 'foo.bar' splits into two words
    const getCell = gridFromLines(['foo.bar             '], COLS);
    sm.selectWordAt(0, 0, getCell, COLS, '.');
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start.col).toBe(0);
    expect(sel!.end.col).toBe(2);
    expect(sm.getSelectedText(getCell, COLS)).toBe('foo');
  });

  it('single-character word at start of row', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['a hello             '], COLS);
    sm.selectWordAt(0, 0, getCell, COLS, DELIMITERS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start).toEqual({ row: 0, col: 0 });
    expect(sel!.end).toEqual({ row: 0, col: 0 });
    expect(sm.getSelectedText(getCell, COLS)).toBe('a');
  });
});

// ---------------------------------------------------------------------------
// TEST-CLIP-004b — selectWordAt edge: word at right boundary
// ---------------------------------------------------------------------------
describe('TEST-CLIP-004b: selectWordAt — word touching right edge', () => {
  const DELIMITERS = ' \t|"\'`&()*,;<=>[]{}~';

  it('word touching the right edge of the line (no trailing space)', () => {
    // Edge case: word runs all the way to col COLS-1
    const COLS_EDGE = 5;
    const sm = new SelectionManager();
    // 'hello' exactly fills a 5-column line
    const getCell = gridFromLines(['hello'], COLS_EDGE);
    sm.selectWordAt(4, 0, getCell, COLS_EDGE, DELIMITERS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start.col).toBe(0);
    expect(sel!.end.col).toBe(4);
    expect(sm.getSelectedText(getCell, COLS_EDGE)).toBe('hello');
  });
});

// ---------------------------------------------------------------------------
// TEST-CLIP-005 — selectLineAt (FS-CLIP-003)
// ---------------------------------------------------------------------------
describe('TEST-CLIP-005: selectLineAt', () => {
  const COLS = 20;

  it('selects from col 0 to last col on the given row', () => {
    const sm = new SelectionManager();
    sm.selectLineAt(0, COLS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start).toEqual({ row: 0, col: 0 });
    expect(sel!.end).toEqual({ row: 0, col: COLS - 1 });
  });

  it('extracts the full line text (trailing spaces trimmed)', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(['hello world         '], COLS);
    sm.selectLineAt(0, COLS);
    expect(sm.getSelectedText(getCell, COLS)).toBe('hello world');
  });

  it('selects the correct row when multiple rows exist', () => {
    const sm = new SelectionManager();
    const getCell = gridFromLines(
      ['first line          ', 'second line         ', 'third line          '],
      COLS,
    );
    sm.selectLineAt(1, COLS);
    expect(sm.getSelectedText(getCell, COLS)).toBe('second line');
  });

  it('selection range row matches the requested row', () => {
    const sm = new SelectionManager();
    sm.selectLineAt(3, COLS);
    const sel = sm.getSelection();
    expect(sel).not.toBeNull();
    expect(sel!.start.row).toBe(3);
    expect(sel!.end.row).toBe(3);
  });
});
