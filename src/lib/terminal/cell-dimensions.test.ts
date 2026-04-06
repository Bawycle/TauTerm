// SPDX-License-Identifier: MPL-2.0

/**
 * F8 — Cell dimensions via Canvas 2D measureText
 *
 * Tests for the measureCellDimensions utility function.
 * OffscreenCanvas is stubbed in jsdom — we verify the contract,
 * not the exact pixel values from a real font.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { measureCellDimensions } from './cell-dimensions.js';

// ---------------------------------------------------------------------------
// OffscreenCanvas stub — jsdom does not implement it
// ---------------------------------------------------------------------------

class OffscreenCanvasStub {
  width: number;
  height: number;
  constructor(w: number, h: number) {
    this.width = w;
    this.height = h;
  }
  getContext(_type: string) {
    return {
      font: '',
      measureText: (_text: string) => ({ width: 7.5 }),
    };
  }
}

let originalOffscreenCanvas: typeof OffscreenCanvas | undefined;

beforeEach(() => {
  originalOffscreenCanvas = typeof OffscreenCanvas !== 'undefined' ? OffscreenCanvas : undefined;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (globalThis as any).OffscreenCanvas = OffscreenCanvasStub;
});

afterEach(() => {
  if (originalOffscreenCanvas) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (globalThis as any).OffscreenCanvas = originalOffscreenCanvas;
  } else {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (globalThis as any).OffscreenCanvas;
  }
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// F8-001: measureCellDimensions returns an object with width and height
// ---------------------------------------------------------------------------

describe('F8-001: measureCellDimensions returns { width, height }', () => {
  it('returns a number width > 0 for standard font', () => {
    const result = measureCellDimensions('monospace', 14, 1.2);
    expect(result.width).toBeGreaterThan(0);
  });

  it('returns a number height > 0 for standard font', () => {
    const result = measureCellDimensions('monospace', 14, 1.2);
    expect(result.height).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// F8-002: height = Math.ceil(fontSize * lineHeight)
// ---------------------------------------------------------------------------

describe('F8-002: cell height is Math.ceil(fontSize * lineHeight)', () => {
  it('fontSize=14, lineHeight=1.2 → height=17', () => {
    const result = measureCellDimensions('monospace', 14, 1.2);
    expect(result.height).toBe(Math.ceil(14 * 1.2)); // 17
  });

  it('fontSize=16, lineHeight=1.5 → height=24', () => {
    const result = measureCellDimensions('monospace', 16, 1.5);
    expect(result.height).toBe(Math.ceil(16 * 1.5)); // 24
  });

  it('fontSize=13, lineHeight=1.0 → height=13', () => {
    const result = measureCellDimensions('monospace', 13, 1.0);
    expect(result.height).toBe(Math.ceil(13 * 1.0)); // 13
  });
});

// ---------------------------------------------------------------------------
// F8-003: width comes from OffscreenCanvas measureText (not DOM)
// ---------------------------------------------------------------------------

describe('F8-003: width is measured via OffscreenCanvas measureText (U+2588)', () => {
  it('uses the value returned by ctx.measureText', () => {
    // The stub returns width=7.5 — function should use that value directly
    const result = measureCellDimensions('monospace', 14, 1.2);
    expect(result.width).toBe(7.5);
  });

  it('sets ctx.font to include fontSize and fontFamily before measuring', () => {
    const measureSpy = vi.fn(() => ({ width: 8 }));
    let capturedFont = '';
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (globalThis as any).OffscreenCanvas = class {
      getContext() {
        return {
          get font() {
            return capturedFont;
          },
          set font(v: string) {
            capturedFont = v;
          },
          measureText: measureSpy,
        };
      }
    };

    measureCellDimensions('JetBrains Mono', 14, 1.2);

    expect(capturedFont).toContain('14px');
    expect(capturedFont).toContain('JetBrains Mono');
    expect(measureSpy).toHaveBeenCalledWith('\u2588');
  });
});
