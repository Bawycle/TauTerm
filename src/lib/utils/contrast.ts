// SPDX-License-Identifier: MPL-2.0

/**
 * WCAG 2.1 contrast ratio utilities.
 *
 * Implements relative luminance (WCAG 2.1 §1.4.3) using sRGB linearization,
 * and the contrast ratio formula: (L1 + 0.05) / (L2 + 0.05) where L1 >= L2.
 *
 * Reference: https://www.w3.org/TR/WCAG21/#contrast-minimum
 */

/**
 * Parse a 6-digit or 3-digit hex color string into [r, g, b] in [0, 255].
 * Returns null if the string is not a valid hex color.
 */
function parseHex(hex: string): [number, number, number] | null {
  const s = hex.trim();
  // Match #rrggbb or #rgb
  const full = s.match(/^#([0-9a-fA-F]{6})$/);
  if (full) {
    const n = parseInt(full[1], 16);
    return [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
  }
  const short = s.match(/^#([0-9a-fA-F]{3})$/);
  if (short) {
    const r = parseInt(short[1][0], 16);
    const g = parseInt(short[1][1], 16);
    const b = parseInt(short[1][2], 16);
    return [r * 17, g * 17, b * 17];
  }
  return null;
}

/**
 * Convert a single sRGB channel value (0–255) to its linearized form.
 * Uses the IEC 61966-2-1 piecewise formula as required by WCAG 2.1.
 */
function linearize(channel: number): number {
  const c = channel / 255;
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

/**
 * Compute the relative luminance of an RGB triplet (values 0–255).
 * WCAG 2.1 formula: L = 0.2126 R + 0.7152 G + 0.0722 B (linearized).
 */
export function relativeLuminance(r: number, g: number, b: number): number {
  return 0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b);
}

/**
 * Compute the WCAG 2.1 contrast ratio between two hex color strings.
 *
 * Returns a value in [1, 21]. Returns 1 (minimum) when either color cannot
 * be parsed — callers should treat an unparseable input as "unknown" rather
 * than a contrast failure.
 *
 * WCAG thresholds for normal text:
 *   - AA (minimum):   ≥ 4.5 : 1
 *   - AAA (enhanced): ≥ 7.0 : 1
 */
export function contrastRatio(hex1: string, hex2: string): number {
  const c1 = parseHex(hex1);
  const c2 = parseHex(hex2);
  if (c1 === null || c2 === null) return 1;
  const l1 = relativeLuminance(...c1);
  const l2 = relativeLuminance(...c2);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/** WCAG 2.1 AA threshold for normal text (4.5:1). */
export const WCAG_AA_THRESHOLD = 4.5;

/** WCAG 2.1 AAA threshold for normal text (7.0:1). */
export const WCAG_AAA_THRESHOLD = 7.0;
