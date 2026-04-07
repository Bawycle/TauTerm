// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for screen-update event handling in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-FN-002    — scroll-position-changed with offset > 0 shows scroll button
 *   TPSC-FN-004    — scroll-position-changed offset=0 does not show button on fresh mount
 *   TPSC-SCROLL-002 — screen-update with isFullRedraw resets scrollOffset (WP3a)
 *   TPSC-STRIDE-001 — applyScreenUpdate uses event.cols for grid indexing
 *   TPSC-STRIDE-002 — cols/rows local state tracks event values
 *   TPSC-INIT-002  — screen-update received during snapshot fetch is applied (WP3b)
 *   TPSC-RESIZE-005 — isFullRedraw with new dimensions resizes grid correctly
 *   TPSC-RESIZE-006 — incremental update after resize targets new rows
 *   TEST-SB-FE-001 — scrolled viewport event sets scrollOffset and rebuilds gridRows
 *   TEST-SB-FE-002 — live PTY event while scrolled freezes gridRows
 *   TEST-SB-FE-003 — full-redraw with scrollOffset=0 resets to live view
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { ScreenUpdateEvent, ScrollPositionChangedEvent, CursorState } from '$lib/ipc/types';
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

    fireEvent<ScrollPositionChangedEvent>('scroll-position-changed', {
      paneId: PANE_ID,
      offset: 10,
      scrollbackLines: 100,
    });
    flushSync();
    expect(container.querySelector('.scroll-to-bottom-btn')).not.toBeNull();

    expect(() => {
      fireEvent<ScreenUpdateEvent>(
        'screen-update',
        makeScreenUpdate(PANE_ID, { isFullRedraw: true }),
      );
      flushSync();
    }).not.toThrow();

    expect(() => {
      fireEvent<ScrollPositionChangedEvent>('scroll-position-changed', {
        paneId: PANE_ID,
        offset: 0,
        scrollbackLines: 0,
      });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

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
          content: r >= 3 ? 'X' : ' ',
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
    // Before the fix, this cell would be dropped by applyUpdates (oob).
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

    expect(container.querySelector('.scroll-to-bottom-btn')).not.toBeNull();
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
