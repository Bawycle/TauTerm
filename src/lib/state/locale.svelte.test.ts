// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for src/lib/state/locale.svelte.ts.
 *
 * The module depends on two external surfaces:
 *   - $lib/paraglide/runtime  → stubbed via vitest alias (src/__mocks__/paraglide-runtime.ts)
 *   - @tauri-apps/api/core    → stubbed via vitest alias (src/__mocks__/tauri-core.ts)
 *
 * Design note: locale.svelte.ts is a module with top-level $state. We do not
 * use vi.resetModules() across tests because that would require re-importing
 * in every test and makes spy setup ordering brittle. Instead, each test that
 * verifies call-through behaviour imports the module once and sets up spies
 * before calling the function under test.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as tauriCore from '@tauri-apps/api/core';
import * as paraglide from '$lib/paraglide/runtime';
import { getActiveLocale, setLocale, initLocale } from '$lib/state/locale.svelte';

beforeEach(() => {
  vi.restoreAllMocks();
  // Reset paraglide stub locale to default so state reads correctly.
  paraglide.setLocale('en');
});

describe('locale state — getActiveLocale', () => {
  it('returns a supported locale string ("en" or "fr")', () => {
    const locale = getActiveLocale();
    expect(['en', 'fr']).toContain(locale);
  });
});

describe('locale state — setLocale', () => {
  it('updates getActiveLocale to the new value', async () => {
    await setLocale('fr');
    expect(getActiveLocale()).toBe('fr');
    // Reset for subsequent tests.
    await setLocale('en');
  });

  it('calls paraglide setLocale with the requested locale', async () => {
    const spy = vi.spyOn(paraglide, 'setLocale');
    await setLocale('fr');
    expect(spy).toHaveBeenCalledWith('fr');
    await setLocale('en');
  });

  it('calls invoke("set_locale") with the locale', async () => {
    const spy = vi.spyOn(tauriCore, 'invoke').mockResolvedValue(undefined);
    await setLocale('en');
    expect(spy).toHaveBeenCalledWith('set_locale', { locale: 'en' });
  });

  it('does not throw when invoke fails (non-fatal IPC error)', async () => {
    vi.spyOn(tauriCore, 'invoke').mockRejectedValue(new Error('IPC unavailable'));
    // Must resolve without throwing — IPC failure is non-fatal (FS-I18N-005).
    await expect(setLocale('fr')).resolves.toBeUndefined();
    // State is still updated despite IPC failure.
    expect(getActiveLocale()).toBe('fr');
    await setLocale('en');
  });
});

describe('locale state — initLocale', () => {
  it('sets locale from the persisted backend value', async () => {
    vi.spyOn(tauriCore, 'invoke').mockResolvedValue('fr');
    await initLocale();
    expect(getActiveLocale()).toBe('fr');
    // Restore default.
    await setLocale('en');
  });

  it('falls back to "en" when invoke returns an unknown locale code (FS-I18N-006)', async () => {
    vi.spyOn(tauriCore, 'invoke').mockResolvedValue('jp'); // unsupported code
    await initLocale();
    // The toSupportedLocale guard must apply.
    expect(getActiveLocale()).toBe('en');
  });

  it('falls back to "en" when IPC is unavailable', async () => {
    vi.spyOn(tauriCore, 'invoke').mockRejectedValue(new Error('no backend'));
    // Ensure we start at 'en'.
    await setLocale('en');
    await initLocale();
    // initLocale must not crash and must leave locale at 'en'.
    expect(getActiveLocale()).toBe('en');
  });

  it('calls invoke("get_locale") to retrieve the persisted value', async () => {
    const spy = vi.spyOn(tauriCore, 'invoke').mockResolvedValue('en');
    await initLocale();
    expect(spy).toHaveBeenCalledWith('get_locale');
  });
});
