// SPDX-License-Identifier: MPL-2.0
/**
 * focus-management.two-pane.test.ts
 *
 * Tests for the two-pane onviewportactive race condition fix.
 *
 * Covered:
 *   TEST-FOCUS-018 — callback ignores null (activeViewportEl unchanged)
 *   TEST-FOCUS-019 — second non-null call overwrites the first
 *   TEST-FOCUS-020 — simulated race: register(el2) then clear(null) → el2 retained
 */

import { describe, it, expect } from 'vitest';

/**
 * Replicates the onviewportactive callback logic from TerminalView.svelte.
 * The callback only sets activeViewportEl when el is non-null; null is ignored.
 */
function makeCallback(initialEl: HTMLElement | null = null) {
  let activeViewportEl: HTMLElement | null = initialEl;
  const callback = (el: HTMLElement | null) => {
    if (el !== null) {
      activeViewportEl = el;
    }
  };
  return { callback, get: () => activeViewportEl };
}

describe('TEST-FOCUS-018: onviewportactive callback ignores null', () => {
  it('does not clear activeViewportEl when null is received', () => {
    const el = document.createElement('div');
    const { callback, get } = makeCallback(el);
    callback(null);
    expect(get()).toBe(el);
  });

  it('does not change activeViewportEl when starting null and receiving null', () => {
    const { callback, get } = makeCallback(null);
    callback(null);
    expect(get()).toBeNull();
  });
});

describe('TEST-FOCUS-019: non-null call overwrites previous registration', () => {
  it('registers pane 2 element after pane 1', () => {
    const el1 = document.createElement('div');
    const el2 = document.createElement('div');
    const { callback, get } = makeCallback();
    callback(el1);
    expect(get()).toBe(el1);
    callback(el2);
    expect(get()).toBe(el2);
  });
});

describe('TEST-FOCUS-020: race simulation — register(el2) then clear(null) retains el2', () => {
  it('pane 1 cleanup (null) after pane 2 registration does not clear activeViewportEl', () => {
    const el2 = document.createElement('div');
    const { callback, get } = makeCallback();
    // Pane 2 becomes active and registers its element
    callback(el2);
    // Pane 1's cleanup fires null — must be ignored
    callback(null);
    // el2 must still be registered
    expect(get()).toBe(el2);
  });

  it('multiple null calls after registration do not clear', () => {
    const el = document.createElement('div');
    const { callback, get } = makeCallback();
    callback(el);
    callback(null);
    callback(null);
    callback(null);
    expect(get()).toBe(el);
  });
});
