// SPDX-License-Identifier: MPL-2.0

/**
 * Pure coordination helper for applying a preferences update.
 *
 * Extracted from TerminalView.handlePreferencesUpdate to make the logic
 * unit-testable without a DOM or a real Tauri backend.
 *
 * Responsibilities:
 *   1. Call the backend via the injected `invoker` to persist the patch.
 *   2. Sync the reactive locale state from the value returned by the backend
 *      (not the patch value — the backend is the source of truth).
 *   3. Return the full updated Preferences.
 *
 * Error handling is intentionally left to the caller.
 */

import type { Preferences, PreferencesPatch } from '$lib/ipc/types';

/**
 * Apply a preferences patch via IPC and sync the locale from the backend result.
 *
 * @param patch       - Partial preferences to update.
 * @param invoker     - IPC call function (injectable for testing). Mirrors the
 *                      signature of `invoke` from `@tauri-apps/api/core`.
 * @param applyLocale - Locale sync function (injectable for testing). Called
 *                      with the language returned by the backend, not the patch.
 * @returns           The full updated Preferences as returned by the backend.
 * @throws            Propagates any rejection from `invoker` — the caller is
 *                    responsible for catching it.
 */
export async function applyPreferencesUpdate(
  patch: PreferencesPatch,
  invoker: (cmd: string, args?: Record<string, unknown>) => Promise<Preferences>,
  applyLocale: (language: unknown) => void,
): Promise<Preferences> {
  const result = await invoker('update_preferences', { patch });
  applyLocale(result.appearance.language);
  return result;
}
