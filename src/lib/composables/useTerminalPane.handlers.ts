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
import { computeCellStyle } from '$lib/terminal/screen.js';

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
    style: '',
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
 * Delegates to computeCellStyle from screen.ts (canonical implementation — P11).
 * Kept as a re-export for callers that already import from this module.
 */
export function cellStyle(cell: CellStyle): string {
  return computeCellStyle(cell);
}
