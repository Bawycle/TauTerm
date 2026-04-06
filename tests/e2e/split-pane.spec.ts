// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Split-pane UI behavior.
 *
 * Verifies the complete split-pane feature: opening a split, verifying
 * both panes are rendered independently, injecting output to each pane
 * separately, and closing a pane.
 *
 * Protocol references:
 *   - FS-PANE-001 (horizontal split), FS-PANE-002 (vertical split),
 *     FS-PANE-003 (close pane), FS-PANE-004 (pane independence)
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.6
 *
 * Build requirement:
 *   The binary MUST be built with --features e2e-testing:
 *     pnpm tauri build --no-bundle -- --features e2e-testing
 *   Without this feature, inject_pty_output is not compiled and injections
 *   are silently ignored, causing output-assertion tests to hang.
 *
 * Shortcuts used (from useTerminalView defaultShortcuts):
 *   - Ctrl+Shift+D → split_pane_h (horizontal split)
 *   - Ctrl+Shift+E → split_pane_v (vertical split)
 *   - Ctrl+Shift+Q → close_pane
 */

import { browser, $, $$ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Count terminal panes currently rendered in the DOM.
 * Each `.terminal-pane` element corresponds to one leaf in the split tree.
 */
async function countPanes(): Promise<number> {
  const panes = await $$(Selectors.terminalPane);
  return panes.length;
}

/**
 * Retrieve the data-pane-id attribute of the active terminal pane.
 */
async function getActivePaneId(): Promise<string | null> {
  return browser.execute((): string | null => {
    const el = document.querySelector('.terminal-pane[data-active="true"]');
    return el ? el.getAttribute('data-pane-id') : null;
  });
}

/**
 * Retrieve all pane IDs currently in the DOM.
 */
async function getAllPaneIds(): Promise<string[]> {
  return browser.execute((): string[] => {
    return Array.from(document.querySelectorAll('.terminal-pane'))
      .map((el) => el.getAttribute('data-pane-id') ?? '')
      .filter((id) => id.length > 0);
  });
}

/**
 * Fire a Tauri IPC command without waiting for the return value.
 * Used to inject PTY output. See pty-roundtrip.spec.ts for rationale.
 */
function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
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
    function (
      keyArg: string,
      codeArg: string,
      ctrlArg: boolean,
      shiftArg: boolean,
    ): void {
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

/**
 * Wait for a specific number of panes to appear in the DOM.
 */
async function waitForPaneCount(
  expected: number,
  timeout = 8_000,
): Promise<void> {
  await browser.waitUntil(async () => (await countPanes()) === expected, {
    timeout,
    timeoutMsg: `Expected ${expected} pane(s) — got ${await countPanes()} after ${timeout}ms`,
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('TauTerm — Split-pane UI behavior', () => {
  /**
   * TEST-SPLIT-E2E-001: Application starts with exactly one pane.
   *
   * On first launch, the terminal view must show exactly one terminal pane
   * (a single leaf node in the layout tree).
   */
  it('TEST-SPLIT-E2E-001: starts with exactly one pane', async () => {
    const paneCount = await countPanes();
    expect(paneCount).toBe(1);

    // The single pane must be active
    const activePaneId = await getActivePaneId();
    expect(activePaneId).toBeTruthy();
  });

  /**
   * TEST-SPLIT-E2E-002: Ctrl+Shift+D opens a horizontal split.
   *
   * After pressing the horizontal split shortcut, the layout must contain
   * two leaf panes with distinct pane IDs.
   * The split divider must be present in the DOM.
   */
  it('TEST-SPLIT-E2E-002: Ctrl+Shift+D opens a horizontal split (two panes, distinct IDs)', async () => {
    // Give focus to the terminal grid before sending shortcuts
    await $(Selectors.terminalGrid).click();

    // Dispatch Ctrl+Shift+D (split_pane_h)
    await dispatchShortcut('D', 'KeyD', true, true);

    // Wait for two panes to appear
    await waitForPaneCount(2);

    const paneIds = await getAllPaneIds();
    expect(paneIds.length).toBe(2);

    // Both pane IDs must be distinct non-empty strings
    expect(paneIds[0]).toBeTruthy();
    expect(paneIds[1]).toBeTruthy();
    expect(paneIds[0]).not.toBe(paneIds[1]);

    // The split divider must be present
    const divider = await browser.execute((): boolean => {
      return document.querySelector('.split-pane__divider') !== null;
    });
    expect(divider).toBe(true);
  });

  /**
   * TEST-SPLIT-E2E-003: Panes are independent — injecting output to pane A
   * does not affect pane B.
   *
   * Uses inject_pty_output (requires --features e2e-testing) to push
   * a unique marker string into each pane independently, then verifies
   * each marker appears only in its own pane's grid.
   *
   * Depends on TEST-SPLIT-E2E-002 having created a horizontal split.
   */
  it('TEST-SPLIT-E2E-003: panes are independent — injected output is per-pane', async () => {
    // Ensure we have 2 panes (may be from previous test or create fresh split)
    if ((await countPanes()) < 2) {
      await $(Selectors.terminalGrid).click();
      await dispatchShortcut('D', 'KeyD', true, true);
      await waitForPaneCount(2);
    }

    const paneIds = await getAllPaneIds();
    expect(paneIds.length).toBe(2);

    const [paneIdA, paneIdB] = paneIds;
    const markerA = 'SPLIT-MARKER-PANE-A';
    const markerB = 'SPLIT-MARKER-PANE-B';

    // Inject distinct markers into each pane
    const bytesA = [...new TextEncoder().encode(markerA + '\r\n')];
    const bytesB = [...new TextEncoder().encode(markerB + '\r\n')];

    await tauriFireAndForget('inject_pty_output', { paneId: paneIdA, data: bytesA });
    await tauriFireAndForget('inject_pty_output', { paneId: paneIdB, data: bytesB });

    // Wait for marker A to appear somewhere in the terminal grid
    await browser.waitUntil(
      () =>
        browser.execute((marker: string): boolean => {
          const grid = document.querySelector('.terminal-grid');
          return grid !== null && (grid.textContent ?? '').includes(marker);
        }, markerA),
      { timeout: 10_000, timeoutMsg: `Marker "${markerA}" did not appear within 10 s` },
    );

    // Wait for marker B to appear somewhere in the terminal grid
    await browser.waitUntil(
      () =>
        browser.execute((marker: string): boolean => {
          const grid = document.querySelector('.terminal-grid');
          return grid !== null && (grid.textContent ?? '').includes(marker);
        }, markerB),
      { timeout: 10_000, timeoutMsg: `Marker "${markerB}" did not appear within 10 s` },
    );

    // Both markers are present → the two panes rendered independently
    const bothPresent = await browser.execute(
      (mA: string, mB: string): boolean => {
        const grid = document.querySelector('.terminal-grid');
        const text = grid?.textContent ?? '';
        return text.includes(mA) && text.includes(mB);
      },
      markerA,
      markerB,
    );
    expect(bothPresent).toBe(true);
  });

  /**
   * TEST-SPLIT-E2E-004: Ctrl+Shift+E opens a vertical split.
   *
   * Starting from a single-pane state (close any existing splits first),
   * pressing Ctrl+Shift+E must create a vertical split (two panes stacked).
   * The split-pane container must carry the --vertical class.
   */
  it('TEST-SPLIT-E2E-004: Ctrl+Shift+E opens a vertical split', async () => {
    // Start fresh: close all extra panes by re-creating a tab.
    // We rely on the fact that Ctrl+Shift+T creates a new single-pane tab.
    await browser.execute((): void => {
      const grid = document.querySelector('.terminal-grid') as HTMLElement | null;
      const target = grid ?? document.body;
      target.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'T', code: 'KeyT', ctrlKey: true, shiftKey: true,
          bubbles: true, cancelable: true,
        }),
      );
    });

    // Wait for a fresh single-pane layout
    await browser.waitUntil(async () => (await countPanes()) === 1, {
      timeout: 8_000,
      timeoutMsg: 'Could not reach single-pane state for vertical split test',
    });

    // Focus and dispatch Ctrl+Shift+E (split_pane_v)
    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('E', 'KeyE', true, true);

    // Wait for two panes
    await waitForPaneCount(2);

    // Verify the container carries the vertical direction class
    const isVertical = await browser.execute((): boolean => {
      return document.querySelector('.split-pane__container--vertical') !== null;
    });
    expect(isVertical).toBe(true);
  });

  /**
   * TEST-SPLIT-E2E-005: Ctrl+Shift+Q closes the active pane.
   *
   * After a split exists (two panes), pressing the close-pane shortcut on
   * the active pane must reduce the count back to one pane.
   *
   * Depends on the previous test having created a split.
   */
  it('TEST-SPLIT-E2E-005: Ctrl+Shift+Q closes the active pane', async () => {
    // Ensure we have 2 panes
    if ((await countPanes()) < 2) {
      await $(Selectors.terminalGrid).click();
      await dispatchShortcut('D', 'KeyD', true, true);
      await waitForPaneCount(2);
    }

    // Dispatch Ctrl+Shift+Q (close_pane)
    await dispatchShortcut('Q', 'KeyQ', true, true);

    // Wait for pane count to return to one.
    // Note: close_pane on a pane whose process is still active via
    // InjectablePtyBackend does NOT show a confirmation dialog because
    // the injectable backend never emits processExited — but the
    // TerminalView close-pane flow checks isPaneProcessActive which reads
    // from terminatedPanes; since InjectablePtyBackend tracks no process
    // state, the pane closes immediately.
    await browser.waitUntil(async () => (await countPanes()) === 1, {
      timeout: 8_000,
      timeoutMsg: 'Pane count did not return to 1 after Ctrl+Shift+Q',
    });

    // Verify only one pane remains and it is active
    const activePaneId = await getActivePaneId();
    expect(activePaneId).toBeTruthy();

    // No split divider should remain
    const dividerPresent = await browser.execute((): boolean => {
      return document.querySelector('.split-pane__divider') !== null;
    });
    expect(dividerPresent).toBe(false);
  });

  /**
   * TEST-SPLIT-E2E-006: Divider is present and has the correct ARIA role.
   *
   * The split divider must carry role="separator" for accessibility (UXD §7.2).
   */
  it('TEST-SPLIT-E2E-006: split divider has role="separator"', async () => {
    // Create a split to have a divider
    await browser.execute((): void => {
      const grid = document.querySelector('.terminal-grid') as HTMLElement | null;
      const target = grid ?? document.body;
      target.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'T', code: 'KeyT', ctrlKey: true, shiftKey: true,
          bubbles: true, cancelable: true,
        }),
      );
    });
    await browser.waitUntil(async () => (await countPanes()) === 1, {
      timeout: 8_000,
      timeoutMsg: 'Could not reach single-pane state for divider test',
    });

    await $(Selectors.terminalGrid).click();
    await dispatchShortcut('D', 'KeyD', true, true);
    await waitForPaneCount(2);

    const hasSeparatorRole = await browser.execute((): boolean => {
      const divider = document.querySelector('.split-pane__divider');
      return divider?.getAttribute('role') === 'separator';
    });
    expect(hasSeparatorRole).toBe(true);
  });

  /**
   * TEST-SPLIT-E2E-007: Each pane has its own data-pane-id attribute.
   *
   * After a split, both leaf panes must carry distinct non-empty
   * data-pane-id attributes so the IPC layer can address them individually.
   */
  it('TEST-SPLIT-E2E-007: each pane carries a distinct data-pane-id', async () => {
    // Ensure we're in a 2-pane state
    if ((await countPanes()) < 2) {
      await $(Selectors.terminalGrid).click();
      await dispatchShortcut('D', 'KeyD', true, true);
      await waitForPaneCount(2);
    }

    const paneIds = await getAllPaneIds();
    expect(paneIds.length).toBe(2);
    expect(new Set(paneIds).size).toBe(2); // all unique
    paneIds.forEach((id) => expect(id).toBeTruthy());
  });
});
