// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive preferences state — replica of the backend's Preferences.
 *
 * Provides:
 *   - A module-level reactive `preferences` object
 *   - DEFAULT_PREFERENCES as fallback when get_preferences fails
 *   - setPreferences() to populate from IPC
 *   - applyPatch() for optimistic updates after update_preferences returns
 *
 * The backend is the source of truth: never derive preferences from local
 * mutations alone — always sync from the value returned by update_preferences.
 */

import type { Preferences } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Default preferences — mirrors Rust defaults in preferences.rs
// Used as fallback when get_preferences fails (FS-PREF-003 graceful degradation).
// ---------------------------------------------------------------------------

export const DEFAULT_PREFERENCES: Preferences = {
  appearance: {
    fontFamily: 'monospace',
    fontSize: 13,
    cursorStyle: 'block',
    cursorBlinkMs: 530,
    themeName: 'umbra',
    opacity: 1.0,
    language: 'en',
    contextMenuHintShown: false,
  },
  terminal: {
    scrollbackLines: 10000,
    allowOsc52Write: false,
    wordDelimiters: ' ,;:.{}[]()"`|\\/',
    bellType: 'visual',
    confirmMultilinePaste: true,
  },
  keyboard: { bindings: {} },
  connections: [],
  themes: [],
};

// ---------------------------------------------------------------------------
// Reactive state — module-level singleton
// ---------------------------------------------------------------------------

/**
 * Current preferences.
 * `undefined` until the first successful get_preferences call.
 * Components should use `preferences ?? DEFAULT_PREFERENCES` for safe access.
 */
export let preferences = $state<Preferences | undefined>(undefined);

// ---------------------------------------------------------------------------
// Updaters
// ---------------------------------------------------------------------------

/**
 * Set the preferences from a full Preferences object (returned by get_preferences
 * or update_preferences).
 */
export function setPreferences(prefs: Preferences): void {
  preferences = prefs;
}

/**
 * Fall back to DEFAULT_PREFERENCES when get_preferences fails.
 * Only sets if preferences is still undefined (first load failure).
 */
export function setPreferencesFallback(): void {
  if (preferences === undefined) {
    preferences = DEFAULT_PREFERENCES;
  }
}
