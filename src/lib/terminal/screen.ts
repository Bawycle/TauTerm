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

import type { SnapshotCell, CellUpdate, CellAttrsDto, Color, ScreenUpdateEvent } from '$lib/ipc';
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
  // P-OPT-2: direct string accumulation — avoids allocating a temporary string[]
  // and the join() call. JS engines optimise short string concatenation efficiently.
  const fg = cell.inverse ? (cell.bg ?? 'var(--term-bg)') : cell.fg;
  const bg = cell.inverse ? (cell.fg ?? 'var(--term-fg)') : cell.bg;
  let s = '';
  if (fg) s = `color:${fg}`;
  if (bg) s += (s ? ';' : '') + `background-color:${bg}`;
  if (cell.bold) s += (s ? ';' : '') + 'font-weight:bold';
  if (cell.italic) s += (s ? ';' : '') + 'font-style:italic';
  if (cell.dim) s += (s ? ';' : '') + 'opacity:var(--term-dim-opacity)';
  if (cell.hidden) s += (s ? ';' : '') + 'color:transparent';

  // Build text-decoration (F6 — extended underline styles SGR 4:1–4:5).
  // F9: strikethrough is rendered via .terminal-pane__cell--strikethrough CSS class
  // (::after pseudo-element at 50% height) — not via text-decoration: line-through.
  if (cell.underline > 0) {
    s += (s ? ';' : '') + 'text-decoration-line:underline';
    const underlineStyle = UNDERLINE_STYLE_MAP[cell.underline];
    if (underlineStyle) s += `;text-decoration-style:${underlineStyle}`;
    s += `;text-decoration-color:${cell.underlineColor ?? 'var(--term-underline-color-default)'}`;
  }

  return s;
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
    hyperlink: cell.hyperlink ?? undefined,
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
  hyperlink?: string | null,
): CellStyle {
  // Apply bold color promotion: ColorDto 'default' variant is not a Color,
  // so we only promote when the fg is a non-default ANSI color.
  // The cast is safe: ColorDto minus 'default' is structurally identical to Color.
  const rawFg = attrs.fg;
  const bold = attrs.bold ?? false;
  const promotedFg =
    rawFg && rawFg.type !== 'default' ? resolveAnsiColor(rawFg as Color, bold) : rawFg;
  const fg = resolveColorDto(promotedFg);
  const bg = resolveColorDto(attrs.bg);
  const underlineColor = resolveColorDto(attrs.underlineColor);
  const result: CellStyle = {
    content,
    fg,
    bg,
    width,
    bold,
    dim: attrs.dim ?? false,
    italic: attrs.italic ?? false,
    underline: attrs.underline ?? 0,
    blink: attrs.blink ?? false,
    inverse: attrs.inverse ?? false,
    hidden: attrs.hidden ?? false,
    strikethrough: attrs.strikethrough ?? false,
    underlineColor,
    hyperlink: hyperlink ?? undefined,
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

  // Apply inverse: swap fg and bg, falling back to terminal tokens for defaults.
  const fg = cell.inverse ? (cell.bg ?? 'var(--term-bg)') : cell.fg;
  const bg = cell.inverse ? (cell.fg ?? 'var(--term-fg)') : cell.bg;

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
/**
 * Default blank cell — used to pad when the snapshot is smaller than rows×cols.
 */
function defaultCell(): CellStyle {
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

export function buildGridFromSnapshot(
  cells: SnapshotCell[],
  rows: number,
  cols: number,
): CellStyle[] {
  const total = rows * cols;
  // Build directly from snapshot data — avoids allocating a full default grid
  // that would be immediately overwritten (A6 opt: ~rows*cols allocations saved).
  const grid: CellStyle[] = cells.slice(0, total).map(cellStyleFromSnapshot);
  // Pad with default cells if the snapshot is smaller than the viewport.
  while (grid.length < total) grid.push(defaultCell());
  return grid;
}

// ---------------------------------------------------------------------------
// P-HT-1: Merge multiple ScreenUpdateEvents into one (rAF coalescing)
// ---------------------------------------------------------------------------

/**
 * Merge a non-empty batch of ScreenUpdateEvent into a single event.
 *
 * Algorithm:
 * 1. Empty batch → throw (caller must guarantee non-empty).
 * 2. Single event → return as-is (fast-path, zero allocation).
 * 3. Group by scrollOffset. Heterogeneous scrollOffsets are merged per-group
 *    and the results returned as an array (caller applies sequentially).
 *    In practice, a single rAF tick almost never mixes scrollOffsets.
 * 4. Within a homogeneous group: discard all events before the last
 *    isFullRedraw (a full-redraw carries the entire screen state).
 * 5. Merge CellUpdate[] into a Map keyed by `row * cols + col` (last-write wins).
 *    The `cols` value comes from the LAST event in the working slice.
 *    Invariant: a resize always produces isFullRedraw=true, so pre-resize events
 *    are discarded at step 4 — cols are always consistent within the working slice.
 * 6. Scalars (cursor, scrollbackLines, scrollOffset, cols, rows) from the LAST event.
 * 7. isFullRedraw = true if any event in the working slice has isFullRedraw=true.
 *
 * For heterogeneous scrollOffset batches, returns an array of merged groups.
 * For homogeneous batches (the common case), returns a single ScreenUpdateEvent.
 */
export function mergeScreenUpdates(
  batch: ScreenUpdateEvent[],
): ScreenUpdateEvent | ScreenUpdateEvent[] {
  if (batch.length === 0) {
    throw new Error('mergeScreenUpdates: batch must be non-empty');
  }
  if (batch.length === 1) {
    return batch[0];
  }

  // Check for heterogeneous scrollOffsets.
  const firstOffset = batch[0].scrollOffset;
  let heterogeneous = false;
  for (let i = 1; i < batch.length; i++) {
    if (batch[i].scrollOffset !== firstOffset) {
      heterogeneous = true;
      break;
    }
  }

  if (heterogeneous) {
    // Group by scrollOffset preserving order, merge each group independently.
    const groups: ScreenUpdateEvent[][] = [];
    let currentGroup: ScreenUpdateEvent[] = [batch[0]];
    let currentOffset = batch[0].scrollOffset;
    for (let i = 1; i < batch.length; i++) {
      if (batch[i].scrollOffset === currentOffset) {
        currentGroup.push(batch[i]);
      } else {
        groups.push(currentGroup);
        currentGroup = [batch[i]];
        currentOffset = batch[i].scrollOffset;
      }
    }
    groups.push(currentGroup);

    // Merge each group and return as array for sequential application.
    return groups.map((g) => mergeHomogeneous(g));
  }

  return mergeHomogeneous(batch);
}

/**
 * Merge a homogeneous batch (all events share the same scrollOffset).
 * Always returns a single ScreenUpdateEvent.
 */
function mergeHomogeneous(batch: ScreenUpdateEvent[]): ScreenUpdateEvent {
  if (batch.length === 1) return batch[0];

  // Find the last full-redraw index.
  let lastFullIdx = -1;
  for (let i = batch.length - 1; i >= 0; i--) {
    if (batch[i].isFullRedraw) {
      lastFullIdx = i;
      break;
    }
  }

  // Working slice: from lastFullIdx (or 0 if no full-redraw) to end.
  const startIdx = lastFullIdx >= 0 ? lastFullIdx : 0;
  const last = batch[batch.length - 1];
  const mergedCols = last.cols;

  // Merge cells: last-write wins via Map keyed by row * mergedCols + col.
  const cellMap = new Map<number, CellUpdate>();
  for (let i = startIdx; i < batch.length; i++) {
    for (const cell of batch[i].cells) {
      cellMap.set(cell.row * mergedCols + cell.col, cell);
    }
  }

  // Determine isFullRedraw: true if any event in working slice is full-redraw.
  let isFullRedraw = false;
  for (let i = startIdx; i < batch.length; i++) {
    if (batch[i].isFullRedraw) {
      isFullRedraw = true;
      break;
    }
  }

  return {
    paneId: last.paneId,
    cells: Array.from(cellMap.values()),
    cursor: last.cursor,
    scrollbackLines: last.scrollbackLines,
    isFullRedraw,
    scrollOffset: last.scrollOffset,
    cols: last.cols,
    rows: last.rows,
  };
}
