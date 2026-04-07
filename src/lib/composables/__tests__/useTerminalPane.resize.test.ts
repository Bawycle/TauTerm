// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for resize / DOM probe behaviour in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-RESIZE-001 — DOM probe is used for row calculation in sendResize
 *   TPSC-RESIZE-003 — probe removed from DOM even when getBoundingClientRect throws
 *   TPSC-RESIZE-004 — resize_pane IPC failure does not crash the component
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { ScreenUpdateEvent, CursorState } from '$lib/ipc/types';
import TerminalPane from '$lib/components/TerminalPane.svelte';

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
// Types
// ---------------------------------------------------------------------------

type ListenerFn<T = unknown> = (event: { event: string; id: number; payload: T }) => void;
type ListenerRegistry = Map<string, Array<ListenerFn>>;

// ---------------------------------------------------------------------------
// Test constants
// ---------------------------------------------------------------------------

const PANE_ID = 'pane-ipc-test';

// ---------------------------------------------------------------------------
// Module-level state (reset in beforeEach)
// ---------------------------------------------------------------------------

let registry: ListenerRegistry = new Map();
const instances: ReturnType<typeof mount>[] = [];

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  registry = new Map();

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
// Helper: fire an event through the registry
// ---------------------------------------------------------------------------

function fireEvent<T>(eventName: string, payload: T): void {
  const handlers = registry.get(eventName) ?? [];
  for (const h of handlers) {
    (h as ListenerFn<T>)({ event: eventName, id: 0, payload });
  }
}

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
// Helper: build minimal screen update fixture
// ---------------------------------------------------------------------------

function makeScreenUpdate(
  paneId: string,
  overrides: Partial<ScreenUpdateEvent> = {},
): ScreenUpdateEvent {
  const cursor: CursorState = { row: 0, col: 0, visible: true, shape: 0, blink: true };
  return {
    paneId,
    cells: [],
    cursor,
    scrollbackLines: 0,
    isFullRedraw: false,
    scrollOffset: 0,
    cols: 80,
    rows: 24,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// TPSC-RESIZE-001: DOM probe is used for row calculation in sendResize
// ---------------------------------------------------------------------------

describe('TPSC-RESIZE-001: DOM probe used for row calculation', () => {
  it('uses 1lh probe height to compute rows instead of analytical formula', async () => {
    // Without the fix, measureCellDimensions returns height = Math.ceil(13 * 1.2) = 16.
    // rows = floor(480 / 16) = 30.
    //
    // With the fix, the DOM probe (mocked to height=20) is used instead.
    // rows = floor(480 / 20) = 24.
    //
    // To make the test discriminant we stub OffscreenCanvas so that
    // measureCellDimensions does NOT throw and returns the analytical height=16.
    // Without the fix this produces rows=30. With the fix, the probe wins and
    // produces rows=24.

    // Stub OffscreenCanvas so measureCellDimensions returns height=16 (13*1.2 ceil).
    class OffscreenCanvasStub {
      constructor(
        public width: number,
        public height: number,
      ) {}
      getContext(_type: string) {
        return {
          font: '',
          measureText: (_text: string) => ({ width: 8 }),
        };
      }
    }
    const savedOC = (globalThis as Record<string, unknown>).OffscreenCanvas;
    (globalThis as Record<string, unknown>).OffscreenCanvas = OffscreenCanvasStub;

    // DOM probe: height=20 (different from analytical 16 — this is what discriminates).
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (
      this: HTMLElement,
    ) {
      if (this.style?.height === '1lh') {
        return {
          height: 20,
          width: 8,
          top: 0,
          left: 0,
          right: 8,
          bottom: 20,
          x: 0,
          y: 0,
          toJSON() {
            return this;
          },
        } as DOMRect;
      }
      // Viewport: 480px tall, 800px wide.
      return {
        height: 480,
        width: 800,
        top: 0,
        left: 0,
        right: 800,
        bottom: 480,
        x: 0,
        y: 0,
        toJSON() {
          return this;
        },
      } as DOMRect;
    });

    const invokeSpy = vi.mocked(tauriCore.invoke);

    await mountPane();

    // Wait for the debounce (50ms) to fire sendResize.
    await new Promise((res) => setTimeout(res, 60));
    flushSync();

    // Restore OffscreenCanvas.
    if (savedOC !== undefined) {
      (globalThis as Record<string, unknown>).OffscreenCanvas = savedOC;
    } else {
      delete (globalThis as Record<string, unknown>).OffscreenCanvas;
    }

    const resizeCalls = invokeSpy.mock.calls.filter(([cmd]) => cmd === 'resize_pane');
    expect(resizeCalls.length).toBeGreaterThanOrEqual(1);

    const lastCall = resizeCalls[resizeCalls.length - 1];
    const calledRows = (lastCall[1] as { rows: number }).rows;

    // With the fix (probe height=20): rows = floor(480/20) = 24.
    // Without the fix (analytical height=16): rows = floor(480/16) = 30.
    expect(calledRows).toBe(24);
  });
});

// ---------------------------------------------------------------------------
// TPSC-RESIZE-003: DOM probe fallback path works when getBoundingClientRect throws
// ---------------------------------------------------------------------------

describe('TPSC-RESIZE-003: probe removed from DOM even when getBoundingClientRect throws', () => {
  it('guarantees removeChild runs even when getBoundingClientRect throws, and still calls resize_pane', async () => {
    // What this test verifies:
    //
    // D2-P16: the probe is now persistent (created once, reused across sendResize calls).
    // When getBoundingClientRect throws on the probe element, sendResize catches the
    // exception and falls back to the analytical formula — it does NOT crash the
    // component and still calls resize_pane.
    //
    // The probe remains attached to the viewport DOM during the pane's lifetime
    // (by design). It is removed in onDestroy via cellMeasureProbe?.remove().
    //
    // What IS discriminant and testable:
    //   - resize_pane is still called (the exception does not abort sendResize).
    //   - No unhandled errors are thrown.
    //   - After unmount, the probe has been removed from the document.

    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (
      this: HTMLElement,
    ) {
      if (this.style?.height === '1lh') {
        throw new Error('simulated getBoundingClientRect failure');
      }
      return {
        height: 480,
        width: 800,
        top: 0,
        left: 0,
        right: 800,
        bottom: 480,
        x: 0,
        y: 0,
        toJSON() {
          return this;
        },
      } as DOMRect;
    });

    const invokeSpy = vi.mocked(tauriCore.invoke);

    const { container, instance } = await mountPane();

    // Wait for the debounce (50ms) to fire sendResize.
    await new Promise((res) => setTimeout(res, 60));
    flushSync();
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    // DISCRIMINANT ASSERTION: resize_pane was called via the fallback path.
    const resizeCalls = invokeSpy.mock.calls.filter(([cmd]) => cmd === 'resize_pane');
    expect(resizeCalls.length).toBeGreaterThanOrEqual(1);

    // CLEANUP ASSERTION (D2-P16): after unmount, onDestroy removes the probe.
    unmount(instance);
    flushSync();

    const probeLeaks = Array.from(document.querySelectorAll('*')).filter(
      (el) => (el as HTMLElement).style?.height === '1lh',
    );
    expect(probeLeaks).toHaveLength(0);

    // Remove from instances to avoid double-unmount in afterEach.
    const idx = instances.indexOf(instance);
    if (idx !== -1) instances.splice(idx, 1);

    // Also clean up container manually since afterEach clears body.
    container.remove();
  });
});

// ---------------------------------------------------------------------------
// TPSC-RESIZE-004: resize_pane IPC failure does not crash the component
// ---------------------------------------------------------------------------

describe('TPSC-RESIZE-004: resize_pane IPC failure is handled gracefully', () => {
  it('component stays functional after resizePane IPC failure', async () => {
    // cols/rows are no longer rolled back on IPC failure — they update solely via
    // screen-update events. This test verifies that an IPC failure in sendResize
    // does not crash the component and that subsequent screen-update events still
    // apply correctly (no stale or broken state).

    let rejectResize!: (err: Error) => void;
    const resizePromise = new Promise<never>((_res, rej) => {
      rejectResize = rej;
    });

    vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'resize_pane') {
        return resizePromise;
      }
      return undefined as never;
    });

    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (
      this: HTMLElement,
    ) {
      if (this.style?.height === '1lh') {
        return {
          height: 20,
          width: 8,
          top: 0,
          left: 0,
          right: 8,
          bottom: 20,
          x: 0,
          y: 0,
          toJSON() {
            return this;
          },
        } as DOMRect;
      }
      return {
        height: 480,
        width: 800,
        top: 0,
        left: 0,
        right: 800,
        bottom: 480,
        x: 0,
        y: 0,
        toJSON() {
          return this;
        },
      } as DOMRect;
    });

    const { container } = await mountPane();

    // Wait for the debounce (50ms) to fire and sendResize to suspend.
    await new Promise((res) => setTimeout(res, 60));
    flushSync();
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    // Trigger IPC failure.
    rejectResize(new Error('simulated IPC failure'));

    // Drain microtasks — catch block runs here.
    for (let i = 0; i < 30; i++) await Promise.resolve();
    flushSync();

    // DISCRIMINANT ASSERTION: the component must still be functional after the failure.
    expect(container.querySelector('.terminal-pane')).not.toBeNull();

    // A subsequent screen-update must still be handled correctly.
    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, { cols: 100, rows: 24, isFullRedraw: true }),
      );
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});
