// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Terminal viewport auto-focus (FS-UX-003).
 *
 * Verifies that the active terminal viewport receives keyboard focus
 * automatically — without a mouse click — in three situations:
 *   1. On application launch.
 *   2. When a new tab is created via Ctrl+Shift+T.
 *   3. When the user switches tabs by clicking the tab bar.
 *
 * The focus assertion checks that document.activeElement carries the class
 * "terminal-grid" (the viewport div has tabindex=0 when active and bears both
 * "terminal-pane__viewport" and "terminal-grid" class names).
 *
 * These tests are intentionally written RED: they will fail until FS-UX-003
 * is implemented (auto-focus logic in the Svelte frontend and/or Tauri setup).
 *
 * Execution order note:
 * WebdriverIO runs specs alphabetically in a single app session.
 * "focus-autofocus" sorts after "app" and before "pty-roundtrip", so this
 * suite runs with exactly one tab open (state left by app.spec.ts).
 *
 * Protocol references:
 * - TEST-FOCUS-001 through TEST-FOCUS-003 (new scenarios — FS-UX-003)
 * - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.4
 * - docs/FS.md — FS-UX-003
 *
 * Implementation note:
 * Focus checks use browser.execute (synchronous script) rather than
 * browser.executeAsync to avoid the tauri-driver/WebKitGTK done-callback
 * stall that affects async scripts.
 */

import { browser, $, $$ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// IPC helpers (same fire-and-forget pattern as other specs)
// ---------------------------------------------------------------------------

/**
 * Fire a Tauri IPC command without waiting for its return value.
 * Safe for inject_pty_output and similar fire-and-forget commands.
 */
function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as any).__TAURI_INTERNALS__.invoke(cmdArg, argsArg);
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

// ---------------------------------------------------------------------------
// Focus assertion helper
// ---------------------------------------------------------------------------

/**
 * Return true if document.activeElement is the active terminal viewport.
 *
 * The viewport element carries both "terminal-pane__viewport" and
 * "terminal-grid" class names when it is the active pane's focusable div.
 * We treat either class as sufficient evidence that the correct element is
 * focused (guards against future class-name refactors breaking only one).
 */
function isViewportFocused(): Promise<boolean> {
  return browser.execute((): boolean => {
    const el = document.activeElement;
    if (el === null || el === document.body) return false;
    return (
      el.classList.contains('terminal-grid') ||
      el.classList.contains('terminal-pane__viewport') ||
      el.classList.contains('terminal-pane__input')
    );
  }) as Promise<boolean>;
}

/**
 * Wait until the active terminal viewport holds keyboard focus.
 *
 * Polls isViewportFocused() with a short interval.  The timeout is kept
 * tight (2 s) because auto-focus must happen immediately on the triggering
 * event — a slow focus would be a UX defect even if it eventually arrived.
 */
async function waitForViewportFocus(timeoutMs = 2_000): Promise<void> {
  await browser.waitUntil(isViewportFocused, {
    timeout: timeoutMs,
    interval: 50,
    timeoutMsg: `Terminal viewport did not receive focus within ${timeoutMs} ms`,
  });
}

// ---------------------------------------------------------------------------
// Tab-management helpers (close extra tabs, leaving exactly one)
// ---------------------------------------------------------------------------

async function countTabs(): Promise<number> {
  return (await $$(Selectors.tab)).length;
}

/**
 * Close the active tab via Ctrl+Shift+W, handling both possible outcomes:
 *
 *   1. Shell idle (InjectablePtySession with has_foreground_process = false):
 *      the tab closes immediately with no dialog.
 *   2. Active foreground process: a confirmation dialog appears; we confirm it.
 *
 * Waits up to 3 s for either the dialog to appear OR the tab count to
 * decrease, then confirms the dialog if it is present.  This makes the helper
 * resilient to both E2E configurations without assuming which one is active.
 */
async function closeTabViaKeyboard(): Promise<void> {
  const tabCountBefore = await countTabs();

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

  // Wait for either the confirmation dialog to appear (foreground process
  // present) or the tab count to decrease (shell idle, direct close).
  await browser.waitUntil(
    async () => {
      const dialogPresent = await browser.execute((): boolean => {
        return document.querySelector('[data-testid="close-confirm-action"]') !== null;
      });
      if (dialogPresent) return true;
      return (await countTabs()) < tabCountBefore;
    },
    { timeout: 3_000, timeoutMsg: 'Tab did not close and no confirmation dialog appeared' },
  );

  // If a dialog appeared, confirm it.
  await browser.execute((): void => {
    const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-action"]');
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  });
}

/**
 * Ensure exactly one tab is open.  Closes surplus tabs one by one.
 * Called in beforeAll to establish a clean baseline regardless of which
 * spec ran before this suite.
 */
async function ensureOneTab(): Promise<void> {
  let n = await countTabs();
  while (n > 1) {
    await closeTabViaKeyboard();
    await browser.waitUntil(async () => (await countTabs()) < n, {
      timeout: 5_000,
      timeoutMsg: `Tab count did not decrease from ${n}`,
    });
    n = await countTabs();
  }
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Terminal viewport auto-focus (FS-UX-003)', () => {
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

    // Wait for the active terminal pane to be present and interactive.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 10_000, timeoutMsg: 'Active terminal pane not ready before focus tests' },
    );
  });

  after(async () => {
    // Leave exactly one tab open so pty-roundtrip.spec.ts starts clean.
    await ensureOneTab();
  });

  // -------------------------------------------------------------------------
  // TEST-FOCUS-001: Focus on launch
  // -------------------------------------------------------------------------

  /**
   * TEST-FOCUS-001: On application launch, the active terminal viewport has
   * keyboard focus without any user interaction.
   *
   * The viewport div must be document.activeElement immediately (or within
   * 2 s of the pane being rendered) so that the user can type in the terminal
   * as soon as the window appears.
   *
   * Acceptance criterion (FS-UX-003 §1):
   *   The active viewport MUST have focus at launch without any click.
   *
   * Intentionally RED before FS-UX-003 implementation.
   */
  it('TEST-FOCUS-001: viewport is focused at launch without user interaction', async () => {
    // At this point we have not clicked anything — the browser was opened by
    // tauri-driver and app.spec.ts only checked window title and .app-shell.
    // Any focus that exists must have been set programmatically.
    await waitForViewportFocus();
    expect(await isViewportFocused()).toBe(true);
  });

  // -------------------------------------------------------------------------
  // TEST-FOCUS-002: Focus after Ctrl+Shift+T (new tab)
  // -------------------------------------------------------------------------

  /**
   * TEST-FOCUS-002: After opening a new tab with Ctrl+Shift+T, the new tab's
   * terminal viewport receives keyboard focus automatically.
   *
   * The user must be able to type in the new terminal immediately, without
   * having to click the viewport first.
   *
   * Acceptance criterion (FS-UX-003 §2):
   *   After Ctrl+Shift+T the new viewport MUST be focused.
   *
   * Intentionally RED before FS-UX-003 implementation.
   *
   * Note: we open a tab, assert focus, then close it in afterEach so the
   * next test starts from a one-tab state.
   */
  it('TEST-FOCUS-002: viewport is focused after Ctrl+Shift+T (new tab)', async () => {
    // Move focus to the active tab button (not the viewport) — this lets
    // browser.keys deliver the shortcut while keeping the terminal unfocused,
    // so the auto-focus on the NEW tab is the only thing that can pass the assertion.
    await $(Selectors.activeTab).click();
    expect(await isViewportFocused()).toBe(false);

    // Open a new tab.
    await browser.keys(['Control', 'Shift', 't']);

    await browser.waitUntil(async () => (await countTabs()) === 2, {
      timeout: 5_000,
      timeoutMsg: 'Second tab did not appear after Ctrl+Shift+T',
    });

    // Auto-focus must kick in without any further user action.
    await waitForViewportFocus();
    expect(await isViewportFocused()).toBe(true);

    // Clean up: close the extra tab so TEST-FOCUS-003 starts with 2 tabs for
    // a meaningful switch, created fresh inside that test.
    await closeTabViaKeyboard();
    await browser.waitUntil(async () => (await countTabs()) === 1, {
      timeout: 5_000,
      timeoutMsg: 'Extra tab was not closed after TEST-FOCUS-002',
    });
  });

  // -------------------------------------------------------------------------
  // TEST-FOCUS-003: Focus after tab-bar click (tab switch)
  // -------------------------------------------------------------------------

  /**
   * TEST-FOCUS-003: After clicking a tab in the tab bar, the newly active
   * terminal viewport receives keyboard focus automatically.
   *
   * Clicking a tab item is a navigation action — focus should move to the
   * terminal content, not stay on the tab button.
   *
   * Acceptance criterion (FS-UX-003 §3):
   *   After a tab-bar click the newly active viewport MUST be focused.
   *
   * Intentionally RED before FS-UX-003 implementation.
   */
  it('TEST-FOCUS-003: viewport is focused after clicking a tab in the tab bar', async () => {
    // Wait for the UI to fully settle after the previous test's tab close + dialog.
    await browser.waitUntil(async () => (await countTabs()) === 1, {
      timeout: 5_000,
      timeoutMsg: 'Expected 1 tab at start of TEST-FOCUS-003',
    });

    // Setup: focus the active tab button so browser.keys can deliver Ctrl+Shift+T.
    // Use dispatchEvent — WebKitGTK marks the tab button as non-interactable
    // briefly after the auto-focus $effect fires (same issue as in TEST-FOCUS-002).
    await browser.execute((): void => {
      const tab = document.querySelector<HTMLElement>('.tab-bar__tab--active');
      tab?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    await browser.keys(['Control', 'Shift', 't']);
    await browser.waitUntil(async () => (await countTabs()) === 2, {
      timeout: 5_000,
      timeoutMsg: 'Second tab did not appear for TEST-FOCUS-003 setup',
    });

    // Now click tab 0 (the first tab) in the tab bar.  We are currently on
    // tab 1 (the newly created one), so this is a genuine tab switch.
    // Use a native dispatchEvent — WebKitGTK marks the element as "not
    // interactable" for a brief window after the auto-focus $effect fires
    // (same workaround used in tab-lifecycle.spec.ts for Ctrl+Shift+W).
    await browser.execute((): void => {
      const tab = document.querySelector<HTMLElement>(".tab-bar__tab[data-tab-index='0']");
      tab?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Wait for the switch to complete — the first tab must be active.
    await browser.waitUntil(
      async () => {
        const activeTab = await $(Selectors.activeTab);
        return (await activeTab.getAttribute('data-tab-index')) === '0';
      },
      { timeout: 5_000, timeoutMsg: 'Tab 0 did not become active after click' },
    );

    // The click was on the tab button, not the viewport — auto-focus must
    // transfer focus to the viewport without further user action.
    await waitForViewportFocus();
    expect(await isViewportFocused()).toBe(true);

    // Clean up: close the extra tab.
    await closeTabViaKeyboard();
    await browser.waitUntil(async () => (await countTabs()) === 1, {
      timeout: 5_000,
      timeoutMsg: 'Extra tab was not closed after TEST-FOCUS-003',
    });
  });
});
