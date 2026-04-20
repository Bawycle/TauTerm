// SPDX-License-Identifier: MPL-2.0

/**
 * TabBarItem split indicator (LayoutPanelLeft icon) tests.
 *
 * Covered:
 *   SPLIT-IND-001 — renders the split indicator wrapper when isMultiPane=true
 *   SPLIT-IND-002 — does not render the split indicator wrapper when isMultiPane=false
 *
 * TabBar derives isMultiPane from tab.layout.type === 'split' and passes it to
 * TabBarItem. Tests use TabBar as the mount point (same pattern as
 * TabBarMiddleClick.test.ts) so the derivation logic is covered end-to-end.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import type { TabState, PaneState } from '$lib/ipc';
import TabBar from '../TabBar.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makePaneState(id: string): PaneState {
  return {
    paneId: id,
    lifecycle: { type: 'running' },
    processTitle: 'bash',
    sshState: null,
    scrollOffset: 0,
    cwd: '/home/user',
  };
}

/** Single-pane tab — layout is a leaf, so isMultiPane=false. */
function makeSinglePaneTab(id: string): TabState {
  const pane = makePaneState('pane-1');
  return {
    id,
    label: 'Single',
    activePaneId: 'pane-1',
    order: 0,
    layout: { type: 'leaf', paneId: 'pane-1', state: pane },
  };
}

/** Multi-pane tab — layout is a split, so isMultiPane=true. */
function makeMultiPaneTab(id: string): TabState {
  const paneA = makePaneState('pane-a');
  const paneB = makePaneState('pane-b');
  return {
    id,
    label: 'Split',
    activePaneId: 'pane-a',
    order: 0,
    layout: {
      type: 'split',
      direction: 'horizontal',
      ratio: 0.5,
      first: { type: 'leaf', paneId: 'pane-a', state: paneA },
      second: { type: 'leaf', paneId: 'pane-b', state: paneB },
    },
  };
}

type TabBarInstance = ReturnType<typeof mount>;

function mountTabBar(
  tabs: TabState[],
  activeTabId: string,
): {
  container: HTMLElement;
  instance: TabBarInstance;
} {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(TabBar, {
    target: container,
    props: {
      tabs,
      activeTabId,
      onTabClick: vi.fn(),
      onTabClose: vi.fn(),
      onNewTab: vi.fn(),
    },
  });
  return { container, instance };
}

const instances: TabBarInstance[] = [];

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
});

// ---------------------------------------------------------------------------
// SPLIT-IND-001: split indicator rendered when isMultiPane=true
// ---------------------------------------------------------------------------

describe('SPLIT-IND-001: split indicator rendered when tab has ≥2 panes', () => {
  it('renders .tab-bar__split-indicator for a tab with a split layout', () => {
    const tab = makeMultiPaneTab('tab-split');
    const { container, instance } = mountTabBar([tab], 'tab-split');
    instances.push(instance);
    const indicator = container.querySelector('.tab-bar__split-indicator');
    expect(indicator).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// SPLIT-IND-002: split indicator absent when isMultiPane=false
// ---------------------------------------------------------------------------

describe('SPLIT-IND-002: split indicator absent when tab has a single pane', () => {
  it('does not render .tab-bar__split-indicator for a single-pane tab', () => {
    const tab = makeSinglePaneTab('tab-single');
    const { container, instance } = mountTabBar([tab], 'tab-single');
    instances.push(instance);
    const indicator = container.querySelector('.tab-bar__split-indicator');
    expect(indicator).toBeNull();
  });
});
