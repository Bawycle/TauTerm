// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive locale state for TauTerm.
 *
 * Wraps Paraglide's `setLocale` / `getLocale` with:
 * - A `$state` rune so components react to locale changes (FS-I18N-004)
 * - IPC persistence via `invoke('set_locale')` (FS-I18N-005)
 * - Fallback to 'en' for unknown locale codes (FS-I18N-006)
 */

import { invoke } from '@tauri-apps/api/core';
import {
  setLocale as paraglideSetLocale,
  getLocale as paraglideGetLocale,
} from '$lib/paraglide/runtime';

/** Supported locale codes (FS-I18N-002). */
export type SupportedLocale = 'en' | 'fr';

const SUPPORTED_LOCALES: ReadonlySet<SupportedLocale> = new Set(['en', 'fr']);

/**
 * Guard: returns the locale if supported, otherwise 'en' (FS-I18N-006).
 */
function toSupportedLocale(raw: unknown): SupportedLocale {
  if (typeof raw === 'string' && SUPPORTED_LOCALES.has(raw as SupportedLocale)) {
    return raw as SupportedLocale;
  }
  return 'en';
}

/**
 * Reactive locale state. Read this in components to react to locale changes.
 *
 * Do not mutate directly — use `setLocale()` instead.
 */
let currentLocale = $state<SupportedLocale>(toSupportedLocale(paraglideGetLocale()));

/**
 * Returns the currently active locale.
 */
export function getActiveLocale(): SupportedLocale {
  return currentLocale;
}

/**
 * Sets the active locale, updates Paraglide's runtime, and persists via IPC.
 *
 * Applies immediately to all UI strings (FS-I18N-004).
 * Persists the selection so it survives restarts (FS-I18N-005).
 *
 * @param locale - Must be 'en' or 'fr'. Unknown values are ignored.
 */
export async function setLocale(locale: SupportedLocale): Promise<void> {
  const safe = toSupportedLocale(locale);
  paraglideSetLocale(safe);
  currentLocale = safe;
  try {
    await invoke<void>('set_locale', { locale: safe });
  } catch (err) {
    // Persistence failure is non-fatal: locale is applied in-session.
    // Callers may surface this error if appropriate; we do not swallow silently.
    console.error('[locale] Failed to persist locale via IPC:', err);
  }
}

/**
 * Initialises the locale from the persisted backend preference.
 * Must be called once at application startup (e.g., in `+layout.svelte`).
 */
export async function initLocale(): Promise<void> {
  try {
    const persisted = await invoke<string>('get_locale');
    const safe = toSupportedLocale(persisted);
    paraglideSetLocale(safe);
    currentLocale = safe;
  } catch {
    // IPC unavailable (e.g., dev without backend) — keep default 'en'.
  }
}
