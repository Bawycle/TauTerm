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
   * When true, a zero-length selection (anchor === active) is treated as
   * a valid single-cell selection. This is set by selectWordAt / selectLineAt
   * (programmatic selections) and cleared on drag start.
   */
  #allowSingleCell = false;

  /**
   * Start a new selection at the given cell position.
   * Clears any existing selection.
   */
  startSelection(pos: CellPosition): void {
    this.#anchor = { ...pos };
    this.#active = { ...pos };
    this.#allowSingleCell = false;
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
    this.#allowSingleCell = false;
  }

  /**
   * Return the current selection range (normalized, start ≤ end),
   * or null if there is no active selection.
   *
   * A zero-length selection (anchor === active) returns null for drag-based
   * selections (a click without drag does not constitute a selection).
   * Programmatic selections (selectWordAt, selectLineAt) may return a
   * single-cell range when #allowSingleCell is true.
   */
  getSelection(): SelectionRange | null {
    if (this.#anchor === null || this.#active === null) return null;
    if (this.#anchor.row === this.#active.row && this.#anchor.col === this.#active.col) {
      if (!this.#allowSingleCell) return null;
      // Single-cell programmatic selection: return a range covering exactly that cell.
      return { start: { ...this.#anchor }, end: { ...this.#active } };
    }
    return normalize(this.#anchor, this.#active);
  }

  /**
   * Select the word at (row, col) using the provided delimiter set.
   * (FS-CLIP-002)
   *
   * A "word" is a maximal run of non-delimiter characters surrounding the
   * clicked cell. If the clicked cell is itself a delimiter, only that single
   * cell is selected.
   *
   * @param col - Column of the clicked cell.
   * @param row - Row of the clicked cell.
   * @param getCell - Callback returning the character at (row, col).
   * @param cols - Terminal width (number of columns).
   * @param wordDelimiters - String of characters that act as word boundaries.
   *   Default mirrors the Rust backend default.
   */
  selectWordAt(
    col: number,
    row: number,
    getCell: GetCellFn,
    cols: number,
    wordDelimiters: string = ' \t|"\'`&()*,;<=>[]{}~',
  ): void {
    this.#allowSingleCell = true;
    const clickedChar = getCell(row, col);
    // If the clicked cell is a delimiter or a wide-char continuation, select
    // only that single cell.
    if (wordDelimiters.includes(clickedChar) || clickedChar === '') {
      this.#anchor = { row, col };
      this.#active = { row, col };
      return;
    }

    // Scan left to find word start.
    let start = col;
    while (start > 0) {
      const ch = getCell(row, start - 1);
      if (wordDelimiters.includes(ch) || ch === '') break;
      start--;
    }

    // Scan right to find word end.
    let end = col;
    while (end < cols - 1) {
      const ch = getCell(row, end + 1);
      if (wordDelimiters.includes(ch) || ch === '') break;
      end++;
    }

    this.#anchor = { row, col: start };
    this.#active = { row, col: end };
  }

  /**
   * Select the entire row at `row`.
   * (FS-CLIP-003)
   *
   * Sets anchor to col 0 and active to the last column so the full row
   * is covered regardless of trailing spaces (those are trimmed at extraction
   * time by `getSelectedText`).
   *
   * @param row - Row index to select.
   * @param cols - Terminal width (number of columns).
   */
  selectLineAt(row: number, cols: number): void {
    this.#allowSingleCell = true;
    this.#anchor = { row, col: 0 };
    this.#active = { row, col: cols - 1 };
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
