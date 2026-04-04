// SPDX-License-Identifier: MPL-2.0

import { describe, it, expect } from 'vitest';
import { validateTheme, buildMinimalValidTheme } from './validate.js';

// ---------------------------------------------------------------------------
// TEST-THEME-003 — Theme validation
// ---------------------------------------------------------------------------

describe('TEST-THEME-003: validateTheme', () => {
  // -----------------------------------------------------------------------
  // Valid theme
  // -----------------------------------------------------------------------
  describe('valid complete theme', () => {
    it('Umbra default values → valid: true, no errors', () => {
      const result = validateTheme(buildMinimalValidTheme());
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('accepts 6-digit hex values', () => {
      const theme = buildMinimalValidTheme();
      theme['term-bg'] = '#ff0000';
      expect(validateTheme(theme).valid).toBe(true);
    });

    it('accepts 8-digit hex values (with alpha)', () => {
      const theme = buildMinimalValidTheme();
      theme['term-bg'] = '#ff0000cc';
      expect(validateTheme(theme).valid).toBe(true);
    });

    it('accepts rgb() notation', () => {
      const theme = buildMinimalValidTheme();
      theme['term-bg'] = 'rgb(22, 20, 15)';
      expect(validateTheme(theme).valid).toBe(true);
    });

    it('accepts oklch() notation', () => {
      const theme = buildMinimalValidTheme();
      theme['term-bg'] = 'oklch(0.2 0.01 40)';
      expect(validateTheme(theme).valid).toBe(true);
    });

    it('accepts "inherit" for selection-fg', () => {
      const theme = buildMinimalValidTheme();
      theme['term-selection-fg'] = 'inherit';
      expect(validateTheme(theme).valid).toBe(true);
    });

    it('extra keys produce warnings, not errors', () => {
      const theme = { ...buildMinimalValidTheme(), 'my-custom-token': '#aabbcc' };
      const result = validateTheme(theme);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
      expect(result.warnings.some((w) => w.includes('my-custom-token'))).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // Missing required token
  // -----------------------------------------------------------------------
  describe('missing required tokens', () => {
    it('missing term-bg → valid: false, error mentions term-bg', () => {
      const theme = buildMinimalValidTheme();
      delete (theme as Record<string, string>)['term-bg'];
      const result = validateTheme(theme);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.includes('term-bg'))).toBe(true);
    });

    it('missing term-color-7 → valid: false, error mentions term-color-7', () => {
      const theme = buildMinimalValidTheme();
      delete (theme as Record<string, string>)['term-color-7'];
      const result = validateTheme(theme);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.includes('term-color-7'))).toBe(true);
    });

    it('multiple missing tokens → error for each', () => {
      const theme = buildMinimalValidTheme();
      delete (theme as Record<string, string>)['term-bg'];
      delete (theme as Record<string, string>)['term-fg'];
      const result = validateTheme(theme);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThanOrEqual(2);
    });
  });

  // -----------------------------------------------------------------------
  // Invalid color format
  // -----------------------------------------------------------------------
  describe('invalid color format', () => {
    it('3-digit hex is invalid → error mentions the token and value', () => {
      const theme = buildMinimalValidTheme();
      theme['term-bg'] = '#fff';
      const result = validateTheme(theme);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.includes('term-bg') && e.includes('#fff'))).toBe(true);
    });

    it('plain color name "red" is invalid', () => {
      const theme = buildMinimalValidTheme();
      theme['term-fg'] = 'red';
      const result = validateTheme(theme);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.includes('term-fg'))).toBe(true);
    });

    it('hsl() notation is invalid (not in spec)', () => {
      const theme = buildMinimalValidTheme();
      theme['term-cursor-bg'] = 'hsl(205, 50%, 52%)';
      const result = validateTheme(theme);
      expect(result.valid).toBe(false);
    });

    it('non-string token value is invalid', () => {
      const theme: Record<string, unknown> = { ...buildMinimalValidTheme() };
      theme['term-bg'] = 42;
      const result = validateTheme(theme);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.includes('term-bg'))).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // Non-object input
  // -----------------------------------------------------------------------
  describe('non-object input', () => {
    it('null → valid: false', () => {
      const result = validateTheme(null);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('string → valid: false', () => {
      const result = validateTheme('not-an-object');
      expect(result.valid).toBe(false);
    });

    it('number → valid: false', () => {
      const result = validateTheme(42);
      expect(result.valid).toBe(false);
    });

    it('array → valid: false', () => {
      const result = validateTheme(['a', 'b']);
      expect(result.valid).toBe(false);
    });

    it('undefined → valid: false', () => {
      const result = validateTheme(undefined);
      expect(result.valid).toBe(false);
    });
  });
});
