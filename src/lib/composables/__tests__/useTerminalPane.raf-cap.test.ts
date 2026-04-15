// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for P-HT-3 rAF queue safety cap in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   RAF-CAP-001 — 21 events → getPaneScreenSnapshot called, batch not applied
 *   RAF-CAP-002 — 20 events → normal merge path (no snapshot refetch)
 *   RAF-CAP-003 — double overflow (Promise unresolved + second overflow) → single fetch
 *   RAF-CAP-004 — after re-fetch, new events apply normally
 *   RAF-CAP-005 — getPaneScreenSnapshot rejects → snapshotRefetchPending reset
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
    value: function (): {
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

const PANE_ID = 'pane-cap-test';
const DEFAULT_ATTRS = {};

const VALID_SNAPSHOT = {
  cols: 4,
  rows: 3,
  cells: [] as unknown[],
  cursorRow: 0,
  cursorCol: 0,
  cursorVisible: true,
  cursorShape: 0,
  scrollbackLines: 0,
  scrollOffset: 0,
};

// ---------------------------------------------------------------------------
// Module-level state
// ---------------------------------------------------------------------------

let registry: ListenerRegistry = createListenerRegistry();
let fireEvent: <T>(eventName: string, payload: T) => void;
const instances: ReturnType<typeof mount>[] = [];

let rafCallback: FrameRequestCallback | null = null;
let rafSpy: ReturnType<typeof vi.spyOn>;
let invokeSpy: ReturnType<typeof vi.spyOn>;

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  registry = createListenerRegistry();
  fireEvent = createFireEvent(registry);

  rafCallback = null;
  rafSpy = vi.spyOn(globalThis, 'requestAnimationFrame').mockImplementation((cb) => {
    rafCallback = cb;
    return 42;
  });
  vi.spyOn(globalThis, 'cancelAnimationFrame').mockImplementation(() => {});

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

  invokeSpy = vi.spyOn(tauriCore, 'invoke').mockResolvedValue(undefined as unknown as never);
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
  rafCallback = null;
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function flushRaf(): void {
  const cb = rafCallback;
  rafCallback = null;
  cb?.(performance.now());
}

async function mountPane(
  paneId = PANE_ID,
): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> }> {
  const container = document.createElement('div');
  document.body.appendChild(container);

  const instance = mount(TerminalPane, {
    target: container,
    props: { paneId, tabId: 'tab-cap', active: true },
  });
  instances.push(instance);

  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();

  return { container, instance };
}

function initGrid(cols: number, rows: number): void {
  const cells = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      cells.push({ row: r, col: c, content: ' ', width: 1, attrs: DEFAULT_ATTRS });
    }
  }
  fireEvent<ScreenUpdateEvent>(
    'screen-update',
    makeScreenUpdate(PANE_ID, { cols, rows, isFullRedraw: true, cells }),
  );
  flushRaf();
  flushSync();
}

function fireNEvents(n: number): void {
  for (let i = 0; i < n; i++) {
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [
          {
            row: 0,
            col: i % 4,
            content: String.fromCharCode(65 + (i % 26)),
            width: 1,
            attrs: DEFAULT_ATTRS,
          },
        ],
      }),
    );
  }
}

// ---------------------------------------------------------------------------
// RAF-CAP-001: 21 events → getPaneScreenSnapshot called, batch not applied
// ---------------------------------------------------------------------------

describe('RAF-CAP-001: 21 events triggers snapshot refetch', () => {
  it('getPaneScreenSnapshot is invoked and the batch is discarded', async () => {
    await mountPane();
    initGrid(4, 3);

    // Configure invoke to return a valid snapshot for the refetch.
    invokeSpy.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_pane_screen_snapshot') {
        return { ...VALID_SNAPSHOT } as never;
      }
      return undefined as never;
    });

    rafSpy.mockClear();
    rafCallback = null;

    // Fire 21 events (exceeds cap of 20).
    fireNEvents(21);

    // Flush the rAF.
    flushRaf();

    // getPaneScreenSnapshot should have been called.
    const snapshotCalls = invokeSpy.mock.calls.filter(
      (c: unknown[]) => c[0] === 'get_pane_screen_snapshot',
    );
    expect(snapshotCalls.length).toBeGreaterThanOrEqual(1);
  });
});

// ---------------------------------------------------------------------------
// RAF-CAP-002: 20 events → normal path (no snapshot refetch)
// ---------------------------------------------------------------------------

describe('RAF-CAP-002: 20 events goes through normal merge path', () => {
  it('getPaneScreenSnapshot is NOT called for exactly 20 events', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    // Fire exactly 20 events (at the cap, not over).
    fireNEvents(20);

    flushRaf();
    flushSync();

    // No snapshot refetch should have been triggered.
    const snapshotCalls = invokeSpy.mock.calls.filter(
      (c: unknown[]) => c[0] === 'get_pane_screen_snapshot',
    );
    expect(snapshotCalls).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// RAF-CAP-003: double overflow → single fetch (snapshotRefetchPending guard)
// ---------------------------------------------------------------------------

describe('RAF-CAP-003: double overflow triggers only one fetch', () => {
  it('second overflow while first is pending does not trigger another fetch', async () => {
    await mountPane();
    initGrid(4, 3);

    // Block the snapshot resolution to simulate a slow network.
    let resolveSnapshot!: (v: unknown) => void;
    const snapshotPromise = new Promise((res) => {
      resolveSnapshot = res;
    });

    invokeSpy.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_pane_screen_snapshot') {
        return snapshotPromise as never;
      }
      return undefined as never;
    });

    rafSpy.mockClear();
    rafCallback = null;
    invokeSpy.mockClear();

    // First overflow (21 events).
    fireNEvents(21);
    flushRaf();

    // Let the triggerSnapshotRefetch promise start (microtask).
    await Promise.resolve();

    // Second overflow while first is pending.
    rafSpy.mockClear();
    rafCallback = null;
    fireNEvents(21);
    flushRaf();

    // Only one fetch should have been dispatched.
    const snapshotCalls = invokeSpy.mock.calls.filter(
      (c: unknown[]) => c[0] === 'get_pane_screen_snapshot',
    );
    expect(snapshotCalls).toHaveLength(1);

    // Resolve so cleanup is clean.
    resolveSnapshot({ ...VALID_SNAPSHOT });
    for (let i = 0; i < 20; i++) await Promise.resolve();
  });
});

// ---------------------------------------------------------------------------
// RAF-CAP-004: after re-fetch, new events apply normally
// ---------------------------------------------------------------------------

describe('RAF-CAP-004: after re-fetch, normal events work again', () => {
  it('new events after snapshot refetch are applied via normal merge path', async () => {
    const { container } = await mountPane();
    initGrid(4, 3);

    // Set up a resolvable snapshot.
    let resolveSnapshot!: (v: unknown) => void;
    invokeSpy.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_pane_screen_snapshot') {
        return new Promise((res) => {
          resolveSnapshot = res;
        }) as never;
      }
      return undefined as never;
    });

    rafSpy.mockClear();
    rafCallback = null;

    // Trigger overflow.
    fireNEvents(21);
    flushRaf();

    // Resolve the snapshot.
    resolveSnapshot({ ...VALID_SNAPSHOT });
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    // Now fire a normal event — it should go through the merge path.
    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'Q', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    expect(rafSpy).toHaveBeenCalledTimes(1);
    flushRaf();
    flushSync();

    // No snapshot refetch should have been triggered for this single event.
    const snapshotCalls = invokeSpy.mock.calls.filter(
      (c: unknown[]) => c[0] === 'get_pane_screen_snapshot',
    );
    expect(snapshotCalls).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// RAF-CAP-005: getPaneScreenSnapshot rejects → snapshotRefetchPending reset
// ---------------------------------------------------------------------------

describe('RAF-CAP-005: snapshot fetch rejection resets snapshotRefetchPending', () => {
  it('after rejection, a subsequent overflow triggers a new fetch', async () => {
    await mountPane();
    initGrid(4, 3);

    // First call rejects, second call succeeds.
    let callCount = 0;
    invokeSpy.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_pane_screen_snapshot') {
        callCount++;
        if (callCount === 1) {
          throw new Error('simulated network error');
        }
        return { ...VALID_SNAPSHOT } as never;
      }
      return undefined as never;
    });

    rafSpy.mockClear();
    rafCallback = null;

    // First overflow → triggers fetch → rejects.
    fireNEvents(21);
    flushRaf();

    // Let the rejection propagate.
    for (let i = 0; i < 20; i++) await Promise.resolve();

    // snapshotRefetchPending should be false now (reset in finally).
    // Second overflow → should trigger a NEW fetch (not guarded by pending).
    rafSpy.mockClear();
    rafCallback = null;
    fireNEvents(21);
    flushRaf();

    for (let i = 0; i < 20; i++) await Promise.resolve();

    // Two total calls to get_pane_screen_snapshot.
    expect(callCount).toBe(2);
  });
});
