// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — TV-INIT-001 backend invariant tests.
 *
 * Covered:
 *   TV-INIT-001 — TerminalView never calls create_tab directly
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import { mockAppWindow, resetMockWindow } from '../../../__mocks__/tauri-window';
import TerminalViewWithProvider from './TerminalViewWithProvider.svelte';
import { makeTab, basePrefs } from './fixtures';

// ---------------------------------------------------------------------------
// jsdom polyfills
// ---------------------------------------------------------------------------

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
if (typeof (globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver === 'undefined') {
  (globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver = ResizeObserverStub;
}

// ---------------------------------------------------------------------------
// Mount helper
// ---------------------------------------------------------------------------

async function settle(): Promise<void> {
  for (let i = 0; i < 50; i++) await Promise.resolve();
  flushSync();
}

// ---------------------------------------------------------------------------
// Test lifecycle
// ---------------------------------------------------------------------------

const instances: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  vi.spyOn(tauriEvent, 'listen').mockResolvedValue(() => {});
});

afterEach(() => {
  instances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* component may have thrown on mount — ignore */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  resetMockWindow();
});

// ---------------------------------------------------------------------------
// TV-INIT-001: TerminalView never calls create_tab directly (backend invariant)
// ---------------------------------------------------------------------------

describe('TV-INIT-001: TerminalView never calls create_tab directly', () => {
  it('does NOT call invoke("create_tab") even when get_session_state returns zero tabs', async () => {
    // The backend (lib.rs setup()) guarantees ≥1 tab before the window is shown.
    // An empty get_session_state response is abnormal; TerminalView must not
    // compensate by calling create_tab — it is not its responsibility.
    const invokeSpy = vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [], activeTabId: '' };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    const createTabCalls = invokeSpy.mock.calls.filter(([cmd]) => cmd === 'create_tab');
    expect(createTabCalls.length).toBe(0);
  });

  it('does NOT call invoke("create_tab") when get_session_state returns a tab', async () => {
    const invokeSpy = vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [makeTab()], activeTabId: 'tab-1' };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    const createTabCalls = invokeSpy.mock.calls.filter(([cmd]) => cmd === 'create_tab');
    expect(createTabCalls.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// E2E-deferred
// ---------------------------------------------------------------------------

describe('TV-INIT-002 [E2E-deferred]: tab created by backend event after failed create_tab', () => {
  it.todo('session-state-changed tab-created event populates tabs when onMount create_tab fails');
});

// eslint-disable-next-line @typescript-eslint/no-unused-vars
const _mockAppWindow = mockAppWindow; // keep import used
