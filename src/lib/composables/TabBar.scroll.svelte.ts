// SPDX-License-Identifier: MPL-2.0
/**
 * TabBar.scroll.svelte.ts — composable for horizontal scroll / overflow detection.
 *
 * Manages ResizeObserver on the outer tab-bar element, computes canScrollLeft/Right,
 * hidden-tab badge colours (bell/output), and exposes scrollTabs / scrollActiveTabIntoView.
 *
 * Dependencies are injected as getter functions to preserve Svelte 5 reactivity
 * across the composable boundary.
 */

import { onMount, onDestroy } from 'svelte';
import type { TabState, PaneNotification } from '$lib/ipc';

// Fixed width of each scroll arrow button — must match CSS .tab-bar__scroll-arrow { width: 24px }.
const SCROLL_ARROW_WIDTH = 24;

export interface TabScrollComposable {
  readonly canScrollLeft: boolean;
  readonly canScrollRight: boolean;
  readonly leftBadge: 'bell' | 'output' | null;
  readonly rightBadge: 'bell' | 'output' | null;
  scrollTabs(direction: 'left' | 'right'): void;
  scrollActiveTabIntoView(): void;
  updateScrollState(): void;
}

interface TabScrollOptions {
  /** Getter for the scrollable tabs container element. */
  tabsContainerEl: () => HTMLDivElement | null;
  /** Getter for the outer tab-bar element (flex: 1 0 0, no overflow:auto). */
  tabBarEl: () => HTMLDivElement | null;
  /** Getter for the new-tab button element (needed for offsetWidth). */
  newTabBtnEl: () => HTMLButtonElement | null;
  /** Getter for the current activeTabId (used for scrollActiveTabIntoView). */
  activeTabId: () => string;
  /** Getter for the sorted tabs list (used for badge computation). */
  sortedTabs: () => TabState[];
  /** Getter for the tab notification of a given tab (root pane notification). */
  tabNotification: (tab: TabState) => PaneNotification | null;
  /** Getter for the tab count — memoised primitive to avoid spurious reruns. */
  tabCount: () => number;
}

export function useTabBarScroll(opts: TabScrollOptions): TabScrollComposable {
  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);

  function hiddenTabsBadge(side: 'left' | 'right'): 'bell' | 'output' | null {
    const container = opts.tabsContainerEl();
    if (!container) return null;
    const containerRect = container.getBoundingClientRect();
    let hasBell = false;
    let hasOutput = false;
    const tabEls = container.querySelectorAll<HTMLElement>('[data-tab-id]');
    for (const el of tabEls) {
      const rect = el.getBoundingClientRect();
      const isHiddenLeft = rect.right <= containerRect.left;
      const isHiddenRight = rect.left >= containerRect.right;
      if (side === 'left' ? isHiddenLeft : isHiddenRight) {
        const tabId = el.getAttribute('data-tab-id');
        const tab = opts.sortedTabs().find((t) => t.id === tabId);
        if (!tab) continue;
        const notif = opts.tabNotification(tab);
        if (notif?.type === 'bell') hasBell = true;
        else if (notif?.type === 'backgroundOutput') hasOutput = true;
      }
    }
    if (hasBell) return 'bell';
    if (hasOutput) return 'output';
    return null;
  }

  const leftBadge = $derived(canScrollLeft ? hiddenTabsBadge('left') : null);
  const rightBadge = $derived(canScrollRight ? hiddenTabsBadge('right') : null);

  function updateScrollState() {
    const container = opts.tabsContainerEl();
    const bar = opts.tabBarEl();
    const newTabBtn = opts.newTabBtnEl();
    if (!container || !container.isConnected) return;
    if (!bar || !newTabBtn) return;

    const { scrollLeft, scrollWidth } = container;
    const tabBarWidth = bar.clientWidth;
    const newTabBtnWidth = newTabBtn.offsetWidth;

    // See comment in original TabBar.svelte → updateScrollState for the
    // WebKitGTK rationale behind using tabBarEl.clientWidth.
    const totalTabsSpace = tabBarWidth - newTabBtnWidth;
    const hasOverflow = scrollWidth > totalTabsSpace + 2;

    const visibleTabsWidth =
      totalTabsSpace -
      (canScrollLeft ? SCROLL_ARROW_WIDTH : 0) -
      (canScrollRight ? SCROLL_ARROW_WIDTH : 0);

    canScrollLeft = hasOverflow && scrollLeft > 1;
    canScrollRight = hasOverflow && scrollLeft + visibleTabsWidth < scrollWidth - 1;
  }

  function scrollActiveTabIntoView() {
    const container = opts.tabsContainerEl();
    if (!container) return;
    const activeEl = container.querySelector<HTMLElement>(`[data-tab-id="${opts.activeTabId()}"]`);
    if (!activeEl) return;
    activeEl.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' });
  }

  function scrollTabs(direction: 'left' | 'right') {
    const container = opts.tabsContainerEl();
    if (!container) return;
    const SCROLL_STEP = 120;
    container.scrollBy({
      left: direction === 'left' ? -SCROLL_STEP : SCROLL_STEP,
      behavior: 'smooth',
    });
  }

  let resizeObserver: ResizeObserver | null = null;

  onMount(() => {
    const bar = opts.tabBarEl();
    const container = opts.tabsContainerEl();
    if (bar && container) {
      resizeObserver = new ResizeObserver(() => updateScrollState());
      resizeObserver.observe(bar);
      updateScrollState();
    }
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
  });

  // Re-check overflow whenever the tab count changes (tab added/removed).
  // Two-path scheduling: rAF (exact, post-layout) + setTimeout (WebKitGTK E2E fallback).
  $effect(() => {
    const _len = opts.tabCount();
    let done = false;
    const rafId = requestAnimationFrame(() => {
      if (!done) {
        done = true;
        updateScrollState();
        scrollActiveTabIntoView();
      }
    });
    const timeoutId = setTimeout(() => {
      if (!done) {
        done = true;
        updateScrollState();
        scrollActiveTabIntoView();
      }
      cancelAnimationFrame(rafId);
    }, 200);
    return () => {
      cancelAnimationFrame(rafId);
      clearTimeout(timeoutId);
    };
  });

  return {
    get canScrollLeft() {
      return canScrollLeft;
    },
    get canScrollRight() {
      return canScrollRight;
    },
    get leftBadge() {
      return leftBadge;
    },
    get rightBadge() {
      return rightBadge;
    },
    scrollTabs,
    scrollActiveTabIntoView,
    updateScrollState,
  };
}
