// SPDX-License-Identifier: MPL-2.0

import { describe, it, expect } from 'vitest';
import {
  resolve256Color,
  resolveColorDto,
  resolveColor,
  cursorShape,
  cursorBlinks,
} from './color.js';
import type { ColorDto, Color } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// TUITC-FN-020: ANSI 16 colors map to CSS token variables
// ---------------------------------------------------------------------------
describe('TUITC-FN-020: ANSI 16 fg colors → CSS token vars', () => {
  it('ANSI index 0 → var(--ansi-black)', () => {
    expect(resolve256Color(0)).toBe('var(--ansi-black)');
  });
  it('ANSI index 1 → var(--ansi-red)', () => {
    expect(resolve256Color(1)).toBe('var(--ansi-red)');
  });
  it('ANSI index 7 → var(--ansi-white)', () => {
    expect(resolve256Color(7)).toBe('var(--ansi-white)');
  });
  it('ANSI index 8 → var(--ansi-bright-black)', () => {
    expect(resolve256Color(8)).toBe('var(--ansi-bright-black)');
  });
  it('ANSI index 15 → var(--ansi-bright-white)', () => {
    expect(resolve256Color(15)).toBe('var(--ansi-bright-white)');
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-021: 256-color cube resolution
// ---------------------------------------------------------------------------
describe('TUITC-FN-021: 256-color cube → correct RGB', () => {
  // Index 196 = pure red in the 6×6×6 cube
  // i = 196 - 16 = 180 → r=180/36=5 → 255, g=(180%36)/6=0 → 0, b=180%6=0 → 0
  it('index 196 (pure red in cube) → rgb(255,0,0)', () => {
    expect(resolve256Color(196)).toBe('rgb(255,0,0)');
  });

  // Index 46 = pure green
  // i = 46 - 16 = 30 → r=0 → 0, g=(30%36)/6=5 → 255, b=30%6=0 → 0
  it('index 46 (pure green in cube) → rgb(0,255,0)', () => {
    expect(resolve256Color(46)).toBe('rgb(0,255,0)');
  });

  // Index 21 = pure blue
  // i = 21 - 16 = 5 → r=0 → 0, g=0 → 0, b=5 → 255
  it('index 21 (pure blue in cube) → rgb(0,0,255)', () => {
    expect(resolve256Color(21)).toBe('rgb(0,0,255)');
  });

  // Index 231 = white (all components = 5)
  it('index 231 → rgb(255,255,255)', () => {
    expect(resolve256Color(231)).toBe('rgb(255,255,255)');
  });
});

// ---------------------------------------------------------------------------
// 256-color grayscale ramp
// ---------------------------------------------------------------------------
describe('256-color grayscale ramp (indices 232–255)', () => {
  it('index 232 → darkest gray rgb(8,8,8)', () => {
    expect(resolve256Color(232)).toBe('rgb(8,8,8)');
  });
  it('index 255 → lightest gray rgb(238,238,238)', () => {
    // 8 + (255-232)*10 = 8 + 230 = 238
    expect(resolve256Color(255)).toBe('rgb(238,238,238)');
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-022: Truecolor → exact RGB via resolveColorDto
// ---------------------------------------------------------------------------
describe('TUITC-FN-022: truecolor → exact RGB string', () => {
  it('rgb(255, 100, 0) → "rgb(255,100,0)"', () => {
    const color: ColorDto = { type: 'rgb', r: 255, g: 100, b: 0 };
    expect(resolveColorDto(color)).toBe('rgb(255,100,0)');
  });

  it('rgb(0, 0, 0) → "rgb(0,0,0)"', () => {
    const color: ColorDto = { type: 'rgb', r: 0, g: 0, b: 0 };
    expect(resolveColorDto(color)).toBe('rgb(0,0,0)');
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-023/024: Default color → undefined (CSS inheritance)
// ---------------------------------------------------------------------------
describe('TUITC-FN-023/024: default color → undefined (CSS inheritance)', () => {
  it('ColorDto default → undefined', () => {
    const color: ColorDto = { type: 'default' };
    expect(resolveColorDto(color)).toBeUndefined();
  });

  it('undefined ColorDto → undefined', () => {
    expect(resolveColorDto(undefined)).toBeUndefined();
  });

  it('undefined Color → undefined', () => {
    expect(resolveColor(undefined)).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// resolveColor (SnapshotCell) — same logic, no default variant
// ---------------------------------------------------------------------------
describe('resolveColor (SnapshotCell Color type)', () => {
  it('ANSI index 1 → var(--ansi-red)', () => {
    const color: Color = { type: 'ansi', index: 1 };
    expect(resolveColor(color)).toBe('var(--ansi-red)');
  });

  it('ansi256 index 196 → rgb(255,0,0)', () => {
    const color: Color = { type: 'ansi256', index: 196 };
    expect(resolveColor(color)).toBe('rgb(255,0,0)');
  });

  it('rgb truecolor → exact rgb string', () => {
    const color: Color = { type: 'rgb', r: 10, g: 20, b: 30 };
    expect(resolveColor(color)).toBe('rgb(10,20,30)');
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-001 to 003: Cursor shape codes
// ---------------------------------------------------------------------------
describe('TUITC-FN-001/002/003: cursorShape() from DECSCUSR code', () => {
  it('code 0 (default) → block', () => {
    expect(cursorShape(0)).toBe('block');
  });
  it('code 1 (blinking block) → block', () => {
    expect(cursorShape(1)).toBe('block');
  });
  it('code 2 (steady block) → block', () => {
    expect(cursorShape(2)).toBe('block');
  });
  it('code 3 (blinking underline) → underline', () => {
    expect(cursorShape(3)).toBe('underline');
  });
  it('code 4 (steady underline) → underline', () => {
    expect(cursorShape(4)).toBe('underline');
  });
  it('code 5 (blinking bar) → bar', () => {
    expect(cursorShape(5)).toBe('bar');
  });
  it('code 6 (steady bar) → bar', () => {
    expect(cursorShape(6)).toBe('bar');
  });
});

// ---------------------------------------------------------------------------
// TUITC-FN-004: Cursor blink flag
// ---------------------------------------------------------------------------
describe('TUITC-FN-004: cursorBlinks()', () => {
  it('code 0 blinks (default blinking block)', () => {
    expect(cursorBlinks(0)).toBe(true);
  });
  it('code 1 blinks', () => {
    expect(cursorBlinks(1)).toBe(true);
  });
  it('code 2 does not blink (steady block)', () => {
    expect(cursorBlinks(2)).toBe(false);
  });
  it('code 3 blinks (blinking underline)', () => {
    expect(cursorBlinks(3)).toBe(true);
  });
  it('code 4 does not blink (steady underline)', () => {
    expect(cursorBlinks(4)).toBe(false);
  });
  it('code 5 blinks (blinking bar)', () => {
    expect(cursorBlinks(5)).toBe(true);
  });
  it('code 6 does not blink (steady bar)', () => {
    expect(cursorBlinks(6)).toBe(false);
  });
});
