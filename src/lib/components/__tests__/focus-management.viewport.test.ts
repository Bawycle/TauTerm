// SPDX-License-Identifier: MPL-2.0

/**
 * Focus management — viewport active callback tests.
 *
 * Covered:
 *   TEST-FOCUS-004 — onviewportactive: called with element when pane active, null when inactive
 *
 * Architecture note on TEST-FOCUS-004:
 *   TerminalPane's $effect calls onviewportactive when active + viewportEl is set.
 *   In JSDOM, ResizeObserver and IntersectionObserver are absent; the viewport
 *   element is rendered but the composable may not have completed its ResizeObserver
 *   setup. We test the prop-callback contract: mount with active=true and assert the
 *   callback was invoked with a non-null element; remount with active=false and assert
 *   null was passed.
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import TerminalPane from '$lib/components/TerminalPane.svelte';

// ---------------------------------------------------------------------------
// JSDOM polyfills
// ---------------------------------------------------------------------------

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
if (typeof (globalThis as Record<string, unknown>).ResizeObserver === 'undefined') {
  (globalThis as Record<string, unknown>).ResizeObserver = ResizeObserverStub;
}

if (typeof Element.prototype.animate === 'undefined') {
  Object.defineProperty(Element.prototype, 'animate', {
    value: function () {
      return {
        finished: Promise.resolve(),
        cancel() {},
        finish() {},
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {
          return true;
        },
      };
    },
    writable: true,
    configurable: true,
  });
}

// ---------------------------------------------------------------------------
// Shared teardown
// ---------------------------------------------------------------------------

const instances: ReturnType<typeof mount>[] = [];

afterEach(() => {
  instances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* ignore */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-004: onviewportactive prop — called with el when active, null when inactive
//
// JSDOM limitation: the $effect in TerminalPane runs after mount. Because
// viewportEl is set via bind:this inside useTerminalPane, it may be populated
// asynchronously after the initial flushSync. We drain the microtask queue
// and call flushSync a second time to catch late effects.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-004: TerminalPane onviewportactive callback', () => {
  beforeEach(() => {
    vi.spyOn(tauriEvent, 'listen').mockResolvedValue(() => {});
    vi.spyOn(tauriCore, 'invoke').mockResolvedValue(undefined as unknown as never);
  });

  it('calls onviewportactive with a non-null HTMLElement when pane is active', async () => {
    const onviewportactive = vi.fn();
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TerminalPane, {
      target: container,
      props: {
        paneId: 'pane-focus-004',
        tabId: 'tab-focus-004',
        active: true,
        onviewportactive,
      },
    });
    instances.push(instance);

    // Drain microtask queue for onMount async chains and effects
    for (let i = 0; i < 30; i++) await Promise.resolve();
    flushSync();
    for (let i = 0; i < 10; i++) await Promise.resolve();
    flushSync();

    // The callback should have been called with a non-null HTMLElement at least once
    const htmlElementCalls = onviewportactive.mock.calls.filter(
      ([arg]) => arg instanceof HTMLElement,
    );
    expect(htmlElementCalls.length).toBeGreaterThan(0);
  });

  it('calls onviewportactive with null on component unmount (effect cleanup)', async () => {
    const onviewportactive = vi.fn();
    const container = document.createElement('div');
    document.body.appendChild(container);

    // Mount as active=true so the effect registers the viewport element.
    const instance = mount(TerminalPane, {
      target: container,
      props: {
        paneId: 'pane-focus-004b',
        tabId: 'tab-focus-004b',
        active: true,
        onviewportactive,
      },
    });
    // Do NOT push to instances — we unmount manually to test the cleanup path.

    for (let i = 0; i < 30; i++) await Promise.resolve();
    flushSync();

    // Explicitly unmount — the $effect cleanup fires and calls onviewportactive(null).
    unmount(instance);
    flushSync();

    const nullCalls = onviewportactive.mock.calls.filter(([arg]) => arg === null);
    expect(nullCalls.length).toBeGreaterThan(0);
  });
});
