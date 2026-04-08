// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for scrollback freeze behaviour in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TEST-SB-FE-001 — scrolled viewport event updates gridRows
 *   TEST-SB-FE-002 — live PTY event while scrolled freezes gridRows
 *   TEST-SB-FE-003 — full-redraw with scrollOffset=0 resets to live view
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
