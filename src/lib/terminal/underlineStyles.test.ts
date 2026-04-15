// SPDX-License-Identifier: MPL-2.0

/**
 * F6 — Extended underline styles (SGR 4:2–4:5)
 *
 * cell.underline is a number 0–5. Each value must produce distinct CSS.
 *
 * cellToCssVars() is the canonical source tested here.
 * cellStyle() (in useTerminalPane) mirrors the same logic and is covered
 * by the same expectations pattern.
 *
 * Covered:
 *   ULINE-001 — underline=0 → no text-decoration-line set
 *   ULINE-002 — underline=1 → text-decoration-line: underline, no style
 *   ULINE-003 — underline=2 → double style
 *   ULINE-004 — underline=3 → wavy style
 *   ULINE-005 — underline=4 → dotted style
 *   ULINE-006 — underline=5 → dashed style
 *   ULINE-COLOR-001 — underlineColor set → text-decoration-color resolves to color
 *   ULINE-COLOR-002 — underlineColor absent → text-decoration-color: var(--term-underline-color-default)
 */

import { describe, it, expect } from 'vitest';
import { cellToCssVars, cellStyleFromSnapshot } from './screen.js';
import type { SnapshotCell } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

function makeCell(overrides: Partial<SnapshotCell> = {}): ReturnType<typeof cellStyleFromSnapshot> {
  return cellStyleFromSnapshot({
    content: 'A',
    width: 1,
    bold: false,
    dim: false,
    italic: false,
    underline: 0,
    blink: false,
    inverse: false,
    hidden: false,
    strikethrough: false,
    fg: null,
    bg: null,
    underlineColor: null,
    hyperlink: null,
    ...overrides,
  });
}

// ---------------------------------------------------------------------------
// ULINE-001: underline=0 → no underline decoration
// ---------------------------------------------------------------------------

describe('ULINE-001: underline=0 → no underline in style', () => {
  it('text-decoration-line is absent when underline=0', () => {
    const cell = makeCell({ underline: 0 });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-line']).toBeUndefined();
    expect(style['text-decoration-style']).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// ULINE-002: underline=1 → single underline (default style)
// ---------------------------------------------------------------------------

describe('ULINE-002: underline=1 → single underline', () => {
  it('text-decoration-line is "underline", no text-decoration-style', () => {
    const cell = makeCell({ underline: 1 });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-line']).toContain('underline');
    expect(style['text-decoration-style']).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// ULINE-003: underline=2 → double
// ---------------------------------------------------------------------------

describe('ULINE-003: underline=2 → double underline', () => {
  it('text-decoration-style is "double"', () => {
    const cell = makeCell({ underline: 2 });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-line']).toContain('underline');
    expect(style['text-decoration-style']).toBe('double');
  });
});

// ---------------------------------------------------------------------------
// ULINE-004: underline=3 → wavy
// ---------------------------------------------------------------------------

describe('ULINE-004: underline=3 → wavy underline', () => {
  it('text-decoration-style is "wavy"', () => {
    const cell = makeCell({ underline: 3 });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-line']).toContain('underline');
    expect(style['text-decoration-style']).toBe('wavy');
  });
});

// ---------------------------------------------------------------------------
// ULINE-005: underline=4 → dotted
// ---------------------------------------------------------------------------

describe('ULINE-005: underline=4 → dotted underline', () => {
  it('text-decoration-style is "dotted"', () => {
    const cell = makeCell({ underline: 4 });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-line']).toContain('underline');
    expect(style['text-decoration-style']).toBe('dotted');
  });
});

// ---------------------------------------------------------------------------
// ULINE-006: underline=5 → dashed
// ---------------------------------------------------------------------------

describe('ULINE-006: underline=5 → dashed underline', () => {
  it('text-decoration-style is "dashed"', () => {
    const cell = makeCell({ underline: 5 });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-line']).toContain('underline');
    expect(style['text-decoration-style']).toBe('dashed');
  });
});

// ---------------------------------------------------------------------------
// ULINE-COLOR-001: underlineColor set → text-decoration-color resolved
// ---------------------------------------------------------------------------

describe('ULINE-COLOR-001: underlineColor present → text-decoration-color resolved', () => {
  it('underlineColor=rgb → text-decoration-color is the resolved CSS value', () => {
    const cell = makeCell({
      underline: 1,
      underlineColor: { type: 'rgb', r: 255, g: 0, b: 128 },
    });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-color']).toBe('rgb(255,0,128)');
  });

  it('underlineColor=ansi → text-decoration-color is the CSS var', () => {
    const cell = makeCell({
      underline: 2,
      underlineColor: { type: 'ansi', index: 1 },
    });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-color']).toBe('var(--term-color-1)');
  });
});

// ---------------------------------------------------------------------------
// ULINE-COLOR-002: no underlineColor → fallback token
// ---------------------------------------------------------------------------

describe('ULINE-COLOR-002: underlineColor absent → fallback token', () => {
  it('text-decoration-color is var(--term-underline-color-default) when no underlineColor', () => {
    const cell = makeCell({ underline: 1 });
    const style = cellToCssVars(cell);
    expect(style['text-decoration-color']).toBe('var(--term-underline-color-default)');
  });
});

// ---------------------------------------------------------------------------
// Strikethrough alongside underline (F9 — CSS class rendering, not text-decoration)
// ---------------------------------------------------------------------------

describe('strikethrough + underline coexist (F9)', () => {
  it('text-decoration-line contains underline but NOT line-through when both are active', () => {
    // F9: strikethrough is rendered via .terminal-pane__cell--strikethrough CSS class
    // (::after pseudo-element at 50% height). It must NOT appear in text-decoration-line.
    const cell = makeCell({ underline: 1, strikethrough: true });
    const style = cellToCssVars(cell);
    const decorLine = style['text-decoration-line'] ?? '';
    expect(decorLine).toContain('underline');
    expect(decorLine).not.toContain('line-through');
  });
});
