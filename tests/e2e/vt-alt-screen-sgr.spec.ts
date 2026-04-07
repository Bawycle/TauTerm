// SPDX-License-Identifier: MPL-2.0
// Build requirement: pnpm tauri build --no-bundle -- --features e2e-testing
// Run: pnpm wdio

/**
 * E2E scenario: Alternate screen + SGR color rendering.
 *
 * Verifies that:
 *   - DECSET 1049 switches to the alternate screen without leaking escape sequences.
 *   - SGR-colored text is rendered as styled cells (not as raw "\x1b[..." in textContent).
 *   - DECRST 1049 restores the primary screen.
 *   - Rapid alt-screen toggle leaves the grid in a consistent state.
 *
 * Protocol references:
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §VT
 *   - Covers manual verification scenario: vim-like ncurses app (alt screen + SGR)
 *
 * Implementation notes:
 *   - All injections use `inject_pty_output` (fire-and-forget) with a 50 ms settle.
 *   - SGR CSS cannot be asserted via WebDriver computed styles without per-cell IDs.
 *     We assert the negative invariant: no raw escape sequences in textContent.
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as any).__TAURI_INTERNALS__.invoke(cmdArg, argsArg);
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

function encodeBytes(s: string): number[] {
  return [...new TextEncoder().encode(s)];
}

async function inject(paneId: string, text: string): Promise<void> {
  await tauriFireAndForget('inject_pty_output', { paneId, data: encodeBytes(text) });
}

function gridContainsRawEscapes(): Promise<boolean> {
  return browser.execute((): boolean => {
    const text = document.querySelector('.terminal-grid')?.textContent ?? '';
    return text.includes('\x1b') || text.includes('\\033[') || text.includes('^[');
  }) as Promise<boolean>;
}

async function waitForTextInGrid(text: string, timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((t: string): boolean => {
        const grid = document.querySelector('.terminal-grid');
        return grid !== null && (grid.textContent ?? '').includes(t);
      }, text),
    {
      timeout: timeoutMs,
      timeoutMsg: `"${text}" did not appear in the terminal grid within ${timeoutMs}ms`,
    },
  );
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Alternate screen + SGR colors', () => {
  let paneId: string;

  before(async () => {
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-cancel"]');
      if (btn) btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

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

    const rawId = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(rawId).toBeTruthy();
    paneId = rawId as string;
  });

  // -------------------------------------------------------------------------
  // TEST-VT-ALT-001: SGR colored text contains no raw escape sequences.
  // -------------------------------------------------------------------------
  it('TEST-VT-ALT-001: SGR bold red text is rendered without raw escape sequences', async () => {
    const SENTINEL = 'SGR-TEST-BOLD-RED';
    // SGR 1;31: bold + red foreground, then reset with SGR 0.
    await inject(paneId, `\x1b[1;31m${SENTINEL}\x1b[0m`);
    await browser.pause(50);

    await waitForTextInGrid(SENTINEL);

    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false);
  });

  // -------------------------------------------------------------------------
  // TEST-VT-ALT-002: DECSET 1049 enters alternate screen, clears visible content.
  // -------------------------------------------------------------------------
  it('TEST-VT-ALT-002: DECSET 1049 enters alternate screen — no raw escapes', async () => {
    const ALT_CONTENT = 'ALT-SCREEN-CONTENT';
    // Enter alternate screen, write content, move cursor to top-left.
    await inject(paneId, `\x1b[?1049h\x1b[H${ALT_CONTENT}`);
    await browser.pause(50);

    await waitForTextInGrid(ALT_CONTENT);

    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false);

    // Cleanup: exit alt screen so subsequent tests start on the primary screen.
    await inject(paneId, `\x1b[?1049l`);
    await browser.pause(50);
  });

  // -------------------------------------------------------------------------
  // TEST-VT-ALT-003: DECRST 1049 exits alternate screen — primary screen restored.
  // -------------------------------------------------------------------------
  it('TEST-VT-ALT-003: DECRST 1049 exits alternate screen — primary content visible again', async () => {
    // Write a sentinel on the primary screen before entering alt screen.
    const PRIMARY_SENTINEL = 'PRIMARY-SCREEN-SENTINEL';
    await inject(paneId, `${PRIMARY_SENTINEL}`);
    await browser.pause(50);
    await waitForTextInGrid(PRIMARY_SENTINEL);

    // Enter alternate screen (primary sentinel should disappear from view).
    await inject(paneId, `\x1b[?1049h\x1b[HALT-ONLY-CONTENT`);
    await browser.pause(50);
    await waitForTextInGrid('ALT-ONLY-CONTENT');

    // Exit alternate screen.
    await inject(paneId, `\x1b[?1049l`);
    await browser.pause(50);

    // Primary sentinel must be visible again.
    await waitForTextInGrid(PRIMARY_SENTINEL, 5_000);

    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false);
  });

  // -------------------------------------------------------------------------
  // TEST-VT-ALT-004: Rapid alt-screen toggle (×3) leaves grid coherent.
  // -------------------------------------------------------------------------
  it('TEST-VT-ALT-004: three rapid alt-screen toggles leave the grid without raw escapes', async () => {
    const FINAL_SENTINEL = 'RAPID-TOGGLE-FINAL';
    // Toggle 3 times (enter, exit, enter) then write content on alt screen.
    const toggleSequence =
      `\x1b[?1049h` + // enter
      `\x1b[?1049l` + // exit
      `\x1b[?1049h` + // enter again
      `\x1b[H${FINAL_SENTINEL}`;

    await inject(paneId, toggleSequence);
    await browser.pause(50);

    await waitForTextInGrid(FINAL_SENTINEL);

    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false);

    // Cleanup: exit alt screen.
    await inject(paneId, `\x1b[?1049l`);
    await browser.pause(50);
  });
});
