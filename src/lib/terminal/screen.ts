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

import type { SnapshotCell, CellUpdate, CellAttrsDto, Color } from '$lib/ipc/types';
import { resolveColorDto, resolveColor, resolveAnsiColor } from './color.js';

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
  /**
   * Pre-computed inline style string for this cell (P11).
   * Computed once at construction time by computeCellStyle(); consumed directly
   * by the template to avoid per-render function calls.
   */
  style: string;
}

// ---------------------------------------------------------------------------
// Cell style string computation (P11)
// ---------------------------------------------------------------------------

const UNDERLINE_STYLE_MAP: Readonly<Record<number, string>> = Object.freeze({
  2: 'double',
  3: 'wavy',
  4: 'dotted',
  5: 'dashed',
});

/**
 * Compute the inline `style` string for a single terminal cell.
 *
 * Handles SGR attributes: color (fg/bg), weight, italic, dim, hidden, and
 * extended underline styles (SGR 4:1–4:5, F6). Strikethrough is rendered via
 * a CSS class (F9), not via text-decoration, so it is not included here.
 *
 * This is the canonical implementation — useTerminalPane.handlers.ts delegates
 * to this function to avoid duplication.
 */
export function computeCellStyle(cell: {
  fg: string | undefined;
  bg: string | undefined;
  bold: boolean;
  italic: boolean;
  dim: boolean;
  hidden: boolean;
  underline: number;
  underlineColor: string | undefined;
  inverse: boolean;
}): string {
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
  if (cell.underline > 0) {
    parts.push('text-decoration-line:underline');
    const underlineStyle = UNDERLINE_STYLE_MAP[cell.underline];
    if (underlineStyle) parts.push(`text-decoration-style:${underlineStyle}`);
    const underlineColor = cell.underlineColor ?? 'var(--term-underline-color-default)';
    parts.push(`text-decoration-color:${underlineColor}`);
  }

  return parts.join(';');
}

/**
 * Build a CellStyle from a SnapshotCell (full snapshot).
 * SnapshotCell.Color has no 'default' variant — absent means use terminal default.
 * Bold color promotion (F5): ANSI fg 1–7 is promoted to 9–15 when bold=true.
 */
export function cellStyleFromSnapshot(cell: SnapshotCell): CellStyle {
  const promotedFg = cell.fg ? resolveAnsiColor(cell.fg, cell.bold) : undefined;
  const fg = resolveColor(promotedFg);
  const bg = resolveColor(cell.bg);
  const underlineColor = resolveColor(cell.underlineColor);
  const result: CellStyle = {
    content: cell.content,
    fg,
    bg,
    width: cell.width,
    bold: cell.bold,
    dim: cell.dim,
    italic: cell.italic,
    underline: cell.underline,
    blink: cell.blink,
    inverse: cell.inverse,
    hidden: cell.hidden,
    strikethrough: cell.strikethrough,
    underlineColor,
    hyperlink: cell.hyperlink,
    style: '',
  };
  result.style = computeCellStyle(result);
  return result;
}

/**
 * Build a CellStyle from a CellUpdate (incremental update event).
 * CellAttrsDto.ColorDto has a 'default' variant.
 * Bold color promotion (F5): ANSI fg 1–7 is promoted to 9–15 when bold=true.
 *
 * @param width - Cell display width from CellUpdate.width (1=normal, 2=wide, 0=phantom).
 */
export function cellStyleFromUpdate(
  content: string,
  attrs: CellAttrsDto,
  width: number,
  hyperlink?: string,
): CellStyle {
  // Apply bold color promotion: ColorDto 'default' variant is not a Color,
  // so we only promote when the fg is a non-default ANSI color.
  // The cast is safe: ColorDto minus 'default' is structurally identical to Color.
  const rawFg = attrs.fg;
  const promotedFg =
    rawFg && rawFg.type !== 'default' ? resolveAnsiColor(rawFg as Color, attrs.bold) : rawFg;
  const fg = resolveColorDto(promotedFg);
  const bg = resolveColorDto(attrs.bg);
  const underlineColor = resolveColorDto(attrs.underlineColor);
  const result: CellStyle = {
    content,
    fg,
    bg,
    width,
    bold: attrs.bold,
    dim: attrs.dim,
    italic: attrs.italic,
    underline: attrs.underline,
    blink: attrs.blink,
    inverse: attrs.inverse,
    hidden: attrs.hidden,
    strikethrough: attrs.strikethrough,
    underlineColor,
    hyperlink,
    style: '',
  };
  result.style = computeCellStyle(result);
  return result;
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
  if (cell.dim) style['opacity'] = 'var(--term-dim-opacity)';

  // Build text-decoration (F6 — extended underline styles SGR 4:1–4:5).
  // F9: strikethrough is rendered via .terminal-pane__cell--strikethrough CSS class
  // (::after pseudo-element at 50% height) — not via text-decoration: line-through.
  const decorLines: string[] = [];
  if (cell.underline > 0) decorLines.push('underline');
  if (decorLines.length > 0) {
    style['text-decoration-line'] = decorLines.join(' ');
  }

  // Underline style — only set when underline is active
  if (cell.underline > 0) {
    const underlineStyle = UNDERLINE_STYLE_MAP[cell.underline];
    if (underlineStyle) style['text-decoration-style'] = underlineStyle;

    // Underline color — resolved or fallback design token
    style['text-decoration-color'] = cell.underlineColor ?? 'var(--term-underline-color-default)';
  }

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
      grid[idx] = cellStyleFromUpdate(update.content, update.attrs, update.width, update.hyperlink);
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
    style: '',
  }));

  for (let i = 0; i < cells.length && i < rows * cols; i++) {
    grid[i] = cellStyleFromSnapshot(cells[i]);
  }

  return grid;
}
