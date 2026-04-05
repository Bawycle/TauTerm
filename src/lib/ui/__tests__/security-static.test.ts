// SPDX-License-Identifier: MPL-2.0

/**
 * UIBC-SEC-014 / UIBC-SEC-015 — Static source analysis
 *
 * These tests inspect the raw .svelte source of each base UI component for
 * banned patterns. They run without a DOM and do not require
 * @testing-library/svelte. They pass even when a component file does not yet
 * exist (the component is treated as clean until created).
 *
 * Covered:
 *   UIBC-SEC-014 — no {@html} directive in base UI component templates
 *   UIBC-SEC-015 — no bind:innerHTML in base UI component templates
 */

import { readFileSync, existsSync } from 'fs';
import { resolve } from 'path';
import { describe, it, expect } from 'vitest';

const COMPONENTS = ['Button', 'TextInput', 'Toggle', 'Dropdown', 'Tooltip', 'Dialog'];
const UI_DIR = resolve(__dirname, '..');

/**
 * Strip all comment regions from Svelte/HTML/JS source so that documentation
 * examples inside comments do not trigger false positives.
 */
function stripComments(src: string): string {
  return src
    .replace(/<!--[\s\S]*?-->/g, '') // HTML comments
    .replace(/\/\/[^\n]*/g, '') // JS line comments
    .replace(/\/\*[\s\S]*?\*\//g, ''); // JS block comments
}

// ---------------------------------------------------------------------------
// UIBC-SEC-014 — {@html} must not appear in component templates
// ---------------------------------------------------------------------------
describe('UIBC-SEC-014 — no {@html} in base UI components', () => {
  for (const name of COMPONENTS) {
    it(`${name}.svelte contains no {@html} outside comments`, () => {
      const filePath = resolve(UI_DIR, `${name}.svelte`);
      if (!existsSync(filePath)) {
        // Component not yet implemented — vacuously clean until created.
        return;
      }
      const source = readFileSync(filePath, 'utf-8');
      expect(stripComments(source), `${name}.svelte must not use {@html}`).not.toContain('{@html');
    });
  }
});

// ---------------------------------------------------------------------------
// UIBC-SEC-015 — bind:innerHTML must not appear in component templates
// ---------------------------------------------------------------------------
describe('UIBC-SEC-015 — no bind:innerHTML in base UI components', () => {
  for (const name of COMPONENTS) {
    it(`${name}.svelte contains no bind:innerHTML`, () => {
      const filePath = resolve(UI_DIR, `${name}.svelte`);
      if (!existsSync(filePath)) {
        return;
      }
      const source = readFileSync(filePath, 'utf-8');
      expect(stripComments(source), `${name}.svelte must not use bind:innerHTML`).not.toContain(
        'bind:innerHTML',
      );
    });
  }
});

// ---------------------------------------------------------------------------
// Combined sweep — all six files at once (alias for CI summary)
// ---------------------------------------------------------------------------
describe('UIBC-SEC-006 alias — all six components free of {@html}', () => {
  it('scans all present component files in one assertion', () => {
    const violations: string[] = [];
    for (const name of COMPONENTS) {
      const filePath = resolve(UI_DIR, `${name}.svelte`);
      if (!existsSync(filePath)) continue;
      const stripped = stripComments(readFileSync(filePath, 'utf-8'));
      if (stripped.includes('{@html')) {
        violations.push(name);
      }
    }
    expect(violations, `Components with {@html}: ${violations.join(', ')}`).toHaveLength(0);
  });
});
