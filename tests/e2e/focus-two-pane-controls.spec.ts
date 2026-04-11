// SPDX-License-Identifier: MPL-2.0

/**
 * focus-two-pane-controls.spec.ts
 *
 * Non-regression E2E tests for UI control buttons after split-pane creation.
 *
 * Bug: after creating a second pane (Ctrl+Shift+D), buttons in the tab-row
 * (fullscreen, SSH, new-tab) became unresponsive due to a race condition in
 * the onviewportactive callback resetting activeViewportEl to null.
 *
 * Fix: TerminalView.svelte — onviewportactive callback.
 *
 * Covered:
 *   TEST-FOCUS-E2E-SPLIT-001 — fullscreen button works after split
 *   TEST-FOCUS-E2E-SPLIT-002 — SSH button opens connection manager after split
 *   TEST-FOCUS-E2E-SPLIT-003 — new-tab button creates a tab after split
 *   TEST-FOCUS-E2E-SPLIT-004 — SSH open+close then fullscreen works (sequential actions)
 *
 * Build requirement:
 *   The binary MUST be built with --features e2e-testing:
 *     pnpm tauri build --no-bundle -- --features e2e-testing
 */

import { browser, $, $$ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Count terminal panes currently rendered in the DOM.
 */
async function countPanes(): Promise<number> {
  const panes = await $$(Selectors.terminalPane);
  return panes.length;
}

/**
 * Count tab-bar tabs currently rendered in the DOM.
 */
async function countTabs(): Promise<number> {
  const tabs = await $$(Selectors.tab);
  return tabs.length;
}

/**
 * Wait for a specific number of panes to appear in the DOM.
 */
async function waitForPaneCount(expected: number, timeout = 8_000): Promise<void> {
  await browser.waitUntil(async () => (await countPanes()) === expected, {
    timeout,
    timeoutMsg: `Expected ${expected} pane(s) — got ${await countPanes()} after ${timeout}ms`,
  });
}

/**
 * Dispatch a keyboard shortcut directly via DOM events.
 * Bypasses potential WebDriver key-delivery quirks in WebKitGTK.
 */
async function dispatchShortcut(
  key: string,
  code: string,
  ctrlKey: boolean,
  shiftKey: boolean,
): Promise<void> {
  await browser.execute(
    function (keyArg: string, codeArg: string, ctrlArg: boolean, shiftArg: boolean): void {
      const grid = document.querySelector('.terminal-grid') as HTMLElement | null;
      const target = grid ?? document.body;
      target.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: keyArg,
          code: codeArg,
          ctrlKey: ctrlArg,
          shiftKey: shiftArg,
          bubbles: true,
          cancelable: true,
        }),
      );
    },
    key,
    code,
    ctrlKey,
    shiftKey,
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('UI controls remain functional after split pane creation', () => {
  /**
   * Reset to a known state before each test: exactly one pane on the active tab.
   *
   * Strategy: dispatch Ctrl+Shift+T to open a fresh tab (always yields 1 pane),
   * then wait until the pane count is 1. This mirrors the reset pattern used in
   * split-pane.spec.ts and is cheaper than trying to close extra panes.
   */
  beforeEach(async () => {
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

    await browser.waitUntil(async () => (await countPanes()) === 1, {
      timeout: 8_000,
      timeoutMsg: 'beforeEach: could not reset to single-pane state',
    });
  });

  /**
   * TEST-FOCUS-E2E-SPLIT-001: Fullscreen button works after split.
   *
   * After creating a second pane, clicking the fullscreen toggle button must
   * activate fullscreen (aria-pressed transitions false → true). The button
   * must be interactive — the regression caused it to be unresponsive.
   */
  it('TEST-FOCUS-E2E-SPLIT-001: fullscreen button is clickable and toggles after split', async () => {
    // Give focus to the terminal grid before the shortcut
    await $(Selectors.terminalGrid).click();

    // Create the second pane: Ctrl+Shift+D (split_pane_h)
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    // The button must start in its non-fullscreen state
    const initialPressed = await $(Selectors.fullscreenToggleBtn).getAttribute('aria-pressed');
    expect(initialPressed).toBe('false');

    // Click the fullscreen toggle
    await $(Selectors.fullscreenToggleBtn).click();

    // aria-pressed must transition to 'true' — button was responsive
    await browser.waitUntil(
      async () => (await $(Selectors.fullscreenToggleBtn).getAttribute('aria-pressed')) === 'true',
      {
        timeout: 5_000,
        timeoutMsg: 'Fullscreen did not activate after split — button may be unresponsive',
      },
    );

    // Restore non-fullscreen state for subsequent tests
    await $(Selectors.fullscreenToggleBtn).click();
    await browser.waitUntil(
      async () => (await $(Selectors.fullscreenToggleBtn).getAttribute('aria-pressed')) === 'false',
      {
        timeout: 5_000,
        timeoutMsg: 'Could not exit fullscreen during cleanup',
      },
    );
  });

  /**
   * TEST-FOCUS-E2E-SPLIT-002: SSH button opens connection manager after split.
   *
   * After creating a second pane, clicking the SSH button must open the
   * connection manager panel. The regression caused the button to be silently
   * non-functional.
   */
  it('TEST-FOCUS-E2E-SPLIT-002: SSH button opens connection manager after split', async () => {
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    // Click the SSH button
    await $(Selectors.sshButton).click();

    // The connection manager panel must appear
    await browser.waitUntil(async () => $(Selectors.connectionManager).isExisting(), {
      timeout: 5_000,
      timeoutMsg: 'Connection manager did not open after split — SSH button may be unresponsive',
    });

    // Cleanup: close the panel by clicking SSH button again
    await $(Selectors.sshButton).click();
  });

  /**
   * TEST-FOCUS-E2E-SPLIT-003: New-tab button creates a tab after split.
   *
   * After creating a second pane, clicking the new-tab button must add a new
   * tab. The regression caused tab-row buttons to become unresponsive.
   */
  it('TEST-FOCUS-E2E-SPLIT-003: new-tab button creates a tab after split', async () => {
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    const tabsBefore = await countTabs();

    // Click the new-tab button
    await $(Selectors.newTabButton).click();

    // A new tab must appear in the tab bar
    await browser.waitUntil(async () => (await countTabs()) === tabsBefore + 1, {
      timeout: 8_000,
      timeoutMsg: `New tab was not created after split — tab count stayed at ${tabsBefore}`,
    });
  });

  /**
   * TEST-FOCUS-E2E-SPLIT-004: SSH open+close then fullscreen works (sequential actions).
   *
   * Verifies that multiple sequential interactions with tab-row controls all
   * work correctly after a split: open SSH panel, close it, then activate
   * fullscreen. Any re-introduction of the focus regression would break the
   * fullscreen step.
   */
  it('TEST-FOCUS-E2E-SPLIT-004: sequential SSH open+close then fullscreen works after split', async () => {
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    // Step 1: open SSH connection manager
    await $(Selectors.sshButton).click();
    await browser.waitUntil(async () => $(Selectors.connectionManager).isExisting(), {
      timeout: 5_000,
      timeoutMsg: 'Connection manager did not open (step 1 of sequential test)',
    });

    // Step 2: close SSH connection manager
    await $(Selectors.sshButton).click();
    await browser.waitUntil(
      async () => !(await $(Selectors.connectionManager).isExisting()),
      {
        timeout: 5_000,
        timeoutMsg: 'Connection manager did not close (step 2 of sequential test)',
      },
    );

    // Step 3: activate fullscreen — button must still be responsive
    await $(Selectors.fullscreenToggleBtn).click();
    await browser.waitUntil(
      async () => (await $(Selectors.fullscreenToggleBtn).getAttribute('aria-pressed')) === 'true',
      {
        timeout: 5_000,
        timeoutMsg: 'Fullscreen did not activate after SSH close (step 3 of sequential test)',
      },
    );

    // Cleanup: exit fullscreen
    await $(Selectors.fullscreenToggleBtn).click();
    await browser.waitUntil(
      async () => (await $(Selectors.fullscreenToggleBtn).getAttribute('aria-pressed')) === 'false',
      {
        timeout: 5_000,
        timeoutMsg: 'Could not exit fullscreen during cleanup',
      },
    );
  });
});
