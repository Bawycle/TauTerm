// SPDX-License-Identifier: MPL-2.0

/**
 * Theme token validation.
 *
 * Validates that a user-supplied theme object:
 *  - Is a plain object (not null, not array)
 *  - Contains all required color tokens (per FS-THEME-004 and the Umbra token set)
 *  - Has valid color values: 6-digit hex, 8-digit hex, rgb(), oklch(), or CSS `inherit`
 *
 * Required tokens derive from FS-THEME-004 and the Umbra default theme in `src/app.css`:
 *  - Terminal surface: background, foreground, cursor bg/fg, selection bg/fg,
 *    and the 16 ANSI palette colors.
 *  - UI shell semantics referenced by all components (bg layers, text, accent, etc.)
 *    are NOT required in user themes — those fall back to the Umbra system tokens.
 *    Only the terminal-specific tokens that affect terminal rendering are required.
 *
 * Per FS-THEME-009 user themes map to the same design tokens as the default theme.
 * Per FS-THEME-004 a theme MUST define at minimum: background, foreground, cursor,
 * selection, and the 16 ANSI palette colors.
 */

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  /** Non-fatal warnings (e.g., unrecognised extra keys). */
  warnings: string[];
}

/**
 * Required color tokens per FS-THEME-004.
 * These are the CSS custom property names (without leading `--`).
 */
const REQUIRED_TOKENS: readonly string[] = [
  // Terminal background / foreground
  'term-bg',
  'term-fg',

  // Cursor
  'term-cursor-bg',
  'term-cursor-fg',
  'term-cursor-unfocused',

  // Selection
  'term-selection-bg',
  'term-selection-fg',
  'term-selection-bg-inactive',

  // ANSI 16-color palette (FS-THEME-004)
  'term-color-0',
  'term-color-1',
  'term-color-2',
  'term-color-3',
  'term-color-4',
  'term-color-5',
  'term-color-6',
  'term-color-7',
  'term-color-8',
  'term-color-9',
  'term-color-10',
  'term-color-11',
  'term-color-12',
  'term-color-13',
  'term-color-14',
  'term-color-15',
];

/**
 * Regex patterns for accepted color value formats.
 * 8-bit hex (#rrggbb), 8-digit hex (#rrggbbaa), rgb(), oklch(), and 'inherit'.
 */
const COLOR_PATTERNS: ReadonlyArray<RegExp> = [
  /^#[0-9a-fA-F]{6}$/, // #rrggbb
  /^#[0-9a-fA-F]{8}$/, // #rrggbbaa
  /^rgb\(\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}\s*\)$/, // rgb(r, g, b)
  /^rgba\(\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*[\d.]+\s*\)$/, // rgba(r, g, b, a)
  /^oklch\([^)]+\)$/, // oklch(...)
  /^inherit$/, // inherit (valid for selection-fg per Umbra theme)
];

function isValidColor(value: string): boolean {
  return COLOR_PATTERNS.some((pattern) => pattern.test(value.trim()));
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * Validate a theme object.
 *
 * @param theme - The value to validate (typically parsed from user input or IPC).
 * @returns A `ValidationResult` with `valid`, `errors`, and `warnings`.
 */
export function validateTheme(theme: unknown): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (!isPlainObject(theme)) {
    return {
      valid: false,
      errors: [
        'Theme must be a plain object (got ' + (theme === null ? 'null' : typeof theme) + ')',
      ],
      warnings: [],
    };
  }

  const themeObj = theme as Record<string, unknown>;

  // Check all required tokens are present with valid values
  for (const token of REQUIRED_TOKENS) {
    if (!(token in themeObj)) {
      errors.push(`Missing required token: "${token}"`);
      continue;
    }
    const value = themeObj[token];
    if (typeof value !== 'string') {
      errors.push(`Token "${token}" must be a string, got ${typeof value}`);
      continue;
    }
    if (!isValidColor(value)) {
      errors.push(`Token "${token}" has invalid color value: "${value}"`);
    }
  }

  // Check for extra keys not in the required set (non-fatal warning)
  const requiredSet = new Set(REQUIRED_TOKENS);
  for (const key of Object.keys(themeObj)) {
    if (!requiredSet.has(key)) {
      warnings.push(`Unrecognised token: "${key}" (will be ignored)`);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Build a minimal valid theme object for testing purposes.
 * Satisfies all FS-THEME-004 requirements with Umbra default values.
 */
export function buildMinimalValidTheme(): Record<string, string> {
  return {
    'term-bg': '#16140f',
    'term-fg': '#ccc7bc',
    'term-cursor-bg': '#7ab3d3',
    'term-cursor-fg': '#16140f',
    'term-cursor-unfocused': '#7ab3d3',
    'term-selection-bg': '#2e6f9c',
    'term-selection-fg': 'inherit',
    'term-selection-bg-inactive': '#1a3a52',
    'term-color-0': '#2c2921',
    'term-color-1': '#c44444',
    'term-color-2': '#5c9e5c',
    'term-color-3': '#b89840',
    'term-color-4': '#4a92bf',
    'term-color-5': '#9b6dbf',
    'term-color-6': '#3d9e8a',
    'term-color-7': '#ccc7bc',
    'term-color-8': '#4a4640',
    'term-color-9': '#e06060',
    'term-color-10': '#82c082',
    'term-color-11': '#d4b860',
    'term-color-12': '#7ab3d3',
    'term-color-13': '#c09cd8',
    'term-color-14': '#6ec4ae',
    'term-color-15': '#f5f2ea',
  };
}
