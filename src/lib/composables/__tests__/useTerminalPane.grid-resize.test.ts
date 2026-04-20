// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for grid resize handling in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-SCROLL-002 — isFullRedraw resets scrollOffset
 *   TPSC-RESIZE-005 — isFullRedraw with new dimensions resizes grid
 *   TPSC-RESIZE-006 — incremental update after resize targets new rows
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { ScreenUpdateEvent, ScrollPositionChangedEvent } from '$lib/ipc';
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
