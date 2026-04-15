// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for IPC listener registration in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-REG      — all six IPC listeners registered on mount
 *   TPSC-FN-003   — scroll-position-changed registers a listener
 *   TPSC-FN-006   — screen-update listener registered; pane survives update
 *   TPSC-FN-007   — events for a different paneId are filtered out
 *   TPSC-MODE-001 — mode-state-changed: handler registered; fires without error
 *   TPSC-CURSOR-001 — cursor-style-changed: handler registered; fires without error
 *   TPSC-BELL-001 — bell-triggered: handler registered; fires without error
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type {
  ScreenUpdateEvent,
  ModeStateChangedEvent,
  CursorStyleChangedEvent,
  BellTriggeredEvent,
  CursorState,
} from '$lib/ipc';
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
  if (vi.isFakeTimers()) vi.useRealTimers();
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
// TPSC-FN-003: scroll-position-changed listener is registered
// ---------------------------------------------------------------------------

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

    expect(() => {
      fireEvent<ScreenUpdateEvent>('screen-update', makeScreenUpdate(OTHER_PANE_ID));
      flushSync();
    }).not.toThrow();

    expect(container.querySelector(`[data-pane-id="${PANE_ID}"]`)).not.toBeNull();
    expect(container.querySelector(`[data-pane-id="${OTHER_PANE_ID}"]`)).toBeNull();
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
// TPSC-CURSOR-002: cursor-style-changed does not overwrite cursor.blink
//
// cursor.blink = DECSET ?12 state, owned exclusively by screen-update.
// cursor-style-changed must update only cursor.shape.
// ---------------------------------------------------------------------------

describe('TPSC-CURSOR-002: cursor-style-changed preserves cursor.blink from screen-update', () => {
  it('cursor.blink=false (from screen-update) survives a cursor-style-changed with blinking shape', async () => {
    vi.useFakeTimers();
    const { container } = await mountPane();

    // Step 1: backend sends screen-update with cursor.blink=false (DECSET ?12 off).
    // cursor.visible=true so the cursor is in the DOM when not blinking.
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cursor: { row: 0, col: 0, visible: true, shape: 0, blink: false },
      }),
    );
    flushSync();

    // Step 2: backend sends cursor-style-changed with shape=1 (blinking block DECSCUSR).
    // This must NOT set cursor.blink=true.
    fireEvent<CursorStyleChangedEvent>('cursor-style-changed', { paneId: PANE_ID, shape: 1 });
    flushSync();

    // Step 3: with cursor.blink=false, currentCursorBlinks = active && false && … = false.
    // Therefore cursorVisible is irrelevant — the cursor renders unconditionally.
    // Advance past one full blink ON phase — cursor must still be visible.
    vi.advanceTimersByTime(533);
    flushSync();

    // If cursor.blink had been overwritten to true, currentCursorBlinks would be true
    // and the cursor would disappear after 533ms. Presence here proves blink was not overwritten.
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-CURSOR-003: screen-update with cursor.blink=true triggers blink cycle
// ---------------------------------------------------------------------------

describe('TPSC-CURSOR-003: screen-update with cursor.blink=true enables the blink cycle', () => {
  it('cursor disappears after cursorBlinkMs when screen-update sets cursor.blink=true', async () => {
    vi.useFakeTimers();
    const { container } = await mountPane();

    // Fire screen-update with cursor.blink=true (backend default after the Rust fix).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cursor: { row: 0, col: 0, visible: true, shape: 1, blink: true },
      }),
    );
    flushSync();

    // Cursor must be visible immediately (cursorVisible starts true).
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();

    // Advance one ON phase — cursor must enter the OFF phase (invisible).
    vi.advanceTimersByTime(533);
    flushSync();

    expect(container.querySelector('.terminal-pane__cursor')).toBeNull();
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
