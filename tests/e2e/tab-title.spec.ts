// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenarios: OSC 0 tab title pipeline.
 *
 * Verifies that OSC 0 sequences emitted by a PTY session are propagated
 * through the VT parser → screen buffer → IPC event → frontend state
 * pipeline and ultimately update the active tab's title in the tab bar DOM.
 *
 * Protocol references:
 * - TEST-OSC-E2E-001: single OSC 0 sequence sets the tab title
 * - TEST-OSC-E2E-002: successive OSC 0 sequences — last title wins
 * - FS-TAB (tab title OSC 0 acceptance criteria)
 *
 * Build requirement:
 * The binary under test MUST be built with `--features e2e-testing`.
 * Without this flag, `inject_pty_output` is compiled out and injections
 * are silently ignored (all assertions will time out).
 *
 * Note on fire-and-forget injection:
 * `inject_pty_output` sends bytes to an unbounded mpsc channel and returns
 * immediately.  We use `browser.execute` (not `browser.executeAsync`) to
 * avoid the done-callback stall in tauri-driver / WebKitGTK.  DOM effects
 * are asserted separately via `browser.waitUntil`.
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// IPC helpers
// ---------------------------------------------------------------------------

/**
 * Fire a Tauri IPC command without waiting for its return value.
 * Safe for `inject_pty_output` and other commands that return immediately.
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
// DOM helpers
// ---------------------------------------------------------------------------

/** Encode a string to an array of UTF-8 bytes for `inject_pty_output`. */
function encodeBytes(s: string): number[] {
  return [...new TextEncoder().encode(s)];
}

/**
 * Return the text content of the active tab's title element in a single
 * browser.execute RPC call to avoid per-element overhead.
 */
async function getActiveTabTitle(): Promise<string> {
  return browser.execute((): string => {
    const activeTab = document.querySelector('.tab-bar__tab[aria-selected="true"]');
    if (!activeTab) return '';
    const titleEl = activeTab.querySelector('.tab-bar__tab-title');
    return titleEl?.textContent ?? '';
  }) as Promise<string>;
}

/**
 * Wait until the active tab's title element contains `expectedTitle`.
 * Uses a single browser.execute scan per poll tick to minimise RPC overhead.
 */
async function waitForActiveTabTitle(expectedTitle: string, timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((expected: string): boolean => {
        const activeTab = document.querySelector('.tab-bar__tab[aria-selected="true"]');
        if (!activeTab) return false;
        const titleEl = activeTab.querySelector('.tab-bar__tab-title');
        return (titleEl?.textContent ?? '').includes(expected);
      }, expectedTitle),
    {
      timeout: timeoutMs,
      timeoutMsg: `Active tab title did not contain "${expectedTitle}" within ${timeoutMs / 1_000} s`,
    },
  );
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — OSC 0 tab title pipeline', () => {
  before(async () => {
    // Ensure the active terminal pane is ready before any test runs.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 15_000, timeoutMsg: 'Active terminal pane did not appear within 15 s' },
    );
  });

  /**
   * TEST-OSC-E2E-001: Single OSC 0 sequence sets the active tab title.
   *
   * Injects the byte sequence `ESC ] 0 ; MyTitle BEL` into the active pane's
   * VT pipeline via `inject_pty_output`.  The sequence must travel through:
   *   VT parser  →  OSC handler  →  title-changed event  →  frontend store
   *   →  TabBarItem title prop  →  .tab-bar__tab-title DOM node
   *
   * Asserts that the active tab's `.tab-bar__tab-title` element contains
   * "MyTitle" within the assertion timeout.
   */
  it('TEST-OSC-E2E-001: OSC 0 sequence sets the active tab title', async () => {
    const paneId = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(paneId).toBeTruthy();

    // ESC ] 0 ; MyTitle BEL
    const osc0 = '\x1b]0;MyTitle\x07';
    await tauriFireAndForget('inject_pty_output', {
      paneId,
      data: encodeBytes(osc0),
    });

    await waitForActiveTabTitle('MyTitle');
    const title = await getActiveTabTitle();
    expect(title).toContain('MyTitle');
  });

  /**
   * TEST-OSC-E2E-002: Successive OSC 0 sequences — last title wins.
   *
   * Injects two OSC 0 sequences back-to-back into the same pane.  The VT
   * parser must process them in order; the final state must reflect the
   * second title ("Second"), not the first ("First").
   *
   * This guards against race conditions where an earlier event arrives in
   * the frontend after a later one (e.g. if events were reordered), or
   * where the title store is not overwritten on each OSC 0.
   */
  it('TEST-OSC-E2E-002: successive OSC 0 sequences — last title wins', async () => {
    const paneId = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(paneId).toBeTruthy();

    // Inject both sequences in a single byte stream so the VT parser
    // processes them sequentially without any inter-call race.
    const twoOsc = '\x1b]0;First\x07\x1b]0;Second\x07';
    await tauriFireAndForget('inject_pty_output', {
      paneId,
      data: encodeBytes(twoOsc),
    });

    // "Second" must win — wait for it.
    await waitForActiveTabTitle('Second');
    const title = await getActiveTabTitle();
    expect(title).toContain('Second');
    expect(title).not.toContain('First');
  });
});
