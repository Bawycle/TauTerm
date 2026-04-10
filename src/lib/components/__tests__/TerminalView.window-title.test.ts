// SPDX-License-Identifier: MPL-2.0

/**
 * TerminalView — OS window title (FS-TAB-010)
 *
 * Covered:
 *   FS-TAB-010 — window title follows "{tab-title} — TauTerm"
 *   FS-TAB-010 — fallback to "Terminal — TauTerm" when no title available
 *   FS-TAB-010 — user label wins over processTitle
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import { mockAppWindow, resetMockWindow } from '../../../__mocks__/tauri-window';
import TerminalViewWithProvider from './TerminalViewWithProvider.svelte';
import { makeTab, makePaneState } from './fixtures';

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

function mockInvoke(
  tabs: Parameters<typeof makeTab>[0][],
  activeTabId: string,
  extraPrefs: Record<string, unknown> = {},
) {
  vi.spyOn(tauriCore, 'invoke').mockImplementation(async (cmd: string) => {
    if (cmd === 'get_session_state') return { tabs: tabs.map((o) => makeTab(o)), activeTabId };
    if (cmd === 'get_preferences')
      return {
        appearance: {
          fontFamily: 'monospace',
          fontSize: 13,
          cursorStyle: 'block',
          cursorBlinkMs: 530,
          themeName: 'umbra',
          opacity: 1.0,
          language: 'en',
          contextMenuHintShown: true,
          hideCursorWhileTyping: false,
          showPaneTitleBar: true,
          ...extraPrefs,
        },
        terminal: {
          scrollbackLines: 1000,
          allowOsc52Write: false,
          wordDelimiters: ' ,;:',
          bellType: 'none',
          confirmMultilinePaste: false,
        },
        keyboard: { bindings: {} },
        connections: [],
        themes: [],
      };
    if (cmd === 'get_connections') return [];
    return undefined;
  });
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
      /* ignore */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  resetMockWindow();
});

// ---------------------------------------------------------------------------
// FS-TAB-010: window title rules
// ---------------------------------------------------------------------------

describe('FS-TAB-010: OS window title follows "{tab-title} — TauTerm"', () => {
  it('sets window title from active tab processTitle', async () => {
    mockInvoke(
      [
        {
          layout: { type: 'leaf', paneId: 'pane-1', state: makePaneState({ processTitle: 'vim' }) },
        },
      ],
      'tab-1',
    );

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container }));
    await settle();

    expect(mockAppWindow.title).toBe('vim \u2014 TauTerm');
  });

  it('falls back to "Terminal — TauTerm" when processTitle is empty', async () => {
    mockInvoke(
      [{ layout: { type: 'leaf', paneId: 'pane-1', state: makePaneState({ processTitle: '' }) } }],
      'tab-1',
    );

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container }));
    await settle();

    expect(mockAppWindow.title).toBe('Terminal \u2014 TauTerm');
  });

  it('user label wins over processTitle', async () => {
    mockInvoke(
      [
        {
          label: 'my-server',
          layout: {
            type: 'leaf',
            paneId: 'pane-1',
            state: makePaneState({ processTitle: 'bash' }),
          },
        },
      ],
      'tab-1',
    );

    const container = document.createElement('div');
    document.body.appendChild(container);
    instances.push(mount(TerminalViewWithProvider, { target: container }));
    await settle();

    expect(mockAppWindow.title).toBe('my-server \u2014 TauTerm');
  });
});
