// SPDX-License-Identifier: MPL-2.0

import { describe, it, expect } from 'vitest';
import {
  cellStyleFromSnapshot,
  cellStyleFromUpdate,
  cellToCssVars,
  buildGridFromSnapshot,
  applyUpdates,
} from './screen.js';
import type { SnapshotCell, CellUpdate, CellAttrsDto } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeSnapshotCell(overrides: Partial<SnapshotCell> = {}): SnapshotCell {
  return {
    content: ' ',
    width: 1,
    bold: false,
    dim: false,
    italic: false,
    underline: 0,
    blink: false,
    inverse: false,
    hidden: false,
    strikethrough: false,
    ...overrides,
  };
}

function makeAttrs(overrides: Partial<CellAttrsDto> = {}): CellAttrsDto {
  return {
    bold: false,
    dim: false,
    italic: false,
    underline: 0,
    blink: false,
    inverse: false,
    hidden: false,
    strikethrough: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// TUITC-FN-010 to 016: Cell attribute rendering
// ---------------------------------------------------------------------------
describe('TUITC-FN-010: bold attribute', () => {
  it('bold=true → cell.bold is true', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ bold: true }));
    expect(cell.bold).toBe(true);
  });
  it('bold=false → cell.bold is false', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ bold: false }));
    expect(cell.bold).toBe(false);
  });
});

describe('TUITC-FN-011: italic attribute', () => {
  it('italic=true → cell.italic is true', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ italic: true }));
    expect(cell.italic).toBe(true);
  });
});

describe('TUITC-FN-012: dim attribute', () => {
  it('dim=true → cell.dim is true', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ dim: true }));
    expect(cell.dim).toBe(true);
  });
});

describe('TUITC-FN-013: underline attribute', () => {
  it('underline=1 (single) → cell.underline is 1', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ underline: 1 }));
    expect(cell.underline).toBe(1);
  });
  it('underline=3 (curly) → cell.underline is 3', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ underline: 3 }));
    expect(cell.underline).toBe(3);
  });
});

describe('TUITC-FN-014: inverse attribute', () => {
  it('inverse=true → cell.inverse is true', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ inverse: true }));
    expect(cell.inverse).toBe(true);
  });
});

describe('TUITC-FN-015: hidden attribute', () => {
  it('hidden=true → cell.hidden is true', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ hidden: true }));
    expect(cell.hidden).toBe(true);
  });
});

describe('TUITC-FN-016: strikethrough attribute', () => {
  it('strikethrough=true → cell.strikethrough is true', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ strikethrough: true }));
    expect(cell.strikethrough).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-020: ANSI color via cellStyleFromSnapshot
// ---------------------------------------------------------------------------
describe('TUITC-FN-020: ANSI fg color resolves to CSS var', () => {
  it('fg ansi index 1 → var(--term-color-1)', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ fg: { type: 'ansi', index: 1 } }));
    expect(cell.fg).toBe('var(--term-color-1)');
  });
});

describe('TUITC-FN-022: Truecolor fg resolves to rgb string', () => {
  it('fg rgb(255,100,0) → "rgb(255,100,0)"', () => {
    const cell = cellStyleFromSnapshot(
      makeSnapshotCell({ fg: { type: 'rgb', r: 255, g: 100, b: 0 } }),
    );
    expect(cell.fg).toBe('rgb(255,100,0)');
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-023/024: Default color → undefined
// ---------------------------------------------------------------------------
describe('TUITC-FN-023/024: absent fg/bg → undefined (CSS inheritance)', () => {
  it('no fg → cell.fg is undefined', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell());
    expect(cell.fg).toBeUndefined();
  });
  it('no bg → cell.bg is undefined', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell());
    expect(cell.bg).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-030: Wide character width=2
// ---------------------------------------------------------------------------
describe('TUITC-FN-030: wide character width', () => {
  it('width=2 → cell.width is 2', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ width: 2, content: '中' }));
    expect(cell.width).toBe(2);
    expect(cell.content).toBe('中');
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-031: Combining character width=0
// ---------------------------------------------------------------------------
describe('TUITC-FN-031: combining character width=0', () => {
  it('width=0 → cell.width is 0', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ width: 0, content: '\u0301' }));
    expect(cell.width).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-070: buildGridFromSnapshot / applyUpdates
// ---------------------------------------------------------------------------
describe('TUITC-FN-071: buildGridFromSnapshot creates correct grid size', () => {
  it('2×4 grid → 8 cells', () => {
    const cells = Array.from({ length: 8 }, (_, i) => makeSnapshotCell({ content: String(i) }));
    const grid = buildGridFromSnapshot(cells, 2, 4);
    expect(grid.length).toBe(8);
    expect(grid[0].content).toBe('0');
    expect(grid[7].content).toBe('7');
  });

  it('empty snapshot → grid filled with default blank cells', () => {
    const grid = buildGridFromSnapshot([], 3, 5);
    expect(grid.length).toBe(15);
    expect(grid[0].content).toBe(' ');
  });
});

describe('TUITC-FN-070: applyUpdates patches only changed cells', () => {
  it('single cell update patches correct index', () => {
    const grid = buildGridFromSnapshot([], 2, 3);
    // Initial: all content ' '
    expect(grid[4].content).toBe(' '); // row 1 col 1 → index 4

    const update: CellUpdate = {
      row: 1,
      col: 1,
      content: 'X',
      width: 1,
      attrs: makeAttrs({ bold: true }),
    };
    applyUpdates(grid, [update], 3);

    expect(grid[4].content).toBe('X');
    expect(grid[4].bold).toBe(true);
    // Other cells unchanged
    expect(grid[0].content).toBe(' ');
  });

  it('out-of-bounds update is ignored gracefully', () => {
    const grid = buildGridFromSnapshot([], 2, 3);
    const badUpdate: CellUpdate = {
      row: 99,
      col: 99,
      content: 'Z',
      width: 1,
      attrs: makeAttrs(),
    };
    // Should not throw
    expect(() => applyUpdates(grid, [badUpdate], 3)).not.toThrow();
    // Grid unchanged
    expect(grid[0].content).toBe(' ');
  });
});

// ---------------------------------------------------------------------------
// TUITC-SEC-001/002: Cell content is plain text — no HTML parsing risk
// ---------------------------------------------------------------------------
describe('TUITC-SEC-001/002: cell content treated as opaque text', () => {
  it('XSS payload stored as literal string, not parsed', () => {
    const xss = '<script>alert(1)</script>';
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ content: xss }));
    // Content is stored as-is — rendering code uses textContent, not innerHTML
    expect(cell.content).toBe(xss);
    // Critically: the content is a string, not a DOM fragment
    expect(typeof cell.content).toBe('string');
  });

  it('HTML entity in content stored as literal characters', () => {
    const htmlEntity = '&lt;img src=x onerror=evil()&gt;';
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ content: htmlEntity }));
    expect(cell.content).toBe(htmlEntity);
  });
});

// ---------------------------------------------------------------------------
// cellStyleFromUpdate: CellAttrsDto (with ColorDto default variant)
// ---------------------------------------------------------------------------
describe('cellStyleFromUpdate: handles ColorDto default variant', () => {
  it('fg with type=default → cell.fg is undefined', () => {
    const cell = cellStyleFromUpdate('A', makeAttrs({ fg: { type: 'default' } }), 1);
    expect(cell.fg).toBeUndefined();
  });

  it('fg with type=rgb → resolved rgb string', () => {
    const cell = cellStyleFromUpdate(
      'A',
      makeAttrs({ fg: { type: 'rgb', r: 0, g: 128, b: 255 } }),
      1,
    );
    expect(cell.fg).toBe('rgb(0,128,255)');
  });

  it('bold=true from CellUpdate', () => {
    const cell = cellStyleFromUpdate('A', makeAttrs({ bold: true }), 1);
    expect(cell.bold).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// P3 — Token usage: cellToCssVars must use CSS custom properties, not hardcoded values
// ---------------------------------------------------------------------------
describe('P3-TOKEN-001: cellToCssVars uses CSS tokens for dim opacity', () => {
  it('dim=true → opacity is var(--term-dim-opacity), not "0.5"', () => {
    const dimCell = cellStyleFromSnapshot(makeSnapshotCell({ dim: true }));
    const vars = cellToCssVars(dimCell);
    expect(vars['opacity']).toBe('var(--term-dim-opacity)');
    expect(vars['opacity']).not.toBe('0.5');
  });

  it('dim=false → opacity key is absent from style vars', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ dim: false }));
    const vars = cellToCssVars(cell);
    expect(vars['opacity']).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// F9 — Strikethrough: cellToCssVars must NOT produce text-decoration line-through
// The --strikethrough class is handled via CSS ::after pseudo-element in TerminalPane.
// ---------------------------------------------------------------------------
describe('F9-001: cellToCssVars does not emit text-decoration line-through for strikethrough', () => {
  it('strikethrough=true → text-decoration-line does not contain "line-through"', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ strikethrough: true }));
    const vars = cellToCssVars(cell);
    const decorLine = vars['text-decoration-line'] ?? '';
    expect(decorLine).not.toContain('line-through');
  });

  it('strikethrough=true without underline → text-decoration-line absent', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ strikethrough: true, underline: 0 }));
    const vars = cellToCssVars(cell);
    expect(vars['text-decoration-line']).toBeUndefined();
  });

  it('strikethrough=true with underline=1 → text-decoration-line contains only "underline"', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ strikethrough: true, underline: 1 }));
    const vars = cellToCssVars(cell);
    expect(vars['text-decoration-line']).toBe('underline');
  });
});

// ---------------------------------------------------------------------------
// F9 — cellStyle (string form in composable) must NOT include line-through for strikethrough
// ---------------------------------------------------------------------------
// Note: cellStyle is tested indirectly via screen.ts exports; the composable's
// cellStyle() is a parallel implementation. We test cellToCssVars here since
// it is the exported testable surface. The composable's cellStyle() is verified
// by the F9 source-order test in TerminalPane.test.ts.

// ---------------------------------------------------------------------------
// F4 — Blink attribute preserved through cellStyleFromSnapshot / cellStyleFromUpdate
// ---------------------------------------------------------------------------
describe('F4-001: blink attribute preserved through snapshot and update paths', () => {
  it('blink=true from snapshot → cell.blink is true', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ blink: true }));
    expect(cell.blink).toBe(true);
  });

  it('blink=false from snapshot → cell.blink is false', () => {
    const cell = cellStyleFromSnapshot(makeSnapshotCell({ blink: false }));
    expect(cell.blink).toBe(false);
  });

  it('blink=true from update → cell.blink is true', () => {
    const cell = cellStyleFromUpdate('A', makeAttrs({ blink: true }), 1);
    expect(cell.blink).toBe(true);
  });

  it('blink=false from update → cell.blink is false', () => {
    const cell = cellStyleFromUpdate('A', makeAttrs({ blink: false }), 1);
    expect(cell.blink).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-030-b/c/d: cellStyleFromUpdate width propagation (WP2-TS)
// ---------------------------------------------------------------------------
describe('TUITC-FN-030-b: cellStyleFromUpdate with width=2 returns width 2', () => {
  it('wide character update produces width=2 in CellStyle', () => {
    const cell = cellStyleFromUpdate('你', makeAttrs(), 2);
    expect(cell.width).toBe(2);
  });
});

describe('TUITC-FN-030-c: cellStyleFromUpdate with width=0 returns width 0', () => {
  it('phantom continuation cell produces width=0 in CellStyle', () => {
    const cell = cellStyleFromUpdate('', makeAttrs(), 0);
    expect(cell.width).toBe(0);
  });
});

describe('TUITC-FN-030-d: applyUpdates propagates width=2 to grid', () => {
  it('wide-char CellUpdate sets grid cell width to 2', () => {
    const grid = buildGridFromSnapshot([], 1, 3);
    const update: CellUpdate = {
      row: 0,
      col: 0,
      content: '你',
      width: 2,
      attrs: makeAttrs(),
      hyperlink: undefined,
    };
    applyUpdates(grid, [update], 3);
    expect(grid[0].width).toBe(2);
    expect(grid[0].content).toBe('你');
  });
});
