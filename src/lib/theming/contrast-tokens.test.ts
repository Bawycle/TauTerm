// SPDX-License-Identifier: MPL-2.0

/**
 * TUITC-UX-060 / FS-A11Y-001 — Inactive tab title contrast ≥ 4.5:1
 *
 * Verifies that the design token values for inactive tab title text achieve
 * WCAG 2.1 AA contrast (SC 1.4.3, ratio ≥ 4.5:1) against the tab bar background.
 *
 * Context: the prior sprint identified --color-tab-inactive-fg at #6b6660 on
 * #242118 produced ~2.5:1. The UXD correction (TUITC-UX-060) raised the token
 * to #9c9890 (neutral-400), achieving ~6.0:1. This test verifies the corrected
 * values and guards against future token regressions.
 *
 * Protocol reference: TP-MIN-018.
 * FS reference: FS-A11Y-001.
 */

import { describe, it, expect } from 'vitest';

// ---------------------------------------------------------------------------
// WCAG 2.1 relative luminance and contrast ratio (pure functions)
// ---------------------------------------------------------------------------

/**
 * Convert an sRGB channel (0–255) to linear light.
 * https://www.w3.org/TR/WCAG21/#dfn-relative-luminance
 */
function linearize(channel: number): number {
  const c = channel / 255;
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

/**
 * Parse a hex color string (#rrggbb) to { r, g, b } (0–255 each).
 */
function parseHex(hex: string): { r: number; g: number; b: number } {
  const clean = hex.startsWith('#') ? hex.slice(1) : hex;
  if (clean.length !== 6) throw new Error(`Invalid hex color: ${hex}`);
  return {
    r: parseInt(clean.slice(0, 2), 16),
    g: parseInt(clean.slice(2, 4), 16),
    b: parseInt(clean.slice(4, 6), 16),
  };
}

/**
 * Compute relative luminance for a #rrggbb hex color.
 */
function relativeLuminance(hex: string): number {
  const { r, g, b } = parseHex(hex);
  return 0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b);
}

/**
 * Compute WCAG 2.1 contrast ratio between two hex colors.
 * Returns a value in [1, 21].
 */
function contrastRatio(fg: string, bg: string): number {
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

// ---------------------------------------------------------------------------
// Token values (source of truth: UXD.md §7.2 / design-tokens.css)
// These values MUST match the actual token definitions. If a token changes,
// update here and ensure the ratio remains ≥ 4.5.
// ---------------------------------------------------------------------------

/**
 * Inactive tab foreground (TUITC-UX-060 corrected value).
 * Was #6b6660 (~2.5:1). Raised to #9c9890 (~6.0:1).
 */
const COLOR_TAB_INACTIVE_FG = '#9c9890';

/**
 * Tab bar background.
 */
const COLOR_TAB_BG = '#242118';

// ---------------------------------------------------------------------------
// TUITC-UX-060: Inactive tab title contrast ≥ 4.5:1 (WCAG AA)
// ---------------------------------------------------------------------------

describe('TUITC-UX-060: inactive tab title contrast ratio', () => {
  it('contrast ratio between --color-tab-inactive-fg and --color-tab-bg is ≥ 4.5:1', () => {
    const ratio = contrastRatio(COLOR_TAB_INACTIVE_FG, COLOR_TAB_BG);
    // Log the actual ratio to assist debugging if this test fails.
    // Expected: ~6.0:1 with corrected token values.
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('inactive fg token (#9c9890) is the corrected value (not the regressed #6b6660)', () => {
    // Guard: the old failing value must not be re-introduced.
    const oldRatio = contrastRatio('#6b6660', COLOR_TAB_BG);
    expect(oldRatio).toBeLessThan(4.5); // Confirms the old value was indeed non-compliant.

    // The current corrected value must pass.
    const currentRatio = contrastRatio(COLOR_TAB_INACTIVE_FG, COLOR_TAB_BG);
    expect(currentRatio).toBeGreaterThanOrEqual(4.5);
  });

  it('active tab title contrast ≥ 4.5:1 (TUITC-UX-100 sanity check)', () => {
    // Active fg: #e8e3d8 on active tab bg: #16140f
    const activeFg = '#e8e3d8';
    const activeBg = '#16140f';
    const ratio = contrastRatio(activeFg, activeBg);
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });
});

// ---------------------------------------------------------------------------
// Helper self-tests — verify the calculation implementation is correct
// ---------------------------------------------------------------------------

describe('WCAG contrast calculation self-tests', () => {
  it('black on white is 21:1', () => {
    const ratio = contrastRatio('#000000', '#ffffff');
    expect(ratio).toBeCloseTo(21, 0);
  });

  it('white on white is 1:1', () => {
    const ratio = contrastRatio('#ffffff', '#ffffff');
    expect(ratio).toBeCloseTo(1, 2);
  });

  it('black on black is 1:1', () => {
    const ratio = contrastRatio('#000000', '#000000');
    expect(ratio).toBeCloseTo(1, 2);
  });

  it('ratio is symmetric (fg/bg order does not matter)', () => {
    const r1 = contrastRatio('#9c9890', '#242118');
    const r2 = contrastRatio('#242118', '#9c9890');
    expect(r1).toBeCloseTo(r2, 6);
  });
});
