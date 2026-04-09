// SPDX-License-Identifier: MPL-2.0

/**
 * FS-SEARCH-006 — scroll-centering formula unit tests.
 *
 * These tests exercise the pure centering formula independently of any Svelte
 * component.  scrollToOffset is responsible for clamping the result into the
 * [0, scrollbackLines] range, so we test the clamped value that scrollToOffset
 * would produce.
 *
 * Formula:
 *   rawOffset     = scrollbackLines - match.scrollbackRow + centerRow
 *   targetOffset  = clamp(rawOffset, 0, scrollbackLines)
 */

import { describe, it, expect } from 'vitest';

function computeTargetOffset(params: {
  scrollbackLines: number;
  scrollbackRow: number;
  rows: number;
}): number {
  const { scrollbackLines, scrollbackRow, rows } = params;
  const centerRow = Math.floor(rows / 2);
  const raw = scrollbackLines - scrollbackRow + centerRow;
  return Math.max(0, Math.min(scrollbackLines, raw));
}

describe('FS-SEARCH-006 scroll-centering formula', () => {
  it('centers a match in the middle of the scrollback with a 50-row viewport', () => {
    // scrollback has 1000 lines, match is at absolute row 500, viewport is 50 rows.
    // centerRow = floor(50 / 2) = 25
    // rawOffset = 1000 - 500 + 25 = 525
    const target = computeTargetOffset({
      scrollbackLines: 1000,
      scrollbackRow: 500,
      rows: 50,
    });
    expect(target).toBe(525);
  });

  it('clamps to 0 when the match is very close to the bottom (raw offset < 0)', () => {
    // scrollback has 1000 lines, match is at absolute row 998 (near bottom),
    // viewport is 50 rows → centerRow = 25.
    // rawOffset = 1000 - 998 + 25 = 27  → still positive, pick a tighter case.
    // match at row 999: raw = 1000 - 999 + 25 = 26  → still positive.
    // To force clamping to 0 we need scrollbackRow > scrollbackLines + centerRow.
    // e.g. scrollbackLines=10, scrollbackRow=40, rows=50 →
    //   centerRow=25, raw=10-40+25=-5 → clamped to 0.
    const target = computeTargetOffset({
      scrollbackLines: 10,
      scrollbackRow: 40,
      rows: 50,
    });
    expect(target).toBe(0);
  });

  it('clamps to scrollbackLines when the match is very close to the top (raw > scrollbackLines)', () => {
    // scrollback has 1000 lines, match is at absolute row 1 (oldest line),
    // viewport is 50 rows → centerRow = 25.
    // rawOffset = 1000 - 1 + 25 = 1024 > 1000 → clamped to 1000.
    const target = computeTargetOffset({
      scrollbackLines: 1000,
      scrollbackRow: 1,
      rows: 50,
    });
    expect(target).toBe(1000);
  });
});
