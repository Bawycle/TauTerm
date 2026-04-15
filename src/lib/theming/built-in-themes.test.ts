// SPDX-License-Identifier: MPL-2.0
import { describe, it, expect } from 'vitest';
import {
  isBuiltInTheme,
  getBuiltInThemeTokens,
  getThemeSwatch,
  BUILT_IN_THEME_NAMES,
} from './built-in-themes';
import type { UserTheme } from '$lib/ipc';

describe('isBuiltInTheme', () => {
  it('returns true for all three built-in names', () => {
    for (const name of BUILT_IN_THEME_NAMES) {
      expect(isBuiltInTheme(name)).toBe(true);
    }
  });
  it('returns false for user theme name', () => {
    expect(isBuiltInTheme('my-custom-theme')).toBe(false);
  });
  it('returns false for empty string', () => {
    expect(isBuiltInTheme('')).toBe(false);
  });
  it('is case-sensitive — UMBRA is not a match', () => {
    expect(isBuiltInTheme('UMBRA')).toBe(false);
  });
});

describe('getBuiltInThemeTokens', () => {
  it('returns null for umbra', () => {
    expect(getBuiltInThemeTokens('umbra')).toBeNull();
  });
  it('returns non-null record for solstice', () => {
    const tokens = getBuiltInThemeTokens('solstice');
    expect(tokens).not.toBeNull();
    expect(typeof tokens).toBe('object');
  });
  it('solstice neutral-50 primitive is a light hex color', () => {
    const tokens = getBuiltInThemeTokens('solstice')!;
    // color-neutral-50 is the deepest background (near-white)
    expect(tokens['color-neutral-50']).toBeDefined();
    const hex = tokens['color-neutral-50'].replace('#', '');
    const r = parseInt(hex.slice(0, 2), 16);
    expect(r).toBeGreaterThan(200); // high red component → light color
  });
  it('solstice has ANSI palette tokens', () => {
    const tokens = getBuiltInThemeTokens('solstice')!;
    expect(tokens['term-color-0']).toBeDefined();
    expect(tokens['term-color-15']).toBeDefined();
  });
  it('solstice term-color-1 (red) is a dark red hex on light bg', () => {
    const tokens = getBuiltInThemeTokens('solstice')!;
    expect(tokens['term-color-1']).toBe('#b01e1e');
  });
  it('returns non-null record for archipel', () => {
    const tokens = getBuiltInThemeTokens('archipel');
    expect(tokens).not.toBeNull();
  });
  it('archipel neutral-900 primitive is a dark hex color', () => {
    const tokens = getBuiltInThemeTokens('archipel')!;
    // color-neutral-900 is the terminal background
    expect(tokens['color-neutral-900']).toBeDefined();
    const hex = tokens['color-neutral-900'].replace('#', '');
    const r = parseInt(hex.slice(0, 2), 16);
    expect(r).toBeLessThan(50); // low red component → dark color
  });
  it('archipel has ANSI palette tokens', () => {
    const tokens = getBuiltInThemeTokens('archipel')!;
    expect(tokens['term-color-0']).toBeDefined();
    expect(tokens['term-color-15']).toBeDefined();
  });
  it('archipel term-color-15 (white bright) is a light color', () => {
    const tokens = getBuiltInThemeTokens('archipel')!;
    expect(tokens['term-color-15']).toBe('#e8f2f6');
  });
  it('returns null for unknown name', () => {
    expect(getBuiltInThemeTokens('nonexistent')).toBeNull();
  });
});

describe('getThemeSwatch', () => {
  it('returns swatch for built-in umbra', () => {
    const swatch = getThemeSwatch('umbra', []);
    expect(swatch.bg).toBe('#16140f');
  });
  it('returns swatch for built-in solstice', () => {
    const swatch = getThemeSwatch('solstice', []);
    // accent is a var() reference since the token itself is a var()
    expect(swatch).toBeDefined();
    expect(typeof swatch.bg).toBe('string');
    expect(typeof swatch.fg).toBe('string');
  });
  it('returns swatch for built-in archipel', () => {
    const swatch = getThemeSwatch('archipel', []);
    expect(swatch).toBeDefined();
    expect(typeof swatch.bg).toBe('string');
  });
  it('falls back to umbra swatch for unknown theme not in user list', () => {
    const swatch = getThemeSwatch('nonexistent', []);
    expect(swatch.bg).toBe('#16140f');
  });
  it('extracts swatch from user theme', () => {
    const ut: UserTheme = {
      name: 'my-theme',
      background: '#112233',
      foreground: '#aabbcc',
      cursorColor: '#ffffff',
      selectionBg: '#334455',
      palette: [
        '#000000',
        '#ff0000',
        '#00ff00',
        '#ffff00',
        '#0000ff',
        '#ff00ff',
        '#00ffff',
        '#ffffff',
        '#888888',
        '#ff8888',
        '#88ff88',
        '#ffff88',
        '#8888ff',
        '#ff88ff',
        '#88ffff',
        '#eeeeee',
      ],
    };
    const swatch = getThemeSwatch('my-theme', [ut]);
    expect(swatch.bg).toBe('#112233');
    expect(swatch.fg).toBe('#aabbcc');
    expect(swatch.cursor).toBe('#ffffff');
    expect(swatch.color1).toBe('#ff0000');
    expect(swatch.color6).toBe('#00ffff');
  });
});
