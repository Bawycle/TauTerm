// SPDX-License-Identifier: MPL-2.0

/**
 * TabBar scroll arrows — visibility logic tests.
 *
 * The overflow detection algorithm (updateScrollState) avoids
 * tabsContainerEl.clientWidth because WebKitGTK resolves max-width:max-content
 * on overflow:auto elements to ~min-content, making clientWidth unreliable.
 *
 * Instead, the algorithm uses:
 *   - tabBarEl.clientWidth     → outer .tab-bar div (flex: 1 0 0, no overflow:auto, WebKitGTK-safe)
 *   - newTabBtnEl.offsetWidth  → fixed-size "+" button
 *   - totalTabsSpace = tabBarWidth − newTabBtnWidth
 *   - hasOverflow = tabsScrollWidth > totalTabsSpace + 2
 *
 * Part 1 — Pure logic tests (no DOM):
 *   TBTC-SCR-001 — no overflow → both arrows hidden
 *   TBTC-SCR-002 — overflow at right edge → right arrow shown, left hidden
 *   TBTC-SCR-003 — overflow, scrolled to end → left arrow shown, right hidden
 *   TBTC-SCR-004 — overflow, scrolled to middle → both arrows shown
 *   TBTC-SCR-005 — bistability protection: tabBarEl.clientWidth is stable (no overflow:auto)
 *   TBTC-SCR-006 — 2 px sub-pixel tolerance boundary
 *   TBTC-SCR-007 — scrollLeft exactly at boundary thresholds
 *   TBTC-SCR-008 — newTabBtn width is subtracted from available space
 *
 * Part 2 — DOM integration tests (component mounted):
 *   TBTC-SCR-010 — arrows absent on initial render with zero-width DOM (jsdom)
 *   TBTC-SCR-011 — right arrow appears when tabs overflow
 *   TBTC-SCR-012 — left arrow appears when scrolled right
 *   TBTC-SCR-013 — both arrows appear in the middle of a long tab list
 *   TBTC-SCR-014 — arrows disappear after tab removal reduces scrollWidth
 *   TBTC-SCR-015 — no arrows when tabs fit in available space
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
 *
 * Uses tabBarWidth and newTabBtnWidth (not tabsClientWidth) for overflow
 * detection — matching the WebKitGTK-safe implementation.
 *
 * prevCanScrollLeft/Right represent the arrow state BEFORE this call, used
 * to compute visibleTabsWidth for the canScrollRight check.
 */
interface ScrollMeasurements {
  /** tabsContainerEl.scrollWidth (total tab content width) */
  tabsScrollWidth: number;
  /** tabBarEl.clientWidth (outer .tab-bar div — WebKitGTK-safe) */
  tabBarWidth: number;
  /** newTabBtnEl.offsetWidth (the "+" button) */
  newTabBtnWidth: number;
  /** tabsContainerEl.scrollLeft */
  tabsScrollLeft: number;
  /** Arrow state before this call (defaults to false = initial state) */
  prevCanScrollLeft?: boolean;
  prevCanScrollRight?: boolean;
}

const SCROLL_ARROW_WIDTH = 24; // matches CSS .tab-bar__scroll-arrow { width: 24px }

/**
 * Mirror of updateScrollState() from TabBar.svelte.
 */
function computeScrollState(m: ScrollMeasurements): { canScrollLeft: boolean; canScrollRight: boolean } {
  const prevLeft = m.prevCanScrollLeft ?? false;
  const prevRight = m.prevCanScrollRight ?? false;

  const totalTabsSpace = m.tabBarWidth - m.newTabBtnWidth;
  const hasOverflow = m.tabsScrollWidth > totalTabsSpace + 2;

  const visibleTabsWidth =
    totalTabsSpace -
    (prevLeft ? SCROLL_ARROW_WIDTH : 0) -
    (prevRight ? SCROLL_ARROW_WIDTH : 0);

  return {
    canScrollLeft: hasOverflow && m.tabsScrollLeft > 1,
    canScrollRight: hasOverflow && m.tabsScrollLeft + visibleTabsWidth < m.tabsScrollWidth - 1,
  };
}

// TBTC-SCR-001
describe('TBTC-SCR-001: no overflow → both arrows hidden', () => {
  it('all tabs fit — no arrows', () => {
    const state = computeScrollState({
      tabsScrollWidth: 400,
      tabBarWidth: 800,
      newTabBtnWidth: 44,  // totalTabsSpace = 756
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('tabs width exactly equals totalTabsSpace → no arrows', () => {
    const state = computeScrollState({
      tabsScrollWidth: 756,
      tabBarWidth: 800,
      newTabBtnWidth: 44,  // totalTabsSpace = 756 ; 756 > 756 + 2 = false
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });
});

// TBTC-SCR-002
describe('TBTC-SCR-002: overflow at right edge → right arrow only', () => {
  it('scrollLeft = 0, content wider than totalTabsSpace → only canScrollRight', () => {
    const state = computeScrollState({
      tabsScrollWidth: 900,
      tabBarWidth: 800,
      newTabBtnWidth: 44,  // totalTabsSpace = 756
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(true);
  });

  it('scrollLeft = 1 (boundary) → still no left arrow', () => {
    const state = computeScrollState({
      tabsScrollWidth: 900,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 1,  // not > 1 → no left arrow
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-003
describe('TBTC-SCR-003: overflow, scrolled to end → left arrow only', () => {
  it('scrolled fully right: scrollLeft + visibleTabsWidth >= scrollWidth - 1', () => {
    // totalTabsSpace=756; no prev arrows → visibleTabsWidth=756
    // scrollLeft=144: 144+756=900 >= 900-1=899 → no right
    // scrollLeft=144 > 1 → left
    const state = computeScrollState({
      tabsScrollWidth: 900,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 144,
    });
    expect(state.canScrollLeft).toBe(true);
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollLeft = 2 (just above threshold) → left arrow appears', () => {
    const state = computeScrollState({
      tabsScrollWidth: 900,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 2,  // > 1 → left arrow
    });
    expect(state.canScrollLeft).toBe(true);
  });
});

// TBTC-SCR-004
describe('TBTC-SCR-004: overflow, scrolled to middle → both arrows', () => {
  it('scrollLeft > 1 and scrollLeft + visibleTabsWidth < scrollWidth - 1', () => {
    // totalTabsSpace=756; visibleTabsWidth=756
    // scrollLeft=100: 100 > 1 ✓; 100+756=856 < 1200-1=1199 ✓
    const state = computeScrollState({
      tabsScrollWidth: 1200,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 100,
    });
    expect(state.canScrollLeft).toBe(true);
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-005 — bistability protection via tabBarEl.clientWidth
describe('TBTC-SCR-005: bistability protection via tabBarEl.clientWidth', () => {
  // Previous bistability bug: used tabsContainerEl.clientWidth, which changed
  // when scroll arrows appeared/disappeared (stealing flex space).
  // This created a feedback loop: arrows appear → clientWidth shrinks → still looks
  // like overflow → arrows stay.
  //
  // Fix: use tabBarEl.clientWidth (flex: 1 0 0, no overflow:auto).
  // tabBarEl.clientWidth is constant regardless of internal arrow visibility —
  // it is always the full available bar width (row − SSH button).
  // Therefore: arrows appearing inside tabBar do NOT affect the overflow threshold.

  it('no overflow when tabsScrollWidth ≤ totalTabsSpace + 2', () => {
    // tabs fit comfortably
    const state = computeScrollState({
      tabsScrollWidth: 570,
      tabBarWidth: 644,   // totalTabsSpace = 644 - 44 = 600; 570 > 602 → false
      newTabBtnWidth: 44,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('genuine overflow when tabsScrollWidth > totalTabsSpace + 2', () => {
    const state = computeScrollState({
      tabsScrollWidth: 650,
      tabBarWidth: 644,   // totalTabsSpace = 600; 650 > 602 → true
      newTabBtnWidth: 44,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollRight).toBe(true);
  });

  it('tabBarWidth stays constant when a 24 px arrow appears (no bistability)', () => {
    // With tabsContainerEl.clientWidth the bug was: when arrow appeared (24px),
    // clientWidth dropped by 24px, so the next check used a smaller baseline.
    // With tabBarEl.clientWidth: it never changes. Same threshold, stable result.
    const baseParams = {
      tabsScrollWidth: 650,
      tabBarWidth: 644,   // constant regardless of arrows inside
      newTabBtnWidth: 44,
      tabsScrollLeft: 0,
    };

    const initial = computeScrollState(baseParams);
    // After right arrow appears, re-run with prevCanScrollRight = true
    const afterArrow = computeScrollState({ ...baseParams, prevCanScrollRight: initial.canScrollRight });

    // hasOverflow threshold is the same in both calls — no bistability
    expect(initial.canScrollRight).toBe(true);
    expect(afterArrow.canScrollRight).toBe(true);  // still overflowing — arrow stays
  });
});

// TBTC-SCR-006 — 2px tolerance
describe('TBTC-SCR-006: 2 px sub-pixel tolerance', () => {
  it('tabsScrollWidth = totalTabsSpace + 2 → no overflow (within tolerance)', () => {
    // totalTabsSpace = 756; 758 > 758 → false
    const state = computeScrollState({
      tabsScrollWidth: 758,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('tabsScrollWidth = totalTabsSpace + 3 → overflow detected', () => {
    const state = computeScrollState({
      tabsScrollWidth: 759,   // 756 + 3 → 759 > 758 → overflow
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-007 — boundary thresholds for scrollLeft
describe('TBTC-SCR-007: scrollLeft boundary thresholds', () => {
  it('scrollLeft = 0 → no left arrow', () => {
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
  });

  it('scrollLeft = 1 → no left arrow (threshold is > 1, not >= 1)', () => {
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 1,
    });
    expect(state.canScrollLeft).toBe(false);
  });

  it('scrollLeft = 2 → left arrow shown', () => {
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 2,
    });
    expect(state.canScrollLeft).toBe(true);
  });

  it('scrollLeft + visibleTabsWidth = scrollWidth - 1 → right arrow hidden (at boundary)', () => {
    // totalTabsSpace = 756; visibleTabsWidth = 756
    // scrollLeft = 243: 243 + 756 = 999 = scrollWidth - 1 = 999 → not < → no right
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 243,
    });
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollLeft + visibleTabsWidth = scrollWidth - 2 → right arrow shown', () => {
    // scrollLeft = 242: 242 + 756 = 998 < 999 → right arrow shown
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabBarWidth: 800,
      newTabBtnWidth: 44,
      tabsScrollLeft: 242,
    });
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-008 — newTabBtn width is subtracted from available space
describe('TBTC-SCR-008: newTabBtn width reduces available space for tabs', () => {
  it('wider newTabBtn leaves less space — more likely to overflow', () => {
    // With narrow newTabBtn (24px): totalTabsSpace = 776; 780 > 778 → overflow
    // With wide newTabBtn (64px): totalTabsSpace = 736; 780 > 738 → overflow (same result)
    // With tiny newTabBtn (0px): totalTabsSpace = 800; 780 > 802 → no overflow
    const withNarrow = computeScrollState({
      tabsScrollWidth: 780,
      tabBarWidth: 800,
      newTabBtnWidth: 24,   // totalTabsSpace = 776; 780 > 778 → overflow
      tabsScrollLeft: 0,
    });
    expect(withNarrow.canScrollRight).toBe(true);

    const withNoBtn = computeScrollState({
      tabsScrollWidth: 780,
      tabBarWidth: 800,
      newTabBtnWidth: 0,    // totalTabsSpace = 800; 780 > 802 → no overflow
      tabsScrollLeft: 0,
    });
    expect(withNoBtn.canScrollRight).toBe(false);
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
 * Mocks:
 *   - tabBar.clientWidth        → totalTabsSpace denominator (WebKitGTK-safe)
 *   - newTabBtn.offsetWidth     → subtracted from tabBar.clientWidth
 *   - tabsContainer.scrollWidth → tab content width
 *   - tabsContainer.scrollLeft  → current scroll position
 */
function triggerScrollState(
  tabBar: HTMLElement,
  tabsContainer: HTMLElement,
  newTabBtn: HTMLElement,
  params: {
    tabBarClientWidth: number;
    newTabBtnOffsetWidth: number;
    tabsScrollWidth: number;
    tabsScrollLeft: number;
  },
): void {
  mockProp(tabBar, 'clientWidth', params.tabBarClientWidth);
  mockProp(newTabBtn, 'offsetWidth', params.newTabBtnOffsetWidth);
  mockProp(tabsContainer, 'scrollWidth', params.tabsScrollWidth);
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

    // jsdom has no layout → all widths are 0 → totalTabsSpace = 0 → no overflow → no arrows
    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
  });
});

// TBTC-SCR-011
describe('TBTC-SCR-011: right arrow appears when tabs overflow to the right', () => {
  it('right arrow rendered when tabsScrollWidth > totalTabsSpace + 2', () => {
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
    const newTabBtn = container.querySelector<HTMLElement>('.tab-bar__new-tab')!;

    // tabBarWidth=600, newTabBtnWidth=44 → totalTabsSpace=556; tabsScrollWidth=800 > 558 → overflow
    triggerScrollState(tabBar, tabsContainer, newTabBtn, {
      tabBarClientWidth: 600,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 800,
      tabsScrollLeft: 0,
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
    const newTabBtn = container.querySelector<HTMLElement>('.tab-bar__new-tab')!;

    // totalTabsSpace = 600 - 44 = 556
    // scrollLeft=244: 244 > 1 ✓; 244+556=800 >= 800-1=799 → no right arrow
    triggerScrollState(tabBar, tabsContainer, newTabBtn, {
      tabBarClientWidth: 600,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 800,
      tabsScrollLeft: 244,
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
    const newTabBtn = container.querySelector<HTMLElement>('.tab-bar__new-tab')!;

    // totalTabsSpace = 556; scrollLeft=100: 100 > 1 ✓; 100+556=656 < 1200-1=1199 ✓
    triggerScrollState(tabBar, tabsContainer, newTabBtn, {
      tabBarClientWidth: 600,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 1200,
      tabsScrollLeft: 100,
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
    const newTabBtn = container.querySelector<HTMLElement>('.tab-bar__new-tab')!;

    // First: overflow present
    triggerScrollState(tabBar, tabsContainer, newTabBtn, {
      tabBarClientWidth: 600,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 800,
      tabsScrollLeft: 0,
    });
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).not.toBeNull();

    // Simulate tabs removed → scrollWidth now fits (400 < 556 + 2)
    triggerScrollState(tabBar, tabsContainer, newTabBtn, {
      tabBarClientWidth: 600,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 400,
      tabsScrollLeft: 0,
    });
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
  });
});

// TBTC-SCR-015 — no arrows when tabs fit in available space
describe('TBTC-SCR-015: no overflow — no arrows (DOM level)', () => {
  it('no arrows when tabsScrollWidth ≤ totalTabsSpace + 2', () => {
    // totalTabsSpace = 600 - 44 = 556; 545 > 558 → false
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
    const newTabBtn = container.querySelector<HTMLElement>('.tab-bar__new-tab')!;

    triggerScrollState(tabBar, tabsContainer, newTabBtn, {
      tabBarClientWidth: 600,
      newTabBtnOffsetWidth: 44,
      tabsScrollWidth: 545,   // 545 ≤ 556 + 2 = 558 → no overflow
      tabsScrollLeft: 0,
    });

    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
  });
});
