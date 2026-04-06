// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Tab switch session isolation.
 *
 * Verifies that clicking a different tab replaces the terminal content with
 * that tab's session, and that switching back restores the original content.
 * Also verifies that the backend active_tab_id stays in sync with the UI.
 *
 * Regression test for the bug where:
 *   1. The SplitPane/TerminalPane was reused in-place on tab switch (no
 *      {#key activeTabId}), leaving the old session's content visible.
 *   2. The backend active_tab_id was never updated when switching tabs
 *      (missing set_active_tab IPC command).
 *
 * Protocol references:
 * - TEST-TAB-SW-001 through TEST-TAB-SW-003 (new scenarios)
 * - FS-TAB-002 (tab switch restores session state)
 * - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.3
 *
 * Note on inject_pty_output:
 * Uses fire-and-forget pattern (browser.execute, not browser.executeAsync)
 * to avoid the tauri-driver/WebKitGTK done-callback stall. DOM effects are
 * asserted via waitUntil instead.
 *
 * Note on tauriInvoke:
 * For commands that return a value, the result is stored in a keyed window
 * property and polled — same workaround for browser.executeAsync unreliability.
 */

import { browser, $, $$ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// IPC helpers
// ---------------------------------------------------------------------------

/**
 * Fire a Tauri IPC command without waiting for its return value.
 * Safe for inject_pty_output and other fire-and-forget commands.
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

/**
 * Invoke a Tauri IPC command and return its resolved value.
 *
 * Stores the result in a uniquely-keyed window property and polls for it,
 * bypassing the browser.executeAsync callback issues in tauri-driver.
 */
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const key = `__tauterm_result_${Date.now()}_${Math.random()}`;
  await browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined, keyArg: string) {
      (window as any).__TAURI_INTERNALS__.invoke(cmdArg, argsArg).then((result: unknown) => {
        (window as any)[keyArg] = result;
      });
    },
    cmd,
    args,
    key,
  );
  await browser.waitUntil(
    () => browser.execute((k: string) => Object.prototype.hasOwnProperty.call(window, k), key),
    { timeout: 5_000, timeoutMsg: `Tauri command "${cmd}" did not resolve within 5 s` },
  );
  return browser.execute((k: string) => (window as any)[k], key) as Promise<T>;
}

// ---------------------------------------------------------------------------
// DOM helpers
// ---------------------------------------------------------------------------

/** Encode a string to an array of UTF-8 bytes for inject_pty_output. */
function encodeBytes(s: string): number[] {
  return [...new TextEncoder().encode(s)];
}

/**
 * Wait until the terminal grid contains the given text.
 * Single browser.execute scan to avoid per-cell RPC overhead.
 */
async function waitForTextInGrid(text: string, timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((t: string): boolean => {
        const grid = document.querySelector('.terminal-grid');
        return grid !== null && (grid.textContent ?? '').includes(t);
      }, text),
    {
      timeout: timeoutMs,
      timeoutMsg: `"${text}" did not appear in the terminal grid within ${timeoutMs / 1_000} s`,
    },
  );
}

/** Read the full text content of the terminal grid in a single RPC call. */
async function getGridText(): Promise<string> {
  return browser.execute(
    (): string => document.querySelector('.terminal-grid')?.textContent ?? '',
  ) as Promise<string>;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Tab switch session isolation', () => {
  // Tab IDs captured during TEST-TAB-SW-001, reused for TEST-TAB-SW-003.
  let tab1Id = '';
  let tab2Id = '';

  before(async () => {
    // tab-lifecycle.spec.ts test 4 leaves the close-confirmation dialog open
    // (Ctrl+Shift+W on the last tab triggers it but the test does not dismiss it).
    // Dismiss it by clicking the Cancel button if present (locale-independent).
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-cancel"]');
      if (btn) {
        btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      }
    });
    await browser.pause(300);

    // Wait for the active terminal pane to be present and interactive.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 10_000, timeoutMsg: 'Active terminal pane not ready before tab-switch tests' },
    );
  });

  /**
   * TEST-TAB-SW-001: Switching to a new tab shows that tab's own content.
   *
   * Creates two tabs, injects distinct marker strings into each pane, then
   * verifies that only the active tab's content is visible in the grid.
   *
   * This also catches the case where auto-switching on tab creation was
   * broken (Ctrl+Shift+T should immediately show the new session).
   */
  it("shows new tab's own content immediately after Ctrl+Shift+T", async () => {
    // Step 1: capture tab 1's pane ID and inject a distinctive marker.
    const pane1Id = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(pane1Id).toBeTruthy();

    const marker1 = 'TAB1-SESSION-CONTENT';
    await tauriFireAndForget('inject_pty_output', {
      paneId: pane1Id,
      data: encodeBytes(marker1),
    });
    await waitForTextInGrid(marker1);

    // Step 2: open a second tab via Ctrl+Shift+T.
    await $(Selectors.terminalGrid).click();
    await browser.keys(['Control', 'Shift', 't']);
    await browser.waitUntil(async () => (await $$(Selectors.tab)).length === 2, {
      timeout: 5_000,
      timeoutMsg: 'Second tab did not appear after Ctrl+Shift+T',
    });

    // Step 3: capture tab 2's pane ID (the newly active pane).
    const pane2Id = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(pane2Id).toBeTruthy();
    // Different pane ID confirms we are on a different session.
    expect(pane2Id).not.toBe(pane1Id);

    // Step 4: inject tab 2's marker and wait for it.
    const marker2 = 'TAB2-SESSION-CONTENT';
    await tauriFireAndForget('inject_pty_output', {
      paneId: pane2Id,
      data: encodeBytes(marker2),
    });
    await waitForTextInGrid(marker2);

    // Step 5: with tab 2 active, only marker2 should be visible.
    const gridText = await getGridText();
    expect(gridText).toContain(marker2);
    expect(gridText).not.toContain(marker1);

    // Capture tab DOM IDs for TEST-TAB-SW-003.
    const allTabs = await $$(Selectors.tab);
    tab1Id = (await allTabs[0].getAttribute('data-tab-id')) ?? '';
    tab2Id = (await allTabs[1].getAttribute('data-tab-id')) ?? '';
  });

  /**
   * TEST-TAB-SW-002: Clicking between tabs switches the session content.
   *
   * Direct regression test for the two-part bug:
   *   - Missing {#key activeTabId} → TerminalPane not recreated on switch.
   *   - handleTabClick only updated local state, never set_active_tab.
   *
   * Clicks tab 1 and asserts only tab 1's content is visible, then
   * clicks tab 2 and asserts only tab 2's content is visible.
   */
  it('switches session content on tab click (regression for missing {#key})', async () => {
    const marker1 = 'TAB1-SESSION-CONTENT';
    const marker2 = 'TAB2-SESSION-CONTENT';

    // Click tab 1 (index 0) and wait for its content to be rendered.
    await $(`${Selectors.tab}[data-tab-index='0']`).click();
    await waitForTextInGrid(marker1);
    expect(await getGridText()).not.toContain(marker2);

    // Click tab 2 (index 1) and wait for its content to reappear.
    await $(`${Selectors.tab}[data-tab-index='1']`).click();
    await waitForTextInGrid(marker2);
    expect(await getGridText()).not.toContain(marker1);
  });

  /**
   * TEST-TAB-SW-003: Backend active_tab_id is kept in sync with the UI.
   *
   * After each tab switch, get_session_state must report the same active
   * tab ID that is currently displayed in the tab bar.
   *
   * Regression test for the missing set_active_tab IPC call in
   * handleTabClick — the backend registry was never updated on tab switch.
   */
  it('backend active_tab_id matches the displayed tab after each switch', async () => {
    interface SessionState {
      activeTabId: string;
    }

    // Currently on tab 2 (from previous test).
    const stateOnTab2 = await tauriInvoke<SessionState>('get_session_state');
    expect(stateOnTab2.activeTabId).toBe(tab2Id);

    // Switch to tab 1 and verify.
    await $(`${Selectors.tab}[data-tab-index='0']`).click();
    await waitForTextInGrid('TAB1-SESSION-CONTENT');

    const stateOnTab1 = await tauriInvoke<SessionState>('get_session_state');
    expect(stateOnTab1.activeTabId).toBe(tab1Id);
  });
});
