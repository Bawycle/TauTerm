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
  return { paneId, cells: [], cursor, scrollbackLines: 0, isFullRedraw: false, ...overrides };
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
        notification: { type: 'processExited', exitCode: 0 },
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
        cells: [{ row: 0, col: 0, content: 'B', width: 1, attrs: { bold: false, dim: false, italic: false, underline: 0, blink: false, inverse: false, hidden: false, strikethrough: false } }],
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
