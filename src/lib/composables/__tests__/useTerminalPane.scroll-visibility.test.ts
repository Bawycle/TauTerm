// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for scroll-position-changed event handling in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-FN-002 — scroll-position-changed with offset > 0 shows scroll button
 *   TPSC-FN-004 — scroll-position-changed offset=0 does not show button on fresh mount
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { ScrollPositionChangedEvent } from '$lib/ipc';
import TerminalPane from '$lib/components/TerminalPane.svelte';
import {
  createListenerRegistry,
  createFireEvent,
  type ListenerFn,
  type ListenerRegistry,
} from './event-registry';

// ---------------------------------------------------------------------------
// jsdom polyfill: element.animate
// ---------------------------------------------------------------------------

if (typeof Element.prototype.animate === 'undefined') {
  Object.defineProperty(Element.prototype, 'animate', {
    value: function (
      _keyframes: unknown,
      _options?: unknown,
    ): {
      finished: Promise<void>;
      cancel(): void;
      finish(): void;
      addEventListener(): void;
      removeEventListener(): void;
      dispatchEvent(): boolean;
    } {
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
// Test constants
// ---------------------------------------------------------------------------

const PANE_ID = 'pane-ipc-test';

// ---------------------------------------------------------------------------
// Module-level state (reset in beforeEach)
// ---------------------------------------------------------------------------

let registry: ListenerRegistry = createListenerRegistry();
let fireEvent: <T>(eventName: string, payload: T) => void;
const instances: ReturnType<typeof mount>[] = [];

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  registry = createListenerRegistry();
  fireEvent = createFireEvent(registry);

  vi.spyOn(tauriEvent, 'listen').mockImplementation(
    async (eventName: string, handler: ListenerFn) => {
      if (!registry.has(eventName)) {
        registry.set(eventName, []);
      }
      registry.get(eventName)!.push(handler as ListenerFn);
      return () => {
        const handlers = registry.get(eventName);
        if (handlers) {
          const idx = handlers.indexOf(handler as ListenerFn);
          if (idx !== -1) handlers.splice(idx, 1);
        }
      };
    },
  );

  vi.spyOn(tauriCore, 'invoke').mockResolvedValue(undefined as unknown as never);
});

afterEach(() => {
  instances.forEach((inst) => {
    try {
      unmount(inst);
    } catch {
      /* component may have thrown on mount */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  registry.clear();
});

// ---------------------------------------------------------------------------
// Helper: mount TerminalPane and drain the onMount async chain
// ---------------------------------------------------------------------------

async function mountPane(
  paneId = PANE_ID,
  active = true,
): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> }> {
  const container = document.createElement('div');
  document.body.appendChild(container);

  const instance = mount(TerminalPane, {
    target: container,
    props: { paneId, tabId: 'tab-1', active },
  });
  instances.push(instance);

  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();

  return { container, instance };
}

// ---------------------------------------------------------------------------
// TPSC-FN-002: scroll-position-changed with offset > 0 shows scroll button
// ---------------------------------------------------------------------------

describe('TPSC-FN-002: scroll-position-changed with offset > 0 shows scroll button', () => {
  it('renders the scroll-to-bottom button when scrollOffset > 0', async () => {
    const { container } = await mountPane();

    fireEvent<ScrollPositionChangedEvent>('scroll-position-changed', {
      paneId: PANE_ID,
      offset: 5,
      scrollbackLines: 100,
    });
    flushSync();

    const btn = container.querySelector('.scroll-to-bottom-btn');
    expect(btn).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-FN-004: offset=0 after offset>0 — button absent when scrolled to bottom
// ---------------------------------------------------------------------------

describe('TPSC-FN-004: scroll-position-changed offset=0 does not show button on fresh mount', () => {
  it('button absent when only offset=0 events are received (at-bottom state)', async () => {
    // Verify that receiving an offset=0 event (at-bottom confirmation) on a
    // freshly-mounted pane (scrollOffset already 0) does not accidentally show
    // the scroll-to-bottom button.
    const { container } = await mountPane();

    expect(container.querySelector('.scroll-to-bottom-btn')).toBeNull();

    fireEvent<ScrollPositionChangedEvent>('scroll-position-changed', {
      paneId: PANE_ID,
      offset: 0,
      scrollbackLines: 50,
    });
    flushSync();

    const btn = container.querySelector('.scroll-to-bottom-btn');
    expect(btn).toBeNull();
  });
});
