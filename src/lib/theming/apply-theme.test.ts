// SPDX-License-Identifier: MPL-2.0
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { applyTheme } from './apply-theme';
import type { UserTheme } from '$lib/ipc';

// jsdom environment is provided globally by vite.config.js test.environment = 'jsdom'
beforeEach(() => {
  // Clean up any style element from previous test
  document.getElementById('tauterm-active-theme')?.remove();
  vi.restoreAllMocks();
});

const noUserThemes: UserTheme[] = [];

function getStyleContent(): string {
  return document.getElementById('tauterm-active-theme')?.innerHTML ?? '';
}

describe('applyTheme', () => {
  it('clears overrides for umbra', () => {
    applyTheme('umbra', noUserThemes);
    expect(getStyleContent()).toBe('');
  });

  it('injects non-empty CSS for solstice', () => {
    applyTheme('solstice', noUserThemes);
    const css = getStyleContent();
    expect(css.length).toBeGreaterThan(0);
    expect(css).toContain('--term-bg');
  });

  it('solstice CSS contains a light background color (neutral-50 primitive)', () => {
    applyTheme('solstice', noUserThemes);
    const css = getStyleContent();
    // The primitive neutral-50 is a direct hex — check it is present and light
    const match = css.match(/--color-neutral-50:\s*(#[0-9a-fA-F]{6})/);
    expect(match).not.toBeNull();
    const hex = match![1].replace('#', '');
    expect(parseInt(hex.slice(0, 2), 16)).toBeGreaterThan(200);
  });

  it('injects non-empty CSS for archipel', () => {
    applyTheme('archipel', noUserThemes);
    const css = getStyleContent();
    expect(css.length).toBeGreaterThan(0);
    expect(css).toContain('--term-bg');
  });

  it('archipel CSS contains a dark background color (neutral-900 primitive)', () => {
    applyTheme('archipel', noUserThemes);
    const css = getStyleContent();
    // neutral-900 is the terminal background (#0c1a1e — very dark)
    const match = css.match(/--color-neutral-900:\s*(#[0-9a-fA-F]{6})/);
    expect(match).not.toBeNull();
    const hex = match![1].replace('#', '');
    expect(parseInt(hex.slice(0, 2), 16)).toBeLessThan(50);
  });

  it('falls back to Umbra and warns for unknown theme', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    applyTheme('nonexistent-theme', noUserThemes);
    expect(getStyleContent()).toBe('');
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('nonexistent-theme'));
  });

  it('applies user theme terminal tokens', () => {
    const userTheme: UserTheme = {
      name: 'my-theme',
      background: '#112233',
      foreground: '#aabbcc',
      cursorColor: '#ffffff',
      selectionBg: '#334455',
      palette: Array(16).fill('#000000') as UserTheme['palette'],
    };
    applyTheme('my-theme', [userTheme]);
    const css = getStyleContent();
    expect(css).toContain('--term-bg: #112233');
    expect(css).toContain('--term-fg: #aabbcc');
  });

  it('replacing a theme replaces style content, not appends', () => {
    applyTheme('solstice', noUserThemes);
    const first = getStyleContent();
    applyTheme('archipel', noUserThemes);
    const second = getStyleContent();
    expect(second).not.toBe(first);
    expect(document.querySelectorAll('#tauterm-active-theme').length).toBe(1);
  });

  it('switching from non-umbra to umbra clears the style element', () => {
    applyTheme('solstice', noUserThemes);
    expect(getStyleContent().length).toBeGreaterThan(0);
    applyTheme('umbra', noUserThemes);
    expect(getStyleContent()).toBe('');
  });

  it('creates exactly one style element per call sequence', () => {
    applyTheme('solstice', noUserThemes);
    applyTheme('archipel', noUserThemes);
    applyTheme('umbra', noUserThemes);
    expect(document.querySelectorAll('#tauterm-active-theme').length).toBe(1);
  });
});
