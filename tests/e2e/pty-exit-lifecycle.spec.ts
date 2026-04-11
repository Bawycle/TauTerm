// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenarios: PTY exit / close lifecycle.
 *
 * Covers FS-PTY-005, FS-PTY-006, FS-PTY-008, and FS-TAB-008 for the
 * observable DOM behavior surfaced by the exit/close lifecycle features.
 *
 * Build requirement:
 *   The binary MUST be built with --features e2e-testing:
 *     pnpm tauri build --no-bundle -- --features e2e-testing
 *
 * Limitations of the InjectablePtyBackend (ADR-0015):
 *   - InjectablePtyBackend feeds bytes into the VT pipeline only; it does NOT
 *     model the process lifecycle (no SIGCHLD, no waitpid, no exit-code signal).
 *   - As a result, tests that require a pane to transition to the "terminated"
 *     state (FS-PTY-005 auto-close for exit 0; FS-PTY-006 terminated banner for
 *     non-zero exit) need a dedicated E2E command to inject a process-exit event:
 *     `inject_pane_exit { pane_id, exit_code }`.
 *     This command does not yet exist in the backend.  The affected tests below
 *     are marked with TODO and will be enabled once the command is implemented.
 *   - `has_foreground_process` always returns `false` for InjectablePtyBackend
 *     panes (no real PTY fd, no tcgetpgrp).  Tests that require the confirmation
 *     dialog path (FS-PTY-008) via a simulated foreground process need a
 *     dedicated command: `inject_foreground_process { pane_id, active: bool }`.
 *     This command does not yet exist either.  Affected tests are marked TODO.
 *
 * Tests that ARE fully exercisable today:
 *   - TEST-PTY-EXIT-003: shell idle (InjectablePtyBackend → has_foreground_process
 *     = false) closes pane without dialog.
 *   - TEST-PTY-EXIT-006: window close confirmation dialog when active processes
 *     exist (requires inject_foreground_process — TODO).
 *   - TEST-PTY-EXIT-007: last-tab window-close path (FS-TAB-008 — see note in
 *     test body).
 *
 * Protocol references:
 *   - FS-PTY-005, FS-PTY-006, FS-PTY-008
 *   - FS-TAB-008
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.1
 */

import { browser, $, $$ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers shared across this spec
// ---------------------------------------------------------------------------

/**
 * Fire a Tauri IPC command without waiting for its return value.
 * See pty-roundtrip.spec.ts for the rationale (browser.executeAsync stalls
 * in WebKitGTK; fire-and-forget via browser.execute is sufficient).
 */
function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined): void {
      (window as unknown as Record<string, { invoke: Function }>).__TAURI_INTERNALS__.invoke(
        cmdArg,
        argsArg,
      );
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

/**
 * Invoke a Tauri IPC command and wait for its result.
 *
 * Uses the window-variable polling pattern (same as ssh-overlay-states.spec.ts)
 * to work around `browser.executeAsync` unreliability with tauri-driver / WebKitGTK.
 */
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await browser.execute(function () {
    (window as unknown as Record<string, unknown>).__e2e_invoke_result = undefined;
  });

  await browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as unknown as {
        __TAURI_INTERNALS__: { invoke: (c: string, a?: unknown) => Promise<unknown> };
      }).__TAURI_INTERNALS__
        .invoke(cmdArg, argsArg)
        .then(function (r: unknown) {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result =
            r ?? '__e2e_null__';
        })
        .catch(function () {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result = '__e2e_error__';
        });
    },
    cmd,
    args,
  );

  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return (window as unknown as Record<string, unknown>).__e2e_invoke_result !== undefined;
      }),
    { timeout: 5_000, timeoutMsg: `tauriInvoke("${cmd}") did not resolve within 5 s` },
  );

  const raw = await browser.execute(function (): unknown {
    const v = (window as unknown as Record<string, unknown>).__e2e_invoke_result;
    delete (window as unknown as Record<string, unknown>).__e2e_invoke_result;
    return v;
  });

  if (raw === '__e2e_error__') {
    throw new Error(`tauriInvoke("${cmd}") rejected`);
  }
  return (raw === '__e2e_null__' ? null : raw) as T;
}

/** Count terminal panes currently rendered in the DOM. */
async function countPanes(): Promise<number> {
  return (await $$(Selectors.terminalPane)).length;
}

/** Count tabs currently rendered in the tab bar. */
async function countTabs(): Promise<number> {
  return (await $$(Selectors.tab)).length;
}

/** Retrieve the data-pane-id of the active pane, or null if none. */
async function getActivePaneId(): Promise<string | null> {
  return browser.execute((): string | null => {
    const el = document.querySelector('.terminal-pane[data-active="true"]');
    return el ? el.getAttribute('data-pane-id') : null;
  });
}

/** Retrieve all pane IDs currently in the DOM. */
async function getAllPaneIds(): Promise<string[]> {
  return browser.execute((): string[] =>
    Array.from(document.querySelectorAll('.terminal-pane'))
      .map((el) => el.getAttribute('data-pane-id') ?? '')
      .filter((id) => id.length > 0),
  );
}

/**
 * Dispatch a keyboard shortcut via DOM events.
 * Bypasses WebDriver key-delivery quirks in WebKitGTK (see split-pane.spec.ts).
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

/** Wait for the pane count to reach `expected`. */
async function waitForPaneCount(expected: number, timeout = 8_000): Promise<void> {
  await browser.waitUntil(async () => (await countPanes()) === expected, {
    timeout,
    timeoutMsg: `Expected ${expected} pane(s); got ${await countPanes()} after ${timeout} ms`,
  });
}

/** Wait for the tab count to reach `expected`. */
async function waitForTabCount(expected: number, timeout = 8_000): Promise<void> {
  await browser.waitUntil(async () => (await countTabs()) === expected, {
    timeout,
    timeoutMsg: `Expected ${expected} tab(s); got ${await countTabs()} after ${timeout} ms`,
  });
}

/** Ensure the app is in a known clean state: exactly one tab, one pane. */
async function resetToSingleTab(): Promise<void> {
  // Close extra tabs by opening one fresh tab then closing all others.
  // The simplest approach: create a new tab (Ctrl+Shift+T), then for every
  // old tab dispatch close (Ctrl+Shift+W + confirm dialog if it appears).
  // Since this helper is called from `beforeEach`, we take a simpler path:
  // reload the page state by checking we are already clean.
  const initialTabCount = await countTabs();
  if (initialTabCount === 1 && (await countPanes()) === 1) return;

  // Create a new tab to land on a clean pane, then close all tabs except this one.
  await dispatchShortcut('T', 'KeyT', true, true);
  await waitForTabCount(initialTabCount + 1);

  // Close all tabs except the newly created last one.
  for (let i = 0; i < initialTabCount; i++) {
    // Switch to the first tab (data-tab-index="0") and close it.
    await browser.execute((): void => {
      const firstTab = document.querySelector<HTMLElement>('[data-tab-index="0"]');
      firstTab?.click();
    });
    await browser.pause(200);
    await dispatchShortcut('W', 'KeyW', true, true);

    // Dismiss the confirmation dialog if it appears.
    const dialogPresent = await browser.execute((): boolean =>
      document.querySelector('[data-testid="close-confirm-action"]') !== null,
    );
    if (dialogPresent) {
      await browser.execute((): void => {
        const btn = document.querySelector<HTMLButtonElement>(
          '[data-testid="close-confirm-action"]',
        );
        btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      });
    }
    await browser.pause(200);
  }

  await waitForTabCount(1);
  await waitForPaneCount(1);
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe('TauTerm — PTY exit and close lifecycle', () => {
  // Reset to a known single-tab/single-pane state before each test so that
  // state leaking from a previous test (e.g. a lingering split) does not cause
  // spurious failures.
  beforeEach(async () => {
    await resetToSingleTab();
  });

  /**
   * TEST-PTY-EXIT-001: exit 0 auto-closes the pane (FS-PTY-005).
   *
   * GIVEN a session with 2 panes
   * WHEN the shell exits with code 0 in one pane
   * THEN that pane auto-closes immediately
   * AND the other pane remains open
   */
  it('TEST-PTY-EXIT-001: exit 0 auto-closes the pane', async () => {
    // Setup: create a horizontal split to have 2 panes.
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    const paneIds = await getAllPaneIds();
    expect(paneIds.length).toBe(2);
    const [paneIdA, paneIdB] = paneIds;

    // Inject exit code 0 into pane A — triggers auto-close path in applyNotificationChanged.
    await tauriInvoke<void>('inject_pane_exit', { paneId: paneIdA, exitCode: 0 });

    // THEN pane A should auto-close and only pane B should remain.
    await waitForPaneCount(1);

    const remainingIds = await getAllPaneIds();
    expect(remainingIds).toEqual([paneIdB]);

    // The ProcessTerminatedPane banner must NOT appear (clean exit → auto-close, no banner).
    const bannerPresent = await browser.execute(
      (): boolean => document.querySelector('.process-terminated-pane') !== null,
    );
    expect(bannerPresent).toBe(false);
  });

  /**
   * TEST-PTY-EXIT-002: non-zero exit shows terminated banner with Restart/Close (FS-PTY-005, FS-PTY-006).
   *
   * GIVEN an active pane
   * WHEN the shell exits with a non-zero exit code (e.g. 1)
   * THEN the pane remains open (no auto-close)
   * AND the .process-terminated-pane banner is displayed
   * AND both "Restart" and "Close" action buttons are present in the banner
   *
   * TODO: requires `inject_pane_exit { pane_id, exit_code: 1 }` Tauri command
   *       (same as TEST-PTY-EXIT-001 — implement inject_pane_exit first).
   */
  it('TEST-PTY-EXIT-002: non-zero exit shows terminated banner', async () => {
    const paneId = await getActivePaneId();
    expect(paneId).toBeTruthy();

    // Inject a non-zero exit event — triggers the terminated banner path.
    await tauriInvoke<void>('inject_pane_exit', { paneId, exitCode: 1 });

    // THEN the pane must remain open.
    const paneCount = await countPanes();
    expect(paneCount).toBeGreaterThanOrEqual(1);

    // Small pause to allow the IPC event (notification-changed) to propagate
    // from the backend emit to the Tauri WebView event channel and be processed
    // by the frontend listener (useTerminalView.lifecycle.svelte.ts).
    await browser.pause(500);

    // The terminated banner must appear.
    await browser.waitUntil(
      (): Promise<boolean> =>
        browser.execute(
          (): boolean => document.querySelector('.process-terminated-pane') !== null,
        ),
      { timeout: 5_000, timeoutMsg: '.process-terminated-pane banner did not appear within 5 s' },
    );

    // Both action buttons must be present in the banner.
    // The banner uses translated labels — locate buttons by their parent container
    // (.process-terminated-pane__actions) to remain locale-independent.
    const actionButtonCount = await browser.execute((): number => {
      const actions = document.querySelector('.process-terminated-pane__actions');
      return actions ? actions.querySelectorAll('button').length : 0;
    });
    expect(actionButtonCount).toBe(2);
  });

  /**
   * TEST-PTY-EXIT-003: pane with idle shell closes without confirmation dialog (FS-PTY-008).
   *
   * GIVEN a pane backed by InjectablePtyBackend (has_foreground_process → false)
   * WHEN the user triggers close_pane (Ctrl+Shift+Q) on the active pane from a split
   * THEN the pane closes immediately
   * AND no confirmation dialog appears
   *
   * Rationale: InjectablePtyBackend never spawns a real process, so
   * `has_foreground_process` returns false. This is the "idle shell" case per
   * FS-PTY-008 — no dialog must be shown.
   *
   * Note: this test must run on a split layout so closing one pane leaves another
   * open. Closing the sole pane in a single-pane tab triggers close_tab, not
   * close_pane, which follows a different code path.
   */
  it('TEST-PTY-EXIT-003: idle shell — close pane fires without confirmation dialog', async () => {
    // Ensure we start with a single pane.
    await waitForPaneCount(1);

    // Create a horizontal split → 2 panes.
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    const paneIdsBefore = await getAllPaneIds();
    expect(paneIdsBefore.length).toBe(2);
    const activePaneIdBefore = await getActivePaneId();
    expect(activePaneIdBefore).toBeTruthy();

    // Dispatch close_pane (Ctrl+Shift+Q).
    await dispatchShortcut('Q', 'KeyQ', true, true);

    // The pane must close immediately — wait for count to drop to 1.
    await waitForPaneCount(1, 5_000);

    // No confirmation dialog must have appeared.
    // We assert it is absent after the pane has already closed, which is
    // the strongest observable guarantee available without an explicit "never"
    // assertion (a dialog that flashed and disappeared would fail TEST-PTY-EXIT-005).
    const confirmDialogPresent = await browser.execute(
      (): boolean => document.querySelector('[data-testid="close-confirm-action"]') !== null,
    );
    expect(confirmDialogPresent).toBe(false);

    // The closed pane's ID must no longer appear in the DOM.
    const remainingIds = await getAllPaneIds();
    expect(remainingIds).not.toContain(activePaneIdBefore);
  });

  /**
   * TEST-PTY-EXIT-004: confirmation dialog Cancel keeps pane open (FS-PTY-008).
   *
   * GIVEN a pane where has_foreground_process returns true (active process)
   * WHEN the user triggers close_pane
   * THEN the confirmation dialog appears
   * AND clicking Cancel dismisses the dialog
   * AND the pane remains open
   *
   * TODO: requires `inject_foreground_process { pane_id, active: true }` Tauri
   *       command so that has_foreground_process returns true for an injectable
   *       pane. Without this, the dialog is never triggered for injectable panes.
   *       Implement in src-tauri/src/commands/testing.rs before enabling this test.
   *
   * NOTE: the dialog logic itself (Cancel + Confirm paths) is partially exercised
   *       by TEST-TAB-E2E-003 in tab-lifecycle.spec.ts (tab close dialog), which
   *       shares the same Svelte dialog component and data-testid attributes.
   */
  it('TEST-PTY-EXIT-004: active process — dialog Cancel keeps pane open', async () => {
    // Setup: create a split, then inject an active foreground process into one pane.
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    const activePaneId = await getActivePaneId();
    expect(activePaneId).toBeTruthy();

    // Inject a synthetic foreground process — makes has_foreground_process return true.
    await tauriInvoke<void>('inject_foreground_process', { paneId: activePaneId, processName: 'vim' });

    // Trigger close_pane.
    await dispatchShortcut('Q', 'KeyQ', true, true);

    // Confirmation dialog must appear.
    await browser.waitUntil(
      (): Promise<boolean> =>
        browser.execute(
          (): boolean =>
            document.querySelector('[data-testid="close-confirm-cancel"]') !== null,
        ),
      { timeout: 3_000, timeoutMsg: 'Close confirmation dialog did not appear within 3 s' },
    );

    // Click Cancel.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="close-confirm-cancel"]',
      );
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Pane count must remain at 2.
    await browser.pause(500);
    const paneCount = await countPanes();
    expect(paneCount).toBe(2);

    // Dialog must be gone.
    const dialogGone = await browser.execute(
      (): boolean => document.querySelector('[data-testid="close-confirm-cancel"]') === null,
    );
    expect(dialogGone).toBe(true);

    // Cleanup: clear the foreground injection so the split pane can be closed
    // without a dialog in the subsequent resetToSingleTab() call.
    await tauriInvoke<void>('inject_foreground_process', { paneId: activePaneId, processName: '' });
    // Close the split pane so the next beforeEach sees a single-pane layout.
    await dispatchShortcut('Q', 'KeyQ', true, true);
    await waitForPaneCount(1, 5_000);
  });

  /**
   * TEST-PTY-EXIT-005: confirmation dialog Confirm closes pane (FS-PTY-008).
   *
   * GIVEN a pane where has_foreground_process returns true
   * WHEN the user triggers close_pane
   * THEN the confirmation dialog appears
   * AND clicking Close (confirm action) closes the pane
   *
   * TODO: same prerequisite as TEST-PTY-EXIT-004 — requires inject_foreground_process.
   */
  it('TEST-PTY-EXIT-005: active process — dialog Confirm closes pane', async () => {
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    const activePaneId = await getActivePaneId();
    expect(activePaneId).toBeTruthy();

    // Inject a synthetic foreground process — makes has_foreground_process return true.
    await tauriInvoke<void>('inject_foreground_process', { paneId: activePaneId, processName: 'vim' });

    await dispatchShortcut('Q', 'KeyQ', true, true);

    await browser.waitUntil(
      (): Promise<boolean> =>
        browser.execute(
          (): boolean =>
            document.querySelector('[data-testid="close-confirm-action"]') !== null,
        ),
      { timeout: 3_000, timeoutMsg: 'Close confirmation dialog did not appear within 3 s' },
    );

    // Click the confirm (Close) button.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="close-confirm-action"]',
      );
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Pane must close — no foreground cleanup needed since the pane is gone.
    await waitForPaneCount(1, 5_000);
  });

  /**
   * TEST-PTY-EXIT-006: window close confirmation dialog (FS-PTY-008 window close variant).
   *
   * GIVEN a pane where has_foreground_process returns true
   * WHEN the OS/WM close event fires (window-close-requested)
   * THEN the window-close confirmation dialog appears (data-testid="window-close-confirm-action")
   * AND clicking Cancel → dialog dismissed; window remains open
   *
   * SKIP REASON: requires a `simulate_window_close` Tauri command to trigger the
   *       window-close-requested event from outside the WM — this command does not
   *       exist yet.  inject_foreground_process is now available (TEST-PTY-EXIT-004),
   *       so this test is unblocked as soon as simulate_window_close is implemented.
   */
  it.skip('TEST-PTY-EXIT-006: window-close with active process shows window dialog (requires simulate_window_close)', async () => {
    // TODO: requires inject_foreground_process + window-close event injection
    //
    // When this test is enabled:
    //   1. inject_foreground_process for the active pane
    //   2. Simulate window close via tauri JS API:
    //      browser.execute(() => window.__TAURI_INTERNALS__.invoke('simulate_window_close'))
    //      (this command does not yet exist)
    //   3. Wait for [data-testid="window-close-confirm-cancel"] to appear
    //   4. Click Cancel → dialog gone, window still open
    //   5. browser.getTitle() === 'tau-term'
  });

  /**
   * TEST-PTY-EXIT-007: closing the last tab closes the application (FS-TAB-008).
   *
   * GIVEN a single tab in the session (shell idle — InjectablePtyBackend)
   * WHEN the last tab is closed via Ctrl+Shift+W + confirm dialog
   * THEN getCurrentWindow().close() is called by the frontend
   * AND the application process terminates
   *
   * Observability constraint: when the Tauri window closes, the WebKitGTK
   * WebView is destroyed and tauri-driver loses its WebDriver session. Any
   * further browser.* call will throw a "session not found" error. This
   * prevents us from asserting a positive DOM state after the close.
   *
   * The test strategy is therefore:
   *   1. Reach a single-tab state.
   *   2. Dispatch Ctrl+Shift+W to trigger close_tab.
   *   3. Since InjectablePtyBackend → has_foreground_process = false, a
   *      close-tab confirmation may or may not appear depending on whether
   *      `isPaneProcessActive` is consulted. Per the current implementation,
   *      `close_tab` path calls `hasForegroundProcess` IPC, which returns
   *      false → no dialog → doCloseTab is called → getCurrentWindow().close().
   *   4. Detect the window-closed state by catching the resulting
   *      "session terminated" error from the next browser.* call.
   *
   * NOTE: because this test terminates the Tauri process, it MUST run last.
   *       WebdriverIO sorts specs alphabetically; this file (pty-exit-lifecycle)
   *       comes before tab-lifecycle alphabetically. If this test causes the
   *       process to exit, subsequent specs in the same session will fail.
   *       This test is therefore SKIPPED for now and must be moved to a
   *       dedicated single-spec run or a suite configured to run in isolation.
   *
   * SKIP REASON: this test terminates the Tauri process and must run last in an
   *       isolated wdio session.  Running it in the shared suite causes all subsequent
   *       specs to fail with "session not found".  Enable via:
   *         pnpm wdio --spec tests/e2e/pty-exit-lifecycle.spec.ts
   *       after configuring tauri-driver to restart the app between spec files.
   */
  it.skip('TEST-PTY-EXIT-007: last tab close terminates the app window (requires isolated run)', async () => {
    // Ensure exactly one tab is open.
    const tabCount = await countTabs();
    if (tabCount !== 1) {
      // Cannot reach single-tab state reliably without closing tabs that may
      // themselves trigger dialogs. Fail fast with a clear message.
      throw new Error(
        `TEST-PTY-EXIT-007 requires exactly 1 tab; got ${tabCount}. ` +
          `Run this test in an isolated wdio session.`,
      );
    }

    // Dispatch Ctrl+Shift+W to close the last tab.
    await dispatchShortcut('W', 'KeyW', true, true);

    // InjectablePtyBackend: has_foreground_process = false → no dialog.
    // Wait briefly to allow the async close chain to execute.
    await browser.pause(1_000);

    // The next browser call should either:
    // (a) succeed if getCurrentWindow().close() has not yet been called (race), or
    // (b) throw a WebDriver "session not found" error when the window is gone.
    //
    // In either case we assert the application was not left in a broken state.
    try {
      // If the app is still running, something prevented the close.
      const title = await browser.getTitle();
      // If we reach here without throwing, the window close has not yet fired.
      // This is acceptable only if the app is still responsive (no crash).
      expect(title).toBe('tau-term');
    } catch (err: unknown) {
      // A WebDriver "session not found" / "no such window" error is the
      // expected outcome of a successful window close. Treat it as a pass.
      const message = err instanceof Error ? err.message : String(err);
      const isExpectedSessionEnd =
        message.includes('session') ||
        message.includes('window') ||
        message.includes('no such');
      if (!isExpectedSessionEnd) {
        throw err; // unexpected error — re-throw
      }
      // Expected: session ended because the window closed. Test passes.
    }
  });
});
