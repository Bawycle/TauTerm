// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive locale state for TauTerm.
 *
 * Wraps Paraglide's `setLocale` / `getLocale` with:
 * - A `$state` rune so components react to locale changes (FS-I18N-004)
 * - IPC persistence via `update_preferences` (FS-I18N-005)
 * - Fallback to 'en' for unknown locale codes (FS-I18N-006)
 */

import { invoke } from '@tauri-apps/api/core';
import {
  setLocale as paraglideSetLocale,
  getLocale as paraglideGetLocale,
  overwriteGetLocale,
} from '$lib/paraglide/runtime';
import type { Preferences } from '$lib/ipc/types';

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

// Wire Paraglide's getLocale() to our $state so that all m.*() calls in Svelte 5
// templates are tracked as reactive dependencies. When currentLocale changes,
// Svelte 5 automatically re-evaluates any template expression that called m.*().
overwriteGetLocale(() => currentLocale);

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
  paraglideSetLocale(safe, { reload: false });
  currentLocale = safe;
  try {
    await invoke<Preferences>('update_preferences', {
      patch: { appearance: { language: safe } },
    });
  } catch (err) {
    // Persistence failure is non-fatal: locale is applied in-session.
    // Callers may surface this error if appropriate; we do not swallow silently.
    console.error('[locale] Failed to persist locale via IPC:', err);
  }
}

/**
 * Applies a locale that was already persisted by the backend (e.g., after
 * `update_preferences` returns). Updates the reactive state and Paraglide's
 * runtime without making an additional IPC call.
 *
 * Idempotent: no-op when the locale is already current.
 * Unknown values fall back to 'en' (FS-I18N-006).
 */
export function applyLocaleChange(language: unknown): void {
  const safe = toSupportedLocale(language);
  if (safe === currentLocale) return;
  paraglideSetLocale(safe, { reload: false });
  currentLocale = safe;
}

/**
 * Initialises the locale from the persisted backend preference.
 * Must be called once at application startup (e.g., in `+layout.svelte`).
 */
export async function initLocale(): Promise<void> {
  try {
    const prefs = await invoke<Preferences>('get_preferences');
    const safe = toSupportedLocale(prefs.appearance.language);
    paraglideSetLocale(safe, { reload: false });
    currentLocale = safe;
  } catch {
    // IPC unavailable (e.g., dev without backend) — keep default 'en'.
  }
}
