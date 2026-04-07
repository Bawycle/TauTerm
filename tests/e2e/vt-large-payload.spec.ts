// SPDX-License-Identifier: MPL-2.0
// Build requirement: pnpm tauri build --no-bundle -- --features e2e-testing
// Run: pnpm wdio

/**
 * E2E scenario: Large payload injection — no crash, no truncation.
 *
 * Verifies that injecting 1 MB of printable ASCII data through the VT pipeline:
 *   - Does not crash the application (pane remains alive).
 *   - Does not leak raw escape sequences into the DOM.
 *   - Leaves the grid with visible content (last rows contain data).
 *
 * Protocol references:
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.2 TEST-VT-003
 *   - Covers manual verification scenario: `cat /dev/urandom | head -c 1M`
 *
 * Implementation notes:
 *   - Data is pure printable ASCII (no control sequences) to avoid unintended
 *     VT state mutations (mode changes, screen clears) that would complicate assertions.
 *   - 1 MB is split into 64 KB chunks to avoid hitting any IPC serialization limit.
 *   - Each chunk is injected fire-and-forget; a single waitUntil at the end waits
 *     for the last sentinel to appear.
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHUNK_SIZE = 64 * 1_024; // 64 KB per inject call
const TOTAL_SIZE = 1_024 * 1_024; // 1 MB total
const CHUNKS = TOTAL_SIZE / CHUNK_SIZE; // 16 chunks

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

function gridHasNonEmptyCell(): Promise<boolean> {
  return browser.execute((): boolean => {
    for (const cell of document.querySelectorAll('.terminal-pane__cell')) {
      const text = cell.textContent ?? '';
      if (text.trim().length > 0 && text !== '\u00a0') return true;
    }
    return false;
  }) as Promise<boolean>;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Large payload injection (1 MB)', () => {
  let paneId: string;

  // Repeating printable ASCII chunk (95 printable ASCII chars cycled).
  const PRINTABLE_ASCII = Array.from({ length: CHUNK_SIZE }, (_, i) =>
    String.fromCharCode(32 + (i % 95)),
  ).join('');

  // Unique sentinel appended after the large payload so we can wait for it.
  const SENTINEL = 'LARGE-PAYLOAD-DONE-E2E';

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
  // TEST-VT-LARGE-001: 1 MB injection — pane survives, no raw escapes.
  // -------------------------------------------------------------------------
  it('TEST-VT-LARGE-001: 1 MB of ASCII data processed without crash or escape leak', async () => {
    // Inject 16 × 64 KB chunks of printable ASCII.
    for (let i = 0; i < CHUNKS; i++) {
      await inject(paneId, PRINTABLE_ASCII);
    }
    // Inject sentinel after the large payload.
    await inject(paneId, `\r\n${SENTINEL}`);

    // Wait for sentinel — confirms all data was processed (not just the first chunk).
    await browser.waitUntil(
      () =>
        browser.execute((s: string): boolean => {
          const grid = document.querySelector('.terminal-grid');
          return grid !== null && (grid.textContent ?? '').includes(s);
        }, SENTINEL),
      {
        timeout: 30_000,
        timeoutMsg: `Sentinel "${SENTINEL}" did not appear after 1 MB injection`,
      },
    );

    // Grid must have at least one visible cell.
    const hasContent = await gridHasNonEmptyCell();
    expect(hasContent).toBe(true);

    // No raw escape sequences must appear in the grid.
    const hasEscapes = await gridContainsRawEscapes();
    expect(hasEscapes).toBe(false);
  });

  // -------------------------------------------------------------------------
  // TEST-VT-LARGE-002: pane is still alive (not terminated) after large injection.
  // -------------------------------------------------------------------------
  it('TEST-VT-LARGE-002: pane remains in Running state after 1 MB injection', async () => {
    const terminated = (await browser.execute((sel: string): boolean => {
      return document.querySelector(sel) !== null;
    }, Selectors.processTerminatedPane)) as boolean;

    expect(terminated).toBe(false);
  });
});
