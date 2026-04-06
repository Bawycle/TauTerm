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

import { browser, $, $$ } from "@wdio/globals";
import { Selectors } from "./helpers/selectors";

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
    return document.querySelector(".tab-bar__scroll-arrow--left") !== null;
  }) as Promise<boolean>;
}

/**
 * Return true if the right scroll arrow is present in the DOM.
 */
async function isRightArrowPresent(): Promise<boolean> {
  return browser.execute((): boolean => {
    return document.querySelector(".tab-bar__scroll-arrow--right") !== null;
  }) as Promise<boolean>;
}

/**
 * Return true if either scroll arrow is present in the DOM.
 */
async function isAnyArrowPresent(): Promise<boolean> {
  return browser.execute((): boolean => {
    return (
      document.querySelector(".tab-bar__scroll-arrow--left") !== null ||
      document.querySelector(".tab-bar__scroll-arrow--right") !== null
    );
  }) as Promise<boolean>;
}

/**
 * Open a new tab via Ctrl+Shift+T and wait for the tab count to increase.
 */
async function openNewTab(expectedCount: number): Promise<void> {
  await browser.execute((): void => {
    const grid = document.querySelector(".terminal-grid") as HTMLElement | null;
    const target = grid ?? document.body;
    target.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "T",
        code: "KeyT",
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
  // Allow the scroll-state timer (200 ms fallback in TabBar.svelte) to fire
  // and Svelte to flush the resulting DOM update.
  await browser.pause(250);
}

/**
 * Close the currently active tab via Ctrl+Shift+W + dialog confirmation.
 * Mirrors the pattern from tab-lifecycle.spec.ts.
 */
async function closeActiveTab(): Promise<void> {
  await browser.execute((): void => {
    const grid = document.querySelector(".terminal-grid") as HTMLElement | null;
    const target = grid ?? document.body;
    target.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "W",
        code: "KeyW",
        ctrlKey: true,
        shiftKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
  });

  // Dismiss the close-confirmation dialog (E2E build never emits processExited).
  await browser.waitUntil(
    () =>
      browser.execute((): boolean => {
        for (const btn of document.querySelectorAll("button")) {
          if ((btn.textContent ?? "").trim() === "Close anyway") return true;
        }
        return false;
      }),
    { timeout: 3_000, timeoutMsg: "Close confirmation dialog did not appear" },
  );
  await browser.execute((): void => {
    for (const btn of document.querySelectorAll("button")) {
      if ((btn.textContent ?? "").trim() === "Close anyway") {
        (btn as HTMLButtonElement).dispatchEvent(
          new MouseEvent("click", { bubbles: true, cancelable: true }),
        );
        break;
      }
    }
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
    const tab = document.querySelector<HTMLElement>(
      `.tab-bar__tab[data-tab-index='${idx}']`,
    );
    tab?.dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true }),
    );
  }, index);
  // Allow layout and scroll-state update after tab switch.
  await browser.pause(150);
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("TauTerm — TabBar scroll arrow visibility", () => {
  before(async () => {
    // Dismiss any lingering dialog from a previous spec.
    await browser.execute((): void => {
      for (const btn of document.querySelectorAll("button")) {
        if ((btn.textContent ?? "").trim() === "Cancel") {
          (btn as HTMLButtonElement).dispatchEvent(
            new MouseEvent("click", { bubbles: true, cancelable: true }),
          );
          return;
        }
      }
    });
    await browser.pause(200);

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
        timeoutMsg:
          "Active terminal pane not ready before scroll-arrow tests",
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
  it("TBTC-E2E-SCR-001: no scroll arrows with a single tab", async () => {
    const tabCount = await countTabs();
    expect(tabCount).toBe(1);

    // Allow layout to settle.
    await browser.pause(200);

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
  it("TBTC-E2E-SCR-002: no scroll arrows after adding a second tab", async () => {
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
  it("TBTC-E2E-SCR-003: no arrows after switching between tabs (regression)", async () => {
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
   * cause the right scroll arrow to appear.
   *
   * Each tab has min-width: 120px.  At 800px window width, after subtracting
   * the new-tab button (44px) and some padding, roughly 6 tabs will overflow.
   * We open tabs until the right arrow appears (or give up after 15 tabs).
   *
   * We check for the right arrow specifically because the scroll position
   * starts at 0 (leftmost), so only the right arrow should show initially.
   */
  it("TBTC-E2E-SCR-004: right arrow appears when tabs overflow", async () => {
    // Start from whatever tab count we have (3 from previous test).
    let currentCount = await countTabs();

    // Open tabs until the right arrow appears, up to a reasonable maximum.
    const MAX_TABS = 15;
    while (currentCount < MAX_TABS) {
      currentCount++;
      await openNewTab(currentCount);

      if (await isRightArrowPresent()) break;
    }

    // The right arrow must be visible at this point.
    expect(await isRightArrowPresent()).toBe(true);

    // The left arrow should NOT be visible because we have not scrolled yet
    // (scroll position is 0 = leftmost).
    expect(await isLeftArrowPresent()).toBe(false);
  });

  // -----------------------------------------------------------------------
  // TBTC-E2E-SCR-005: Left arrow appears after scrolling right
  // -----------------------------------------------------------------------

  /**
   * TBTC-E2E-SCR-005: Clicking the right scroll arrow shifts the tab strip
   * and the left scroll arrow must appear.
   *
   * After clicking the right arrow, scrollLeft > 0, which satisfies the
   * canScrollLeft condition.  Both arrows should be visible (there are tabs
   * hidden on both sides).
   */
  it("TBTC-E2E-SCR-005: left arrow appears after scrolling right", async () => {
    // Prerequisite: the right arrow must be present (from TBTC-E2E-SCR-004).
    expect(await isRightArrowPresent()).toBe(true);

    // Click the right scroll arrow via DOM dispatchEvent.
    await browser.execute((): void => {
      const arrow = document.querySelector<HTMLElement>(
        ".tab-bar__scroll-arrow--right",
      );
      arrow?.dispatchEvent(
        new MouseEvent("click", { bubbles: true, cancelable: true }),
      );
    });

    // Wait for the smooth scroll to complete and the left arrow to appear.
    await browser.waitUntil(
      () => isLeftArrowPresent(),
      {
        timeout: 3_000,
        timeoutMsg:
          "Left scroll arrow did not appear after clicking the right arrow",
      },
    );

    expect(await isLeftArrowPresent()).toBe(true);
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
  it("TBTC-E2E-SCR-006: arrows disappear when tabs no longer overflow", async () => {
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

    // Allow layout to settle after the last tab removal.
    await browser.pause(200);

    // With only 2 tabs, both arrows must be absent.
    expect(await isLeftArrowPresent()).toBe(false);
    expect(await isRightArrowPresent()).toBe(false);
  });
});
