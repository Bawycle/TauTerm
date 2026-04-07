// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for notification-changed IPC event handling in useTerminalPane.svelte.ts.
 *
 * Covered:
 *   TPSC-NOTIF-001 — notification-changed backgroundOutput: fires without error
 *   TPSC-NOTIF-002 — notification-changed processExited: fires without error
 *   TPSC-NOTIF-003 — notification-changed for different paneId: filtered
 *   TPSC-NOTIF-004 — notification-changed null: clears pulse without error
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import type { NotificationChangedEvent } from '$lib/ipc/types';
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
const OTHER_PANE_ID = 'pane-other';
const TAB_ID = 'tab-1';

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
    props: { paneId, tabId: TAB_ID, active },
  });
  instances.push(instance);

  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();

  return { container, instance };
}

// ---------------------------------------------------------------------------
// TPSC-NOTIF-001: notification-changed backgroundOutput
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

// ---------------------------------------------------------------------------
// TPSC-NOTIF-002: notification-changed processExited
// ---------------------------------------------------------------------------

describe('TPSC-NOTIF-002: notification-changed processExited fires without error', () => {
  it('handles processExited notification for an inactive pane', async () => {
    const { container } = await mountPane(PANE_ID, false);

    expect(() => {
      fireEvent<NotificationChangedEvent>('notification-changed', {
        tabId: TAB_ID,
        paneId: PANE_ID,
        notification: { type: 'processExited', exitCode: 0, signalName: null },
      });
      flushSync();
    }).not.toThrow();

    expect(container.querySelector('.terminal-pane')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TPSC-NOTIF-003: notification-changed for different paneId is ignored
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// TPSC-NOTIF-004: notification-changed null clears pulse without error
// ---------------------------------------------------------------------------

describe('TPSC-NOTIF-004: notification-changed null clears pulse without error', () => {
  it('handles the clear signal (notification=null) after a pulse was set', async () => {
    const { container } = await mountPane(PANE_ID, false);

    // Trigger a pulse first.
    fireEvent<NotificationChangedEvent>('notification-changed', {
      tabId: TAB_ID,
      paneId: PANE_ID,
      notification: { type: 'backgroundOutput' },
    });
    flushSync();

    // Clear it.
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
