// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — TV-RISK-003 preferences failure fallback tests.
 *
 * Covered:
 *   TV-RISK-003 — get_preferences failure → fallback to defaults
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import { resetMockWindow } from '../../../__mocks__/tauri-window';
import TerminalViewWithProvider from './TerminalViewWithProvider.svelte';
import { makeTab } from './fixtures';

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
// TV-RISK-003: get_preferences failure → fallback to DEFAULT_PREFERENCES
// ---------------------------------------------------------------------------

describe('TV-RISK-003: get_preferences failure falls back to default preferences', () => {
  it('mounts without throwing when get_preferences rejects', async () => {
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [makeTab()], activeTabId: 'tab-1' };
      if (cmd === 'get_preferences') throw new Error('IPC failure');
      if (cmd === 'get_connections') return [];
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);

    let mountError: unknown = null;
    try {
      const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
      instances.push(instance);
      await settle();
    } catch (err) {
      mountError = err;
    }

    expect(mountError).toBeNull();
  });

  it('renders the pane area (not a blank/broken screen) when get_preferences rejects', async () => {
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [makeTab()], activeTabId: 'tab-1' };
      if (cmd === 'get_preferences') throw new Error('IPC failure');
      if (cmd === 'get_connections') return [];
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    // The pane area must be present — a blank screen would have no terminal-view__pane-area.
    const paneArea = container.querySelector('.terminal-view__pane-area');
    expect(paneArea).not.toBeNull();
  });

  it('does not show an "undefined" or broken state in the tab bar when get_preferences rejects', async () => {
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [makeTab()], activeTabId: 'tab-1' };
      if (cmd === 'get_preferences') throw new Error('IPC failure');
      if (cmd === 'get_connections') return [];
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    // The terminal view element must exist.
    const terminalView = container.querySelector('.terminal-view');
    expect(terminalView).not.toBeNull();

    // No "undefined" text content should be rendered anywhere.
    const allText = container.textContent ?? '';
    expect(allText).not.toContain('undefined');
  });
});
