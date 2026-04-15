// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — SSH-CLOSE-002: auto-close pane when SSH Closed state fires.
 *
 * Covered:
 *   SSH-CLOSE-002 — onSshStateChanged('closed') calls doClosePane(pane_id)
 *
 * The handler lives in setupViewListeners (useTerminalView.lifecycle.svelte.ts).
 * We test it by:
 *   1. Spying on tauriEvent.listen to capture the 'ssh-state-changed' handler.
 *   2. Spying on tauriCore.invoke to intercept 'close_pane'.
 *   3. Mounting TerminalView so setupViewListeners is called.
 *   4. Manually invoking the captured handler with { state: { type: 'closed' } }.
 *   5. Asserting invoke('close_pane') was called with the expected pane ID.
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import { resetMockWindow } from '../../../__mocks__/tauri-window';
import TerminalViewWithProvider from './TerminalViewWithProvider.svelte';
import { makeTab, basePrefs } from './fixtures';
import type { SshStateChangedEvent } from '$lib/ipc';

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
// Helpers
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
  // Default listen mock — no-op, overridden per test.
  vi.spyOn(tauriEvent, 'listen').mockResolvedValue(() => {});
});

afterEach(() => {
  instances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* ignore */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  resetMockWindow();
});

// ---------------------------------------------------------------------------
// SSH-CLOSE-002: doClosePane called when ssh-state-changed fires with 'closed'
// ---------------------------------------------------------------------------

describe('SSH-CLOSE-002: ssh-state-changed closed triggers doClosePane', () => {
  it('calls invoke("close_pane") with the pane ID when SSH state becomes closed', async () => {
    const paneId = 'pane-ssh-1';
    const tab = makeTab({
      id: 'tab-ssh-1',
      activePaneId: paneId,
      layout: {
        type: 'leaf',
        paneId,
        state: {
          paneId,
          lifecycle: { type: 'running' },
          processTitle: 'ssh',
          sshState: { type: 'connected' },
          scrollOffset: 0,
        },
      },
    });

    // Capture the handler registered for 'ssh-state-changed'.
    let capturedSshHandler: ((event: { payload: SshStateChangedEvent }) => void) | null = null;

    vi.spyOn(tauriEvent, 'listen').mockImplementation(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      async (eventName: string, handler: any) => {
        if (eventName === 'ssh-state-changed') {
          capturedSshHandler = handler;
        }
        return () => {};
      },
    );

    let closePanePaneId: string | null = null;
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'get_session_state') return { tabs: [tab], activeTabId: tab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      if (cmd === 'close_pane') {
        closePanePaneId = (args as { paneId: string })?.paneId ?? null;
        // Return null tab state (last pane in tab — tab disappears)
        return null;
      }
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    // The handler must have been registered by setupViewListeners.
    expect(capturedSshHandler).not.toBeNull();

    // Simulate the backend emitting ssh-state-changed with type='closed'.
    const closedEvent: SshStateChangedEvent = { paneId, state: { type: 'closed' } };
    capturedSshHandler!({ payload: closedEvent });
    await settle();

    expect(closePanePaneId).toBe(paneId);
  });

  it('does NOT call invoke("close_pane") when SSH state is connected (not closed)', async () => {
    const paneId = 'pane-ssh-2';
    const tab = makeTab({ id: 'tab-ssh-2', activePaneId: paneId });

    let capturedSshHandler: ((event: { payload: SshStateChangedEvent }) => void) | null = null;

    vi.spyOn(tauriEvent, 'listen').mockImplementation(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      async (eventName: string, handler: any) => {
        if (eventName === 'ssh-state-changed') {
          capturedSshHandler = handler;
        }
        return () => {};
      },
    );

    let closePaneCalled = false;
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [tab], activeTabId: tab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      if (cmd === 'close_pane') {
        closePaneCalled = true;
        return null;
      }
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    expect(capturedSshHandler).not.toBeNull();

    // Fire a 'connected' state — must NOT trigger close_pane.
    const connectedEvent: SshStateChangedEvent = { paneId, state: { type: 'connected' } };
    capturedSshHandler!({ payload: connectedEvent });
    await settle();

    expect(closePaneCalled).toBe(false);
  });

  it('does NOT call invoke("close_pane") when SSH state is disconnected', async () => {
    const paneId = 'pane-ssh-3';
    const tab = makeTab({ id: 'tab-ssh-3', activePaneId: paneId });

    let capturedSshHandler: ((event: { payload: SshStateChangedEvent }) => void) | null = null;

    vi.spyOn(tauriEvent, 'listen').mockImplementation(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      async (eventName: string, handler: any) => {
        if (eventName === 'ssh-state-changed') {
          capturedSshHandler = handler;
        }
        return () => {};
      },
    );

    let closePaneCalled = false;
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [tab], activeTabId: tab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      if (cmd === 'close_pane') {
        closePaneCalled = true;
        return null;
      }
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    expect(capturedSshHandler).not.toBeNull();

    // Fire a 'disconnected' state — must NOT trigger close_pane.
    const disconnectedEvent: SshStateChangedEvent = {
      paneId,
      state: { type: 'disconnected', reason: null },
    };
    capturedSshHandler!({ payload: disconnectedEvent });
    await settle();

    expect(closePaneCalled).toBe(false);
  });
});
