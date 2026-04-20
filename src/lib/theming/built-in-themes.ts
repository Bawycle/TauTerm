// SPDX-License-Identifier: MPL-2.0
import type { UserTheme } from '$lib/ipc';

export const BUILT_IN_THEME_NAMES = ['umbra', 'solstice', 'archipel'] as const;
export type BuiltInThemeName = (typeof BUILT_IN_THEME_NAMES)[number];

export function isBuiltInTheme(name: string): name is BuiltInThemeName {
  return (BUILT_IN_THEME_NAMES as readonly string[]).includes(name);
}

// ---------------------------------------------------------------------------
// Solstice — cool-shifted light theme (AD.md §8.5, §8.6, §8.7)
// Keys are CSS custom property names WITHOUT the leading '--'.
// ---------------------------------------------------------------------------
const SOLSTICE_TOKENS: Record<string, string> = {
  // Primitive color scale (§8.5)
  'color-neutral-50': '#f4f6f8',
  'color-neutral-100': '#e8ecf0',
  'color-neutral-150': '#dde2e8',
  'color-neutral-200': '#cdd4db',
  'color-neutral-300': '#b0bac4',
  'color-neutral-400': '#8494a2',
  'color-neutral-500': '#5c6e7d',
  'color-neutral-700': '#2e3f50',
  'color-neutral-850': '#1c2a36',
  'color-neutral-950': '#111d26',

  'color-arctic-900': '#0a1824',
  'color-arctic-700': '#1a3a5c',
  'color-arctic-500': '#2c5f8f',
  'color-arctic-400': '#3a7ab8',
  'color-arctic-300': '#5c9fd4',
  'color-arctic-200': '#9dc6e8',
  'color-arctic-100': '#d4e8f5',

  'color-amber-800': '#4d2e00',
  'color-amber-600': '#8c5200',
  'color-amber-400': '#c87800',
  'color-amber-100': '#fff0cc',

  'color-red-800': '#5c1010',
  'color-red-600': '#a01e1e',
  'color-red-400': '#c83232',
  'color-red-100': '#fde8e8',

  'color-green-800': '#1a4020',
  'color-green-600': '#2a7034',
  'color-green-400': '#3a9445',
  'color-green-100': '#e4f5e8',

  // Semantic tokens — UI shell (§8.5 continued)
  'color-bg-base': 'var(--color-neutral-50)',
  'color-bg-surface': 'var(--color-neutral-100)',
  'color-bg-raised': 'var(--color-neutral-150)',
  'color-bg-overlay': 'var(--color-neutral-50)',

  'color-border': 'var(--color-neutral-200)',
  'color-border-subtle': 'var(--color-neutral-150)',
  'color-divider': 'var(--color-neutral-200)',
  'color-divider-active': 'var(--color-arctic-500)',

  'color-text-primary': 'var(--color-neutral-850)',
  'color-text-secondary': 'var(--color-neutral-500)',
  'color-text-tertiary': 'var(--color-neutral-400)',
  'color-text-inverted': 'var(--color-neutral-50)',
  'color-text-heading': 'var(--color-neutral-500)',

  'color-icon-default': 'var(--color-neutral-500)',
  'color-icon-active': 'var(--color-neutral-850)',

  'color-accent': 'var(--color-arctic-500)',
  'color-accent-subtle': 'var(--color-arctic-100)',
  'color-accent-text': 'var(--color-arctic-700)',
  'color-hover-bg': 'var(--color-neutral-150)',
  'color-active-bg': 'var(--color-neutral-200)',
  'color-focus-ring': 'var(--color-arctic-500)',
  'color-focus-ring-offset': 'var(--color-neutral-50)',

  'color-activity': 'var(--color-green-600)',
  'color-indicator-output': 'var(--color-green-600)',
  'color-indicator-bell': 'var(--color-amber-600)',
  'color-process-end': 'var(--color-neutral-400)',
  'color-bell': 'var(--color-amber-600)',
  'color-error': 'var(--color-red-600)',
  'color-error-bg': 'var(--color-red-100)',
  'color-error-text': 'var(--color-red-800)',
  'color-warning': 'var(--color-amber-600)',
  'color-warning-bg': 'var(--color-amber-100)',
  'color-warning-text': 'var(--color-amber-800)',
  'color-success': 'var(--color-green-600)',
  'color-success-text': 'var(--color-green-800)',

  // Component tokens — tab bar
  'color-tab-bg': 'var(--color-neutral-100)',
  'color-tab-active-bg': 'var(--color-neutral-50)',
  'color-tab-active-fg': 'var(--color-neutral-850)',
  'color-tab-inactive-bg': 'transparent',
  'color-tab-inactive-fg': 'var(--color-neutral-400)',
  'color-tab-hover-bg': 'var(--color-neutral-150)',
  'color-tab-hover-fg': 'var(--color-neutral-700)',
  'color-tab-close-fg': 'var(--color-neutral-400)',
  'color-tab-close-hover-fg': 'var(--color-neutral-850)',
  'color-tab-new-fg': 'var(--color-neutral-400)',
  'color-tab-new-hover-fg': 'var(--color-neutral-850)',

  // SSH indicators
  'color-ssh-connected': 'var(--color-arctic-500)',
  'color-ssh-badge-bg': 'var(--color-arctic-100)',
  'color-ssh-badge-fg': 'var(--color-arctic-700)',
  'color-ssh-disconnected-bg': 'var(--color-red-100)',
  'color-ssh-disconnected-fg': 'var(--color-red-800)',
  'color-ssh-connecting-fg': 'var(--color-amber-600)',

  // Pane borders
  'color-pane-border-active': 'var(--color-arctic-500)',
  'color-pane-border-inactive': 'var(--color-neutral-200)',

  // Scrollbar
  'color-scrollbar-track': 'transparent',
  'color-scrollbar-thumb': 'var(--color-neutral-300)',
  'color-scrollbar-thumb-hover': 'var(--color-neutral-400)',

  // Form inputs
  'color-bg-input': 'var(--color-neutral-100)',

  // Terminal surface tokens (§8.5 terminal section)
  'term-bg': 'var(--color-neutral-50)',
  'term-fg': 'var(--color-neutral-950)',

  'term-cursor-bg': 'var(--color-arctic-700)',
  'term-cursor-fg': 'var(--color-neutral-50)',
  'term-cursor-unfocused': '#2c5f8f',

  'term-selection-bg': 'var(--color-arctic-200)',
  'term-selection-fg': 'inherit',
  'term-selection-bg-inactive': 'var(--color-arctic-100)',
  'term-selection-flash': 'var(--color-arctic-300)',

  'term-search-match-bg': 'var(--color-amber-100)',
  'term-search-match-fg': 'var(--color-amber-800)',
  'term-search-active-bg': '#f5d87a',
  'term-search-active-fg': 'var(--color-neutral-950)',

  'term-hyperlink-fg': 'var(--color-arctic-700)',
  'term-hyperlink-underline': 'var(--color-arctic-500)',

  'term-dim-opacity': '0.5',
  'term-underline-color-default': 'inherit',
  'term-strikethrough-position': '50%',
  'term-strikethrough-thickness': '1px',
  'term-blink-on-duration': '533ms',
  'term-blink-off-duration': '266ms',

  'shadow-overlay': '0 8px 32px rgba(17, 29, 38, 0.20)',
  'shadow-raised': '0 2px 8px  rgba(17, 29, 38, 0.12)',

  // ANSI palette (§8.6)
  'term-color-0': '#dde2e8',
  'term-color-1': '#b01e1e',
  'term-color-2': '#2e7a38',
  'term-color-3': '#7a5800',
  'term-color-4': '#2056a0',
  'term-color-5': '#7a2e8c',
  'term-color-6': '#1a6e78',
  'term-color-7': '#2e3f50',
  'term-color-8': '#b0bac4',
  'term-color-9': '#c83232',
  'term-color-10': '#3a9445',
  'term-color-11': '#9e7200',
  'term-color-12': '#2c5f8f',
  'term-color-13': '#9e3ab8',
  'term-color-14': '#1e8898',
  'term-color-15': '#111d26',
};

// ---------------------------------------------------------------------------
// Archipel — blue-green dark theme (AD.md §9.5, §9.6, §9.7)
// ---------------------------------------------------------------------------
const ARCHIPEL_TOKENS: Record<string, string> = {
  // Primitive color scale (§9.5)
  'color-neutral-950': '#060e10',
  'color-neutral-900': '#0c1a1e',
  'color-neutral-850': '#102228',
  'color-neutral-800': '#162c34',
  'color-neutral-750': '#1e3840',
  'color-neutral-700': '#274850',
  'color-neutral-600': '#385e68',
  'color-neutral-500': '#507888',
  'color-neutral-400': '#7aacba',
  'color-neutral-300': '#a8cdd6',
  'color-neutral-200': '#ccdfea',
  'color-neutral-100': '#e8f2f6',

  'color-coral-800': '#5c1a0e',
  'color-coral-600': '#b03018',
  'color-coral-500': '#d44030',
  'color-coral-400': '#e86050',
  'color-coral-300': '#f29080',
  'color-coral-100': '#3d1008',

  'color-turquoise-700': '#004a50',
  'color-turquoise-500': '#008898',
  'color-turquoise-400': '#00b4c8',
  'color-turquoise-300': '#40d4e4',

  'color-lime-700': '#304800',
  'color-lime-500': '#6aaa00',
  'color-lime-400': '#90cc10',
  'color-lime-300': '#b4e040',

  'color-amber-700': '#4d3200',
  'color-amber-500': '#b07000',
  'color-amber-400': '#d49020',
  'color-amber-300': '#e8b460',

  'color-red-700': '#3d0e10',
  'color-red-500': '#a02020',
  'color-red-400': '#cc3a3a',
  'color-red-300': '#e87070',

  // Semantic tokens — UI shell (§9.5 continued)
  'color-bg-base': 'var(--color-neutral-950)',
  'color-bg-surface': 'var(--color-neutral-800)',
  'color-bg-raised': 'var(--color-neutral-750)',
  'color-bg-overlay': 'var(--color-neutral-900)',

  'color-border': 'var(--color-neutral-700)',
  'color-border-subtle': 'var(--color-neutral-750)',
  'color-divider': 'var(--color-neutral-700)',
  'color-divider-active': 'var(--color-coral-500)',

  'color-text-primary': 'var(--color-neutral-300)',
  'color-text-secondary': 'var(--color-neutral-400)',
  'color-text-tertiary': 'var(--color-neutral-500)',
  'color-text-inverted': 'var(--color-neutral-950)',
  'color-text-heading': 'var(--color-neutral-400)',

  'color-icon-default': 'var(--color-neutral-400)',
  'color-icon-active': 'var(--color-neutral-300)',

  'color-accent': 'var(--color-coral-500)',
  'color-accent-subtle': 'var(--color-coral-100)',
  'color-accent-text': 'var(--color-coral-300)',
  'color-hover-bg': 'var(--color-neutral-750)',
  'color-active-bg': 'var(--color-neutral-700)',
  'color-focus-ring': 'var(--color-coral-500)',
  'color-focus-ring-offset': 'var(--color-neutral-950)',

  'color-activity': 'var(--color-lime-400)',
  'color-indicator-output': 'var(--color-lime-400)',
  'color-indicator-bell': 'var(--color-amber-400)',
  'color-process-end': 'var(--color-neutral-400)',
  'color-bell': 'var(--color-amber-400)',
  'color-error': 'var(--color-red-400)',
  'color-error-bg': 'var(--color-red-700)',
  'color-error-text': 'var(--color-red-300)',
  'color-warning': 'var(--color-amber-400)',
  'color-warning-bg': 'var(--color-amber-700)',
  'color-warning-text': 'var(--color-amber-300)',
  'color-success': 'var(--color-lime-400)',
  'color-success-text': 'var(--color-lime-300)',

  // Component tokens — tab bar
  'color-tab-bg': 'var(--color-neutral-800)',
  'color-tab-active-bg': 'var(--color-neutral-900)',
  'color-tab-active-fg': 'var(--color-neutral-200)',
  'color-tab-inactive-bg': 'transparent',
  'color-tab-inactive-fg': 'var(--color-neutral-500)',
  'color-tab-hover-bg': 'var(--color-neutral-750)',
  'color-tab-hover-fg': 'var(--color-neutral-400)',
  'color-tab-close-fg': 'var(--color-neutral-500)',
  'color-tab-close-hover-fg': 'var(--color-neutral-300)',
  'color-tab-new-fg': 'var(--color-neutral-500)',
  'color-tab-new-hover-fg': 'var(--color-neutral-300)',

  // SSH indicators
  'color-ssh-connected': 'var(--color-turquoise-300)',
  'color-ssh-badge-bg': 'var(--color-turquoise-700)',
  'color-ssh-badge-fg': 'var(--color-turquoise-300)',
  'color-ssh-disconnected-bg': 'var(--color-red-700)',
  'color-ssh-disconnected-fg': 'var(--color-red-300)',
  'color-ssh-connecting-fg': 'var(--color-amber-400)',

  // Pane borders
  'color-pane-border-active': 'var(--color-coral-500)',
  'color-pane-border-inactive': 'var(--color-neutral-700)',

  // Scrollbar
  'color-scrollbar-track': 'transparent',
  'color-scrollbar-thumb': 'var(--color-neutral-600)',
  'color-scrollbar-thumb-hover': 'var(--color-neutral-500)',

  // Form inputs
  'color-bg-input': 'var(--color-neutral-850)',

  // Terminal surface tokens (§9.5 terminal section)
  'term-bg': 'var(--color-neutral-900)',
  'term-fg': 'var(--color-neutral-100)',

  'term-cursor-bg': 'var(--color-coral-400)',
  'term-cursor-fg': 'var(--color-neutral-950)',
  'term-cursor-unfocused': '#d44030',

  'term-selection-bg': 'var(--color-turquoise-700)',
  'term-selection-fg': 'inherit',
  'term-selection-bg-inactive': 'var(--color-neutral-800)',
  'term-selection-flash': 'var(--color-turquoise-400)',

  'term-search-match-bg': 'var(--color-amber-700)',
  'term-search-match-fg': 'var(--color-amber-300)',
  'term-search-active-bg': '#6b5018',
  'term-search-active-fg': 'var(--color-neutral-200)',

  'term-hyperlink-fg': 'var(--color-turquoise-300)',
  'term-hyperlink-underline': 'var(--color-turquoise-400)',

  'term-dim-opacity': '0.5',
  'term-underline-color-default': 'inherit',
  'term-strikethrough-position': '50%',
  'term-strikethrough-thickness': '1px',
  'term-blink-on-duration': '533ms',
  'term-blink-off-duration': '266ms',

  // ANSI palette (§9.6)
  'term-color-0': '#1e3840',
  'term-color-1': '#cc3a3a',
  'term-color-2': '#4a9e50',
  'term-color-3': '#c89030',
  'term-color-4': '#4888c8',
  'term-color-5': '#a048c0',
  'term-color-6': '#1aa4b0',
  'term-color-7': '#a8cdd6',
  'term-color-8': '#385e68',
  'term-color-9': '#f07060',
  'term-color-10': '#80d040',
  'term-color-11': '#e8b040',
  'term-color-12': '#80b8f0',
  'term-color-13': '#d080e8',
  'term-color-14': '#40d8e8',
  'term-color-15': '#e8f2f6',
};

/**
 * Returns the full token map for a built-in theme, or null for 'umbra'
 * (Umbra is the CSS baseline — no override needed).
 * Unknown names also return null and fall back to Umbra.
 */
export function getBuiltInThemeTokens(name: string): Record<string, string> | null {
  if (name === 'solstice') return SOLSTICE_TOKENS;
  if (name === 'archipel') return ARCHIPEL_TOKENS;
  return null;
}

// ---------------------------------------------------------------------------
// Swatch colors — static summary for theme picker UI
// ---------------------------------------------------------------------------

export type SwatchColors = {
  bg: string;
  fg: string;
  accent: string;
  cursor: string;
  color1: string; // ANSI red (index 1) — representative warm color
  color6: string; // ANSI cyan (index 6) — representative cool color
};

const BUILT_IN_SWATCHES: Record<string, SwatchColors> = {
  umbra: {
    bg: '#16140f',
    fg: '#ccc7bc',
    accent: '#4a92bf',
    cursor: '#7ab3d3',
    color1: '#c44444',
    color6: '#3d9e8a',
  },
  solstice: {
    bg: SOLSTICE_TOKENS['term-bg'] ?? '#f4f6f8',
    fg: SOLSTICE_TOKENS['term-fg'] ?? '#1a2030',
    accent: SOLSTICE_TOKENS['color-accent'] ?? '#1a3a5c',
    cursor: SOLSTICE_TOKENS['term-cursor-bg'] ?? '#1a3a5c',
    color1: SOLSTICE_TOKENS['term-color-1'] ?? '#b01e1e',
    color6: SOLSTICE_TOKENS['term-color-6'] ?? '#1a6e78',
  },
  archipel: {
    bg: ARCHIPEL_TOKENS['term-bg'] ?? '#0c1a1e',
    fg: ARCHIPEL_TOKENS['term-fg'] ?? '#c8e0e6',
    accent: ARCHIPEL_TOKENS['color-accent'] ?? '#d44030',
    cursor: ARCHIPEL_TOKENS['term-cursor-bg'] ?? '#d44030',
    color1: ARCHIPEL_TOKENS['term-color-1'] ?? '#cc3a3a',
    color6: ARCHIPEL_TOKENS['term-color-6'] ?? '#1aa4b0',
  },
};

/**
 * Returns a set of representative swatch colors for the named theme.
 * Falls back to Umbra swatch for unknown themes that are not in userThemes.
 */
export function getThemeSwatch(name: string, userThemes: UserTheme[]): SwatchColors {
  if (name in BUILT_IN_SWATCHES) return BUILT_IN_SWATCHES[name];
  const ut = userThemes.find((t) => t.name === name);
  if (!ut) return BUILT_IN_SWATCHES['umbra'];
  return {
    bg: ut.background,
    fg: ut.foreground,
    accent: ut.palette[4] ?? '#4a92bf',
    cursor: ut.cursorColor,
    color1: ut.palette[1] ?? '#c44444',
    color6: ut.palette[6] ?? '#3d9e8a',
  };
}
