// SPDX-License-Identifier: MPL-2.0

/**
 * Keyboard shortcut resolution tests (FS-KBD-002, FS-KBD-003).
 *
 * Covered:
 *   TEST-SPRINT-006a — effectiveShortcut prefers user binding over default
 *   TEST-SPRINT-006b — effectiveShortcut falls back to default when no user binding
 *   TEST-SPRINT-006c — effectiveShortcut returns '' for unknown action
 *   TEST-SPRINT-006d — matchesShortcut: Ctrl+Shift+T matches correctly
 *   TEST-SPRINT-006e — matchesShortcut: modifier mismatch returns false
 *   TEST-SPRINT-006f — matchesShortcut: F2 (no modifiers) matches
 *   TEST-SPRINT-006g — matchesShortcut: Ctrl+, matches comma key
 *   TEST-SPRINT-006h — matchesShortcut: case-insensitive single-char key
 *
 * effectiveShortcut and matchesShortcut are defined locally in TerminalView.svelte
 * and not exported. These tests mirror the implementation as pure functions.
 * Any change to the TerminalView logic must be reflected here.
 *
 * The integration test (user bindings saved/loaded via update_preferences IPC)
 * is deferred to the functional test protocol (KBD-PERSIST-001).
 */

import { describe, it, expect } from 'vitest';

// ---------------------------------------------------------------------------
// Mirror of TerminalView.svelte pure functions
// ---------------------------------------------------------------------------

type Preferences = { keyboard?: { bindings?: Record<string, string> } } | null | undefined;

const defaultShortcuts: Record<string, string> = {
  new_tab: 'Ctrl+Shift+T',
  close_tab: 'Ctrl+Shift+W',
  paste: 'Ctrl+Shift+V',
  search: 'Ctrl+Shift+F',
  preferences: 'Ctrl+,',
  next_tab: 'Ctrl+Tab',
  prev_tab: 'Ctrl+Shift+Tab',
  rename_tab: 'F2',
};

function effectiveShortcut(actionId: string, preferences: Preferences): string {
  return preferences?.keyboard?.bindings?.[actionId] ?? defaultShortcuts[actionId] ?? '';
}

function matchesShortcut(
  event: {
    key: string;
    ctrlKey: boolean;
    altKey: boolean;
    shiftKey: boolean;
  },
  shortcut: string,
): boolean {
  if (!shortcut) return false;
  const parts = shortcut.split('+');
  const requiredCtrl = parts.includes('Ctrl');
  const requiredAlt = parts.includes('Alt');
  const requiredShift = parts.includes('Shift');
  const keyParts = parts.filter((p) => !['Ctrl', 'Alt', 'Shift', 'Meta'].includes(p));
  if (keyParts.length !== 1) return false;
  const requiredKey = keyParts[0];

  if (event.ctrlKey !== requiredCtrl) return false;
  if (event.altKey !== requiredAlt) return false;
  if (event.shiftKey !== requiredShift) return false;

  const eventKey = event.key;
  if (requiredKey.length === 1) {
    return eventKey.toLowerCase() === requiredKey.toLowerCase();
  }
  return eventKey === requiredKey;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function key(k: string, mods: { ctrl?: boolean; shift?: boolean; alt?: boolean } = {}) {
  return {
    key: k,
    ctrlKey: mods.ctrl ?? false,
    shiftKey: mods.shift ?? false,
    altKey: mods.alt ?? false,
  };
}

// ---------------------------------------------------------------------------
// TEST-SPRINT-006a — effectiveShortcut prefers user binding
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006a: effectiveShortcut prefers user binding over default', () => {
  it('returns user binding when present', () => {
    // TEST-SPRINT-006a
    const prefs: Preferences = { keyboard: { bindings: { new_tab: 'Ctrl+Alt+T' } } };
    expect(effectiveShortcut('new_tab', prefs)).toBe('Ctrl+Alt+T');
  });

  it('user binding overrides default for close_tab', () => {
    // TEST-SPRINT-006a
    const prefs: Preferences = { keyboard: { bindings: { close_tab: 'Ctrl+W' } } };
    expect(effectiveShortcut('close_tab', prefs)).toBe('Ctrl+W');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-006b — effectiveShortcut falls back to default
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006b: effectiveShortcut falls back to hardcoded default', () => {
  it('returns default when preferences is null', () => {
    // TEST-SPRINT-006b
    expect(effectiveShortcut('new_tab', null)).toBe('Ctrl+Shift+T');
  });

  it('returns default when keyboard bindings are empty', () => {
    // TEST-SPRINT-006b
    const prefs: Preferences = { keyboard: { bindings: {} } };
    expect(effectiveShortcut('new_tab', prefs)).toBe('Ctrl+Shift+T');
  });

  it('returns default when bindings map is absent', () => {
    // TEST-SPRINT-006b
    const prefs: Preferences = { keyboard: {} };
    expect(effectiveShortcut('search', prefs)).toBe('Ctrl+Shift+F');
  });

  it('returns default rename_tab shortcut as F2', () => {
    // TEST-SPRINT-006b
    expect(effectiveShortcut('rename_tab', null)).toBe('F2');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-006c — effectiveShortcut returns '' for unknown action
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006c: effectiveShortcut returns empty string for unknown action', () => {
  it('returns empty string for unregistered action', () => {
    // TEST-SPRINT-006c
    expect(effectiveShortcut('nonexistent_action', null)).toBe('');
  });

  it('returns empty string even with preferences present but action missing', () => {
    // TEST-SPRINT-006c
    const prefs: Preferences = { keyboard: { bindings: { new_tab: 'Ctrl+T' } } };
    expect(effectiveShortcut('zoom_in', prefs)).toBe('');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-006d — matchesShortcut: Ctrl+Shift+T
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006d: matchesShortcut Ctrl+Shift+T', () => {
  it('matches Ctrl+Shift+T event', () => {
    // TEST-SPRINT-006d
    expect(matchesShortcut(key('T', { ctrl: true, shift: true }), 'Ctrl+Shift+T')).toBe(true);
  });

  it('does not match without Ctrl', () => {
    // TEST-SPRINT-006d
    expect(matchesShortcut(key('T', { shift: true }), 'Ctrl+Shift+T')).toBe(false);
  });

  it('does not match without Shift', () => {
    // TEST-SPRINT-006d
    expect(matchesShortcut(key('T', { ctrl: true }), 'Ctrl+Shift+T')).toBe(false);
  });

  it('does not match wrong key', () => {
    // TEST-SPRINT-006d
    expect(matchesShortcut(key('W', { ctrl: true, shift: true }), 'Ctrl+Shift+T')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-006e — matchesShortcut: modifier mismatch
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006e: matchesShortcut modifier mismatch returns false', () => {
  it('Alt modifier present when not required', () => {
    // TEST-SPRINT-006e
    expect(matchesShortcut(key('t', { ctrl: true, shift: true, alt: true }), 'Ctrl+Shift+T')).toBe(
      false,
    );
  });

  it('no modifiers when Ctrl required', () => {
    // TEST-SPRINT-006e
    expect(matchesShortcut(key('t'), 'Ctrl+T')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-006f — matchesShortcut: F2 (no modifiers)
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006f: matchesShortcut F2 key (no modifiers)', () => {
  it('matches plain F2', () => {
    // TEST-SPRINT-006f
    expect(matchesShortcut(key('F2'), 'F2')).toBe(true);
  });

  it('does not match F2 with Ctrl', () => {
    // TEST-SPRINT-006f
    expect(matchesShortcut(key('F2', { ctrl: true }), 'F2')).toBe(false);
  });

  it('does not match F3 for F2 shortcut', () => {
    // TEST-SPRINT-006f
    expect(matchesShortcut(key('F3'), 'F2')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-006g — matchesShortcut: Ctrl+, comma
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006g: matchesShortcut Ctrl+, (preferences shortcut)', () => {
  it('matches Ctrl+, event', () => {
    // TEST-SPRINT-006g
    expect(matchesShortcut(key(',', { ctrl: true }), 'Ctrl+,')).toBe(true);
  });

  it('does not match Ctrl+. for Ctrl+,', () => {
    // TEST-SPRINT-006g
    expect(matchesShortcut(key('.', { ctrl: true }), 'Ctrl+,')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-006h — matchesShortcut: case-insensitive single-char key
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-006h: matchesShortcut single-char keys are case-insensitive', () => {
  it('lowercase event key matches uppercase shortcut key', () => {
    // TEST-SPRINT-006h: Shift changes event.key case in browsers
    expect(matchesShortcut(key('t', { ctrl: true, shift: true }), 'Ctrl+Shift+T')).toBe(true);
  });

  it('uppercase event key matches lowercase shortcut key', () => {
    // TEST-SPRINT-006h
    expect(matchesShortcut(key('T', { ctrl: true, shift: true }), 'Ctrl+Shift+t')).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

describe('matchesShortcut edge cases', () => {
  it('empty shortcut string returns false', () => {
    expect(matchesShortcut(key('T', { ctrl: true }), '')).toBe(false);
  });

  it('shortcut with multiple non-modifier keys (malformed) returns false', () => {
    // Parts after filtering modifiers would be length > 1 → not supported
    expect(matchesShortcut(key('A', { ctrl: true }), 'Ctrl+A+B')).toBe(false);
  });
});
