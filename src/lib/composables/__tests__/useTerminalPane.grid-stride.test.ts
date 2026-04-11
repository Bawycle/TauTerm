// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for grid stride handling in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-STRIDE-001 — applyScreenUpdate uses event.cols for grid indexing
 *   TPSC-STRIDE-002 — cols/rows local state tracks event values
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { ScreenUpdateEvent } from '$lib/ipc/types';
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

  vi.spyOn(globalThis, 'requestAnimationFrame').mockImplementation((cb) => {
    cb(performance.now());
    return 1;
  });
  vi.spyOn(globalThis, 'cancelAnimationFrame').mockImplementation(() => {});
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
// TPSC-STRIDE-001: ScreenUpdateEvent stride — event cols are authoritative
// ---------------------------------------------------------------------------

describe('TPSC-STRIDE-001: applyScreenUpdate uses event.cols for grid indexing', () => {
  it('screen-update with non-default cols triggers isFullRedraw rebuild without throwing', async () => {
    // Fire a screen-update with cols=10 (non-default) and isFullRedraw: true.
    // The composable must use event.cols=10 as the grid stride — not its local
    // state (default 80).
    const { container } = await mountPane();

    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, {
          cols: 10,
          rows: 3,
          isFullRedraw: true,
          cells: [
            {
              row: 0,
              col: 9,
              content: 'X',
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
        }),
      );
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-STRIDE-002: cols/rows local state tracks event values
// ---------------------------------------------------------------------------

describe('TPSC-STRIDE-002: cols/rows local state tracks event values', () => {
  it('ondimensionschange is called with event cols/rows when they differ from local state', async () => {
    const dimensionsCalls: Array<[number, number]> = [];
    const onDimensionsChange = vi.fn((c: number, r: number) => {
      dimensionsCalls.push([c, r]);
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalPane, {
      target: container,
      props: {
        paneId: PANE_ID,
        tabId: 'tab-1',
        active: true,
        ondimensionschange: onDimensionsChange,
      },
    });
    instances.push(instance);

    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    const callsBefore = dimensionsCalls.length;

    // Fire screen-update with cols=10, rows=3 — different from the local state (80×24).
    // applyScreenUpdate must sync local state and call ondimensionschange(10, 3).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, { cols: 10, rows: 3, isFullRedraw: true }),
    );
    flushSync();

    const callsAfter = dimensionsCalls.slice(callsBefore);
    const hasDimension = callsAfter.some(([c, r]) => c === 10 && r === 3);
    expect(
      hasDimension,
      `Expected ondimensionschange(10, 3) after stride event, got: ${JSON.stringify(callsAfter)}`,
    ).toBe(true);
  });
});
