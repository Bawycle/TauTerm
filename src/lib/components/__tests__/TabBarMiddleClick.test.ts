// SPDX-License-Identifier: MPL-2.0

/**
 * TabBar middle-click close tests.
 *
 * Covered:
 *   UXD-TAB-MIDDLECLICK-001 — middle-click (button=1) fires onTabClose
 *   UXD-TAB-MIDDLECLICK-002 — left-click (button=0) does NOT fire onTabClose
 *   UXD-TAB-MIDDLECLICK-003 — right-click (button=2) does NOT fire onTabClose
 *   UXD-TAB-MIDDLECLICK-004 — middle-click calls preventDefault (suppresses autoscroll)
 *
 * These tests verify the middle-click handler logic using direct event dispatch
 * on a tab element rendered by the TabBar component.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import type { TabState, PaneState } from '$lib/ipc';
import TabBar from '../TabBar.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makePaneState(overrides: Partial<PaneState> = {}): PaneState {
  return {
    paneId: 'pane-1',
    lifecycle: { type: 'running' },
    processTitle: 'bash',
    sshState: null,
    scrollOffset: 0,
    cwd: '/home/user',
    ...overrides,
  };
}

function makeTabState(overrides: Partial<TabState> = {}): TabState {
  const pane = makePaneState();
  return {
    id: 'tab-1',
    label: 'My Tab',
    activePaneId: 'pane-1',
    order: 0,
    layout: { type: 'leaf', paneId: 'pane-1', state: pane },
    ...overrides,
  };
}

type TabBarInstance = ReturnType<typeof mount>;

function mountTabBar(props: {
  tabs: TabState[];
  activeTabId: string;
  onTabClick?: (tabId: string) => void;
  onTabClose?: (tabId: string) => void;
  onNewTab?: () => void;
}): { container: HTMLElement; instance: TabBarInstance } {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(TabBar, {
    target: container,
    props: {
      tabs: props.tabs,
      activeTabId: props.activeTabId,
      onTabClick: props.onTabClick ?? vi.fn(),
      onTabClose: props.onTabClose ?? vi.fn(),
      onNewTab: props.onNewTab ?? vi.fn(),
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
// Middle-click close tests
// ---------------------------------------------------------------------------

describe('UXD-TAB-MIDDLECLICK-001: middle-click fires onTabClose', () => {
  it('mousedown with button=1 calls onTabClose with the correct tabId', () => {
    const tab = makeTabState({ id: 'tab-abc' });
    const onTabClose = vi.fn();
    const { container, instance } = mountTabBar({
      tabs: [tab],
      activeTabId: 'tab-abc',
      onTabClose,
    });
    instances.push(instance);

    const tabEl = container.querySelector('[data-tab-id="tab-abc"]');
    expect(tabEl).not.toBeNull();

    // Dispatch mousedown with button=1 (middle button)
    const event = new MouseEvent('mousedown', {
      bubbles: true,
      cancelable: true,
      button: 1,
    });
    tabEl!.dispatchEvent(event);

    expect(onTabClose).toHaveBeenCalledTimes(1);
    expect(onTabClose).toHaveBeenCalledWith('tab-abc');
  });
});

describe('UXD-TAB-MIDDLECLICK-002: left-click does NOT fire onTabClose', () => {
  it('mousedown with button=0 does not call onTabClose', () => {
    const tab = makeTabState({ id: 'tab-abc' });
    const onTabClose = vi.fn();
    const { container, instance } = mountTabBar({
      tabs: [tab],
      activeTabId: 'tab-abc',
      onTabClose,
    });
    instances.push(instance);

    const tabEl = container.querySelector('[data-tab-id="tab-abc"]');
    expect(tabEl).not.toBeNull();

    const event = new MouseEvent('mousedown', {
      bubbles: true,
      cancelable: true,
      button: 0,
    });
    tabEl!.dispatchEvent(event);

    expect(onTabClose).not.toHaveBeenCalled();
  });
});

describe('UXD-TAB-MIDDLECLICK-003: right-click does NOT fire onTabClose', () => {
  it('mousedown with button=2 does not call onTabClose', () => {
    const tab = makeTabState({ id: 'tab-abc' });
    const onTabClose = vi.fn();
    const { container, instance } = mountTabBar({
      tabs: [tab],
      activeTabId: 'tab-abc',
      onTabClose,
    });
    instances.push(instance);

    const tabEl = container.querySelector('[data-tab-id="tab-abc"]');
    expect(tabEl).not.toBeNull();

    const event = new MouseEvent('mousedown', {
      bubbles: true,
      cancelable: true,
      button: 2,
    });
    tabEl!.dispatchEvent(event);

    expect(onTabClose).not.toHaveBeenCalled();
  });
});

describe('UXD-TAB-MIDDLECLICK-004: middle-click prevents default (no autoscroll)', () => {
  it('middle-click mousedown event is preventDefault-ed', () => {
    const tab = makeTabState({ id: 'tab-abc' });
    const { container, instance } = mountTabBar({
      tabs: [tab],
      activeTabId: 'tab-abc',
    });
    instances.push(instance);

    const tabEl = container.querySelector('[data-tab-id="tab-abc"]');
    expect(tabEl).not.toBeNull();

    const event = new MouseEvent('mousedown', {
      bubbles: true,
      cancelable: true,
      button: 1,
    });
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault');
    tabEl!.dispatchEvent(event);

    expect(preventDefaultSpy).toHaveBeenCalledTimes(1);
  });
});
