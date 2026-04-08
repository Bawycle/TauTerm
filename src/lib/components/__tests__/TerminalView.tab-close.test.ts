// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — TV-CLOSE-001 tab close confirmation tests.
 *
 * Covered:
 *   TV-CLOSE-001 — tab close confirmation dialog (4 tests)
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import { resetMockWindow } from '../../../__mocks__/tauri-window';
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
// TV-CLOSE-001: Tab close confirmation dialog for active processes
// ---------------------------------------------------------------------------

describe('TV-CLOSE-001: tab close confirmation dialog', () => {
  /**
   * Mounts TerminalView with one pre-existing tab whose pane is NOT in
   * terminatedPanes (i.e. the process is considered active).
   * Returns the mount container and the invoke spy.
   */
  async function mountWithActiveTab(): Promise<{
    container: HTMLElement;
    invokeSpy: ReturnType<typeof vi.spyOn>;
  }> {
    const existingTab = makeTab();

    const invokeSpy = vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [existingTab], activeTabId: existingTab.id };
      if (cmd === 'get_preferences') return basePrefs;
      if (cmd === 'get_connections') return [];
      if (cmd === 'close_tab') return undefined;
      // FS-PTY-008: simulate a non-shell foreground process active in the pane
      if (cmd === 'has_foreground_process') return true;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    return { container, invokeSpy };
  }

  it('shows the close-confirmation dialog when closing a tab with a running process', async () => {
    const { container } = await mountWithActiveTab();

    // Find the close button for tab-1.
    // TabBar renders a .tab-bar__close button for each tab.
    const closeBtn = container.querySelector<HTMLButtonElement>('.tab-bar__close');
    expect(closeBtn).not.toBeNull();

    closeBtn!.click();
    await settle();

    // Bits UI Dialog portals into document.body.
    // The confirmation dialog title is m.close_confirm_title() = "Close terminal?"
    const dialogTitle =
      document.body.querySelector('[data-dialog-title]') ??
      Array.from(document.body.querySelectorAll('*')).find(
        (el) => el.textContent?.trim() === 'Close terminal?',
      );
    expect(dialogTitle).not.toBeNull();
  });

  it('does NOT call invoke("close_tab") immediately when process is active', async () => {
    const { container, invokeSpy } = await mountWithActiveTab();

    const closeBtn = container.querySelector<HTMLButtonElement>('.tab-bar__close');
    closeBtn!.click();
    await settle();

    const closeTabCalls = invokeSpy.mock.calls.filter(
      ([cmd]: [string, ...unknown[]]) => cmd === 'close_tab',
    );
    expect(closeTabCalls.length).toBe(0);
  });

  it('calls invoke("close_tab") after clicking "Close anyway" in the dialog', async () => {
    const { container, invokeSpy } = await mountWithActiveTab();

    // Open the confirmation dialog by clicking the tab close button.
    const closeBtn = container.querySelector<HTMLButtonElement>('.tab-bar__close');
    closeBtn!.click();
    await settle();

    // Locate the destructive "Close anyway" button (m.close_confirm_action()).
    // It is rendered inside the dialog footer in document.body.
    const confirmBtn = Array.from(document.body.querySelectorAll('button')).find(
      (btn) => btn.textContent?.trim() === 'Close anyway',
    );
    expect(confirmBtn).not.toBeNull();

    confirmBtn!.click();
    await settle();

    const closeTabCalls = invokeSpy.mock.calls.filter(
      ([cmd]: [string, ...unknown[]]) => cmd === 'close_tab',
    );
    expect(closeTabCalls.length).toBe(1);
    expect(closeTabCalls[0][1]).toMatchObject({ tabId: 'tab-1' });
  });

  it('does NOT call invoke("close_tab") after clicking Cancel', async () => {
    const { container, invokeSpy } = await mountWithActiveTab();

    const closeBtn = container.querySelector<HTMLButtonElement>('.tab-bar__close');
    closeBtn!.click();
    await settle();

    const cancelBtn = Array.from(document.body.querySelectorAll('button')).find(
      (btn) => btn.textContent?.trim() === 'Cancel',
    );
    expect(cancelBtn).not.toBeNull();

    cancelBtn!.click();
    await settle();

    const closeTabCalls = invokeSpy.mock.calls.filter(
      ([cmd]: [string, ...unknown[]]) => cmd === 'close_tab',
    );
    expect(closeTabCalls.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// E2E-deferred
// ---------------------------------------------------------------------------

describe('TV-CLOSE-002 [E2E-deferred]: pane close confirmation dialog', () => {
  it.todo('Ctrl+Shift+Q on an active pane shows the close-pane confirmation dialog');
});

describe('TV-CLOSE-003 [E2E-deferred]: dialog cancel preserves tab', () => {
  it.todo('clicking Cancel in the close-confirmation dialog leaves the tab open');
});
