// SPDX-License-Identifier: MPL-2.0

/**
 * TabBar scroll arrows — visibility logic tests.
 *
 * The root cause of the bistability bug (FS-UX-012): comparing scrollWidth
 * against tabsContainerEl.clientWidth creates a self-reinforcing cycle — the
 * arrows themselves consume 48 px from the flex layout, shrinking clientWidth,
 * which keeps "overflow detected" true even when all tabs would fit without
 * arrows.
 *
 * The fix: compare scrollWidth against (tabBar.clientWidth - newTabBtn.offsetWidth),
 * i.e. the available width in the *parent* bar, unaffected by the arrows.
 *
 * Part 1 — Pure logic tests (no DOM):
 *   TBTC-SCR-001 — no overflow → both arrows hidden
 *   TBTC-SCR-002 — overflow at right edge → right arrow shown, left hidden
 *   TBTC-SCR-003 — overflow, scrolled to end → left arrow shown, right hidden
 *   TBTC-SCR-004 — overflow, scrolled to middle → both arrows shown
 *   TBTC-SCR-005 — bistability scenario: tabs exceed tabsContainer.clientWidth
 *                   but fit in available bar width → no arrows (regression guard)
 *   TBTC-SCR-006 — 2 px sub-pixel tolerance boundary
 *   TBTC-SCR-007 — scrollLeft exactly at boundary thresholds
 *   TBTC-SCR-008 — newTabBtn absent → fallback width 44 used
 *
 * Part 2 — DOM integration tests (component mounted):
 *   TBTC-SCR-010 — arrows absent on initial render with zero-width DOM (jsdom)
 *   TBTC-SCR-011 — right arrow appears when scrollWidth > availableWidth
 *   TBTC-SCR-012 — left arrow appears when scrolled right
 *   TBTC-SCR-013 — both arrows appear in the middle of a long tab list
 *   TBTC-SCR-014 — arrows disappear after tab removal reduces scrollWidth
 *   TBTC-SCR-015 — bistability: no arrows when tabs just exceed tabsContainer width
 *                   but fit in bar-minus-newTabBtn width (regression guard, DOM level)
 */

import { describe, it, expect, afterEach, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import TabBarWithProvider from './TabBarWithProvider.svelte';
import type { TabState, PaneState } from '$lib/ipc/types';

// ---------------------------------------------------------------------------
// Part 1 — Pure logic (mirror of updateScrollState, no DOM)
// ---------------------------------------------------------------------------

/**
 * Mirror of the updateScrollState logic from TabBar.svelte.
 * Inputs are the raw DOM measurements the function reads.
 * Output is the derived { canScrollLeft, canScrollRight } state.
 */
interface ScrollMeasurements {
  /** tabBar.clientWidth (parent .tab-bar element) */
  tabBarClientWidth: number;
  /** newTabBtn.offsetWidth — null when the button is not found (uses fallback 44) */
  newTabBtnOffsetWidth: number | null;
  /** tabsContainerEl.scrollWidth (total content width, unaffected by arrows) */
  tabsScrollWidth: number;
  /** tabsContainerEl.clientWidth (visible width — may be biased by arrows) */
  tabsClientWidth: number;
  /** tabsContainerEl.scrollLeft */
  tabsScrollLeft: number;
}

function computeScrollState(m: ScrollMeasurements): { canScrollLeft: boolean; canScrollRight: boolean } {
  const availableWidth = m.tabBarClientWidth - (m.newTabBtnOffsetWidth ?? 44);
  const hasOverflow = m.tabsScrollWidth > availableWidth + 2;
  return {
    canScrollLeft: hasOverflow && m.tabsScrollLeft > 1,
    canScrollRight: hasOverflow && m.tabsScrollLeft + m.tabsClientWidth < m.tabsScrollWidth - 1,
  };
}

// TBTC-SCR-001
describe('TBTC-SCR-001: no overflow → both arrows hidden', () => {
  it('all tabs fit — no arrows', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 400,   // tabs total width
      tabsClientWidth: 756,   // 800 - 44
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('tabs width exactly equals available width → no arrows', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 756,   // exactly fills available space
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });
});

// TBTC-SCR-002
describe('TBTC-SCR-002: overflow at right edge → right arrow only', () => {
  it('scrollLeft = 0, content wider than bar → only canScrollRight', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 900,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(true);
  });

  it('scrollLeft = 1 (boundary) → still no left arrow', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 900,
      tabsClientWidth: 756,
      tabsScrollLeft: 1,       // not > 1 → no left arrow
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-003
describe('TBTC-SCR-003: overflow, scrolled to end → left arrow only', () => {
  it('scrolled fully right (scrollLeft + clientWidth >= scrollWidth - 1)', () => {
    // scrollWidth=900, clientWidth=756, scrollLeft=144  → 144+756=900 >= 900-1=899 ✓ no right
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 900,
      tabsClientWidth: 756,
      tabsScrollLeft: 144,
    });
    expect(state.canScrollLeft).toBe(true);
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollLeft = 2 (just above threshold) → left arrow appears', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 900,
      tabsClientWidth: 756,
      tabsScrollLeft: 2,       // > 1 → left arrow
    });
    expect(state.canScrollLeft).toBe(true);
  });
});

// TBTC-SCR-004
describe('TBTC-SCR-004: overflow, scrolled to middle → both arrows', () => {
  it('scrollLeft > 1 and scrollLeft + clientWidth < scrollWidth - 1', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 1200,
      tabsClientWidth: 756,
      tabsScrollLeft: 100,    // 100 > 1 ✓ ; 100+756=856 < 1199 ✓
    });
    expect(state.canScrollLeft).toBe(true);
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-005 — bistability regression guard
describe('TBTC-SCR-005: bistability — tabs fit bar-minus-newTab but exceed tabsContainer', () => {
  it('scrollWidth exceeds tabsClientWidth but not availableWidth → no arrows', () => {
    // Scenario: bar is 648px, newTab is 44px, arrows are 24px each.
    // availableWidth = 648 - 44 = 604
    // tabsClientWidth = 648 - 44 - 48 = 556  ← what the OLD (buggy) code used
    // tabsScrollWidth = 570  ← fits in 604 but not in 556
    //
    // Bug: 570 > 556 + 2=558 → true → arrows shown → loop
    // Fix: 570 > 604 + 2=606 → false → no arrows (correct)
    const state = computeScrollState({
      tabBarClientWidth: 648,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 570,
      tabsClientWidth: 556,   // shrunk as if arrows were already present
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollWidth just above availableWidth → arrows correctly shown', () => {
    // Same bar but tabs genuinely overflow: scrollWidth = 650 > 604 + 2
    const state = computeScrollState({
      tabBarClientWidth: 648,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 650,
      tabsClientWidth: 556,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-006 — 2px tolerance
describe('TBTC-SCR-006: 2 px sub-pixel tolerance', () => {
  it('scrollWidth = availableWidth + 2 → no overflow (within tolerance)', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 758,   // 756 + 2 = at boundary, not >, so no overflow
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollWidth = availableWidth + 3 → overflow detected', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 759,   // 756 + 3 → overflow
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-007 — boundary thresholds for scrollLeft
describe('TBTC-SCR-007: scrollLeft boundary thresholds', () => {
  it('scrollLeft = 0 → no left arrow', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
  });

  it('scrollLeft = 1 → no left arrow (threshold is > 1, not >= 1)', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 1,
    });
    expect(state.canScrollLeft).toBe(false);
  });

  it('scrollLeft = 2 → left arrow shown', () => {
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 2,
    });
    expect(state.canScrollLeft).toBe(true);
  });

  it('scrollLeft + clientWidth = scrollWidth - 1 → right arrow hidden (at boundary)', () => {
    // 244 + 756 = 1000 = 1000 - 1 + 1 → NOT < 999 → no right arrow
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 244,   // 244 + 756 = 1000, not < 999
    });
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollLeft + clientWidth = scrollWidth - 2 → right arrow shown', () => {
    // 243 + 756 = 999 = 1000 - 1 → NOT < 999 ... wait, 999 < 999 is false.
    // 242 + 756 = 998 < 999 → right arrow
    const state = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 242,   // 242 + 756 = 998 < 999
    });
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-008 — newTabBtn absent
describe('TBTC-SCR-008: newTabBtn absent → fallback offset 44', () => {
  it('null newTabBtnOffsetWidth uses 44 as fallback', () => {
    // availableWidth = 800 - 44 = 756
    const withFallback = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: null,
      tabsScrollWidth: 400,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    // Equivalent to newTabBtnOffsetWidth = 44
    const withExplicit = computeScrollState({
      tabBarClientWidth: 800,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 400,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(withFallback).toEqual(withExplicit);
  });
});

// ---------------------------------------------------------------------------
// Part 2 — DOM integration tests (component mounted in jsdom)
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

function makeTab(id: string, order: number = 0, overrides: Partial<TabState> = {}): TabState {
  const pane = makePaneState({ id: `pane-${id}` });
  return {
    id,
    label: null,
    activePaneId: `pane-${id}`,
    order,
    layout: { type: 'leaf', paneId: `pane-${id}`, state: pane },
    ...overrides,
  };
}

/**
 * Override a read-only DOM property (clientWidth, scrollWidth, offsetWidth)
 * on an element using Object.defineProperty.
 */
function mockProp(el: Element, prop: string, value: number): void {
  Object.defineProperty(el, prop, { value, configurable: true, writable: false });
}

/**
 * Set up DOM measurements and trigger updateScrollState via a scroll event.
 *
 * @param tabBar       - .tab-bar root element
 * @param tabsContainer - .tab-bar__tabs element
 * @param params       - width/scroll measurements to inject
 */
function triggerScrollState(
  tabBar: HTMLElement,
  tabsContainer: HTMLElement,
  params: {
    tabBarWidth: number;
    tabsScrollWidth: number;
    tabsClientWidth: number;
    tabsScrollLeft: number;
    newTabBtnWidth?: number;
  },
): void {
  const newTabBtn = tabBar.querySelector<HTMLElement>('.tab-bar__new-tab');

  mockProp(tabBar, 'clientWidth', params.tabBarWidth);
  if (newTabBtn) {
    mockProp(newTabBtn, 'offsetWidth', params.newTabBtnWidth ?? 44);
  }
  mockProp(tabsContainer, 'scrollWidth', params.tabsScrollWidth);
  mockProp(tabsContainer, 'clientWidth', params.tabsClientWidth);
  // scrollLeft is normally writable
  Object.defineProperty(tabsContainer, 'scrollLeft', {
    value: params.tabsScrollLeft,
    configurable: true,
    writable: true,
  });

  tabsContainer.dispatchEvent(new Event('scroll'));
  flushSync();
}

const mountedInstances: ReturnType<typeof mount>[] = [];

afterEach(() => {
  mountedInstances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* already unmounted */
    }
  });
  mountedInstances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

// TBTC-SCR-010
describe('TBTC-SCR-010: no arrows on initial render (jsdom zero-width DOM)', () => {
  it('neither arrow is rendered immediately after mount', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [makeTab('t1'), makeTab('t2'), makeTab('t3')],
        activeTabId: 't1',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    mountedInstances.push(instance);
    flushSync();

    // jsdom has no layout → all widths are 0 → no overflow → no arrows
    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
  });
});

// TBTC-SCR-011
describe('TBTC-SCR-011: right arrow appears when tabs overflow to the right', () => {
  it('right arrow rendered when scrollWidth > availableWidth + 2', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [makeTab('t1'), makeTab('t2'), makeTab('t3')],
        activeTabId: 't1',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    mountedInstances.push(instance);
    flushSync();

    const tabBar = container.querySelector<HTMLElement>('.tab-bar')!;
    const tabsContainer = container.querySelector<HTMLElement>('.tab-bar__tabs')!;

    triggerScrollState(tabBar, tabsContainer, {
      tabBarWidth: 600,
      tabsScrollWidth: 800,    // overflows available 556
      tabsClientWidth: 556,
      tabsScrollLeft: 0,
      newTabBtnWidth: 44,
    });

    expect(container.querySelector('.tab-bar__scroll-arrow--right')).not.toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
  });
});

// TBTC-SCR-012
describe('TBTC-SCR-012: left arrow appears when scrolled right', () => {
  it('left arrow rendered when scrollLeft > 1 and there is overflow', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [makeTab('t1'), makeTab('t2'), makeTab('t3')],
        activeTabId: 't1',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    mountedInstances.push(instance);
    flushSync();

    const tabBar = container.querySelector<HTMLElement>('.tab-bar')!;
    const tabsContainer = container.querySelector<HTMLElement>('.tab-bar__tabs')!;

    // Simulate scrolled to the very end: scrollLeft + clientWidth >= scrollWidth - 1
    // 244 + 556 = 800, 800 >= 800 - 1 = 799 → no right arrow
    // scrollLeft 244 > 1 → left arrow shown
    triggerScrollState(tabBar, tabsContainer, {
      tabBarWidth: 600,
      tabsScrollWidth: 800,
      tabsClientWidth: 556,
      tabsScrollLeft: 244,
      newTabBtnWidth: 44,
    });

    expect(container.querySelector('.tab-bar__scroll-arrow--left')).not.toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
  });
});

// TBTC-SCR-013
describe('TBTC-SCR-013: both arrows in middle of long tab list', () => {
  it('both arrows rendered when scrolled past left edge but right content remains', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [makeTab('t1'), makeTab('t2'), makeTab('t3'), makeTab('t4'), makeTab('t5')],
        activeTabId: 't1',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    mountedInstances.push(instance);
    flushSync();

    const tabBar = container.querySelector<HTMLElement>('.tab-bar')!;
    const tabsContainer = container.querySelector<HTMLElement>('.tab-bar__tabs')!;

    // scrollLeft=100 > 1 ✓; 100+556=656 < 1200-1=1199 ✓
    triggerScrollState(tabBar, tabsContainer, {
      tabBarWidth: 600,
      tabsScrollWidth: 1200,
      tabsClientWidth: 556,
      tabsScrollLeft: 100,
      newTabBtnWidth: 44,
    });

    expect(container.querySelector('.tab-bar__scroll-arrow--left')).not.toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).not.toBeNull();
  });
});

// TBTC-SCR-014
describe('TBTC-SCR-014: arrows disappear after tabs removed', () => {
  it('right arrow gone after scroll event reflects reduced content', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [makeTab('t1'), makeTab('t2'), makeTab('t3')],
        activeTabId: 't1',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    mountedInstances.push(instance);
    flushSync();

    const tabBar = container.querySelector<HTMLElement>('.tab-bar')!;
    const tabsContainer = container.querySelector<HTMLElement>('.tab-bar__tabs')!;

    // First: overflow present
    triggerScrollState(tabBar, tabsContainer, {
      tabBarWidth: 600,
      tabsScrollWidth: 800,
      tabsClientWidth: 556,
      tabsScrollLeft: 0,
      newTabBtnWidth: 44,
    });
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).not.toBeNull();

    // Simulate tabs removed → scrollWidth now fits
    triggerScrollState(tabBar, tabsContainer, {
      tabBarWidth: 600,
      tabsScrollWidth: 400,   // now fits in 556
      tabsClientWidth: 556,
      tabsScrollLeft: 0,
      newTabBtnWidth: 44,
    });
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
  });
});

// TBTC-SCR-015 — bistability regression guard (DOM level)
describe('TBTC-SCR-015: bistability regression — DOM level', () => {
  it('no arrows when tabs exceed tabsContainer.clientWidth but fit in bar-minus-newTab', () => {
    // This is the exact bug scenario.
    //
    // bar = 648px, newTab = 44px, arrows = 24px each.
    // tabsScrollWidth = 570  (tabs total content)
    // tabsClientWidth = 556  (bar - newTab - 2 arrows = 648 - 44 - 48)
    //
    // Old code: 570 > 556 + 2 = 558 → true → arrows shown → bistability
    // New code: available = 648 - 44 = 604; 570 > 604 + 2 = 606 → false → no arrows
    const container = document.createElement('div');
    document.body.appendChild(container);

    const instance = mount(TabBarWithProvider, {
      target: container,
      props: {
        tabs: [makeTab('t1'), makeTab('t2'), makeTab('t3')],
        activeTabId: 't1',
        onTabClick: () => {},
        onTabClose: () => {},
        onNewTab: () => {},
      },
    });
    mountedInstances.push(instance);
    flushSync();

    const tabBar = container.querySelector<HTMLElement>('.tab-bar')!;
    const tabsContainer = container.querySelector<HTMLElement>('.tab-bar__tabs')!;

    triggerScrollState(tabBar, tabsContainer, {
      tabBarWidth: 648,
      tabsScrollWidth: 570,
      tabsClientWidth: 556,   // as if arrows were already consuming 48px
      tabsScrollLeft: 0,
      newTabBtnWidth: 44,
    });

    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
  });
});
