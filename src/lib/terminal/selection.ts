// SPDX-License-Identifier: MPL-2.0

/**
 * Terminal text selection state management.
 *
 * Implements character-cell-based selection per FS-CLIP-001 and FS-CLIP-002.
 * Decoupled from the screen buffer — content is retrieved via a callback.
 *
 * Design notes:
 *  - Selection is defined in logical cell coordinates (row, col).
 *  - The internal anchor/active model supports drag in any direction.
 *  - `getSelectedText` normalizes direction before extracting text.
 *  - Wide characters (width === 2) occupy two cells; the second cell (width === 0
 *    as a continuation) must be skipped during text extraction.
 */

export interface CellPosition {
  row: number;
  col: number;
}

export interface SelectionRange {
  /** Normalized: start is always ≤ end in reading order. */
  start: CellPosition;
  end: CellPosition;
}

/**
 * Callback signature for retrieving cell content.
 * Returns the character at (row, col), or an empty string for blank cells.
 * Returns null / undefined to signal that (row, col) is a wide-char continuation
 * cell (width === 0 in the screen model) — callers should skip it.
 */
export type GetCellFn = (row: number, col: number) => string;

/** Compare two positions: returns negative if a < b, positive if a > b, 0 if equal. */
function comparePositions(a: CellPosition, b: CellPosition): number {
  if (a.row !== b.row) return a.row - b.row;
  return a.col - b.col;
}

/**
 * Normalize a selection so that `start` is always before or equal to `end`
 * in reading order (top-left to bottom-right).
 */
function normalize(anchor: CellPosition, active: CellPosition): SelectionRange {
  if (comparePositions(anchor, active) <= 0) {
    return { start: { ...anchor }, end: { ...active } };
  }
  return { start: { ...active }, end: { ...anchor } };
}

export class SelectionManager {
  /** The position where the selection started (mouse down). */
  #anchor: CellPosition | null = null;
  /** The current active end of the selection (mouse is here). */
  #active: CellPosition | null = null;

  /**
   * Start a new selection at the given cell position.
   * Clears any existing selection.
   */
  startSelection(pos: CellPosition): void {
    this.#anchor = { ...pos };
    this.#active = { ...pos };
  }

  /**
   * Extend the selection to the given position (mouse move / shift+click).
   * Has no effect if no selection has been started.
   */
  extendSelection(pos: CellPosition): void {
    if (this.#anchor === null) return;
    this.#active = { ...pos };
  }

  /**
   * Clear the selection entirely.
   */
  clearSelection(): void {
    this.#anchor = null;
    this.#active = null;
  }

  /**
   * Return the current selection range (normalized, start ≤ end),
   * or null if there is no active selection.
   *
   * A zero-length selection (anchor === active) returns null — a single cell
   * click without drag does not constitute a selection.
   */
  getSelection(): SelectionRange | null {
    if (this.#anchor === null || this.#active === null) return null;
    if (this.#anchor.row === this.#active.row && this.#anchor.col === this.#active.col) {
      return null;
    }
    return normalize(this.#anchor, this.#active);
  }

  /**
   * Extract the text covered by the current selection.
   *
   * @param getCell - Callback returning the character at (row, col).
   *   - Return an empty string `""` for a blank (space) cell.
   *   - Return `"\x00"` or the empty string for wide-char continuation cells
   *     (width === 0 in the screen model). The extractor skips empty strings
   *     mid-cell but includes trailing spaces on each row.
   * @param cols - Number of columns in the terminal (used to build rows).
   *
   * Multi-line selection: rows are joined with `\n`.
   * Trailing spaces on each row are trimmed (standard terminal copy behavior).
   */
  getSelectedText(getCell: GetCellFn, cols: number): string {
    const range = this.getSelection();
    if (range === null) return '';

    const { start, end } = range;
    const lines: string[] = [];

    for (let row = start.row; row <= end.row; row++) {
      const colStart = row === start.row ? start.col : 0;
      const colEnd = row === end.row ? end.col : cols - 1;

      let line = '';
      let col = colStart;
      while (col <= colEnd) {
        const ch = getCell(row, col);
        // Wide-char continuation cells return empty string — skip them
        if (ch === '') {
          col++;
          continue;
        }
        line += ch;
        col++;
      }

      // Trim trailing spaces on each line (standard terminal copy behavior)
      lines.push(line.trimEnd());
    }

    return lines.join('\n');
  }
}
