// SPDX-License-Identifier: MPL-2.0

/**
 * Mouse reporting encoding unit tests.
 *
 * Covers FS-MOUSE-001 to FS-MOUSE-006 (mouse reporting VT encoding) and
 * SEC-BLK-021 (Shift+Click/Wheel bypasses mouse reporting for text selection).
 *
 * TDD red phase — functions imported here do NOT yet exist in mouse.ts.
 * These tests will FAIL until mouse.ts is created and exported from the module.
 *
 * Expected module: `src/lib/terminal/mouse.ts`
 * Expected exports:
 *   - encodeMouseEvent(mode, encoding, button, col, row, event): Uint8Array | null
 *       mode      — MouseReportingMode: 'none' | 'normal' | 'button-event' | 'any-event'
 *       encoding  — MouseEncoding: 'default' | 'sgr'
 *       button    — 0 = left, 1 = middle, 2 = right, 3 = release, 64 = wheel-up, 65 = wheel-down
 *       col/row   — 1-based terminal cell coordinates
 *       event     — modifier flags { shiftKey: boolean, ctrlKey: boolean, altKey: boolean }
 *       release   — whether this is a button release event (for non-SGR encoding)
 *
 * X10 (default) encoding: ESC [ M <cb+32> <cx+32> <cy+32>
 * SGR encoding:           ESC [ < cb ; cx ; cy M  (press) / m (release)
 */

import { describe, it, expect } from 'vitest';

// TDD: import defensively — tests fail until mouse.ts exports these functions.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mouse = await import('./mouse.js').catch(() => ({})) as any;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function bytes(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

function toArr(result: Uint8Array | null): number[] | null {
  return result === null ? null : Array.from(result);
}

function noMods(): { shiftKey: boolean; ctrlKey: boolean; altKey: boolean } {
  return { shiftKey: false, ctrlKey: false, altKey: false };
}

function shiftMods(): { shiftKey: boolean; ctrlKey: boolean; altKey: boolean } {
  return { shiftKey: true, ctrlKey: false, altKey: false };
}

// ---------------------------------------------------------------------------
// MOUSE-001: mode=none — no VT sequence produced
// ---------------------------------------------------------------------------

describe('MOUSE-001: reporting mode none → null for any click', () => {
  it('left click with mode=none returns null', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined(); // FAIL until mouse.ts exists
    const result = fn_('none', 'default', 0, 5, 3, noMods(), false);
    expect(result).toBeNull();
  });

  it('wheel event with mode=none returns null', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('none', 'default', 64, 5, 3, noMods(), false);
    expect(result).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// MOUSE-002: mode=normal (1000) — X10 encoding
// Left click at col=1, row=1 → ESC[M<32+0><32+1><32+1> = ESC[M !!! (space=32)
// ESC [ M (32+button) (32+col) (32+row)
// ---------------------------------------------------------------------------

describe('MOUSE-002: normal mode, default encoding — X10 sequence', () => {
  it('left press at col=1, row=1 produces X10 sequence', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    // button=0 (left), col=1, row=1, press
    // Expected: ESC [ M \x20 \x21 \x21  (space=32 for button=0, !!=33 for col/row 1)
    const result = fn_('normal', 'default', 0, 1, 1, noMods(), false);
    expect(result).not.toBeNull();
    // ESC=0x1b, [=0x5b, M=0x4d, cb=32, cx=33, cy=33
    expect(toArr(result)).toEqual([0x1b, 0x5b, 0x4d, 32, 33, 33]);
  });

  it('left press at col=5, row=3 produces correct X10 coords', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    // cb=32 (button 0, no mods), cx=32+5=37, cy=32+3=35
    const result = fn_('normal', 'default', 0, 5, 3, noMods(), false);
    expect(toArr(result)).toEqual([0x1b, 0x5b, 0x4d, 32, 37, 35]);
  });

  it('button release sends cb=3+32=35 in X10 mode', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    // Release encodes button as 3 in X10 mode
    const result = fn_('normal', 'default', 0, 5, 3, noMods(), true);
    expect(toArr(result)).toEqual([0x1b, 0x5b, 0x4d, 35, 37, 35]);
  });
});

// ---------------------------------------------------------------------------
// MOUSE-003: SGR encoding (1006) — CSI < cb ; cx ; cy M/m
// ---------------------------------------------------------------------------

describe('MOUSE-003: SGR encoding — CSI < cb;cx;cy M (press) / m (release)', () => {
  it('left press at col=5, row=3 in SGR mode', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    // SGR press: ESC [ < 0 ; 5 ; 3 M
    const result = fn_('normal', 'sgr', 0, 5, 3, noMods(), false);
    expect(result).not.toBeNull();
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[<0;5;3M')));
  });

  it('left release at col=5, row=3 in SGR mode uses "m" terminator', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    // SGR release: ESC [ < 0 ; 5 ; 3 m
    const result = fn_('normal', 'sgr', 0, 5, 3, noMods(), true);
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[<0;5;3m')));
  });

  it('right button (2) press in SGR mode', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('normal', 'sgr', 2, 10, 7, noMods(), false);
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[<2;10;7M')));
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-021 / MOUSE-004: Shift+Click bypasses mouse reporting
// When shiftKey is true, encodeMouseEvent must return null regardless of mode.
// This allows the browser / OS text selection to work normally.
// ---------------------------------------------------------------------------

describe('SEC-BLK-021: Shift+Click bypasses mouse reporting', () => {
  it('Shift+left-click with mode=normal returns null', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('normal', 'default', 0, 5, 3, shiftMods(), false);
    expect(result).toBeNull();
  });

  it('Shift+left-click with mode=normal, SGR encoding returns null', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('normal', 'sgr', 0, 5, 3, shiftMods(), false);
    expect(result).toBeNull();
  });

  it('Shift+left-click with mode=any-event returns null', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('any-event', 'sgr', 0, 5, 3, shiftMods(), false);
    expect(result).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// MOUSE-005: Wheel events — button 64 (up) and 65 (down)
// ---------------------------------------------------------------------------

describe('MOUSE-005: wheel events in normal mode with SGR encoding', () => {
  it('wheel-up (button 64) in SGR mode produces ESC[<64;col;rowM', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('normal', 'sgr', 64, 3, 5, noMods(), false);
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[<64;3;5M')));
  });

  it('wheel-down (button 65) in SGR mode produces ESC[<65;col;rowM', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('normal', 'sgr', 65, 3, 5, noMods(), false);
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[<65;3;5M')));
  });

  it('wheel-up in X10 mode produces correct cb (64+32=96)', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    // X10: ESC [ M (32+64=96) (32+col) (32+row)
    const result = fn_('normal', 'default', 64, 1, 1, noMods(), false);
    expect(toArr(result)).toEqual([0x1b, 0x5b, 0x4d, 96, 33, 33]);
  });
});

// ---------------------------------------------------------------------------
// MOUSE-006: Shift+Wheel bypasses mouse reporting (same rule as Shift+Click)
// ---------------------------------------------------------------------------

describe('MOUSE-006: Shift+Wheel bypasses mouse reporting', () => {
  it('Shift+wheel-up with mode=normal returns null', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('normal', 'sgr', 64, 3, 5, shiftMods(), false);
    expect(result).toBeNull();
  });

  it('Shift+wheel-down with mode=normal returns null', () => {
    const fn_ = mouse.encodeMouseEvent;
    expect(fn_).toBeDefined();
    const result = fn_('normal', 'sgr', 65, 3, 5, shiftMods(), false);
    expect(result).toBeNull();
  });
});
