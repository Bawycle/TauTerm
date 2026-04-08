// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — TV-WCLOSE and TV-LTAB window close tests.
 *
 * Covered:
 *   TV-WCLOSE-001 — WM close with idle shell closes window
 *   TV-WCLOSE-002 — WM close with active process shows dialog
 *   TV-WCLOSE-003 — Confirming window-close dialog closes window
 *   TV-WCLOSE-004 — destroy() called exactly once on WM close
 *   TV-LTAB-001 — Closing last tab closes window
 *   TV-LTAB-002 — Exit 0 in last pane closes window
 *
 * These tests exercise the onCloseRequested handler registered in useTerminalView
 * via the tauri-window mock. The mock faithfully mirrors Tauri 2's onCloseRequested
 * contract: simulateCloseRequest() runs handlers, then calls destroy() automatically
 * if no handler called event.preventDefault(). Production code uses destroy() for
 * all programmatic closes (including last-tab and dialog-confirm) — never close().
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import { mockAppWindow, resetMockWindow } from '../../../__mocks__/tauri-window';
import TerminalViewWithProvider from './TerminalViewWithProvider.svelte';
import { makeTab, makePaneState, basePrefs } from './fixtures';

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
// TV-WCLOSE-001
// ---------------------------------------------------------------------------

describe('TV-WCLOSE-001: WM close with idle shell closes window without dialog', () => {
  it('closes the window directly when no pane has a foreground process', async () => {
    const existingTab = makeTab();
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [existingTab], activeTabId: existingTab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      // FS-PTY-008: idle shell — no foreground process
      if (cmd === 'has_foreground_process') return false;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container, props: {} }));
    await settle();

    await mockAppWindow.simulateCloseRequest();
    await settle();

    expect(mockAppWindow.closed).toBe(true);
    // No window-close dialog should be in the DOM
    const dialog = Array.from(document.body.querySelectorAll('*')).find((el) =>
      el.textContent?.includes('Close window?'),
    );
    expect(dialog).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// TV-WCLOSE-002
// ---------------------------------------------------------------------------

describe('TV-WCLOSE-002: WM close with active process shows confirmation dialog', () => {
  it('shows the window-close dialog instead of closing when a process is running', async () => {
    const existingTab = makeTab();
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [existingTab], activeTabId: existingTab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      // FS-PTY-008: non-shell foreground process active
      if (cmd === 'has_foreground_process') return true;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container, props: {} }));
    await settle();

    await mockAppWindow.simulateCloseRequest();
    await settle();

    expect(mockAppWindow.closed).toBe(false);
    const dialog = Array.from(document.body.querySelectorAll('*')).find((el) =>
      el.textContent?.includes('Close window?'),
    );
    expect(dialog).not.toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// TV-WCLOSE-003
// ---------------------------------------------------------------------------

describe('TV-WCLOSE-003: confirming window-close dialog closes the window', () => {
  it('closes the window when the user clicks Confirm in the window-close dialog', async () => {
    const existingTab = makeTab();
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [existingTab], activeTabId: existingTab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      if (cmd === 'has_foreground_process') return true;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container, props: {} }));
    await settle();

    // Trigger WM close → dialog appears
    await mockAppWindow.simulateCloseRequest();
    await settle();

    // Click the confirm button ("Close anyway")
    const confirmBtn = Array.from(document.body.querySelectorAll('button')).find(
      (btn) => btn.textContent?.trim() === 'Close anyway',
    );
    expect(confirmBtn).not.toBeUndefined();
    confirmBtn!.click();
    await settle();

    expect(mockAppWindow.closed).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// TV-WCLOSE-004
// ---------------------------------------------------------------------------

describe('TV-WCLOSE-004: idle shell — destroy() called exactly once, no re-entry', () => {
  it('destroy() is called exactly once via Tauri wrapper; no programmatic close needed', async () => {
    const existingTab = makeTab();
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [existingTab], activeTabId: existingTab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      if (cmd === 'has_foreground_process') return false;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container, props: {} }));
    await settle();

    await mockAppWindow.simulateCloseRequest();
    await settle();

    // The handler did NOT call event.preventDefault() (no active processes),
    // so simulateCloseRequest()'s wrapper called destroy() exactly once.
    // No programmatic destroy() was called by production code directly.
    expect(mockAppWindow.destroyCallCount).toBe(1);
    expect(mockAppWindow.closed).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// TV-LTAB-001
// ---------------------------------------------------------------------------

describe('TV-LTAB-001: closing the last tab closes the window (FS-TAB-008)', () => {
  it('calls window.destroy() when the last tab is closed with an idle shell', async () => {
    const existingTab = makeTab();
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [existingTab], activeTabId: existingTab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      if (cmd === 'has_foreground_process') return false;
      if (cmd === 'close_tab') return undefined;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container, props: {} }));
    await settle();

    const closeBtn = container.querySelector<HTMLButtonElement>('.tab-bar__close');
    expect(closeBtn).not.toBeNull();
    closeBtn!.click();
    await settle();

    expect(mockAppWindow.closed).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// TV-LTAB-002
// ---------------------------------------------------------------------------

describe('TV-LTAB-002: exit 0 in last pane of last tab closes window (FS-PTY-005 + FS-TAB-008)', () => {
  it('calls window.destroy() when notification-changed processExited(0) removes the last pane', async () => {
    const pane = makePaneState({ id: 'pane-x' });
    const tab = makeTab({
      id: 'tab-x',
      activePaneId: 'pane-x',
      layout: { type: 'leaf', paneId: 'pane-x', state: pane },
    });

    // Capture the notification-changed listener so we can fire it manually.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let notificationHandler: ((e: any) => void) | null = null;
    vi.spyOn(tauriEvent, 'listen').mockImplementation(async (event, handler) => {
      if (event === 'notification-changed') notificationHandler = handler;
      return () => {};
    });

    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [tab], activeTabId: tab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      // close_pane returns null → last pane of the tab removed → removeTab called
      if (cmd === 'close_pane') return null;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container, props: {} }));
    await settle();

    expect(notificationHandler).not.toBeNull();

    // Simulate ProcessExited(exitCode: 0) for the pane.
    notificationHandler!({
      event: 'notification-changed',
      id: 1,
      payload: {
        tabId: tab.id,
        paneId: 'pane-x',
        notification: { type: 'processExited', exitCode: 0, signalName: null },
      },
    });
    await settle();

    expect(mockAppWindow.closed).toBe(true);
  });
});
