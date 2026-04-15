// SPDX-License-Identifier: MPL-2.0

/**
 * F5 — Bold color promotion ANSI 1–7 → 9–15
 *
 * When a cell has bold=true and its fg color is ANSI index in [1, 7],
 * resolveAnsiColor() must return the bright variant (index + 8).
 * Index 0 (black) is NOT promoted. Non-ANSI colors are NOT promoted.
 *
 * Covered:
 *   BOLD-PROMO-001 — bold=true, ansi 1 → index 9
 *   BOLD-PROMO-002 — bold=true, ansi 7 → index 15
 *   BOLD-PROMO-003 — bold=true, ansi 0 → NOT promoted (stays 0)
 *   BOLD-PROMO-004 — bold=false, ansi 3 → NOT promoted (stays 3)
 *   BOLD-PROMO-005 — bold=true, ansi 8 → NOT double-promoted (stays 8)
 *   BOLD-PROMO-006 — bold=true, ansi 15 → NOT double-promoted (stays 15)
 *   BOLD-PROMO-007 — bold=true, ansi256 index 3 → NOT promoted
 *   BOLD-PROMO-008 — bold=true, truecolor → NOT promoted
 */

import { describe, it, expect } from 'vitest';
import { resolveAnsiColor } from './color.js';
import type { Color } from '$lib/ipc';

describe('BOLD-PROMO-001: bold + ansi 1 → promotes to index 9', () => {
  it('returns ansi index 9 (bright-red)', () => {
    const color: Color = { type: 'ansi', index: 1 };
    const result = resolveAnsiColor(color, true);
    expect(result).toEqual({ type: 'ansi', index: 9 });
  });
});

describe('BOLD-PROMO-002: bold + ansi 7 → promotes to index 15', () => {
  it('returns ansi index 15 (bright-white)', () => {
    const color: Color = { type: 'ansi', index: 7 };
    const result = resolveAnsiColor(color, true);
    expect(result).toEqual({ type: 'ansi', index: 15 });
  });
});

describe('BOLD-PROMO-003: bold + ansi 0 → NOT promoted', () => {
  it('returns ansi index 0 unchanged (black is not promoted)', () => {
    const color: Color = { type: 'ansi', index: 0 };
    const result = resolveAnsiColor(color, true);
    expect(result).toEqual({ type: 'ansi', index: 0 });
  });
});

describe('BOLD-PROMO-004: not bold + ansi 3 → NOT promoted', () => {
  it('returns ansi index 3 unchanged (bold=false)', () => {
    const color: Color = { type: 'ansi', index: 3 };
    const result = resolveAnsiColor(color, false);
    expect(result).toEqual({ type: 'ansi', index: 3 });
  });
});

describe('BOLD-PROMO-005: bold + ansi 8 → NOT double-promoted', () => {
  it('returns ansi index 8 unchanged (already in bright range)', () => {
    const color: Color = { type: 'ansi', index: 8 };
    const result = resolveAnsiColor(color, true);
    expect(result).toEqual({ type: 'ansi', index: 8 });
  });
});

describe('BOLD-PROMO-006: bold + ansi 15 → NOT double-promoted', () => {
  it('returns ansi index 15 unchanged (already in bright range)', () => {
    const color: Color = { type: 'ansi', index: 15 };
    const result = resolveAnsiColor(color, true);
    expect(result).toEqual({ type: 'ansi', index: 15 });
  });
});

describe('BOLD-PROMO-007: bold + ansi256 → NOT promoted', () => {
  it('returns ansi256 color unchanged (not a 16-color ANSI type)', () => {
    const color: Color = { type: 'ansi256', index: 3 };
    const result = resolveAnsiColor(color, true);
    expect(result).toEqual({ type: 'ansi256', index: 3 });
  });
});

describe('BOLD-PROMO-008: bold + truecolor → NOT promoted', () => {
  it('returns rgb color unchanged', () => {
    const color: Color = { type: 'rgb', r: 255, g: 0, b: 0 };
    const result = resolveAnsiColor(color, true);
    expect(result).toEqual({ type: 'rgb', r: 255, g: 0, b: 0 });
  });
});

// ---------------------------------------------------------------------------
// Verify all ANSI 1–7 are promoted correctly
// ---------------------------------------------------------------------------

describe('BOLD-PROMO-ALL: all ANSI indices 1–7 get promoted when bold=true', () => {
  for (let i = 1; i <= 7; i++) {
    it(`ansi index ${i} → index ${i + 8}`, () => {
      const color: Color = { type: 'ansi', index: i };
      const result = resolveAnsiColor(color, true);
      expect(result).toEqual({ type: 'ansi', index: i + 8 });
    });
  }
});
