// SPDX-License-Identifier: MPL-2.0

/**
 * Pure helper functions extracted from useTerminalPane.svelte.ts.
 *
 * All functions here are stateless: they take all their inputs as arguments
 * and return a result — no $state, no $derived, no $effect.
 * This makes them independently testable and keeps the composable focused on
 * reactive orchestration.
 */

import type { CellStyle } from '$lib/terminal/screen.js';

// ---------------------------------------------------------------------------
// Cell helpers
// ---------------------------------------------------------------------------

/** Return a blank terminal cell with all attributes at their default values. */
export function defaultCell(): CellStyle {
  return {
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
  };
}

/**
 * Build a full 2D grid of CellStyle from a flat array.
 *
 * @param r       Number of rows
 * @param c       Number of columns
 * @param grid    Flat source array (row-major order)
 * @returns       2D array [row][col] → CellStyle
 */
export function buildFullGridRows(r: number, c: number, grid: CellStyle[]): CellStyle[][] {
  return Array.from({ length: r }, (_, row) =>
    Array.from({ length: c }, (_, col) => grid[row * c + col] ?? defaultCell()),
  );
}

// ---------------------------------------------------------------------------
// Mouse helpers
// ---------------------------------------------------------------------------

/**
 * Convert a pixel-level MouseEvent position to a terminal cell coordinate.
 *
 * @param event       The mouse event
 * @param viewportEl  The terminal viewport element
 * @param cols        Current number of columns
 * @param rows        Current number of rows
 */
export function pixelToCell(
  event: MouseEvent,
  viewportEl: HTMLDivElement | undefined,
  cols: number,
  rows: number,
): { row: number; col: number } {
  if (!viewportEl) return { row: 0, col: 0 };
  const rect = viewportEl.getBoundingClientRect();
  const cw = Math.max(1, rect.width / cols);
  const ch = Math.max(1, rect.height / rows);
  return {
    col: Math.max(0, Math.min(cols - 1, Math.floor((event.clientX - rect.left) / cw))),
    row: Math.max(0, Math.min(rows - 1, Math.floor((event.clientY - rect.top) / ch))),
  };
}

/**
 * Map a MouseEvent.button value to the VT mouse button code.
 *
 * Returns 0 (left), 1 (middle), 2 (right), or 3 (other).
 */
export function mouseButtonCode(event: MouseEvent): number {
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

// ---------------------------------------------------------------------------
// Cell style rendering
// ---------------------------------------------------------------------------

/**
 * Compute the inline `style` string for a single terminal cell.
 *
 * Handles SGR attributes: color (fg/bg), weight, italic, dim, hidden, and
 * extended underline styles (SGR 4:1–4:5, F6). Strikethrough is rendered via
 * a CSS class (F9), not via text-decoration, so it is not included here.
 */
export function cellStyle(cell: CellStyle): string {
  const parts: string[] = [];
  const fg = cell.inverse ? cell.bg : cell.fg;
  const bg = cell.inverse ? cell.fg : cell.bg;
  if (fg) parts.push(`color:${fg}`);
  if (bg) parts.push(`background-color:${bg}`);
  if (cell.bold) parts.push('font-weight:bold');
  if (cell.italic) parts.push('font-style:italic');
  if (cell.dim) parts.push('opacity:var(--term-dim-opacity)');
  if (cell.hidden) parts.push('color:transparent');

  // Build text-decoration (F6 — extended underline styles SGR 4:1–4:5).
  // F9: strikethrough is rendered via .terminal-pane__cell--strikethrough CSS class
  // (::after pseudo-element at 50% height) — not via text-decoration: line-through.
  const decLines: string[] = [];
  if (cell.underline > 0) decLines.push('underline');
  if (decLines.length) parts.push(`text-decoration-line:${decLines.join(' ')}`);

  if (cell.underline > 0) {
    const underlineStyleMap: Record<number, string> = {
      2: 'double',
      3: 'wavy',
      4: 'dotted',
      5: 'dashed',
    };
    const underlineStyle = underlineStyleMap[cell.underline];
    if (underlineStyle) parts.push(`text-decoration-style:${underlineStyle}`);
    const underlineColor = cell.underlineColor ?? 'var(--term-underline-color-default)';
    parts.push(`text-decoration-color:${underlineColor}`);
  }

  return parts.join(';');
}
