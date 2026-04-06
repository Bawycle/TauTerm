// SPDX-License-Identifier: MPL-2.0

/**
 * Cell dimension measurement via Canvas 2D API (F8).
 *
 * Uses OffscreenCanvas and measureText on U+2588 (FULL BLOCK) to obtain the
 * precise advance width of a monospace character, independent of DOM layout.
 * Cell height is derived analytically from fontSize * lineHeight.
 *
 * This module has no Svelte or DOM layout dependencies and is fully testable
 * in a jsdom/vitest environment (OffscreenCanvas can be stubbed).
 */

/**
 * Measure the dimensions of a single terminal cell using Canvas 2D.
 *
 * @param fontFamily - CSS font-family string (e.g. "JetBrains Mono, monospace")
 * @param fontSize   - Font size in pixels (e.g. 14)
 * @param lineHeight - Line height multiplier (e.g. 1.2)
 * @returns { width, height } in pixels. Width is the raw measureText result
 *          (not rounded) so callers can choose their own rounding strategy.
 *          Height is Math.ceil(fontSize * lineHeight).
 */
export function measureCellDimensions(
  fontFamily: string,
  fontSize: number,
  lineHeight: number,
): { width: number; height: number } {
  const canvas = new OffscreenCanvas(100, 100);
  const ctx = canvas.getContext('2d')!;
  ctx.font = `${fontSize}px ${fontFamily}`;
  const width = ctx.measureText('\u2588').width; // U+2588 FULL BLOCK
  const height = Math.ceil(fontSize * lineHeight);
  return { width, height };
}
