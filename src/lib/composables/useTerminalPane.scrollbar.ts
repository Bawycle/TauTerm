// SPDX-License-Identifier: MPL-2.0

/**
 * Pure scrollbar computation helpers extracted from useTerminalPane.svelte.ts.
 *
 * All functions are stateless — they receive their inputs as arguments and
 * return a value. The reactive $derived expressions and $state mutations that
 * drive the scrollbar remain in useTerminalPane.svelte.ts; these helpers
 * implement only the arithmetic so it can be read and tested independently.
 */

// ---------------------------------------------------------------------------
// Thumb geometry
// ---------------------------------------------------------------------------

/**
 * Compute the scrollbar thumb height as a percentage of the track height.
 *
 * Returns 0 when there is no scrollback (nothing to scroll).
 *
 * @param rows           Number of visible terminal rows
 * @param scrollbackLines  Total number of scrollback lines in history
 */
export function scrollbarThumbHeightPct(rows: number, scrollbackLines: number): number {
  if (scrollbackLines <= 0) return 0;
  // Minimum thumb height: 32px out of a 400px (default) track.
  return Math.max((32 / (rows * 16 || 400)) * 100, (rows / (rows + scrollbackLines)) * 100);
}

/**
 * Compute the scrollbar thumb top offset as a percentage of the track height.
 *
 * When `scrollOffset === 0` (bottom of scrollback), the thumb is at the
 * bottom of the track. When `scrollOffset === scrollbackLines` (top), the
 * thumb is at the top.
 *
 * @param rows             Number of visible terminal rows
 * @param scrollbackLines  Total number of scrollback lines in history
 * @param scrollOffset     Current scroll offset (0 = live view / bottom)
 * @param thumbHeightPct   Pre-computed thumb height percentage (avoids re-computing)
 */
export function scrollbarThumbTopPct(
  rows: number,
  scrollbackLines: number,
  scrollOffset: number,
  thumbHeightPct: number,
): number {
  if (scrollbackLines <= 0) return 100 - thumbHeightPct;
  if (scrollOffset > 0) {
    return ((scrollbackLines - scrollOffset) / (scrollbackLines + rows)) * 100;
  }
  return 100 - thumbHeightPct;
}

// ---------------------------------------------------------------------------
// Drag / click-to-jump
// ---------------------------------------------------------------------------

/**
 * Convert a pointer clientY coordinate (on the scrollbar track) to a scroll
 * offset value.
 *
 * @param clientY          Pointer Y position in viewport coordinates
 * @param scrollbarEl      The scrollbar track element
 * @param scrollbackLines  Total scrollback line count
 * @param rows             Number of visible terminal rows
 * @param currentOffset    Current scroll offset (used as fallback if no element)
 * @returns                New scroll offset clamped to [0, scrollbackLines]
 */
export function scrollbarYToOffset(
  clientY: number,
  scrollbarEl: HTMLDivElement | undefined,
  scrollbackLines: number,
  rows: number,
  currentOffset: number,
): number {
  if (!scrollbarEl) return currentOffset;
  const rect = scrollbarEl.getBoundingClientRect();
  const fraction = Math.max(0, Math.min(1, (clientY - rect.top) / rect.height));
  const totalLines = rows + scrollbackLines;
  const targetLine = Math.round(fraction * totalLines);
  return Math.max(0, Math.min(scrollbackLines, scrollbackLines - targetLine + rows));
}
