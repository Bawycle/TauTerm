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

import type { ColorDto, Color } from '$lib/ipc/types';

/**
 * CSS custom property names for ANSI 0–15.
 * These map to the `--ansi-*` tokens defined in src/app.css.
 */
const ANSI_16_VARS: readonly string[] = [
  'var(--ansi-black)',        // 0
  'var(--ansi-red)',          // 1
  'var(--ansi-green)',        // 2
  'var(--ansi-yellow)',       // 3
  'var(--ansi-blue)',         // 4
  'var(--ansi-magenta)',      // 5
  'var(--ansi-cyan)',         // 6
  'var(--ansi-white)',        // 7
  'var(--ansi-bright-black)', // 8
  'var(--ansi-bright-red)',   // 9
  'var(--ansi-bright-green)', // 10
  'var(--ansi-bright-yellow)',// 11
  'var(--ansi-bright-blue)',  // 12
  'var(--ansi-bright-magenta)',// 13
  'var(--ansi-bright-cyan)', // 14
  'var(--ansi-bright-white)',// 15
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
export function resolveColorDto(color: ColorDto | undefined): string | undefined {
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
export function resolveColor(color: Color | undefined): string | undefined {
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
