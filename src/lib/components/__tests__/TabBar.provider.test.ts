// SPDX-License-Identifier: MPL-2.0

/**
 * TabBar — Tooltip.Provider context tests.
 *
 * These tests document and guard against the regression where TabBar was
 * rendered without an ancestor <Tooltip.Provider>, causing Bits UI v2 to
 * throw "Context 'Tooltip.Provider' not found".
 *
 * The fix (wrapping the app root in <Tooltip.Provider> in +page.svelte)
 * is validated here at the unit level by:
 *   1. Verifying that TabBar mounts cleanly when a Tooltip.Provider ancestor
 *      is present (the correct production setup).
 *   2. Documenting the failure mode (no provider) as an expected error so
 *      the contract is explicit in the test suite.
 *
 * Covered:
 *   TBTC-CTX-001 — TabBar mounts without error inside Tooltip.Provider
 *   TBTC-CTX-002 — TabBar throws context error when mounted without Tooltip.Provider
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

  it('renders the new-tab button (uses Tooltip.Root — requires provider)', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [],
        activeTabId: '',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    instances.push(instance);
    flushSync();

    // The new-tab button is rendered by the Tooltip.Trigger snippet inside TabBar.
    // Its presence confirms the Tooltip.Root tree initialised without a context error.
    const newTabBtn = container.querySelector('.tab-bar__new-tab');
    expect(newTabBtn).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TBTC-CTX-002 — TabBar throws context error without Tooltip.Provider
//
// This test documents the failure mode that motivated the fix.
// Bits UI v2 requires Tooltip.Provider as an ancestor of Tooltip.Root.
// Mounting TabBar directly (without the wrapper) triggers the error.
// ---------------------------------------------------------------------------

describe('TBTC-CTX-002: TabBar throws context error without Tooltip.Provider', () => {
  it('throws "Context \\"Tooltip.Provider\\" not found" when mounted without a provider', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    // Suppress the expected console.error from Svelte's error boundary so the
    // test output stays clean.
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

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
      // flushSync materialises the Tooltip.Root context lookup.
      flushSync();
    }).toThrow(/Tooltip\.Provider/);

    consoleError.mockRestore();
  });
});
