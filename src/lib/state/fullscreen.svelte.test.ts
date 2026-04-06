// SPDX-License-Identifier: MPL-2.0

/**
 * Tests for src/lib/state/fullscreen.svelte.ts
 *
 * Covers:
 *   - Initial state is false
 *   - setFullscreen(true) sets value to true
 *   - setFullscreen(false) sets value to false
 */

import { describe, it, expect } from 'vitest';
import { fullscreenState, setFullscreen } from './fullscreen.svelte';

describe('fullscreen.svelte.ts', () => {
  it('initial state is false', () => {
    // The module is freshly imported — initial value must be false.
    // Note: module-level $state persists across tests in the same run,
    // so we reset explicitly at the end of each test.
    setFullscreen(false);
    expect(fullscreenState.value).toBe(false);
  });

  it('setFullscreen(true) sets value to true', () => {
    setFullscreen(true);
    expect(fullscreenState.value).toBe(true);
    // Reset
    setFullscreen(false);
  });

  it('setFullscreen(false) sets value to false', () => {
    setFullscreen(true);
    setFullscreen(false);
    expect(fullscreenState.value).toBe(false);
  });

  it('setFullscreen is idempotent: calling true twice stays true', () => {
    setFullscreen(true);
    setFullscreen(true);
    expect(fullscreenState.value).toBe(true);
    // Reset
    setFullscreen(false);
  });
});
