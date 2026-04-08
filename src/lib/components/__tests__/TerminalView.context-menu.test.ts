// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — TV-RISK-001 context menu hint tests.
 *
 * Covered:
 *   TV-RISK-001 — handleContextMenuHintDismiss syncs contextMenuHintShown
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
// TV-RISK-001: handleContextMenuHintDismiss syncs preferences.appearance.contextMenuHintShown
// ---------------------------------------------------------------------------

describe('TV-RISK-001: handleContextMenuHintDismiss syncs contextMenuHintShown', () => {
  /**
   * Mounts TerminalView with contextMenuHintShown = false so the hint overlay
   * becomes visible after 2 s. We use fake timers to advance past the 2 s delay,
   * then dispatch contextmenu to trigger handleContextMenuHintDismiss.
   */
  async function mountWithHintVisible(): Promise<{
    container: HTMLElement;
    invokeSpy: ReturnType<typeof vi.spyOn>;
  }> {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });

    // jsdom does not implement scrollIntoView or element.animate — stub them
    // so TabBar's smooth-scroll logic does not throw under fake timers.
    if (!HTMLElement.prototype.scrollIntoView) {
      HTMLElement.prototype.scrollIntoView = () => {};
    }
    if (!HTMLElement.prototype.animate) {
      (HTMLElement.prototype as { animate: unknown }).animate = () => ({
        finished: Promise.resolve(),
        cancel: () => {},
      });
    }

    const prefsWithHintNotShown = {
      ...basePrefs,
      appearance: { ...basePrefs.appearance, contextMenuHintShown: false },
    };

    const invokeSpy = vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [makeTab()], activeTabId: 'tab-1' };
      if (cmd === 'get_preferences') return prefsWithHintNotShown;
      if (cmd === 'get_connections') return [];
      if (cmd === 'mark_context_menu_used') return undefined;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);

    // Drain microtasks for onMount async sequence (cannot use real settle() with fake timers).
    for (let i = 0; i < 50; i++) await Promise.resolve();
    flushSync();

    // Advance past the 2 s hint-reveal timer.
    vi.advanceTimersByTime(2500);
    flushSync();

    return { container, invokeSpy };
  }

  afterEach(() => {
    vi.useRealTimers();
  });

  it('calls invoke("mark_context_menu_used") when contextmenu fires in the pane area after hint is visible', async () => {
    const { container, invokeSpy } = await mountWithHintVisible();

    const paneArea = container.querySelector('.terminal-view__pane-area');
    expect(paneArea).not.toBeNull();
    paneArea!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }));

    // Drain microtasks for the async handleContextMenuHintDismiss.
    for (let i = 0; i < 50; i++) await Promise.resolve();
    flushSync();

    const calls = invokeSpy.mock.calls.filter(
      ([cmd]: [string, ...unknown[]]) => cmd === 'mark_context_menu_used',
    );
    expect(calls.length).toBe(1);
  });

  it('does NOT call invoke("mark_context_menu_used") when hint is not visible (contextMenuHintShown = true)', async () => {
    // Reset to real timers for this test since it doesn't need fake timers.
    vi.useRealTimers();

    // basePrefs already has contextMenuHintShown: true — hint stays hidden.
    vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'get_session_state') return { tabs: [makeTab()], activeTabId: 'tab-1' };
      if (cmd === 'get_preferences') return basePrefs; // contextMenuHintShown: true
      if (cmd === 'get_connections') return [];
      if (cmd === 'mark_context_menu_used') return undefined;
      return undefined;
    });

    const container = document.createElement('div');
    document.body.appendChild(container);
    const instance = mount(TerminalViewWithProvider, { target: container, props: {} });
    instances.push(instance);
    await settle();

    const invokeSpy = vi.spyOn(tauriCore, 'invoke');

    const paneArea = container.querySelector('.terminal-view__pane-area');
    expect(paneArea).not.toBeNull();
    paneArea!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }));
    await settle();

    const calls = invokeSpy.mock.calls.filter(([cmd]) => cmd === 'mark_context_menu_used');
    expect(calls.length).toBe(0);
  });
});
