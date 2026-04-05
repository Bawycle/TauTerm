// SPDX-License-Identifier: MPL-2.0

/**
 * FS-I18N-001 — Locale catalogue parity tests.
 *
 * Verifies that:
 *   1. Every key present in en.json is also present in fr.json (and vice-versa).
 *   2. No key in either catalogue maps to an empty string.
 *   3. The component-specific keys expected for TabBar, TerminalPane, and
 *      TerminalView are present in both catalogues with non-empty values.
 *
 * These tests run entirely in the Node.js / vitest environment: they read the
 * JSON catalogue files directly and compare them. No Svelte runtime or DOM is
 * needed.
 *
 * Protocol reference: TP-MIN-006, TP-MIN-007, TP-MIN-008.
 * FS reference: FS-I18N-001, FS-I18N-002.
 */

import { describe, it, expect } from 'vitest';
import enRaw from './messages/en.json';
import frRaw from './messages/fr.json';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Strip the $schema meta-key — it is not a locale string.
function userKeys(catalogue: Record<string, string>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(catalogue)) {
    if (k !== '$schema') out[k] = v;
  }
  return out;
}

const en = userKeys(enRaw as Record<string, string>);
const fr = userKeys(frRaw as Record<string, string>);

// ---------------------------------------------------------------------------
// FS-I18N-001 — catalogue parity (en ↔ fr)
// ---------------------------------------------------------------------------

describe('FS-I18N-001: catalogue parity — en.json and fr.json have identical key sets', () => {
  it('every en key is present in fr', () => {
    const enKeys = Object.keys(en);
    const missingInFr = enKeys.filter((k) => !(k in fr));
    expect(missingInFr, `Keys in en.json missing from fr.json: ${missingInFr.join(', ')}`).toEqual(
      [],
    );
  });

  it('every fr key is present in en', () => {
    const frKeys = Object.keys(fr);
    const missingInEn = frKeys.filter((k) => !(k in en));
    expect(missingInEn, `Keys in fr.json missing from en.json: ${missingInEn.join(', ')}`).toEqual(
      [],
    );
  });
});

// ---------------------------------------------------------------------------
// FS-I18N-001 — no empty values
// ---------------------------------------------------------------------------

describe('FS-I18N-001: no empty values — all keys map to non-empty strings', () => {
  it('en.json has no empty values', () => {
    const emptyKeys = Object.entries(en)
      .filter(([, v]) => typeof v !== 'string' || v.trim() === '')
      .map(([k]) => k);
    expect(emptyKeys, `en.json has empty values for: ${emptyKeys.join(', ')}`).toEqual([]);
  });

  it('fr.json has no empty values', () => {
    const emptyKeys = Object.entries(fr)
      .filter(([, v]) => typeof v !== 'string' || v.trim() === '')
      .map(([k]) => k);
    expect(emptyKeys, `fr.json has empty values for: ${emptyKeys.join(', ')}`).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// TP-MIN-006 — TabBar component i18n keys
// Keys that must be present for TabBar.svelte to be fully internationalised.
// ---------------------------------------------------------------------------

describe('TP-MIN-006: TabBar component keys present in both catalogues', () => {
  const tabBarKeys = [
    'tab_new',
    'tab_close',
    'tab_untitled',
    'tab_bar_new_tab',
    'tab_bar_close_tab',
    'tab_bar_new_tab_tooltip',
    'tab_bar_tabs_aria_label',
    'tab_bar_ssh_badge_aria_label',
    'tab_bar_rename_input_aria_label',
    'tab_bar_rename_confirm',
    'tab_context_menu_aria_label',
    'action_close_tab',
    'action_close_other_tabs',
    'action_rename',
    'action_new_tab',
  ];

  for (const key of tabBarKeys) {
    it(`key "${key}" is present and non-empty in en.json`, () => {
      expect(en).toHaveProperty(key);
      expect(en[key].trim()).not.toBe('');
    });

    it(`key "${key}" is present and non-empty in fr.json`, () => {
      expect(fr).toHaveProperty(key);
      expect(fr[key].trim()).not.toBe('');
    });
  }
});

// ---------------------------------------------------------------------------
// TP-MIN-007 — TerminalPane component i18n keys
// ---------------------------------------------------------------------------

describe('TP-MIN-007: TerminalPane component keys present in both catalogues', () => {
  const terminalPaneKeys = [
    'terminal_pane_aria_label',
    'terminal_output_aria_label',
    'pane_split_horizontal',
    'pane_split_vertical',
    'pane_close',
    'pane_navigate_up',
    'pane_navigate_down',
    'pane_navigate_left',
    'pane_navigate_right',
    'scroll_to_bottom',
    'context_menu_hint',
    'paste_confirm_title',
    'paste_confirm_body',
    'paste_confirm_action',
    'paste_confirm_dont_ask',
    'action_copy',
    'action_paste',
    'action_search',
  ];

  for (const key of terminalPaneKeys) {
    it(`key "${key}" is present and non-empty in en.json`, () => {
      expect(en).toHaveProperty(key);
      expect(en[key].trim()).not.toBe('');
    });

    it(`key "${key}" is present and non-empty in fr.json`, () => {
      expect(fr).toHaveProperty(key);
      expect(fr[key].trim()).not.toBe('');
    });
  }
});

// ---------------------------------------------------------------------------
// TP-MIN-008 — TerminalView component i18n keys
// ---------------------------------------------------------------------------

describe('TP-MIN-008: TerminalView component keys present in both catalogues', () => {
  const terminalViewKeys = ['terminal_view_empty', 'terminal_view_aria_label', 'app_title'];

  for (const key of terminalViewKeys) {
    it(`key "${key}" is present and non-empty in en.json`, () => {
      expect(en).toHaveProperty(key);
      expect(en[key].trim()).not.toBe('');
    });

    it(`key "${key}" is present and non-empty in fr.json`, () => {
      expect(fr).toHaveProperty(key);
      expect(fr[key].trim()).not.toBe('');
    });
  }
});
