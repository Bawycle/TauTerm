// SPDX-License-Identifier: MPL-2.0

/**
 * Tests for src/lib/state/session.svelte.ts
 *
 * Covers:
 *   - setInitialSession
 *   - applySessionDelta (all change types)
 *   - removeTab / addTab / updateTab
 *   - collectLeafPanes
 *   - findNeighbourPaneId
 *   - getActivePanes / getActiveTab
 */

import { describe, it, expect, beforeEach } from 'vitest';
import type { TabState, PaneState, PaneNode } from '$lib/ipc/types';

// We need to import the module freshly in each test group to reset $state.
// Svelte 5 module-level $state is shared across tests in the same worker;
// we work around this by re-reading state from the module after each mutation.

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

function makeLeafNode(paneId = 'pane-1', state?: PaneState): PaneNode {
  return { type: 'leaf', paneId, state: state ?? makePaneState({ id: paneId }) };
}

function makeTab(overrides: Partial<TabState> = {}): TabState {
  return {
    id: 'tab-1',
    label: null,
    activePaneId: 'pane-1',
    order: 0,
    layout: makeLeafNode('pane-1'),
    ...overrides,
  };
}

describe('session.svelte.ts — collectLeafPanes', () => {
  it('returns a single leaf for a leaf node', async () => {
    const { collectLeafPanes } = await import('./session.svelte');
    const result = collectLeafPanes(makeLeafNode('p1'));
    expect(result).toHaveLength(1);
    expect(result[0].paneId).toBe('p1');
  });

  it('returns two leaves for a split node', async () => {
    const { collectLeafPanes } = await import('./session.svelte');
    const split: PaneNode = {
      type: 'split',
      direction: 'horizontal',
      ratio: 0.5,
      first: makeLeafNode('p1'),
      second: makeLeafNode('p2'),
    };
    const result = collectLeafPanes(split);
    expect(result).toHaveLength(2);
    expect(result.map((p) => p.paneId)).toEqual(['p1', 'p2']);
  });

  it('collects deeply nested leaves in order', async () => {
    const { collectLeafPanes } = await import('./session.svelte');
    const tree: PaneNode = {
      type: 'split',
      direction: 'vertical',
      ratio: 0.5,
      first: {
        type: 'split',
        direction: 'horizontal',
        ratio: 0.5,
        first: makeLeafNode('p1'),
        second: makeLeafNode('p2'),
      },
      second: makeLeafNode('p3'),
    };
    const result = collectLeafPanes(tree);
    expect(result.map((p) => p.paneId)).toEqual(['p1', 'p2', 'p3']);
  });
});

describe('session.svelte.ts — setInitialSession', () => {
  it('populates tabs and activeTabId from snapshot', async () => {
    const { setInitialSession, sessionState } = await import('./session.svelte');
    const tab = makeTab({ id: 'tab-42' });
    setInitialSession({ tabs: [tab], activeTabId: 'tab-42' });
    expect(sessionState.activeTabId).toBe('tab-42');
    expect(sessionState.tabs).toHaveLength(1);
    expect(sessionState.tabs[0].id).toBe('tab-42');
  });
});

describe('session.svelte.ts — applySessionDelta', () => {
  beforeEach(async () => {
    const { setInitialSession } = await import('./session.svelte');
    setInitialSession({ tabs: [], activeTabId: '' });
  });

  it('tab-created: adds a new tab', async () => {
    const { applySessionDelta, sessionState } = await import('./session.svelte');
    const tab = makeTab({ id: 'new-tab' });
    applySessionDelta({ changeType: 'tab-created', tab });
    expect(sessionState.tabs.some((t) => t.id === 'new-tab')).toBe(true);
  });

  it('tab-closed: removes the closed tab and updates activeTabId', async () => {
    const { setInitialSession, applySessionDelta, sessionState } = await import('./session.svelte');
    const tab1 = makeTab({ id: 'tab-1' });
    const tab2 = makeTab({ id: 'tab-2', order: 1 });
    setInitialSession({ tabs: [tab1, tab2], activeTabId: 'tab-1' });

    applySessionDelta({
      changeType: 'tab-closed',
      closedTabId: 'tab-1',
      activeTabId: 'tab-2',
    });

    expect(sessionState.tabs.some((t) => t.id === 'tab-1')).toBe(false);
    expect(sessionState.activeTabId).toBe('tab-2');
  });

  it('active-tab-changed: updates activeTabId', async () => {
    const { setInitialSession, applySessionDelta, sessionState } = await import('./session.svelte');
    const tab1 = makeTab({ id: 'tab-1' });
    const tab2 = makeTab({ id: 'tab-2', order: 1 });
    setInitialSession({ tabs: [tab1, tab2], activeTabId: 'tab-1' });

    applySessionDelta({ changeType: 'active-tab-changed', activeTabId: 'tab-2' });

    expect(sessionState.activeTabId).toBe('tab-2');
  });

  it('pane-metadata-changed: updates the affected tab in-place', async () => {
    const { setInitialSession, applySessionDelta, sessionState } = await import('./session.svelte');
    const tab = makeTab({ id: 'tab-1', label: null });
    setInitialSession({ tabs: [tab], activeTabId: 'tab-1' });

    const updatedTab = { ...tab, label: 'My Shell' };
    applySessionDelta({ changeType: 'pane-metadata-changed', tab: updatedTab });

    expect(sessionState.tabs[0].label).toBe('My Shell');
  });
});

describe('session.svelte.ts — addTab / removeTab / updateTab', () => {
  it('addTab appends the tab and sets it active', async () => {
    const { setInitialSession, addTab, sessionState } = await import('./session.svelte');
    setInitialSession({ tabs: [], activeTabId: '' });

    addTab(makeTab({ id: 'fresh' }));

    expect(sessionState.tabs.some((t) => t.id === 'fresh')).toBe(true);
    expect(sessionState.activeTabId).toBe('fresh');
  });

  it('removeTab removes the tab and updates activeTabId', async () => {
    const { setInitialSession, removeTab, sessionState } = await import('./session.svelte');
    const tab1 = makeTab({ id: 't1' });
    const tab2 = makeTab({ id: 't2', order: 1 });
    setInitialSession({ tabs: [tab1, tab2], activeTabId: 't1' });

    removeTab('t1');

    expect(sessionState.tabs.some((t) => t.id === 't1')).toBe(false);
    expect(sessionState.activeTabId).toBe('t2');
  });

  it('updateTab replaces the matching tab', async () => {
    const { setInitialSession, updateTab, sessionState } = await import('./session.svelte');
    const tab = makeTab({ id: 'tab-x', label: null });
    setInitialSession({ tabs: [tab], activeTabId: 'tab-x' });

    updateTab({ ...tab, label: 'renamed' });

    expect(sessionState.tabs[0].label).toBe('renamed');
  });
});

describe('session.svelte.ts — findNeighbourPaneId', () => {
  it('returns null when only one pane exists', async () => {
    const { setInitialSession, findNeighbourPaneId } = await import('./session.svelte');
    const tab = makeTab({ id: 'tab-1', activePaneId: 'pane-1' });
    setInitialSession({ tabs: [tab], activeTabId: 'tab-1' });

    expect(findNeighbourPaneId('right')).toBeNull();
    expect(findNeighbourPaneId('left')).toBeNull();
  });

  it('returns the next pane for right direction', async () => {
    const { setInitialSession, findNeighbourPaneId } = await import('./session.svelte');
    const split: PaneNode = {
      type: 'split',
      direction: 'horizontal',
      ratio: 0.5,
      first: makeLeafNode('pane-A'),
      second: makeLeafNode('pane-B'),
    };
    const tab: TabState = {
      id: 'tab-1',
      label: null,
      activePaneId: 'pane-A',
      order: 0,
      layout: split,
    };
    setInitialSession({ tabs: [tab], activeTabId: 'tab-1' });

    expect(findNeighbourPaneId('right')).toBe('pane-B');
    expect(findNeighbourPaneId('left')).toBeNull();
  });

  it('returns the previous pane for left direction', async () => {
    const { setInitialSession, findNeighbourPaneId } = await import('./session.svelte');
    const split: PaneNode = {
      type: 'split',
      direction: 'horizontal',
      ratio: 0.5,
      first: makeLeafNode('pane-A'),
      second: makeLeafNode('pane-B'),
    };
    const tab: TabState = {
      id: 'tab-1',
      label: null,
      activePaneId: 'pane-B',
      order: 0,
      layout: split,
    };
    setInitialSession({ tabs: [tab], activeTabId: 'tab-1' });

    expect(findNeighbourPaneId('left')).toBe('pane-A');
    expect(findNeighbourPaneId('right')).toBeNull();
  });
});
