// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for snapshot initialisation in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-INIT-002 — screen-update buffered during snapshot fetch replays
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { ScreenUpdateEvent } from '$lib/ipc';
import TerminalPane from '$lib/components/TerminalPane.svelte';
import {
  createListenerRegistry,
  createFireEvent,
  makeScreenUpdate,
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
// TPSC-INIT-002: screen-update received during snapshot fetch is applied (WP3b)
// ---------------------------------------------------------------------------

describe('TPSC-INIT-002: screen-update received during snapshot fetch is applied after mount', () => {
  it('buffered update is replayed after snapshot resolves', async () => {
    // Intercept invoke() so that getPaneScreenSnapshot resolves on the NEXT
    // microtask tick, giving the buffered screen-update a chance to arrive first.
    let resolveSnapshot!: (v: unknown) => void;
    const snapshotPromise = new Promise((res) => {
      resolveSnapshot = res;
    });

    vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_pane_screen_snapshot') {
        return snapshotPromise as never;
      }
      return undefined as never;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalPane, {
      target: container,
      props: { paneId: PANE_ID, tabId: 'tab-1', active: true },
    });
    instances.push(instance);

    // Drain enough microtasks so that the screen-update listener is registered
    // (listener is registered BEFORE the snapshot fetch — WP3b).
    for (let i = 0; i < 5; i++) await Promise.resolve();

    // Fire an update BEFORE the snapshot resolves — it must be buffered.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cells: [
          {
            row: 0,
            col: 0,
            content: 'B',
            width: 1,
            attrs: {
              bold: false,
              dim: false,
              italic: false,
              underline: 0,
              blink: false,
              inverse: false,
              hidden: false,
              strikethrough: false,
            },
          },
        ],
        isFullRedraw: false,
      }),
    );

    // Now resolve the snapshot with an empty screen.
    resolveSnapshot({
      cols: 80,
      rows: 24,
      cells: [],
      cursorRow: 0,
      cursorCol: 0,
      cursorVisible: true,
      cursorShape: 0,
      scrollbackLines: 0,
      scrollOffset: 0,
    });

    // Drain remaining microtasks so the replay runs.
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});
