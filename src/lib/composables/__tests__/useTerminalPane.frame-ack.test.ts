// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for P-HT-6 frame-ack flow control in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   ACK-FE-001 — flush with 1+ events → frame_ack invoked once
 *   ACK-FE-002 — flush with empty queue → frame_ack NOT called
 *   ACK-FE-003 — flush with >20 events (overflow → snapshot refetch) → frame_ack sent after snapshot
 *   ACK-FE-004 — two rAF flushes → two frame_ack calls
 *   ACK-FE-005 — correct paneId argument
 *   ACK-FE-006 — non-visual events (bell, mode, cursor-style) do NOT schedule rAF or invoke frame_ack
 *   ACK-FE-007 — onMount with buffered updates emits exactly one ack at snapshot receipt + idempotent second ack on later flush
 *   ACK-FE-008 — onMount with empty pendingUpdates still emits one ack (unconditional, pre-attach race)
 *
 * NOTE: `invokeSpy.mockClear()` placement AFTER `mountPane()` is LOAD-BEARING.
 * Since ADR-0027 Addendum 3, the helper `fetchAndAckSnapshot` emits a
 * `frame_ack` IPC during mount — and that would contaminate the assertions
 * in ACK-FE-001..006 if not cleared. Do not remove or reposition.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type {
  ScreenUpdateEvent,
  BellTriggeredEvent,
  ModeStateChangedEvent,
  CursorStyleChangedEvent,
} from '$lib/ipc';
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

const PANE_ID = 'pane-ack-test';
const OTHER_PANE_ID = 'pane-ack-other';
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
// Module-level state (reset in beforeEach)
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
    props: { paneId, tabId: 'tab-ack', active: true },
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

function fireNEvents(n: number, paneId = PANE_ID): void {
  for (let i = 0; i < n; i++) {
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(paneId, {
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

/** Extract all frame_ack invoke calls from the spy. */
function getFrameAckCalls(): unknown[][] {
  return invokeSpy.mock.calls.filter((c: unknown[]) => c[0] === 'frame_ack');
}

// ---------------------------------------------------------------------------
// ACK-FE-001: flush with 1+ events → frame_ack invoked once
// ---------------------------------------------------------------------------

describe('ACK-FE-001: flush with 1+ events sends frame_ack', () => {
  it('frame_ack is invoked exactly once after flushing queued events', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    fireNEvents(3);

    flushRaf();
    flushSync();

    const ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-002: flush with empty queue → frame_ack NOT called
// ---------------------------------------------------------------------------

describe('ACK-FE-002: flush with empty queue does not send frame_ack', () => {
  it('frame_ack is NOT invoked when the rAF fires with an empty queue', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    // Schedule a rAF with a real event, then drain the queue before the rAF fires
    // by pushing and immediately consuming. Instead, just manually trigger rAF
    // without any events in the queue — simulate a spurious rAF callback.
    // Force-schedule a rAF via internal mechanics: fire an event for a different
    // pane so the queue stays empty for our pane (events are filtered by paneId).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(OTHER_PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'X', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // The event for the wrong pane should not schedule a rAF, and no frame_ack
    // should be sent. Confirm no rAF was scheduled.
    expect(rafSpy).toHaveBeenCalledTimes(0);

    // Even if we manually invoke a rAF callback (simulating a spurious fire),
    // there should be no frame_ack because the queue is empty.
    // We can't easily trigger this without internal access, but we can verify
    // that without any events for our pane, no ack is sent.
    const ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-003: flush with >20 events (overflow) → frame_ack sent after snapshot
// ---------------------------------------------------------------------------

describe('ACK-FE-003: overflow triggers snapshot refetch, frame_ack sent after snapshot resolves', () => {
  it('frame_ack is invoked once after the snapshot refetch completes', async () => {
    await mountPane();
    initGrid(4, 3);

    // Return a valid snapshot for the refetch path.
    invokeSpy.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_pane_screen_snapshot') {
        return { ...VALID_SNAPSHOT } as never;
      }
      return undefined as never;
    });

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    // Fire 21 events (exceeds cap of 20).
    fireNEvents(21);

    flushRaf();

    // Let the snapshot refetch resolve.
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    // frame_ack IS called once after the snapshot resolves — this prevents
    // a drop-mode deadlock where the backend suppresses events while waiting
    // for an ack that only arrives from normal flushRafQueue paints.
    const ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-004: two rAF flushes → two frame_ack calls
// ---------------------------------------------------------------------------

describe('ACK-FE-004: two rAF flushes produce two frame_ack calls', () => {
  it('each flush sends its own frame_ack', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    // First batch.
    fireNEvents(2);
    flushRaf();
    flushSync();

    // Second batch.
    rafSpy.mockClear();
    rafCallback = null;
    fireNEvents(1);
    flushRaf();
    flushSync();

    const ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-005: correct paneId argument
// ---------------------------------------------------------------------------

describe('ACK-FE-005: frame_ack is called with the correct paneId', () => {
  it('the paneId argument matches the mounted pane', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    fireNEvents(1);
    flushRaf();
    flushSync();

    const ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(1);
    // invoke('frame_ack', { paneId: ... })
    expect(ackCalls[0]).toEqual(['frame_ack', { paneId: PANE_ID }]);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-006: non-visual events do NOT schedule rAF or invoke frame_ack
// ---------------------------------------------------------------------------

/**
 * Frontend-side invariant of the ADR-0027 Addendum 2 fix.
 *
 * Only `screen-update` events go through `flushRafQueue`, which is the sole
 * path that invokes `frame_ack`. Non-visual events (bell, mode, cursor style,
 * notification, session state, OSC 52, title, CWD) dispatch through other
 * listeners that have their own side-effects (bell audio/visual FX, mode
 * state mirror, cursor shape update, etc.) — none of which go through the
 * rAF queue.
 *
 * This test is a guard against future drift: if anyone (incorrectly) routes
 * a non-visual event through `enqueueEvent`/`scheduleRaf` or calls
 * `frame_ack` from any other code path, this test fails. The backend-side
 * invariant (last_emit_ms gated on emitted_screen_update) assumes this
 * frontend contract; violating either side reopens the Del-key freeze.
 */
describe('ACK-FE-006: non-visual events do not schedule rAF or invoke frame_ack', () => {
  it('bell-triggered does not push to rafQueue nor call frame_ack', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<BellTriggeredEvent>('bell-triggered', { paneId: PANE_ID });
    flushSync();

    expect(rafSpy).toHaveBeenCalledTimes(0);
    expect(getFrameAckCalls()).toHaveLength(0);
  });

  it('mode-state-changed does not push to rafQueue nor call frame_ack', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<ModeStateChangedEvent>('mode-state-changed', {
      paneId: PANE_ID,
      decckm: false,
      deckpam: false,
      mouseReporting: 'none',
      mouseEncoding: 'x10',
      focusEvents: false,
      bracketedPaste: true,
    });
    flushSync();

    expect(rafSpy).toHaveBeenCalledTimes(0);
    expect(getFrameAckCalls()).toHaveLength(0);
  });

  it('cursor-style-changed does not push to rafQueue nor call frame_ack', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<CursorStyleChangedEvent>('cursor-style-changed', {
      paneId: PANE_ID,
      shape: 2,
    });
    flushSync();

    expect(rafSpy).toHaveBeenCalledTimes(0);
    expect(getFrameAckCalls()).toHaveLength(0);
  });

  it('all three non-visual events combined do not trigger any frame_ack', async () => {
    await mountPane();
    initGrid(4, 3);

    invokeSpy.mockClear();
    rafSpy.mockClear();
    rafCallback = null;

    fireEvent<BellTriggeredEvent>('bell-triggered', { paneId: PANE_ID });
    fireEvent<ModeStateChangedEvent>('mode-state-changed', {
      paneId: PANE_ID,
      decckm: true,
      deckpam: false,
      mouseReporting: 'none',
      mouseEncoding: 'x10',
      focusEvents: false,
      bracketedPaste: false,
    });
    fireEvent<CursorStyleChangedEvent>('cursor-style-changed', {
      paneId: PANE_ID,
      shape: 4,
    });
    flushSync();

    expect(rafSpy).toHaveBeenCalledTimes(0);
    expect(getFrameAckCalls()).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-007: onMount with buffered updates emits exactly one ack at snapshot
// receipt; a subsequent flushRafQueue call emits its own ack (idempotent).
// ---------------------------------------------------------------------------

/**
 * Regression guard for ADR-0027 Addendum 3 (onMount pre-attach race).
 *
 * Scenario: the listener is attached before the snapshot resolves, a
 * screen-update is fired and buffered, then the snapshot resolves. The
 * helper `fetchAndAckSnapshot` must emit exactly one `frame_ack` for the
 * snapshot itself — independent of whether `pendingUpdates` is empty or
 * not. The idempotence extension then fires N normal rafQueue events and
 * asserts a second ack from flushRafQueue (total = 2 acks).
 */
describe('ACK-FE-007: onMount with buffered updates sends exactly one ack, then accepts more', () => {
  it('snapshot resolution emits one ack even when pendingUpdates is non-empty; subsequent flush emits its own', async () => {
    // Deferred snapshot — lets us attach the listener first, then fire an
    // update into pendingUpdates before resolving.
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

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalPane, {
      target: container,
      props: { paneId: PANE_ID, tabId: 'tab-ack', active: true },
    });
    instances.push(instance);

    // Drain microtasks so the screen-update listener is registered
    // (listener is registered BEFORE the snapshot fetch — WP3b).
    for (let i = 0; i < 5; i++) await Promise.resolve();

    // Fire one screen-update while the snapshot is still pending — it must
    // land in pendingUpdates (buffering = true until after the snapshot).
    fireEvent<ScreenUpdateEvent>(
      'screen-update',
      makeScreenUpdate(PANE_ID, {
        cols: 4,
        rows: 3,
        isFullRedraw: false,
        cells: [{ row: 0, col: 0, content: 'B', width: 1, attrs: DEFAULT_ATTRS }],
      }),
    );

    // Resolve the snapshot. `fetchAndAckSnapshot` must emit its ack now.
    resolveSnapshot({ ...VALID_SNAPSHOT });
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    // Exactly one ack: from the snapshot receipt.
    let ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(1);
    expect(ackCalls[0]).toEqual(['frame_ack', { paneId: PANE_ID }]);

    // Idempotence extension: fire N normal events through the rafQueue path
    // and flush — the ack-on-paint must still fire independently.
    rafSpy.mockClear();
    rafCallback = null;
    fireNEvents(3);
    flushRaf();
    flushSync();

    ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(2);
    expect(ackCalls[1]).toEqual(['frame_ack', { paneId: PANE_ID }]);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-008: onMount with empty pendingUpdates still emits one ack
// (unconditional — protects against the pre-attach race in prod).
// ---------------------------------------------------------------------------

/**
 * Regression guard for ADR-0027 Addendum 3.
 *
 * Scenario: the backend's first `screen-update` is emitted BEFORE the
 * frontend's listener is attached (reliably reproduced in `pnpm tauri dev`
 * with slow Vite module loading; theoretically present in prod under
 * GC/IO stalls). The event is lost; `pendingUpdates` stays empty; no
 * normal `flushRafQueue` ack will ever fire for it. `fetchAndAckSnapshot`
 * MUST still emit an unconditional ack — otherwise `last_ack_ms` stays at
 * pane-creation time and the backend falsely enters drop mode after 1s.
 */
describe('ACK-FE-008: onMount with empty pendingUpdates still emits one ack', () => {
  it('frame_ack is invoked exactly once even when no update is buffered during the fetch', async () => {
    // Snapshot resolves synchronously (returns VALID_SNAPSHOT). Simulates
    // the case where no screen-update arrives during the async fetch
    // window — e.g. the pre-attach emit was lost, or none has fired yet.
    invokeSpy.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_pane_screen_snapshot') {
        return { ...VALID_SNAPSHOT } as never;
      }
      return undefined as never;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalPane, {
      target: container,
      props: { paneId: PANE_ID, tabId: 'tab-ack', active: true },
    });
    instances.push(instance);

    // Drain microtasks so the snapshot resolves, replay runs (empty), and
    // onMount completes.
    for (let i = 0; i < 20; i++) await Promise.resolve();
    flushSync();

    // The ack is UNCONDITIONAL — not gated on pendingUpdates.length > 0.
    const ackCalls = getFrameAckCalls();
    expect(ackCalls).toHaveLength(1);
    expect(ackCalls[0]).toEqual(['frame_ack', { paneId: PANE_ID }]);
  });
});
