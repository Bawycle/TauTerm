// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Tab create and close lifecycle.
 *
 * Verifies that the tab management UI (TabBar) correctly reflects the session
 * state when tabs are created and closed via keyboard shortcuts.
 *
 * DEFERRED: build required — these tests require a working build with the
 * `create_tab` IPC command wired to a real PTY backend. Tab creation and
 * the tab bar rendering must both be operational.
 *
 * Protocol references:
 * - TEST-TAB-001 through TEST-TAB-003
 * - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.3
 * - FS-TAB-001 (open tab), FS-TAB-003 (close tab), FS-TAB-004 (tab count)
 */

import { browser, $, $$ } from "@wdio/globals";

/** Helper: count the number of tab elements currently in the DOM. */
async function countTabs(): Promise<number> {
  const tabs = await $$(".tab-bar .tab");
  return tabs.length;
}

describe("TauTerm — Tab create and close lifecycle", () => {
  /**
   * TEST-TAB-E2E-001: Application starts with exactly one tab.
   *
   * On first launch, the tab bar must show exactly one tab in active state.
   * No additional tabs are created by default.
   */
  it("starts with exactly one tab", async () => {
    const tabBar = await $(".tab-bar");
    await expect(tabBar).toExist();

    const tabCount = await countTabs();
    expect(tabCount).toBe(1);

    // The single tab must be in the active state.
    const activeTab = await $(".tab-bar .tab[aria-selected='true']");
    await expect(activeTab).toExist();
  });

  /**
   * TEST-TAB-E2E-002: Ctrl+Shift+T opens a new tab.
   *
   * After pressing the new-tab shortcut, the tab bar must show a second tab
   * and the second tab must become active.
   *
   * [DEFERRED: requires create_tab IPC + PTY spawn]
   */
  it("creates a second tab with Ctrl+Shift+T", async () => {
    // Open a new tab via keyboard shortcut (FS-TAB-001 / FS-KBD-002).
    await browser.keys(["Control", "Shift", "t"]);

    // Wait for a second tab to appear in the tab bar.
    await browser.waitUntil(
      async () => (await countTabs()) === 2,
      {
        timeout: 5_000,
        timeoutMsg:
          "Second tab did not appear within 5 seconds after Ctrl+Shift+T " +
          "[DEFERRED: requires create_tab IPC implementation]",
      }
    );

    // The new tab must be active.
    const activeTab = await $(".tab-bar .tab[aria-selected='true']");
    const tabIndex = await activeTab.getAttribute("data-tab-index");
    expect(Number(tabIndex)).toBe(1); // second tab (0-indexed)
  });

  /**
   * TEST-TAB-E2E-003: Ctrl+Shift+W closes the active tab.
   *
   * After pressing the close-tab shortcut on the second tab, the tab bar must
   * revert to one tab and the first tab must become active again.
   *
   * Depends on TEST-TAB-E2E-002 having opened a second tab.
   *
   * [DEFERRED: requires close_tab IPC]
   */
  it("closes the active tab with Ctrl+Shift+W and reverts to one tab", async () => {
    // Ensure we start with 2 tabs (continuation from previous test).
    // In isolation, create a tab first.
    if ((await countTabs()) < 2) {
      await browser.keys(["Control", "Shift", "t"]);
      await browser.waitUntil(async () => (await countTabs()) === 2, {
        timeout: 5_000,
        timeoutMsg: "Could not create second tab for close test setup",
      });
    }

    // Close the active tab.
    await browser.keys(["Control", "Shift", "w"]);

    // Wait for the tab count to drop back to one.
    await browser.waitUntil(
      async () => (await countTabs()) === 1,
      {
        timeout: 5_000,
        timeoutMsg:
          "Tab count did not return to 1 within 5 seconds after Ctrl+Shift+W " +
          "[DEFERRED: requires close_tab IPC implementation]",
      }
    );

    // The remaining tab must be active.
    const activeTab = await $(".tab-bar .tab[aria-selected='true']");
    await expect(activeTab).toExist();
  });

  /**
   * TEST-TAB-E2E-004: Closing the last tab does not crash the application.
   *
   * Pressing Ctrl+Shift+W when only one tab is open must either:
   * (a) Do nothing (tab is protected — per FS-TAB-003), or
   * (b) Prompt the user before closing.
   * The application must remain open and responsive.
   *
   * [DEFERRED: requires close_tab IPC + FS-TAB-003 guard]
   */
  it("does not crash when attempting to close the last tab", async () => {
    // Ensure we are down to one tab.
    await browser.waitUntil(
      async () => (await countTabs()) === 1,
      { timeout: 3_000, timeoutMsg: "Expected 1 tab before last-tab close test" }
    );

    // Attempt to close the only tab.
    await browser.keys(["Control", "Shift", "w"]);

    // Wait a moment for any crash to manifest.
    await browser.pause(500);

    // Application must still be responsive — window title is still correct.
    const title = await browser.getTitle();
    expect(title).toBe("tau-term");

    // The tab bar must still show at least one tab.
    const tabCount = await countTabs();
    expect(tabCount).toBeGreaterThanOrEqual(1);
  });
});
