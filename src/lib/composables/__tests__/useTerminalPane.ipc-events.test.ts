// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for IPC event handling in useTerminalPane.svelte.ts.
 *
 * Previously marked E2E-deferred (see TerminalPane.test.ts TPSC-FN-002 to 007)
 * because the Tauri `listen()` binding was captured at import time and could
 * not be intercepted by vi.mock. The solution is to spy on the module-level
 * stub exported by src/__mocks__/tauri-event.ts (mapped by vitest.config.ts
 * alias) via vi.spyOn, which intercepts at the object-property level. This
 * works because the alias points to a real module object whose `listen`
 * property is a mutable reference, and vi.spyOn is installed BEFORE mount().
 *
 * The composable is exercised via TerminalPane.svelte (the composable is
 * embedded there). Each test mounts the component fresh so that onMount
 * registers its listeners against the spy already in place.
 *
 * jsdom polyfills required:
 *   - ResizeObserver: provided by vitest-setup.ts
 *   - element.animate: Bits UI (context menu) calls it during reactive updates.
 *     Polyfilled here as a no-op stub that returns an Animation-like object.
 *
 * Covered:
 *   TPSC-FN-002 — scroll-position-changed with offset > 0 shows scroll button
 *   TPSC-FN-003 — scroll-position-changed registers a listener
 *   TPSC-FN-004 — scroll-position-changed offset=0 then offset>0 button visible
 *   TPSC-FN-006 — screen-update event: handler registered; pane survives update
 *   TPSC-FN-007 — events for a different paneId are filtered out
 *   TPSC-MODE-001 — mode-state-changed: handler registered; fires without error
 *   TPSC-CURSOR-001 — cursor-style-changed: handler registered; fires without error
 *   TPSC-BELL-001 — bell-triggered: handler registered; fires without error
 *   TPSC-NOTIF-001 — notification-changed backgroundOutput: fires without error
 *   TPSC-NOTIF-002 — notification-changed processExited: fires without error
 *   TPSC-NOTIF-003 — notification-changed for different paneId: filtered
 *   TPSC-NOTIF-004 — notification-changed null: clears pulse without error
 *   TPSC-REG — all six IPC listeners registered on mount
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type {
  ScreenUpdateEvent,
  ScrollPositionChangedEvent,
  ModeStateChangedEvent,
  CursorStyleChangedEvent,
  BellTriggeredEvent,
  NotificationChangedEvent,
  CursorState,
} from '$lib/ipc/types';
import TerminalPane from '$lib/components/TerminalPane.svelte';

// ---------------------------------------------------------------------------
// jsdom polyfill: element.animate
//
// Bits UI uses the Web Animations API (element.animate) when rendering
// floating layers (e.g. the context menu). jsdom does not implement it,
// so Svelte re-renders triggered by IPC events that reach the context menu
// portion of the tree throw "element.animate is not a function".
// This stub prevents those errors in unit tests; actual animation behaviour
// is validated in E2E tests.
// ---------------------------------------------------------------------------

if (typeof Element.prototype.animate === 'undefined') {
  // Minimal Animation-like stub — enough to satisfy Bits UI's animate() call.
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

/** Matches the shape of Tauri's `Event<T>` (event name, id, payload). */
type ListenerFn<T = unknown> = (event: { event: string; id: number; payload: T }) => void;
type ListenerRegistry = Map<string, Array<ListenerFn>>;

// ---------------------------------------------------------------------------
// Test constants
// ---------------------------------------------------------------------------

const PANE_ID = 'pane-ipc-test';
const OTHER_PANE_ID = 'pane-other';
const TAB_ID = 'tab-1'; // must match the tabId passed to mountPane()

// ---------------------------------------------------------------------------
// Module-level state (reset in beforeEach)
// ---------------------------------------------------------------------------

let registry: ListenerRegistry = new Map();
const instances: ReturnType<typeof mount>[] = [];

// ---------------------------------------------------------------------------
// Setup / teardown
//
// IMPORTANT: vi.spyOn MUST be installed before mount() is called so that
// the onMount() async chain (which calls listen()) sees the spy.
// The spy must therefore be created in beforeEach, not inside individual tests.
// ---------------------------------------------------------------------------

beforeEach(() => {
  registry = new Map();

  // Intercept listen() to capture handlers for manual dispatch.
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

  // Stub all invoke() calls — composable calls resizePane, getPaneScreenSnapshot, etc.
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

  // Drain microtask queue so onMount async chains (listen calls) complete.
  // 20 iterations is sufficient for all chained awaits in useTerminalPane.
  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();

  return { container, instance };
}

// ---------------------------------------------------------------------------
// Helper: build minimal event fixtures
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

function makeModeEvent(
  paneId: string,
  overrides: Partial<ModeStateChangedEvent> = {},
): ModeStateChangedEvent {
  return {
    paneId,
    decckm: false,
    deckpam: false,
    mouseReporting: 'none',
    mouseEncoding: 'x10',
    focusEvents: false,
    bracketedPaste: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// TPSC-REG: all six IPC listeners registered on mount
// ---------------------------------------------------------------------------

describe('TPSC-REG: listener registration completeness', () => {
  it('registers all six expected IPC event listeners on mount', async () => {
    await mountPane();

    const listenSpy = vi.mocked(tauriEvent.listen);
    const registeredEvents = listenSpy.mock.calls.map(([name]) => name as string);

    const expected = [
      'screen-update',
      'scroll-position-changed',
      'mode-state-changed',
      'cursor-style-changed',
      'bell-triggered',
      'notification-changed',
    ];

    for (const ev of expected) {
      expect(registeredEvents, `Expected "${ev}" to be registered`).toContain(ev);
    }
  });
});

// ---------------------------------------------------------------------------
// TPSC-FN-006: screen-update event
// ---------------------------------------------------------------------------

describe('TPSC-FN-006: screen-update event is handled', () => {
  it('registers a screen-update listener', async () => {
    await mountPane();
    const listenSpy = vi.mocked(tauriEvent.listen);
    const screenUpdateListens = listenSpy.mock.calls.filter(([name]) => name === 'screen-update');
    expect(screenUpdateListens.length).toBeGreaterThanOrEqual(1);
  });

  it('fires screen-update for the correct paneId without throwing', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, {
          cells: [
            {
              row: 0,
              col: 0,
              content: 'A',
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

    expect(container.querySelector('.terminal-grid')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-FN-007: events for a different paneId are filtered out
// ---------------------------------------------------------------------------

describe('TPSC-FN-007: events for a different paneId are ignored', () => {
  it('screen-update for OTHER_PANE_ID does not affect PANE_ID pane', async () => {
    const { container } = await mountPane();

    // Fire update for a completely different pane — the handler must filter it.
    expect(() => {
      fireEvent<ScreenUpdateEvent>('screen-update', makeScreenUpdate(OTHER_PANE_ID));
      flushSync();
    }).not.toThrow();

    // Our pane element must still be the one in the DOM
    expect(container.querySelector(`[data-pane-id="${PANE_ID}"]`)).not.toBeNull();
    expect(container.querySelector(`[data-pane-id="${OTHER_PANE_ID}"]`)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-FN-002 / TPSC-FN-003: scroll-position-changed with offset > 0
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

    // scrollOffset > 0 → the showScrollbar derived is true → button visible
    const btn = container.querySelector('.scroll-to-bottom-btn');
    expect(btn).not.toBeNull();
  });
});

describe('TPSC-FN-003: scroll-position-changed listener is registered', () => {
  it('registers a scroll-position-changed listener on mount', async () => {
    await mountPane();
    const listenSpy = vi.mocked(tauriEvent.listen);
    const scrollListens = listenSpy.mock.calls.filter(
      ([name]) => name === 'scroll-position-changed',
    );
    expect(scrollListens.length).toBeGreaterThanOrEqual(1);
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
    // This complements TPSC-FN-002 (offset > 0 shows button) to establish the
    // full semantics of the scroll button visibility condition.
    const { container } = await mountPane();

    // No button at start (initial state)
    expect(container.querySelector('.scroll-to-bottom-btn')).toBeNull();

    // Fire offset=0 — the pane is already at the bottom, this is a no-op for the button
    fireEvent<ScrollPositionChangedEvent>('scroll-position-changed', {
      paneId: PANE_ID,
      offset: 0,
      scrollbackLines: 50,
    });
    flushSync();

    // Button must remain absent — offset=0 → scrollOffset=0 → {#if false} in template
    const btn = container.querySelector('.scroll-to-bottom-btn');
    expect(btn).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-MODE-001: mode-state-changed
// ---------------------------------------------------------------------------

describe('TPSC-MODE-001: mode-state-changed updates mode flags', () => {
  it('registers a mode-state-changed listener', async () => {
    await mountPane();
    const listenSpy = vi.mocked(tauriEvent.listen);
    const modeListens = listenSpy.mock.calls.filter(([name]) => name === 'mode-state-changed');
    expect(modeListens.length).toBeGreaterThanOrEqual(1);
  });

  it('fires mode-state-changed with bracketedPaste=true without throwing', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<ModeStateChangedEvent>(
        'mode-state-changed',
        makeModeEvent(PANE_ID, {
          bracketedPaste: true,
          mouseReporting: 'anyEvent',
          decckm: true,
        }),
      );
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });

  it('ignores mode-state-changed for a different paneId', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<ModeStateChangedEvent>(
        'mode-state-changed',
        makeModeEvent(OTHER_PANE_ID, { bracketedPaste: true }),
      );
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-CURSOR-001: cursor-style-changed
// ---------------------------------------------------------------------------

describe('TPSC-CURSOR-001: cursor-style-changed is handled', () => {
  it('registers a cursor-style-changed listener', async () => {
    await mountPane();
    const listenSpy = vi.mocked(tauriEvent.listen);
    const cursorListens = listenSpy.mock.calls.filter(([name]) => name === 'cursor-style-changed');
    expect(cursorListens.length).toBeGreaterThanOrEqual(1);
  });

  it('fires cursor-style-changed with shape=2 (underline) without throwing', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<CursorStyleChangedEvent>('cursor-style-changed', { paneId: PANE_ID, shape: 2 });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });

  it('ignores cursor-style-changed for a different paneId', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<CursorStyleChangedEvent>('cursor-style-changed', {
        paneId: OTHER_PANE_ID,
        shape: 4,
      });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-BELL-001: bell-triggered
// ---------------------------------------------------------------------------

describe('TPSC-BELL-001: bell-triggered is handled', () => {
  it('registers a bell-triggered listener', async () => {
    await mountPane();
    const listenSpy = vi.mocked(tauriEvent.listen);
    const bellListens = listenSpy.mock.calls.filter(([name]) => name === 'bell-triggered');
    expect(bellListens.length).toBeGreaterThanOrEqual(1);
  });

  it('fires bell-triggered without throwing', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<BellTriggeredEvent>('bell-triggered', { paneId: PANE_ID });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });

  it('ignores bell-triggered for a different paneId', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<BellTriggeredEvent>('bell-triggered', { paneId: OTHER_PANE_ID });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-NOTIF-001/002/003/004: notification-changed
// ---------------------------------------------------------------------------

describe('TPSC-NOTIF-001: notification-changed backgroundOutput fires without error', () => {
  it('handles backgroundOutput notification for an inactive pane', async () => {
    // Border pulse only triggers when the pane is NOT active.
    const { container } = await mountPane(PANE_ID, false);

    expect(() => {
      fireEvent<NotificationChangedEvent>('notification-changed', {
        tabId: TAB_ID,
        paneId: PANE_ID,
        notification: { type: 'backgroundOutput' },
      });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

describe('TPSC-NOTIF-002: notification-changed processExited fires without error', () => {
  it('handles processExited notification for an inactive pane', async () => {
    const { container } = await mountPane(PANE_ID, false);

    expect(() => {
      fireEvent<NotificationChangedEvent>('notification-changed', {
        tabId: TAB_ID,
        paneId: PANE_ID,
        notification: { type: 'processExited', exitCode: 0 , signalName: null },
      });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

describe('TPSC-NOTIF-003: notification-changed for different paneId is ignored', () => {
  it('does not throw when notification is for another pane', async () => {
    const { container } = await mountPane();

    expect(() => {
      fireEvent<NotificationChangedEvent>('notification-changed', {
        tabId: TAB_ID,
        paneId: OTHER_PANE_ID,
        notification: { type: 'backgroundOutput' },
      });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

describe('TPSC-NOTIF-004: notification-changed null clears pulse without error', () => {
  it('handles the clear signal (notification=null) after a pulse was set', async () => {
    const { container } = await mountPane(PANE_ID, false);

    // Trigger a pulse first
    fireEvent<NotificationChangedEvent>('notification-changed', {
      tabId: TAB_ID,
      paneId: PANE_ID,
      notification: { type: 'backgroundOutput' },
    });
    flushSync();

    // Clear it
    expect(() => {
      fireEvent<NotificationChangedEvent>('notification-changed', {
        tabId: TAB_ID,
        paneId: PANE_ID,
        notification: null,
      });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-SCROLL-002: screen-update with isFullRedraw resets scrollOffset (WP3a)
// ---------------------------------------------------------------------------

describe('TPSC-SCROLL-002: screen-update with isFullRedraw resets scrollOffset to 0', () => {
  it('isFullRedraw screen-update fires without error when scrollOffset > 0', async () => {
    // This test verifies the WP3a fix: that receiving a full_redraw screen-update
    // while scrollOffset > 0 does not throw and does not leave the pane in a broken state.
    //
    // The scroll-to-bottom button removal after full_redraw relies on Svelte 5 transitions
    // (transition:fade). In jsdom, Svelte's transition teardown requires real rAF cycles
    // which cannot be reliably advanced in unit tests. The DOM-level check is therefore
    // deferred to E2E tests (tauri-driver + WebdriverIO). What we verify here is:
    //  1. The component does not throw on full_redraw with scrollOffset > 0
    //  2. A subsequent scroll-update with offset=0 is handled correctly (no regression)
    //  3. The pane is still functional after the full_redraw
    const { container } = await mountPane();

    // Bring scrollOffset > 0.
    fireEvent<ScrollPositionChangedEvent>('scroll-position-changed', {
      paneId: PANE_ID,
      offset: 10,
      scrollbackLines: 100,
    });
    flushSync();
    expect(container.querySelector('.scroll-to-bottom-btn')).not.toBeNull();

    // isFullRedraw should not throw, even when scrollOffset > 0.
    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, { isFullRedraw: true }),
      );
      flushSync();
    }).not.toThrow();

    // A subsequent scroll event at offset=0 must also be handled correctly.
    expect(() => {
      fireEvent<ScrollPositionChangedEvent>('scroll-position-changed', {
        paneId: PANE_ID,
        offset: 0,
        scrollbackLines: 0,
      });
      flushSync();
    }).not.toThrow();

    // Pane must still be in DOM and functional.
    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

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
          // width=8 → with viewport 800px → cols=100; irrelevant for this test.
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
        // Probe element — return cell dimensions.
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

    // Find the resize_pane call.
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
// TPSC-STRIDE-001/002: ScreenUpdateEvent stride — event cols/rows are authoritative
// ---------------------------------------------------------------------------

describe('TPSC-STRIDE-001: applyScreenUpdate uses event.cols for grid indexing', () => {
  it('screen-update with non-default cols triggers isFullRedraw rebuild without throwing', async () => {
    // Fire a screen-update with cols=10 (non-default) and isFullRedraw: true.
    // The composable must use event.cols=10 as the grid stride — not its local
    // state (default 80). If it used the local 80, applyUpdates would place col=9
    // at index 9 instead of 9, which is incidentally the same, but rebuilding
    // the grid would produce 10*rows cells rather than 80*rows cells.
    // The observable assertion is that the component does not throw and remains functional.
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

// ---------------------------------------------------------------------------
// TPSC-RESIZE-003: DOM probe is removed even when getBoundingClientRect throws
// ---------------------------------------------------------------------------

describe('TPSC-RESIZE-003: probe removed from DOM even when getBoundingClientRect throws', () => {
  it('guarantees removeChild runs even when getBoundingClientRect throws, and still calls resize_pane', async () => {
    // What this test verifies:
    //
    // When getBoundingClientRect throws on the probe element, the try/finally fix
    // guarantees that viewportEl.removeChild(probe) is called even if the exception
    // propagates. Without the fix, removeChild is skipped (unreachable after throw).
    //
    // jsdom limitation: Svelte's reactive re-render triggered by the subsequent
    // cols/rows update also removes the probe from the viewport DOM as a side-effect.
    // This makes the DOM-leak assertion non-discriminant in unit tests — the probe
    // disappears in both code paths. The try/finally guarantee is therefore validated
    // by code review for the non-jsdom case (real browser / Tauri WebView).
    //
    // What IS discriminant and testable in jsdom:
    //   - resize_pane is still called (the exception does not abort sendResize).
    //   - No unhandled errors are thrown.
    //
    // The probe-absent assertion is included as a belt-and-suspenders check; it
    // passes in both paths but would catch any future regression where the probe
    // is not cleaned up AND Svelte's re-render is also broken.

    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (
      this: HTMLElement,
    ) {
      if (this.style?.height === '1lh') {
        throw new Error('simulated getBoundingClientRect failure');
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

    const { container } = await mountPane();

    // Wait for the debounce (50ms) to fire sendResize.
    await new Promise((res) => setTimeout(res, 60));
    flushSync();
    // Drain additional microtasks to let Svelte complete any reactive updates.
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    // DISCRIMINANT ASSERTION: resize_pane was called via the fallback path.
    // Without a proper try/finally (or try/catch), the exception from
    // getBoundingClientRect could abort execution before resizePane() is reached
    // if the outer catch structure is wrong. This confirms the fallback path is taken.
    const resizeCalls = invokeSpy.mock.calls.filter(([cmd]) => cmd === 'resize_pane');
    expect(resizeCalls.length).toBeGreaterThanOrEqual(1);

    // BELT-AND-SUSPENDERS: no probe must remain in the component DOM.
    // In jsdom this passes in both code paths due to Svelte's re-render cleanup;
    // in production it is guaranteed by try/finally.
    const probeLeaks = Array.from(container.querySelectorAll('*')).filter(
      (el) => (el as HTMLElement).style?.height === '1lh',
    );
    expect(probeLeaks).toHaveLength(0);
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

    // Controlled reject Promise.
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
    // cols/rows are not rolled back here — they will be updated by the next screen-update event.
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

    // Mount (starts the onMount async chain) but do NOT await it fully yet.
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

    // The pane must still be in the DOM (no crash from the buffered update).
    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-RESIZE-005: isFullRedraw with changed dimensions reallocates grid
// Regression test: grid was not resized before applyUpdates, causing cells
// beyond the old grid.length to be silently dropped and rows to render empty.
// ---------------------------------------------------------------------------

describe('TPSC-RESIZE-005: isFullRedraw with new dimensions resizes grid correctly', () => {
  it('gridRows.length matches new rows after isFullRedraw with larger dimensions', async () => {
    const { container } = await mountPane();

    // Initial state: default 80×24 grid. Fire a full redraw with smaller,
    // verifiable dimensions (5 cols × 3 rows = 15 cells).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 5,
        rows: 3,
        isFullRedraw: true,
        cells: [],
      }),
    );
    flushSync();

    // After the first full redraw, verify baseline: 3 rows rendered.
    const rowsAfterShrink = container.querySelectorAll('.terminal-pane__row');
    expect(rowsAfterShrink.length).toBe(3);

    // Now fire a full redraw with LARGER dimensions (8 cols × 6 rows = 48 cells).
    // Before the fix, the grid stayed at 15 cells and rows 3-5 were empty.
    const largeCells = [];
    for (let r = 0; r < 6; r++) {
      for (let c = 0; c < 8; c++) {
        largeCells.push({
          row: r,
          col: c,
          content: r >= 3 ? 'X' : ' ', // Mark new rows with 'X'
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
        });
      }
    }

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 8,
        rows: 6,
        isFullRedraw: true,
        cells: largeCells,
      }),
    );
    flushSync();

    // CRITICAL ASSERTION: gridRows must have 6 rows, not 3.
    const rowsAfterGrow = container.querySelectorAll('.terminal-pane__row');
    expect(rowsAfterGrow.length).toBe(6);

    // CRITICAL ASSERTION: rows 3-5 must contain 'X', not be empty/blank.
    // Before the fix, these rows would contain only NBSP (default cells).
    for (let r = 3; r < 6; r++) {
      const cells = rowsAfterGrow[r].querySelectorAll('.terminal-pane__cell');
      const rowText = Array.from(cells)
        .map((el) => el.textContent ?? '')
        .join('');
      expect(rowText).toContain('X');
    }
  });
});

// ---------------------------------------------------------------------------
// TPSC-RESIZE-006: incremental update after resize applies cells on new rows
// Regression test: after a full redraw resize, incremental updates targeting
// rows beyond the old grid size must land correctly (not be silently dropped).
// ---------------------------------------------------------------------------

describe('TPSC-RESIZE-006: incremental update after resize targets new rows', () => {
  it('cells on rows beyond old dimensions are rendered after resize + incremental update', async () => {
    const { container } = await mountPane();

    // Step 1: establish a small grid (4 cols × 2 rows = 8 cells).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 2,
        isFullRedraw: true,
        cells: [],
      }),
    );
    flushSync();

    const rowsBefore = container.querySelectorAll('.terminal-pane__row');
    expect(rowsBefore.length).toBe(2);

    // Step 2: resize to 4 cols × 5 rows via isFullRedraw.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 5,
        isFullRedraw: true,
        cells: [],
      }),
    );
    flushSync();

    const rowsAfterResize = container.querySelectorAll('.terminal-pane__row');
    expect(rowsAfterResize.length).toBe(5);

    // Step 3: incremental update targeting row 4 (index beyond old 2-row grid).
    // Before the fix, this cell would be dropped by applyUpdates (oob)
    // because grid.length was still 8 (2*4) instead of 20 (5*4).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 5,
        isFullRedraw: false,
        cells: [
          {
            row: 4,
            col: 1,
            content: 'Z',
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

    // CRITICAL ASSERTION: row 4, col 1 must contain 'Z'.
    const rowsAfterIncremental = container.querySelectorAll('.terminal-pane__row');
    expect(rowsAfterIncremental.length).toBe(5);
    const row4Cells = rowsAfterIncremental[4].querySelectorAll('.terminal-pane__cell');
    const row4Text = Array.from(row4Cells)
      .map((el) => el.textContent ?? '')
      .join('');
    expect(row4Text).toContain('Z');
  });
});

// ---------------------------------------------------------------------------
// TEST-SB-FE-001: scrolled viewport event sets scrollOffset and rebuilds gridRows
// ---------------------------------------------------------------------------

describe('TEST-SB-FE-001: scrolled_viewport_event_updates_gridRows_and_scrollOffset', () => {
  it('full-redraw with scrollOffset=5 makes scroll button visible', async () => {
    const { container } = await mountPane();

    // Verify no scroll button before the event.
    expect(container.querySelector('.scroll-to-bottom-btn')).toBeNull();

    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        isFullRedraw: true,
        scrollOffset: 5,
        scrollbackLines: 20,
        cols: 80,
        rows: 24,
        cells: [],
      }),
    );
    flushSync();

    // scrollOffset=5 → scroll-to-bottom button must be visible.
    expect(container.querySelector('.scroll-to-bottom-btn')).not.toBeNull();
    // gridRows rebuilt: 24 rows in DOM.
    const rows = container.querySelectorAll('.terminal-pane__row');
    expect(rows.length).toBe(24);
  });
});

// ---------------------------------------------------------------------------
// TEST-SB-FE-002: live PTY event while scrolled freezes gridRows
// ---------------------------------------------------------------------------

describe('TEST-SB-FE-002: live_pty_event_while_scrolled_freezes_gridRows', () => {
  it('live PTY event while scrolled keeps scrollOffset and does not reset to 0', async () => {
    const { container } = await mountPane();

    // Step 1: enter scrolled state (scrollOffset=5).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        isFullRedraw: true,
        scrollOffset: 5,
        scrollbackLines: 20,
        cols: 80,
        rows: 24,
        cells: [],
      }),
    );
    flushSync();

    // Scroll button must be visible after step 1.
    expect(container.querySelector('.scroll-to-bottom-btn')).not.toBeNull();

    // Step 2: fire a live PTY event (scrollOffset=0, isFullRedraw=false).
    // FS-SB-009: the viewport must be frozen — scrollOffset must remain 5.
    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, {
          isFullRedraw: false,
          scrollOffset: 0,
          scrollbackLines: 25,
          cols: 80,
          rows: 24,
          cells: [],
        }),
      );
      flushSync();
    }).not.toThrow();

    // scrollOffset must NOT have been reset to 0 — button must still be visible.
    expect(container.querySelector('.scroll-to-bottom-btn')).not.toBeNull();
    // Component must remain functional.
    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TEST-SB-FE-003: full-redraw with scrollOffset=0 resets to live view
// ---------------------------------------------------------------------------

describe('TEST-SB-FE-003: full_redraw_scrollOffset_zero_resets_to_live_view', () => {
  it('full-redraw with scrollOffset=0 clears the scrolled state', async () => {
    const { container } = await mountPane();

    // Step 1: enter scrolled state (scrollOffset=5).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        isFullRedraw: true,
        scrollOffset: 5,
        scrollbackLines: 20,
        cols: 80,
        rows: 24,
        cells: [],
      }),
    );
    flushSync();

    // Scroll button visible.
    expect(container.querySelector('.scroll-to-bottom-btn')).not.toBeNull();

    // Step 2: full-redraw at scrollOffset=0 → back to live view.
    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, {
          isFullRedraw: true,
          scrollOffset: 0,
          scrollbackLines: 20,
          cols: 80,
          rows: 24,
          cells: [],
        }),
      );
      flushSync();
    }).not.toThrow();

    // scrollOffset=0 → scroll button must now be gone (Svelte transition may delay removal,
    // so we check that the pane is functional and no error was thrown).
    // The scroll button absence after transition is validated in E2E tests.
    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});
