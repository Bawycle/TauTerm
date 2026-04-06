// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Tab create and close lifecycle.
 *
 * Verifies that the tab management UI (TabBar) correctly reflects the session
 * state when tabs are created and closed via keyboard shortcuts.
 *
 * Protocol references:
 * - TEST-TAB-001 through TEST-TAB-003
 * - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.3
 * - FS-TAB-001 (open tab), FS-TAB-003 (close tab), FS-TAB-004 (tab count)
 */

import { browser, $, $$ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

/** Helper: count the number of tab elements currently in the DOM. */
async function countTabs(): Promise<number> {
  const tabs = await $$(Selectors.tab);
  return tabs.length;
}

describe('TauTerm — Tab create and close lifecycle', () => {
  /**
   * TEST-TAB-E2E-001: Application starts with exactly one tab.
   *
   * On first launch, the tab bar must show exactly one tab in active state.
   * No additional tabs are created by default.
   */
  it('starts with exactly one tab', async () => {
    const tabBar = await $(Selectors.tabBar);
    await expect(tabBar).toExist();

    const tabCount = await countTabs();
    expect(tabCount).toBe(1);

    // The single tab must be in the active state.
    const activeTab = await $(Selectors.activeTab);
    await expect(activeTab).toExist();
  });

  /**
   * TEST-TAB-E2E-002: Ctrl+Shift+T opens a new tab.
   *
   * After pressing the new-tab shortcut, the tab bar must show a second tab
   * and the second tab must become active (data-tab-index="1", 0-indexed).
   */
  it('creates a second tab with Ctrl+Shift+T', async () => {
    // Give focus to the app window before sending keyboard shortcuts.
    await $(Selectors.terminalGrid).click();

    // Open a new tab via keyboard shortcut (FS-TAB-001 / FS-KBD-002).
    await browser.keys(['Control', 'Shift', 't']);

    // Wait for a second tab to appear in the tab bar.
    await browser.waitUntil(async () => (await countTabs()) === 2, {
      timeout: 5_000,
      timeoutMsg: 'Second tab did not appear within 5 seconds after Ctrl+Shift+T',
    });

    // The new tab must be active and carry the correct index attribute.
    const activeTab = await $(Selectors.activeTab);
    const tabIndex = await activeTab.getAttribute('data-tab-index');
    expect(tabIndex).toBe('1'); // second tab, 0-indexed
  });

  /**
   * TEST-TAB-E2E-003: Ctrl+Shift+W closes the active tab.
   *
   * After pressing the close-tab shortcut on the second tab, the tab bar must
   * revert to one tab and the first tab must become active again.
   *
   * Depends on TEST-TAB-E2E-002 having opened a second tab.
   */
  it('closes the active tab with Ctrl+Shift+W and reverts to one tab', async () => {
    // Ensure we start with 2 tabs (continuation from previous test).
    // In isolation, create a tab first.
    if ((await countTabs()) < 2) {
      await $(Selectors.terminalGrid).click();
      await browser.keys(['Control', 'Shift', 't']);
      await browser.waitUntil(async () => (await countTabs()) === 2, {
        timeout: 5_000,
        timeoutMsg: 'Could not create second tab for close test setup',
      });
    }

    // Dispatch Ctrl+Shift+W directly via the DOM to ensure the Svelte keydown
    // handler receives it, bypassing any WebDriver key-delivery quirks in
    // WebKitGTK (browser.keys() reliably triggers Ctrl+Shift+T but may not
    // reliably deliver Ctrl+Shift+W after a tab switch).
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

    // FS-PTY-008: InjectablePtyBackend never emits processExited, so the tab is
    // always considered to have an active process → the close confirmation dialog
    // will appear. Wait for the confirm button (via data-testid, locale-independent),
    // then dispatch a native click event so Svelte 5's event listeners fire correctly.
    await browser.waitUntil(
      async () => {
        return browser.execute((): boolean => {
          return document.querySelector('[data-testid="close-confirm-action"]') !== null;
        });
      },
      { timeout: 3_000, timeoutMsg: 'Close confirmation dialog did not appear within 3 s' },
    );
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-action"]');
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Wait for the tab count to drop back to one.
    await browser.waitUntil(async () => (await countTabs()) === 1, {
      timeout: 5_000,
      timeoutMsg: 'Tab count did not return to 1 within 5 seconds after Ctrl+Shift+W',
    });

    // The remaining tab must be active.
    const activeTab = await $(Selectors.activeTab);
    await expect(activeTab).toExist();
  });

  /**
   * TEST-TAB-E2E-004: Closing the last tab does not crash the application.
   *
   * Pressing Ctrl+Shift+W when only one tab is open must either:
   * (a) Do nothing (tab is protected — per FS-TAB-003), or
   * (b) Prompt the user before closing.
   * The application must remain open and responsive.
   */
  it('does not crash when attempting to close the last tab', async () => {
    // Ensure we are down to one tab.
    await browser.waitUntil(async () => (await countTabs()) === 1, {
      timeout: 3_000,
      timeoutMsg: 'Expected 1 tab before last-tab close test',
    });

    // Dispatch Ctrl+Shift+W via DOM to ensure the Svelte handler receives it.
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

    // Wait a moment for any crash to manifest.
    await browser.pause(500);

    // Application must still be responsive — window title is still correct.
    const title = await browser.getTitle();
    expect(title).toBe('tau-term');

    // The tab bar must still show at least one tab.
    const tabCount = await countTabs();
    expect(tabCount).toBeGreaterThanOrEqual(1);
  });
});
