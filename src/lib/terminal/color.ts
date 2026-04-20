// SPDX-License-Identifier: MPL-2.0

/**
 * Terminal color resolution utilities.
 *
 * Maps ColorDto / Color values from IPC types to CSS color strings.
 * ANSI 16 palette colors resolve to CSS custom property references (design tokens)
 * so that they are theme-remappable per FS-VT-023.
 *
 * References:
 *  - FS-VT-020: ANSI 16-color palette
 *  - FS-VT-021: 256-color (6×6×6 cube + grayscale ramp)
 *  - FS-VT-022: Truecolor (24-bit RGB)
 *  - FS-VT-023: ANSI 0–15 must be remappable via active theme
 */

import type { ColorDto, Color } from '$lib/ipc';

/**
 * CSS custom property names for ANSI 0–15.
 * These map to the `--term-color-N` tokens defined in src/app.css (Umbra baseline)
 * and overridden at runtime by the active theme via applyTheme().
 */
export const ANSI_16_VARS: readonly string[] = [
  'var(--term-color-0)', // 0
  'var(--term-color-1)', // 1
  'var(--term-color-2)', // 2
  'var(--term-color-3)', // 3
  'var(--term-color-4)', // 4
  'var(--term-color-5)', // 5
  'var(--term-color-6)', // 6
  'var(--term-color-7)', // 7
  'var(--term-color-8)', // 8
  'var(--term-color-9)', // 9
  'var(--term-color-10)', // 10
  'var(--term-color-11)', // 11
  'var(--term-color-12)', // 12
  'var(--term-color-13)', // 13
  'var(--term-color-14)', // 14
  'var(--term-color-15)', // 15
];

/**
 * Resolve the 6×6×6 color cube index (16–231) to an RGB string.
 * Index i (16 ≤ i ≤ 231): r=(i-16)/36, g=((i-16)%36)/6, b=(i-16)%6
 * Each component: 0→0, 1→95, 2→135, 3→175, 4→215, 5→255.
 */
function cubeComponent(n: number): number {
  return n === 0 ? 0 : 55 + n * 40;
}

/**
 * Resolve the grayscale ramp (232–255) to an RGB string.
 * Index i (232 ≤ i ≤ 255): value = 8 + (i - 232) * 10.
 */
function grayscaleValue(i: number): number {
  return 8 + (i - 232) * 10;
}

/**
 * Resolve a 256-color index to a CSS color string.
 *  - 0–15: returns ANSI token CSS var (theme-remappable)
 *  - 16–231: 6×6×6 cube absolute RGB
 *  - 232–255: grayscale ramp absolute RGB
 */
export function resolve256Color(index: number): string {
  if (index < 16) {
    return ANSI_16_VARS[index] ?? 'var(--term-fg)';
  }
  if (index <= 231) {
    const i = index - 16;
    const r = cubeComponent(Math.floor(i / 36));
    const g = cubeComponent(Math.floor((i % 36) / 6));
    const b = cubeComponent(i % 6);
    return `rgb(${r},${g},${b})`;
  }
  // 232–255 grayscale
  const v = grayscaleValue(index);
  return `rgb(${v},${v},${v})`;
}

/**
 * Resolve a `ColorDto` (from screen-update events) to a CSS color string.
 * Returns undefined when the color is 'default' (caller uses CSS inheritance).
 */
export function resolveColorDto(color: ColorDto | null | undefined): string | undefined {
  if (!color) return undefined;
  switch (color.type) {
    case 'default':
      return undefined;
    case 'ansi':
      return ANSI_16_VARS[color.index] ?? 'var(--term-fg)';
    case 'ansi256':
      return resolve256Color(color.index);
    case 'rgb':
      return `rgb(${color.r},${color.g},${color.b})`;
  }
}

/**
 * Resolve a `Color` (from ScreenSnapshot cells — no 'default' variant) to a CSS color string.
 * Returns undefined when color is absent (caller uses CSS inheritance).
 */
export function resolveColor(color: Color | null | undefined): string | undefined {
  if (!color) return undefined;
  switch (color.type) {
    case 'ansi':
      return ANSI_16_VARS[color.index] ?? 'var(--term-fg)';
    case 'ansi256':
      return resolve256Color(color.index);
    case 'rgb':
      return `rgb(${color.r},${color.g},${color.b})`;
  }
}

/**
 * Cursor shape codes from DECSCUSR (FS-VT-030):
 *  0 → default (block)
 *  1 → blinking block
 *  2 → steady block
 *  3 → blinking underline
 *  4 → steady underline
 *  5 → blinking bar
 *  6 → steady bar
 */
export type CursorShape = 'block' | 'underline' | 'bar';

export function cursorShape(shapeCode: number): CursorShape {
  switch (shapeCode) {
    case 3:
    case 4:
      return 'underline';
    case 5:
    case 6:
      return 'bar';
    default:
      return 'block';
  }
}

export function cursorBlinks(shapeCode: number): boolean {
  // Odd codes (1, 3, 5) blink; 0 (default) also blinks; even codes (2, 4, 6) are steady
  return shapeCode === 0 || shapeCode === 1 || shapeCode === 3 || shapeCode === 5;
}

/**
 * Bold color promotion (FS-VT-020 bold-bright rule).
 *
 * When bold=true and the color is an ANSI 16-color index in [1, 7],
 * return the bright variant (index + 8). All other cases return the
 * color unchanged:
 *   - Index 0 (black) is NOT promoted.
 *   - Indices 8–15 are already bright — no double promotion.
 *   - ansi256, rgb colors are NOT promoted.
 *   - bold=false: no promotion.
 */
export function resolveAnsiColor(color: Color, bold: boolean): Color {
  if (!bold) return color;
  if (color.type !== 'ansi') return color;
  if (color.index >= 1 && color.index <= 7) {
    return { type: 'ansi', index: color.index + 8 };
  }
  return color;
}
