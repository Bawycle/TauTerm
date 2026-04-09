// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: TabBar scroll arrow visibility.
 *
 * Validates that the horizontal scroll arrows (ChevronLeft / ChevronRight)
 * appear only when the tab strip genuinely overflows the available width,
 * and disappear when it does not.
 *
 * This suite exists because the overflow detection relies on real CSS layout
 * measurements (scrollWidth, clientWidth, getBoundingClientRect) that jsdom
 * cannot replicate.  Only a real browser (WebKitGTK via tauri-driver) can
 * exercise the actual layout and trigger the bug where arrows appeared
 * spuriously after tab switches.
 *
 * Protocol references:
 * - TBTC-E2E-SCR-001 through TBTC-E2E-SCR-006
 * - UXD section 6.2, section 12.2 (scroll arrow behaviour)
 *
 * Execution order note:
 * WebdriverIO runs specs alphabetically in a single app session.
 * "tab-bar-scroll-arrows" sorts after "tab-bar" and before "tab-lifecycle",
 * so this suite starts from whatever state the preceding spec left.
 * The before() hook normalises to a single-tab baseline.
 *
 * Note on arrow detection:
 * The scroll arrows are conditionally rendered ({#if canScrollLeft} / {#if
 * canScrollRight}), so they are absent from the DOM when not needed.
 * We assert non-existence via isExisting() === false, not visibility.
 */

import { browser, $, $$ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// DOM helpers
// ---------------------------------------------------------------------------

/** Count the number of tab elements currently in the DOM. */
async function countTabs(): Promise<number> {
  return (await $$(Selectors.tab)).length;
}

/**
 * Return true if the left scroll arrow is present in the DOM.
 * The arrow is conditionally rendered, so absent === no overflow to the left.
 */
async function isLeftArrowPresent(): Promise<boolean> {
  return browser.execute((): boolean => {
    return document.querySelector('.tab-bar__scroll-arrow--left') !== null;
  }) as Promise<boolean>;
}

/**
 * Return true if the right scroll arrow is present in the DOM.
 */
async function isRightArrowPresent(): Promise<boolean> {
  return browser.execute((): boolean => {
    return document.querySelector('.tab-bar__scroll-arrow--right') !== null;
  }) as Promise<boolean>;
}

/**
 * Return true if either scroll arrow is present in the DOM.
 */
async function isAnyArrowPresent(): Promise<boolean> {
  return browser.execute((): boolean => {
    return (
      document.querySelector('.tab-bar__scroll-arrow--left') !== null ||
      document.querySelector('.tab-bar__scroll-arrow--right') !== null
    );
  }) as Promise<boolean>;
}

/**
 * Wait for the tab bar scroll state to settle.
 *
 * TabBar.svelte updates `canScrollLeft` / `canScrollRight` in response to
 * scroll and resize events via a 200 ms debounce/fallback timer. Rather than
 * sleeping for a fixed duration we poll until the arrow presence is stable
 * across two consecutive observations (i.e. the scroll-state timer has fired
 * and Svelte has flushed the resulting DOM update).
 */
async function waitForScrollStateStable(): Promise<void> {
  await browser.waitUntil(
    async () => {
      const state1 = await isAnyArrowPresent();
      // Check again after a short interval to confirm the state is stable.
      await new Promise((r) => setTimeout(r, 60));
      const state2 = await isAnyArrowPresent();
      return state1 === state2;
    },
    {
      timeout: 1_500,
      interval: 80,
      timeoutMsg: 'Tab bar scroll state did not stabilise within 1.5 s',
    },
  );
}

/**
 * Open a new tab via Ctrl+Shift+T and wait for the tab count to increase.
 */
async function openNewTab(expectedCount: number): Promise<void> {
  await browser.execute((): void => {
    const grid = document.querySelector('.terminal-grid') as HTMLElement | null;
    const target = grid ?? document.body;
    target.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'T',
        code: 'KeyT',
        ctrlKey: true,
        shiftKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await browser.waitUntil(async () => (await countTabs()) === expectedCount, {
    timeout: 5_000,
    timeoutMsg: `Tab count did not reach ${expectedCount} after Ctrl+Shift+T`,
  });
  // Wait for the scroll-state timer (200 ms fallback in TabBar.svelte) to fire
  // and for Svelte to flush the resulting DOM update.
  await waitForScrollStateStable();
}

/**
 * Close the currently active tab via Ctrl+Shift+W.
 *
 * Handles both cases:
 *   - PTY tabs: a confirmation dialog appears and must be confirmed.
 *   - SSH tabs (no PTY process): the tab closes directly without a dialog.
 */
async function closeActiveTab(): Promise<void> {
  const tabsBefore = await browser.execute(
    (): number => document.querySelectorAll('.tab-bar__tab').length,
  );

  await browser.execute((): void => {
    const grid = document.querySelector('.terminal-grid') as HTMLElement | null;
    const target = grid ?? document.body;
    target.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'W',
        code: 'KeyW',
        ctrlKey: true,
        shiftKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
  });

  // Wait for either the confirmation dialog OR the tab to disappear directly.
  await browser.waitUntil(
    () =>
      browser.execute(
        (before: number): boolean =>
          document.querySelector('[data-testid="close-confirm-action"]') !== null ||
          document.querySelectorAll('.tab-bar__tab').length < before,
        tabsBefore,
      ),
    { timeout: 3_000, timeoutMsg: 'Tab did not close and no confirmation dialog appeared' },
  );

  // Confirm via dialog if present.
  await browser.execute((): void => {
    const btn = document.querySelector<HTMLButtonElement>(
      '[data-testid="close-confirm-action"]',
    );
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  });
}

/**
 * Ensure exactly one tab is open. Closes surplus tabs one by one.
 */
async function ensureOneTab(): Promise<void> {
  let n = await countTabs();
  while (n > 1) {
    await closeActiveTab();
    await browser.waitUntil(async () => (await countTabs()) < n, {
      timeout: 5_000,
      timeoutMsg: `Tab count did not decrease from ${n}`,
    });
    n = await countTabs();
  }
}

/**
 * Click a tab by its 0-based index.
 * Uses dispatchEvent to avoid WebKitGTK interactability timing issues.
 */
async function clickTabByIndex(index: number): Promise<void> {
  await browser.execute((idx: number): void => {
    const tab = document.querySelector<HTMLElement>(`.tab-bar__tab[data-tab-index='${idx}']`);
    tab?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  }, index);
  // Wait for layout and scroll-state to settle after tab switch.
  await waitForScrollStateStable();
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — TabBar scroll arrow visibility', () => {
  before(async () => {
    // Dismiss any lingering dialog from a previous spec (locale-independent).
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-cancel"]');
      if (btn) {
        btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      }
    });
    // Wait for the dialog to be gone before proceeding.
    await browser.waitUntil(
      () =>
        browser.execute(function (): boolean {
          return document.querySelector('[data-testid="close-confirm-cancel"]') === null;
        }),
      { timeout: 3_000, timeoutMsg: "Close confirmation dialog did not disappear after dismiss" },
    );

    // Normalise to a single-tab state.
    await ensureOneTab();

    // Wait for the active terminal pane to be present.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      {
        timeout: 10_000,
        timeoutMsg: 'Active terminal pane not ready before scroll-arrow tests',
      },
    );
  });

  after(async () => {
    // Leave exactly one tab open for following specs.
    await ensureOneTab();
  });

  // -----------------------------------------------------------------------
  // TBTC-E2E-SCR-001: No arrows at startup with a single tab
  // -----------------------------------------------------------------------

  /**
   * TBTC-E2E-SCR-001: With a single tab, the tab bar must not show any scroll
   * arrows because there is no overflow.
   *
   * A single tab is always narrower than the window width, so both
   * canScrollLeft and canScrollRight must be false and the arrow buttons
   * must be absent from the DOM.
   */
  it('TBTC-E2E-SCR-001: no scroll arrows with a single tab', async () => {
    const tabCount = await countTabs();
    expect(tabCount).toBe(1);

    // Wait for layout to settle and scroll state to be computed.
    await waitForScrollStateStable();

    expect(await isLeftArrowPresent()).toBe(false);
    expect(await isRightArrowPresent()).toBe(false);
  });

  // -----------------------------------------------------------------------
  // TBTC-E2E-SCR-002: No arrows after adding a second tab (window is wide)
  // -----------------------------------------------------------------------

  /**
   * TBTC-E2E-SCR-002: Adding a second tab in a normally-sized window must
   * not trigger overflow arrows.
   *
   * Two tabs (min-width 120px each = 240px) plus the new-tab button (44px)
   * fit easily in any reasonable window width (>= 800px).  No arrows should
   * appear.
   */
  it('TBTC-E2E-SCR-002: no scroll arrows after adding a second tab', async () => {
    await openNewTab(2);

    expect(await isLeftArrowPresent()).toBe(false);
    expect(await isRightArrowPresent()).toBe(false);
  });

  // -----------------------------------------------------------------------
  // TBTC-E2E-SCR-003: No arrows after switching tabs (regression scenario)
  // -----------------------------------------------------------------------

  /**
   * TBTC-E2E-SCR-003: Switching between tabs that fit in the bar must NOT
   * cause scroll arrows to appear.
   *
   * This is the regression scenario for the bug where tab-switch triggered
   * a spurious overflow detection.  The root cause was measuring clientWidth
   * on the tabs container while the arrows were already stealing space from
   * the flex layout, creating a bistable feedback loop.
   *
   * With 2-3 tabs that easily fit the window, switching between them must
   * leave both arrows absent.
   */
  it('TBTC-E2E-SCR-003: no arrows after switching between tabs (regression)', async () => {
    // Ensure we have at least 2 tabs.
    if ((await countTabs()) < 2) {
      await openNewTab(2);
    }

    // Add a third tab to increase coverage.
    await openNewTab(3);

    // Switch to tab 0.
    await clickTabByIndex(0);
    expect(await isAnyArrowPresent()).toBe(false);

    // Switch to tab 1.
    await clickTabByIndex(1);
    expect(await isAnyArrowPresent()).toBe(false);

    // Switch to tab 2.
    await clickTabByIndex(2);
    expect(await isAnyArrowPresent()).toBe(false);

    // Switch back to tab 0 — one more round to catch delayed re-renders.
    await clickTabByIndex(0);
    expect(await isAnyArrowPresent()).toBe(false);
  });

  // -----------------------------------------------------------------------
  // TBTC-E2E-SCR-004: Arrows appear when tabs genuinely overflow
  // -----------------------------------------------------------------------

  /**
   * TBTC-E2E-SCR-004: Adding enough tabs to exceed the tab bar width must
   * cause scroll arrows to appear.
   *
   * Each tab has min-width: 120px.  At 800px window width, after subtracting
   * the new-tab button (44px) and some padding, roughly 6 tabs will overflow.
   * We open tabs until any scroll arrow appears (or give up after 15 tabs).
   *
   * Note on scroll position: when a new tab is added, the tab bar auto-scrolls
   * to keep the new (active) tab visible.  At overflow time the tab bar is
   * therefore scrolled to the rightmost position — only the LEFT arrow appears
   * (there is nothing further right to scroll to).  We therefore check for the
   * presence of ANY arrow rather than the right arrow specifically.
   */
  it('TBTC-E2E-SCR-004: scroll arrows appear when tabs overflow', async () => {
    // Start from whatever tab count we have (3 from previous test).
    let currentCount = await countTabs();

    // Open tabs until any arrow appears, up to a reasonable maximum.
    const MAX_TABS = 15;
    while (currentCount < MAX_TABS) {
      currentCount++;
      await openNewTab(currentCount);

      if (await isAnyArrowPresent()) break;
    }

    // At least one arrow must be visible — tabs genuinely overflow.
    expect(await isAnyArrowPresent()).toBe(true);

    // Auto-scroll brings the last tab into view: the bar is scrolled to the
    // right end, so the LEFT arrow (hidden tabs to the left) must be present.
    expect(await isLeftArrowPresent()).toBe(true);
  });

  // -----------------------------------------------------------------------
  // TBTC-E2E-SCR-005: Left arrow appears after scrolling right
  // -----------------------------------------------------------------------

  /**
   * TBTC-E2E-SCR-005: Scrolling the tab strip back to the start causes the
   * right scroll arrow to appear.
   *
   * After TBTC-E2E-SCR-004 the bar is auto-scrolled to the rightmost position
   * (left arrow present, right arrow absent).  Resetting scrollLeft to 0
   * means all overflowing content is to the right → only the right arrow shows.
   *
   * We set scrollLeft directly (instant, no animation) to avoid racing against
   * any residual smooth-scroll animation from the previous test.
   */
  it('TBTC-E2E-SCR-005: right arrow appears when scrolled back to the start', async () => {
    // Prerequisite: the left arrow must be present (from TBTC-E2E-SCR-004).
    expect(await isLeftArrowPresent()).toBe(true);

    // Reset scroll position to 0 instantly (bypasses smooth-scroll animation).
    // Dispatching the scroll event ensures updateScrollState() fires.
    await browser.execute((): void => {
      const tabs = document.querySelector<HTMLElement>('.tab-bar__tabs');
      if (!tabs) return;
      // Override scroll-behavior so the assignment is instant.
      tabs.style.scrollBehavior = 'auto';
      tabs.scrollLeft = 0;
      tabs.dispatchEvent(new Event('scroll'));
    });

    // Wait for Svelte to flush the DOM update and the scroll state to stabilise.
    await waitForScrollStateStable();

    // At scrollLeft=0 there is content to the right → right arrow shown.
    expect(await isRightArrowPresent()).toBe(true);
    // At scrollLeft=0 nothing is hidden to the left → left arrow absent.
    expect(await isLeftArrowPresent()).toBe(false);
  });

  // -----------------------------------------------------------------------
  // TBTC-E2E-SCR-006: Arrows disappear when tabs no longer overflow
  // -----------------------------------------------------------------------

  /**
   * TBTC-E2E-SCR-006: Closing enough tabs to eliminate overflow must cause
   * both scroll arrows to disappear.
   *
   * We close tabs until only 2 remain (guaranteed to fit the window width),
   * then assert that neither arrow is present.
   *
   * This catches bugs where the overflow state is only computed on scroll
   * events but not on tab removal.
   */
  it('TBTC-E2E-SCR-006: arrows disappear when tabs no longer overflow', async () => {
    // Close tabs until we have 2 left.
    let n = await countTabs();
    while (n > 2) {
      await closeActiveTab();
      await browser.waitUntil(async () => (await countTabs()) < n, {
        timeout: 5_000,
        timeoutMsg: `Tab count did not decrease from ${n}`,
      });
      n = await countTabs();
    }

    expect(n).toBe(2);

    // Wait for layout to settle after the last tab removal.
    await waitForScrollStateStable();

    // With only 2 tabs, both arrows must be absent.
    expect(await isLeftArrowPresent()).toBe(false);
    expect(await isRightArrowPresent()).toBe(false);
  });
});
