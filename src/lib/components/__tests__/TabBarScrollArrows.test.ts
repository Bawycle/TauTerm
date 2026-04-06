// SPDX-License-Identifier: MPL-2.0

/**
 * TabBar scroll arrows — visibility logic tests.
 *
 * The bistability fix (FS-UX-012): comparing scrollWidth directly against
 * tabsContainerEl.clientWidth. Bistability protection is structural: the
 * `.tab-bar` uses `flex: 1 0 0` in its parent row, so tabsContainerEl.clientWidth
 * is always the correct available width at measurement time — it starts from 0
 * and grows to `rowWidth − newTabBtn − sshBtn`, never exceeding that budget.
 *
 * Note: the new-tab "+" button was moved out of TabBar into TerminalView, so
 * TabBar no longer needs to subtract a newTabBtn width from its measurements.
 *
 * Part 1 — Pure logic tests (no DOM):
 *   TBTC-SCR-001 — no overflow → both arrows hidden
 *   TBTC-SCR-002 — overflow at right edge → right arrow shown, left hidden
 *   TBTC-SCR-003 — overflow, scrolled to end → left arrow shown, right hidden
 *   TBTC-SCR-004 — overflow, scrolled to middle → both arrows shown
 *   TBTC-SCR-005 — bistability protection: CSS-constrained clientWidth always
 *                   reflects the true available space; direct comparison is sound
 *   TBTC-SCR-006 — 2 px sub-pixel tolerance boundary
 *   TBTC-SCR-007 — scrollLeft exactly at boundary thresholds
 *   TBTC-SCR-008 — (obsolete: newTabBtn no longer in TabBar)
 *
 * Part 2 — DOM integration tests (component mounted):
 *   TBTC-SCR-010 — arrows absent on initial render with zero-width DOM (jsdom)
 *   TBTC-SCR-011 — right arrow appears when scrollWidth > tabsClientWidth + 2
 *   TBTC-SCR-012 — left arrow appears when scrolled right
 *   TBTC-SCR-013 — both arrows appear in the middle of a long tab list
 *   TBTC-SCR-014 — arrows disappear after tab removal reduces scrollWidth
 *   TBTC-SCR-015 — no arrows when scrollWidth ≤ tabsClientWidth + 2 (no overflow)
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
  /** tabsContainerEl.scrollWidth (total content width) */
  tabsScrollWidth: number;
  /** tabsContainerEl.clientWidth (visible width of the scroll container) */
  tabsClientWidth: number;
  /** tabsContainerEl.scrollLeft */
  tabsScrollLeft: number;
}

/**
 * Mirror of updateScrollState() from TabBar.svelte.
 * The implementation reads only tabsContainerEl properties — no parent bar measurement.
 */
function computeScrollState(m: ScrollMeasurements): { canScrollLeft: boolean; canScrollRight: boolean } {
  const hasOverflow = m.tabsScrollWidth > m.tabsClientWidth + 2;
  return {
    canScrollLeft: hasOverflow && m.tabsScrollLeft > 1,
    canScrollRight: hasOverflow && m.tabsScrollLeft + m.tabsClientWidth < m.tabsScrollWidth - 1,
  };
}

// TBTC-SCR-001
describe('TBTC-SCR-001: no overflow → both arrows hidden', () => {
  it('all tabs fit — no arrows', () => {
    const state = computeScrollState({
      tabsScrollWidth: 400,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('tabs width exactly equals tabsClientWidth → no arrows', () => {
    const state = computeScrollState({
      tabsScrollWidth: 756,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });
});

// TBTC-SCR-002
describe('TBTC-SCR-002: overflow at right edge → right arrow only', () => {
  it('scrollLeft = 0, content wider than container → only canScrollRight', () => {
    const state = computeScrollState({
      tabsScrollWidth: 900,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(true);
  });

  it('scrollLeft = 1 (boundary) → still no left arrow', () => {
    const state = computeScrollState({
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
      tabsScrollWidth: 900,
      tabsClientWidth: 756,
      tabsScrollLeft: 144,
    });
    expect(state.canScrollLeft).toBe(true);
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollLeft = 2 (just above threshold) → left arrow appears', () => {
    const state = computeScrollState({
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
      tabsScrollWidth: 1200,
      tabsClientWidth: 756,
      tabsScrollLeft: 100,    // 100 > 1 ✓ ; 100+756=856 < 1199 ✓
    });
    expect(state.canScrollLeft).toBe(true);
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-005 — bistability protection via CSS-constrained layout
describe('TBTC-SCR-005: bistability protection via CSS flex: 1 0 0', () => {
  // The previous bistability bug used tabBar.clientWidth (content-sized) to
  // compute availableWidth.  Adding a 24 px arrow expanded tabBar.clientWidth
  // by 24 px, inflating availableWidth and turning off overflow detection.
  //
  // The fix is two-part:
  //   1. CSS: .tab-bar uses `flex: 1 0 0` (basis 0), so it is always
  //      constrained to rowWidth − newTabBtn − sshBtn.  tabsClientWidth is
  //      therefore the true available width at the time of measurement.
  //   2. JS: compare scrollWidth > tabsClientWidth + 2 directly.
  //
  // When tabsClientWidth is properly constrained, the comparison is always
  // measured in a stable state.

  it('no overflow when scrollWidth ≤ tabsClientWidth + 2', () => {
    // tabs fit comfortably — no arrows
    const state = computeScrollState({
      tabsScrollWidth: 570,
      tabsClientWidth: 600,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('genuine overflow when scrollWidth > tabsClientWidth + 2', () => {
    // tabs genuinely exceed available space — right arrow shown
    const state = computeScrollState({
      tabsScrollWidth: 650,
      tabsClientWidth: 600,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-006 — 2px tolerance
describe('TBTC-SCR-006: 2 px sub-pixel tolerance', () => {
  it('scrollWidth = tabsClientWidth + 2 → no overflow (within tolerance)', () => {
    const state = computeScrollState({
      tabsScrollWidth: 758,   // 756 + 2 = at boundary, not >, so no overflow
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollWidth = tabsClientWidth + 3 → overflow detected', () => {
    const state = computeScrollState({
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
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 0,
    });
    expect(state.canScrollLeft).toBe(false);
  });

  it('scrollLeft = 1 → no left arrow (threshold is > 1, not >= 1)', () => {
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 1,
    });
    expect(state.canScrollLeft).toBe(false);
  });

  it('scrollLeft = 2 → left arrow shown', () => {
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 2,
    });
    expect(state.canScrollLeft).toBe(true);
  });

  it('scrollLeft + clientWidth = scrollWidth - 1 → right arrow hidden (at boundary)', () => {
    // 244 + 756 = 1000, not < 999 → no right arrow
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 244,   // 244 + 756 = 1000, not < 999
    });
    expect(state.canScrollRight).toBe(false);
  });

  it('scrollLeft + clientWidth = scrollWidth - 2 → right arrow shown', () => {
    // 242 + 756 = 998 < 999 → right arrow
    const state = computeScrollState({
      tabsScrollWidth: 1000,
      tabsClientWidth: 756,
      tabsScrollLeft: 242,   // 242 + 756 = 998 < 999
    });
    expect(state.canScrollRight).toBe(true);
  });
});

// TBTC-SCR-008 — (obsolete: newTabBtn was moved out of TabBar into TerminalView)
describe('TBTC-SCR-008: newTabBtn no longer in TabBar', () => {
  it.todo('newTabBtn is now in TerminalView — TabBar no longer reads its width');
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
 * The implementation reads only tabsContainerEl properties — tabBar width is
 * not used.
 *
 * @param _tabBar      - .tab-bar root element (unused, kept for call-site clarity)
 * @param tabsContainer - .tab-bar__tabs element
 * @param params       - tabsContainer measurements to inject
 */
function triggerScrollState(
  _tabBar: HTMLElement,
  tabsContainer: HTMLElement,
  params: {
    tabsScrollWidth: number;
    tabsClientWidth: number;
    tabsScrollLeft: number;
  },
): void {
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
      tabsScrollWidth: 800,    // overflows 556px container
      tabsClientWidth: 556,
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

    // Simulate scrolled to the very end: scrollLeft + clientWidth >= scrollWidth - 1
    // 244 + 556 = 800, 800 >= 800 - 1 = 799 → no right arrow
    // scrollLeft 244 > 1 → left arrow shown
    triggerScrollState(tabBar, tabsContainer, {
      tabsScrollWidth: 800,
      tabsClientWidth: 556,
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

    // scrollLeft=100 > 1 ✓; 100+556=656 < 1200-1=1199 ✓
    triggerScrollState(tabBar, tabsContainer, {
      tabsScrollWidth: 1200,
      tabsClientWidth: 556,
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

    // First: overflow present
    triggerScrollState(tabBar, tabsContainer, {
      tabsScrollWidth: 800,
      tabsClientWidth: 556,
      tabsScrollLeft: 0,
    });
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).not.toBeNull();

    // Simulate tabs removed → scrollWidth now fits
    triggerScrollState(tabBar, tabsContainer, {
      tabsScrollWidth: 400,   // now fits in 556
      tabsClientWidth: 556,
      tabsScrollLeft: 0,
    });
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
  });
});

// TBTC-SCR-015 — no arrows when scrollWidth does not exceed tabsClientWidth + 2
describe('TBTC-SCR-015: no overflow — no arrows (DOM level)', () => {
  it('no arrows when scrollWidth ≤ tabsClientWidth + 2', () => {
    // Scenario: scrollWidth = 545, clientWidth = 556.
    // 545 > 558 → false → no arrows.
    //
    // Bistability protection is structural: .tab-bar uses flex: 1 0 0, so
    // tabsClientWidth reflects the true available width from the start.
    // The JS comparison is a simple direct check — no newTabBtn subtraction.
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
      tabsScrollWidth: 545,   // tabs fit within 556 + 2 tolerance
      tabsClientWidth: 556,
      tabsScrollLeft: 0,
    });

    expect(container.querySelector('.tab-bar__scroll-arrow--left')).toBeNull();
    expect(container.querySelector('.tab-bar__scroll-arrow--right')).toBeNull();
  });
});
