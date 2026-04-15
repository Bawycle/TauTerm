// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for P-OPT-1 rAF batching in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   RAF-BATCH-001 — single event schedules exactly one rAF; cell appears after flush
 *   RAF-BATCH-002 — two events with distinct cells → both applied in one rAF tick
 *   RAF-BATCH-003 — two consecutive events → scheduleRaf() is idempotent (1 rAF call)
 *   RAF-BATCH-004 — event during buffering phase → goes to pendingUpdates, rAF not scheduled
 *   RAF-BATCH-005 — event for a different paneId → filtered, rAF not scheduled
 *   RAF-BATCH-006 — onDestroy with pending rAF → cancelAnimationFrame called with correct id
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

const PANE_ID = 'pane-raf-test';
const OTHER_PANE_ID = 'pane-other';
const DEFAULT_ATTRS = {};

// ---------------------------------------------------------------------------
// Module-level state (reset in beforeEach)
// ---------------------------------------------------------------------------

let registry: ListenerRegistry = createListenerRegistry();
let fireEvent: <T>(eventName: string, payload: T) => void;
const instances: ReturnType<typeof mount>[] = [];

let rafCallback: FrameRequestCallback | null = null;
let rafSpy: ReturnType<typeof vi.spyOn>;
let cancelRafSpy: ReturnType<typeof vi.spyOn>;

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  registry = createListenerRegistry();
  fireEvent = createFireEvent(registry);

  rafCallback = null;
  rafSpy = vi.spyOn(globalThis, 'requestAnimationFrame').mockImplementation((cb) => {
    rafCallback = cb;
    return 42; // fake rafId
  });
  cancelRafSpy = vi.spyOn(globalThis, 'cancelAnimationFrame').mockImplementation(() => {});

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
  rafCallback = null;
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Flush the pending rAF callback (simulates the browser calling it).
 * Sets rafCallback to null first so that a re-scheduled rAF inside the flush
 * is captured in the new rafCallback slot.
 */
function flushRaf(): void {
  const cb = rafCallback;
  rafCallback = null;
  cb?.(performance.now());
}

/**
 * Mount a TerminalPane and drain the onMount async chain, including the
 * snapshot fetch. After this resolves, buffering === false and the rAF path
 * is active.
 */
async function mountPane(
  paneId = PANE_ID,
): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> }> {
  const container = document.createElement('div');
  document.body.appendChild(container);

  const instance = mount(TerminalPane, {
    target: container,
    props: { paneId, tabId: 'tab-raf', active: true },
  });
  instances.push(instance);

  // Drain enough microtasks for onMount async chain + snapshot resolution.
  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();

  return { container, instance };
}

/**
 * Establish an initial grid via a full-redraw event so that differential
 * updates have a valid gridRows structure to mutate.
 */
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
  // Full-redraw goes through the rAF path like all live events (P-OPT-1).
  // Flush the pending rAF so the grid is fully initialized before the test
  // exercises the differential update path.
  flushRaf();
  flushSync();
}

function getCellText(container: HTMLElement, rowIdx: number, colIdx: number): string | null {
  const rows = container.querySelectorAll('.terminal-pane__row');
  if (!rows[rowIdx]) return null;
  const cells = rows[rowIdx].querySelectorAll('.terminal-pane__cell');
  if (!cells[colIdx]) return null;
  return cells[colIdx].textContent ?? null;
}

// ---------------------------------------------------------------------------
// RAF-BATCH-001: single event → rAF scheduled → cell present after flush
// ---------------------------------------------------------------------------

describe('RAF-BATCH-001: single event schedules one rAF; cell appears after flush', () => {
  it('requestAnimationFrame called once; cell content visible only after flushRaf', async () => {
    const { container } = await mountPane();
    initGrid(4, 3);

    // Reset the rAF spy count: initGrid may have triggered unrelated rAF calls
    // (e.g. scrollbar animation). We care only about the post-init differential update.
    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'A', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // rAF must be scheduled (exactly once at this point).
    expect(rafSpy).toHaveBeenCalledTimes(1);

    // Cell must NOT be applied yet (rAF hasn't fired).
    // The previous content from initGrid was ' ' (space).
    expect(getCellText(container, 0, 0)).toBe(' ');

    // Fire the rAF callback and let Svelte flush.
    flushRaf();
    await flushSync();

    // Now the cell must show the new content.
    expect(getCellText(container, 0, 0)).toBe('A');
  });
});

// ---------------------------------------------------------------------------
// RAF-BATCH-002: two events with distinct cells → both applied in one rAF tick
// ---------------------------------------------------------------------------

describe('RAF-BATCH-002: two events — both cells applied in one rAF tick', () => {
  it('both updates are present in the DOM after a single rAF flush', async () => {
    const { container } = await mountPane();
    initGrid(4, 3);

    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'A', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 1, content: 'B', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // Before flush: neither cell updated.
    expect(getCellText(container, 0, 0)).toBe(' ');
    expect(getCellText(container, 0, 1)).toBe(' ');

    flushRaf();
    await flushSync();

    // After flush: both cells must reflect the updates — no "last wins" regression.
    expect(getCellText(container, 0, 0)).toBe('A');
    expect(getCellText(container, 0, 1)).toBe('B');
  });
});

// ---------------------------------------------------------------------------
// RAF-BATCH-003: two consecutive events → scheduleRaf() is idempotent (1 call)
// ---------------------------------------------------------------------------

describe('RAF-BATCH-003: two consecutive events → scheduleRaf is idempotent', () => {
  it('requestAnimationFrame called exactly once regardless of event count', async () => {
    await mountPane();
    initGrid(4, 3);

    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'X', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 1, col: 1, content: 'Y', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // scheduleRaf() must guard against double-scheduling: rafId !== null on the
    // second call means only one rAF handle is ever requested.
    expect(rafSpy).toHaveBeenCalledTimes(1);
  });
});

// ---------------------------------------------------------------------------
// RAF-BATCH-004: event during buffering phase → pendingUpdates, no rAF
// ---------------------------------------------------------------------------

describe('RAF-BATCH-004: event during buffering → goes to pendingUpdates, rAF not scheduled', () => {
  it('requestAnimationFrame is not called while buffering is true', async () => {
    // Block the snapshot to keep buffering === true for the duration of this test.
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
      props: { paneId: PANE_ID, tabId: 'tab-raf-buf', active: true },
    });
    instances.push(instance);

    // Drain enough microtasks so the screen-update listener is registered
    // (registered before the snapshot fetch per WP3b), but do NOT resolve
    // the snapshot so buffering remains true.
    for (let i = 0; i < 5; i++) await Promise.resolve();

    rafSpy.mockClear();
    rafCallback = null;

    // Fire an event while snapshot is still pending (buffering === true).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'Z', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // rAF must NOT have been scheduled — the buffering branch returns early.
    expect(rafSpy).toHaveBeenCalledTimes(0);

    // Unblock the snapshot so onDestroy can clean up gracefully.
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
    for (let i = 0; i < 20; i++) await Promise.resolve();
  });
});

// ---------------------------------------------------------------------------
// RAF-BATCH-005: event for a different paneId → filtered, rAF not scheduled
// ---------------------------------------------------------------------------

describe('RAF-BATCH-005: event for wrong paneId → filtered, rAF not scheduled', () => {
  it('requestAnimationFrame is not called when paneId does not match', async () => {
    await mountPane();
    initGrid(4, 3);

    rafSpy.mockClear();
    rafCallback = null;

    // Fire an event targeting a different pane.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(OTHER_PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'W', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    expect(rafSpy).toHaveBeenCalledTimes(0);
  });
});

// ---------------------------------------------------------------------------
// RAF-BATCH-006: onDestroy with pending rAF → cancelAnimationFrame called
// ---------------------------------------------------------------------------

describe('RAF-BATCH-006: onDestroy with pending rAF cancels the animation frame', () => {
  it('cancelAnimationFrame is called with the registered rafId on unmount', async () => {
    const { container, instance } = await mountPane();
    initGrid(4, 3);

    rafSpy.mockClear();
    cancelRafSpy.mockClear();
    rafCallback = null;

    // Schedule a rAF by firing an event — do NOT flush it.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'D', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // Confirm rAF was scheduled (fake id = 42 per our mock).
    expect(rafSpy).toHaveBeenCalledTimes(1);
    expect(rafCallback).not.toBeNull();

    // Unmount — onDestroy must cancel the pending rAF.
    unmount(instance);
    // Remove from tracked instances so afterEach doesn't double-unmount.
    const idx = instances.indexOf(instance);
    if (idx !== -1) instances.splice(idx, 1);

    document.body.removeChild(container);

    // cancelAnimationFrame must have been called with the id returned by our mock (42).
    expect(cancelRafSpy).toHaveBeenCalledWith(42);
  });
});

// ---------------------------------------------------------------------------
// RAF-BATCH-007: N partial events on distinct cells → single applyScreenUpdate,
//                all cells present (P-HT-1 coalescing)
// ---------------------------------------------------------------------------

describe('RAF-BATCH-007: N partials on distinct cells → merged into one apply, all cells present', () => {
  it('three partial events merged — all three cells visible after one rAF flush', async () => {
    const { container } = await mountPane();
    initGrid(4, 3);

    rafSpy.mockClear();
    rafCallback = null;

    // Fire 3 events targeting distinct cells.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'A', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 1, col: 1, content: 'B', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 2, col: 2, content: 'C', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // Only one rAF should be scheduled.
    expect(rafSpy).toHaveBeenCalledTimes(1);

    flushRaf();
    await flushSync();

    // All three cells must be visible.
    expect(getCellText(container, 0, 0)).toBe('A');
    expect(getCellText(container, 1, 1)).toBe('B');
    expect(getCellText(container, 2, 2)).toBe('C');
  });
});

// ---------------------------------------------------------------------------
// RAF-BATCH-008: full-redraw followed by partial in same rAF tick →
//                single call, grid reflects both
// ---------------------------------------------------------------------------

describe('RAF-BATCH-008: full-redraw + partial in same tick → single merged apply', () => {
  it('full-redraw grid is established and partial overlay is applied', async () => {
    const { container } = await mountPane();
    initGrid(4, 3);

    rafSpy.mockClear();
    rafCallback = null;

    // Full-redraw: fill entire grid with spaces (implicit from blank cells).
    const fullCells = [];
    for (let r = 0; r < 3; r++) {
      for (let c = 0; c < 4; c++) {
        fullCells.push({ row: r, col: c, content: '.', width: 1, attrs: DEFAULT_ATTRS });
      }
    }
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: true,
        cells: fullCells,
      }),
    );

    // Partial update on top of the full-redraw (same rAF tick).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'Z', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    expect(rafSpy).toHaveBeenCalledTimes(1);

    flushRaf();
    await flushSync();

    // The full-redraw set everything to '.', then the partial overwrote (0,0) with 'Z'.
    expect(getCellText(container, 0, 0)).toBe('Z');
    expect(getCellText(container, 0, 1)).toBe('.');
    expect(getCellText(container, 1, 0)).toBe('.');
  });
});
