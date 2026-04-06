// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — integration tests for IPC-driven behaviours.
 *
 * These tests cover two regressions that were previously caught only by E2E:
 *
 *   TV-INIT-001 — Backend invariant: TerminalView NEVER calls create_tab directly.
 *     The backend (lib.rs setup()) guarantees ≥1 tab before the window is shown,
 *     so TerminalView must not call create_tab on mount under any condition —
 *     including when get_session_state returns zero tabs (abnormal state).
 *   TV-CLOSE-001 — Tab close confirmation: handleTabClose shows the
 *     "Close terminal?" dialog when the target tab has a running process, and
 *     invoke('close_tab') is called only after the user confirms.
 *
 * Setup notes:
 *   - TerminalView uses TabBar → Tooltip.Root, which requires a Tooltip.Provider
 *     ancestor. The TerminalViewWithProvider wrapper supplies that context.
 *   - All IPC is mocked via vi.spyOn on the module-level stubs (tauri-core /
 *     tauri-event aliases configured in vitest.config.ts).
 *   - Bits UI Dialog renders via a portal into document.body — DOM queries use
 *     document.body, not the mount container.
 *   - vi.spyOn intercepts must be set up BEFORE mount() because onMount captures
 *     the invoke / listen references at component initialisation time.
 *   - ResizeObserver is stubbed for jsdom compatibility (used by TerminalPane).
 *
 * E2E-deferred (require real Tauri backend / full render pipeline):
 *   TV-INIT-002 — Tab populated by backend session-state-changed event
 *   TV-CLOSE-002 — Pane close confirmation dialog
 *   TV-CLOSE-003 — Dialog cancel does not call close_tab
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import TerminalViewWithProvider from './TerminalViewWithProvider.svelte';
import type { TabState, PaneState } from '$lib/ipc/types';

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
// Fixtures
// ---------------------------------------------------------------------------

function makePaneState(overrides: Partial<PaneState> = {}): PaneState {
  return {
    id: 'pane-1',
    sessionType: 'local',
    processTitle: 'bash',
    cwd: '/home/user',
    sshConnectionId: null,
    sshState: null,
    notification: null,
    ...overrides,
  };
}

function makeTab(overrides: Partial<TabState> = {}): TabState {
  const pane = makePaneState();
  return {
    id: 'tab-1',
    label: null,
    activePaneId: 'pane-1',
    order: 0,
    layout: { type: 'leaf', paneId: 'pane-1', state: pane },
    ...overrides,
  };
}

const basePrefs = {
  appearance: {
    fontFamily: 'monospace',
    fontSize: 13,
    cursorStyle: 'block',
    cursorBlinkMs: 530,
    themeName: 'umbra',
    opacity: 1.0,
    language: 'en',
    contextMenuHintShown: true, // suppress hint overlay in tests
  },
  terminal: {
    scrollbackLines: 10000,
    allowOsc52Write: false,
    wordDelimiters: ' ,;:.{}[]()"`|\\/',
    bellType: 'visual',
    confirmMultilinePaste: true,
  },
  keyboard: { bindings: {} },
};

// ---------------------------------------------------------------------------
// Mount helper
// ---------------------------------------------------------------------------

/**
 * Drains the microtask queue sufficiently for onMount async sequences to
 * complete (invoke + listen chains), then synchronises Svelte's reactive
 * scheduler.
 */
async function settle(): Promise<void> {
  for (let i = 0; i < 50; i++) await Promise.resolve();
  flushSync();
}

// ---------------------------------------------------------------------------
// Test lifecycle
// ---------------------------------------------------------------------------

const instances: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  // Stub listen() globally so all event subscriptions in onMount resolve
  // instantly without hanging.
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

describe('TV-INIT-002 [E2E-deferred]: tab created by backend event after failed create_tab', () => {
  it.todo('session-state-changed tab-created event populates tabs when onMount create_tab fails');
});

describe('TV-CLOSE-002 [E2E-deferred]: pane close confirmation dialog', () => {
  it.todo('Ctrl+Shift+Q on an active pane shows the close-pane confirmation dialog');
});

describe('TV-CLOSE-003 [E2E-deferred]: dialog cancel preserves tab', () => {
  it.todo('clicking Cancel in the close-confirmation dialog leaves the tab open');
});
