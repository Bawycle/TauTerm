// SPDX-License-Identifier: MPL-2.0

/**
 * TabBar — Tooltip.Provider context tests.
 *
 * Historical context: TabBar previously included a new-tab "+" button wrapped
 * in Tooltip.Root, which required a Tooltip.Provider ancestor.  The button was
 * moved to TerminalView (outside the scrollable tab zone — UXD §7.1.1).
 * TabBar no longer uses Tooltip.Root and does not require a provider.
 *
 * Covered:
 *   TBTC-CTX-001 — TabBar mounts without error (with or without Tooltip.Provider)
 *   TBTC-CTX-002 — TabBar mounts without throwing even without Tooltip.Provider
 *
 * Note: Bits UI Dialog/Tooltip portals are rendered into document.body;
 * DOM queries use document.body when needed.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import { Tooltip } from 'bits-ui';
import TabBar from '../TabBar.svelte';
import TabBarWithProvider from './TabBarWithProvider.svelte';
import type { TabState, PaneState } from '$lib/ipc/types';

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

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

const instances: ReturnType<typeof mount>[] = [];

afterEach(() => {
  instances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* ignore — component may have already unmounted */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// TBTC-CTX-001 — TabBar mounts without error inside Tooltip.Provider
// ---------------------------------------------------------------------------

describe('TBTC-CTX-001: TabBar mounts cleanly inside Tooltip.Provider', () => {
  it('mounts without throwing when wrapped in Tooltip.Provider', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    expect(() => {
      const instance = mount(TabBarWithProvider, {
        target: container,
        props: {
          tabs: [makeTab()],
          activeTabId: 'tab-1',
          onTabClick: () => {},
          onTabClose: () => {},
          onNewTab: () => {},
        },
      });
      instances.push(instance);
      flushSync();
    }).not.toThrow();
  });

  it('renders the tab bar container element (role=tablist)', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [makeTab()],
        activeTabId: 'tab-1',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    instances.push(instance);
    flushSync();

    const tablist = container.querySelector('[role="tablist"]');
    expect(tablist).not.toBeNull();
  });

  it.todo(
    'new-tab button moved to TerminalView (outside scrollable zone) — no longer rendered by TabBar',
  );
});

// ---------------------------------------------------------------------------
// TBTC-CTX-002 — TabBar no longer requires Tooltip.Provider
//
// The new-tab button (which used Tooltip.Root) was moved to TerminalView.
// TabBar now mounts cleanly without a Tooltip.Provider ancestor.
// ---------------------------------------------------------------------------

describe('TBTC-CTX-002: TabBar mounts without Tooltip.Provider (no Tooltip.Root)', () => {
  it('mounts without throwing when there is no Tooltip.Provider ancestor', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    expect(() => {
      const instance = mount(TabBar, {
        target: container,
        props: {
          tabs: [makeTab()],
          activeTabId: 'tab-1',
          onTabClick: () => {},
          onTabClose: () => {},
          onNewTab: () => {},
        },
      });
      instances.push(instance);
      flushSync();
    }).not.toThrow();
  });
});
