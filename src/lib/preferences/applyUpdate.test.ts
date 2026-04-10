// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for applyPreferencesUpdate.
 *
 * These tests are pure — no DOM, no Tauri runtime, no module mocks.
 * All dependencies are injected as vi.fn() stubs.
 *
 * Test IDs:
 *   TV-PREF-001 — applyLocale receives the language from the backend result, not the patch
 *   TV-PREF-002 — invoker is called with the exact patch passed by the caller
 *   TV-PREF-003 — if invoker rejects, applyLocale is never called
 *   TV-PREF-004 — the function returns the full Preferences from the backend
 *   TV-PREF-005 — works with an empty patch {} (no crash, applyLocale still called)
 */

import { describe, it, expect, vi } from 'vitest';
import { applyPreferencesUpdate } from './applyUpdate';
import type { Preferences, PreferencesPatch } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

function makePreferences(overrides: Partial<Preferences['appearance']> = {}): Preferences {
  return {
    appearance: {
      fontFamily: 'monospace',
      fontSize: 14,
      cursorStyle: 'block',
      cursorBlinkMs: 530,
      themeName: 'Umbra',
      opacity: 1.0,
      language: 'en',
      contextMenuHintShown: false,
      fullscreen: false,
      hideCursorWhileTyping: true,
      ...overrides,
    },
    terminal: {
      scrollbackLines: 10000,
      allowOsc52Write: false,
      wordDelimiters: ' \t',
      bellType: 'none',
      confirmMultilinePaste: true,
    },
    keyboard: { bindings: {} },
    connections: [],
    themes: [],
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('applyPreferencesUpdate', () => {
  it('TV-PREF-001 — applyLocale receives the language from the backend, not the patch', async () => {
    // The patch requests 'fr', but the backend returns 'en' (e.g., unsupported value
    // was normalised). The frontend must trust the backend return value.
    const backendResult = makePreferences({ language: 'en' });
    const invoker = vi.fn().mockResolvedValue(backendResult);
    const applyLocale = vi.fn();

    const patch: PreferencesPatch = { appearance: { language: 'fr' } };
    await applyPreferencesUpdate(patch, invoker, applyLocale);

    expect(applyLocale).toHaveBeenCalledOnce();
    expect(applyLocale).toHaveBeenCalledWith('en');
  });

  it('TV-PREF-002 — invoker is called with update_preferences and the exact patch', async () => {
    const backendResult = makePreferences({ language: 'fr' });
    const invoker = vi.fn().mockResolvedValue(backendResult);
    const applyLocale = vi.fn();

    const patch: PreferencesPatch = {
      appearance: { language: 'fr', fontSize: 16 },
      terminal: { scrollbackLines: 5000 },
    };
    await applyPreferencesUpdate(patch, invoker, applyLocale);

    expect(invoker).toHaveBeenCalledOnce();
    expect(invoker).toHaveBeenCalledWith('update_preferences', { patch });
  });

  it('TV-PREF-003 — if invoker rejects, applyLocale is never called', async () => {
    const invoker = vi.fn().mockRejectedValue(new Error('IPC failure'));
    const applyLocale = vi.fn();

    const patch: PreferencesPatch = { appearance: { language: 'fr' } };
    await expect(applyPreferencesUpdate(patch, invoker, applyLocale)).rejects.toThrow(
      'IPC failure',
    );

    expect(applyLocale).not.toHaveBeenCalled();
  });

  it('TV-PREF-004 — returns the full Preferences object from the backend', async () => {
    const backendResult = makePreferences({ language: 'fr', fontSize: 18 });
    const invoker = vi.fn().mockResolvedValue(backendResult);
    const applyLocale = vi.fn();

    const result = await applyPreferencesUpdate({}, invoker, applyLocale);

    expect(result).toBe(backendResult);
    expect(result.appearance.fontSize).toBe(18);
    expect(result.appearance.language).toBe('fr');
  });

  it('TV-PREF-005 — works with an empty patch without crashing, applyLocale is still called', async () => {
    const backendResult = makePreferences({ language: 'en' });
    const invoker = vi.fn().mockResolvedValue(backendResult);
    const applyLocale = vi.fn();

    await expect(applyPreferencesUpdate({}, invoker, applyLocale)).resolves.toBe(backendResult);

    expect(invoker).toHaveBeenCalledWith('update_preferences', { patch: {} });
    expect(applyLocale).toHaveBeenCalledWith('en');
  });
});
