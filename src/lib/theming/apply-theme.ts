// SPDX-License-Identifier: MPL-2.0
import type { UserTheme } from '$lib/ipc/types';
import { isBuiltInTheme, getBuiltInThemeTokens } from './built-in-themes';

const STYLE_ELEMENT_ID = 'tauterm-active-theme';

function getOrCreateStyleElement(): HTMLStyleElement {
  let el = document.getElementById(STYLE_ELEMENT_ID) as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement('style');
    el.id = STYLE_ELEMENT_ID;
    document.head.appendChild(el);
  }
  return el;
}

function tokensToCSS(tokens: Record<string, string>): string {
  const entries = Object.entries(tokens)
    .map(([key, value]) => `  --${key}: ${value};`)
    .join('\n');
  return `:root {\n${entries}\n}`;
}

function userThemeToCSS(theme: UserTheme): string {
  const tokens: Record<string, string> = {
    'term-bg': theme.background,
    'term-fg': theme.foreground,
    'term-cursor-bg': theme.cursorColor,
    'term-cursor-unfocused': theme.cursorColor,
    'term-selection-bg': theme.selectionBg,
  };
  theme.palette.forEach((color, i) => {
    tokens[`term-color-${i}`] = color;
  });
  if (theme.lineHeight !== undefined && theme.lineHeight !== null) {
    tokens['line-height-terminal'] = String(theme.lineHeight);
  }
  return tokensToCSS(tokens);
}

/**
 * Apply the named theme to the document.
 * - 'umbra': clears overrides (app.css Umbra tokens are the baseline)
 * - built-in non-Umbra: injects full token set from AD.md
 * - user-created: injects terminal surface tokens only
 * - unknown: falls back to Umbra (clears overrides) with a warning
 */
export function applyTheme(name: string, userThemes: UserTheme[]): void {
  const styleEl = getOrCreateStyleElement();

  if (name === 'umbra') {
    styleEl.textContent = '';
    return;
  }

  if (isBuiltInTheme(name)) {
    const tokens = getBuiltInThemeTokens(name);
    // textContent is used instead of innerHTML to prevent any risk of HTML injection
    // from user-supplied color values. CSS in a <style> element is parsed as CSS, not HTML.
    styleEl.textContent = tokens ? tokensToCSS(tokens) : '';
    return;
  }

  const userTheme = userThemes.find((t) => t.name === name);
  if (userTheme) {
    styleEl.textContent = userThemeToCSS(userTheme);
    return;
  }

  // Unknown theme — fall back to Umbra
  console.warn(`[TauTerm] Unknown theme '${name}', falling back to Umbra`);
  styleEl.textContent = '';
}
