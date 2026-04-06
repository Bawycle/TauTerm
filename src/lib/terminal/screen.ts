// SPDX-License-Identifier: MPL-2.0

/**
 * Screen state management for TerminalPane.
 *
 * Maintains a mutable grid of rendered cell styles, derived from:
 *  - Initial state: ScreenSnapshot (get_pane_screen_snapshot IPC response)
 *  - Incremental updates: ScreenUpdateEvent (screen-update event)
 *
 * This module is pure TypeScript with no Svelte or DOM dependencies,
 * making it fully unit-testable with vitest.
 *
 * Security: never uses innerHTML or evaluates content as HTML.
 * All content is treated as opaque text (set via textContent).
 */

import type { SnapshotCell, CellUpdate, CellAttrsDto } from '$lib/ipc/types';
import { resolveColorDto, resolveColor } from './color.js';

export interface CellStyle {
  /** Text content of the cell — empty string for blank cells. */
  content: string;
  /** CSS color string for foreground, or undefined for terminal default. */
  fg: string | undefined;
  /** CSS color string for background, or undefined for terminal default. */
  bg: string | undefined;
  /** Width: 1 = normal, 2 = wide (CJK), 0 = continuation (skip render). */
  width: number;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  /** Underline style: 0=none, 1=single, 2=double, 3=curly, 4=dotted, 5=dashed */
  underline: number;
  blink: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
  underlineColor: string | undefined;
  /**
   * OSC 8 hyperlink URI, if any (FS-VT-070–073).
   * Undefined when no hyperlink is active on this cell.
   */
  hyperlink: string | undefined;
}

/**
 * Build a CellStyle from a SnapshotCell (full snapshot).
 * SnapshotCell.Color has no 'default' variant — absent means use terminal default.
 */
export function cellStyleFromSnapshot(cell: SnapshotCell): CellStyle {
  return {
    content: cell.content,
    fg: resolveColor(cell.fg),
    bg: resolveColor(cell.bg),
    width: cell.width,
    bold: cell.bold,
    dim: cell.dim,
    italic: cell.italic,
    underline: cell.underline,
    blink: cell.blink,
    inverse: cell.inverse,
    hidden: cell.hidden,
    strikethrough: cell.strikethrough,
    underlineColor: resolveColor(cell.underlineColor),
    hyperlink: cell.hyperlink,
  };
}

/**
 * Build a CellStyle from a CellUpdate (incremental update event).
 * CellAttrsDto.ColorDto has a 'default' variant.
 */
export function cellStyleFromUpdate(
  content: string,
  attrs: CellAttrsDto,
  hyperlink?: string,
): CellStyle {
  return {
    content,
    fg: resolveColorDto(attrs.fg),
    bg: resolveColorDto(attrs.bg),
    width: 1, // CellUpdate always represents a single-width cell position
    bold: attrs.bold,
    dim: attrs.dim,
    italic: attrs.italic,
    underline: attrs.underline,
    blink: attrs.blink,
    inverse: attrs.inverse,
    hidden: attrs.hidden,
    strikethrough: attrs.strikethrough,
    underlineColor: resolveColorDto(attrs.underlineColor),
    hyperlink,
  };
}

/**
 * Build CSS inline style string for a cell.
 * Returns an object suitable for Svelte's style directive or inline style attribute.
 * NEVER produces HTML — callers set textContent, not innerHTML.
 */
export function cellToCssVars(cell: CellStyle): Record<string, string> {
  const style: Record<string, string> = {};

  // Apply inverse: swap fg and bg
  const fg = cell.inverse ? cell.bg : cell.fg;
  const bg = cell.inverse ? cell.fg : cell.bg;

  if (fg) style['color'] = fg;
  if (bg) style['background-color'] = bg;

  if (cell.bold) style['font-weight'] = 'bold';
  if (cell.italic) style['font-style'] = 'italic';
  if (cell.dim) style['opacity'] = '0.5';

  // Build text-decoration
  const decorations: string[] = [];
  if (cell.underline > 0) decorations.push('underline');
  if (cell.strikethrough) decorations.push('line-through');
  if (decorations.length > 0) style['text-decoration'] = decorations.join(' ');

  if (cell.hidden) style['color'] = 'transparent';

  return style;
}

/**
 * Apply a list of CellUpdate entries to a flat cell grid.
 * The grid is row-major: index = row * cols + col.
 * Mutates `grid` in place.
 */
export function applyUpdates(grid: CellStyle[], updates: CellUpdate[], cols: number): void {
  for (const update of updates) {
    const idx = update.row * cols + update.col;
    if (idx >= 0 && idx < grid.length) {
      grid[idx] = cellStyleFromUpdate(update.content, update.attrs, update.hyperlink);
    }
  }
}

/**
 * Build an initial grid from a ScreenSnapshot.
 * Returns a flat row-major array of CellStyle with rows*cols entries.
 */
export function buildGridFromSnapshot(
  cells: SnapshotCell[],
  rows: number,
  cols: number,
): CellStyle[] {
  const grid: CellStyle[] = new Array(rows * cols).fill(null).map(() => ({
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
  }));

  for (let i = 0; i < cells.length && i < rows * cols; i++) {
    grid[i] = cellStyleFromSnapshot(cells[i]);
  }

  return grid;
}
