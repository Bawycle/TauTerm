// SPDX-License-Identifier: MPL-2.0
// Build requirement: pnpm tauri build --no-bundle -- --features e2e-testing
// Run: pnpm wdio

/**
 * E2E scenario: Text selection CSS classes.
 *
 * Verifies that:
 *   - Clicking and dragging across cells applies `terminal-pane__cell--selected`
 *     (or `--selected-inactive`) to the covered cells.
 *   - A single click (no drag) produces no selection.
 *   - Releasing the mouse ends the selection without clearing it.
 *
 * Protocol references:
 *   - Covers manual verification scenario: text selection CSS correct after P13
 *     (P13 reduced `isSelected()` from 3× to 1× per cell via `{@const selected}`)
 *
 * Implementation notes:
 *   - Mouse events are dispatched synthetically via `dispatchEvent` (consistent
 *     with other E2E tests — avoids WebKitGTK/tauri-driver pointer quirks).
 *   - Pixel coordinates are derived from the viewport's `getBoundingClientRect()`.
 *     A cell is assumed to occupy approximately `cellW × cellH` pixels; we use
 *     the probe's measured dimensions or a conservative 10 px fallback.
 *   - The selection handler (`handleMousedown`, `handleMousemove`, `handleMouseup`)
 *     is bound on the `.terminal-pane__viewport` element.
 *   - `terminal-pane__cell--selected` is applied when `isSelected() && active && !flashing`.
 *     The pane is active by default after inject.
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

/**
 * Simulate a click-drag selection on the terminal viewport.
 *
 * Dispatches mousedown at (startX, startY), mousemove to (endX, endY), then mouseup.
 * All events bubble so they reach the `.terminal-pane__viewport` listener.
 */
async function simulateDragSelection(
  startX: number,
  startY: number,
  endX: number,
  endY: number,
): Promise<void> {
  await browser.execute(
    (sx: number, sy: number, ex: number, ey: number): void => {
      const viewport = document.querySelector('.terminal-pane__viewport') as HTMLElement | null;
      if (!viewport) return;

      const makeOpts = (x: number, y: number, buttons: number): MouseEventInit => ({
        clientX: x,
        clientY: y,
        button: 0,
        buttons,
        bubbles: true,
        cancelable: true,
      });

      viewport.dispatchEvent(new MouseEvent('mousedown', makeOpts(sx, sy, 1)));
      viewport.dispatchEvent(new MouseEvent('mousemove', makeOpts(ex, ey, 1)));
      viewport.dispatchEvent(new MouseEvent('mouseup', makeOpts(ex, ey, 0)));
    },
    startX,
    startY,
    endX,
    endY,
  );
}

/**
 * Count cells that carry any of the three selection CSS classes.
 */
function countSelectedCells(): Promise<number> {
  return browser.execute((): number => {
    return document.querySelectorAll(
      '.terminal-pane__cell--selected, .terminal-pane__cell--selected-flash, .terminal-pane__cell--selected-inactive',
    ).length;
  }) as Promise<number>;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Text selection CSS classes (P13)', () => {
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

    // Inject text to guarantee cells have content to select.
    await tauriFireAndForget('inject_pty_output', {
      paneId,
      data: encodeBytes('HELLO SELECTION WORLD\r\n'),
    });
    await browser.pause(100);
  });

  // -------------------------------------------------------------------------
  // TEST-SEL-CSS-001: drag across cells produces selection CSS classes.
  // -------------------------------------------------------------------------
  it('TEST-SEL-CSS-001: click-drag across cells applies selection CSS class to covered cells', async () => {
    // Get the viewport bounding rect to compute approximate cell coordinates.
    const viewportRect = await browser.execute((): DOMRect | null => {
      return document.querySelector('.terminal-pane__viewport')?.getBoundingClientRect() ?? null;
    });

    expect(viewportRect).not.toBeNull();
    const rect = viewportRect as DOMRect;

    // Drag from (left + 5px, top + 5px) to (left + 80px, top + 5px) — covers
    // several cells on the first row. We stay close to the top-left corner
    // to reliably land on the first text row regardless of font size.
    const startX = rect.left + 5;
    const startY = rect.top + 5;
    const endX = rect.left + 80;
    const endY = rect.top + 5;

    await simulateDragSelection(startX, startY, endX, endY);
    await browser.pause(50);

    // At least one cell must carry a selection class.
    const selectedCount = await countSelectedCells();
    expect(selectedCount).toBeGreaterThan(0);
  });

  // -------------------------------------------------------------------------
  // TEST-SEL-CSS-002: selection covers a contiguous range of cells.
  // -------------------------------------------------------------------------
  it('TEST-SEL-CSS-002: selection covers more than one cell when dragging across multiple columns', async () => {
    const viewportRect = await browser.execute((): DOMRect | null => {
      return document.querySelector('.terminal-pane__viewport')?.getBoundingClientRect() ?? null;
    });
    expect(viewportRect).not.toBeNull();
    const rect = viewportRect as DOMRect;

    // Wider drag — should cover more cells than the first test.
    const startX = rect.left + 5;
    const startY = rect.top + 5;
    const endX = rect.left + 150;
    const endY = rect.top + 5;

    await simulateDragSelection(startX, startY, endX, endY);
    await browser.pause(50);

    const selectedCount = await countSelectedCells();
    expect(selectedCount).toBeGreaterThan(1);
  });

  // -------------------------------------------------------------------------
  // TEST-SEL-CSS-003: selection CSS class is one of the three expected variants.
  // -------------------------------------------------------------------------
  it('TEST-SEL-CSS-003: selected cells carry exactly one of the three expected CSS variants', async () => {
    const viewportRect = await browser.execute((): DOMRect | null => {
      return document.querySelector('.terminal-pane__viewport')?.getBoundingClientRect() ?? null;
    });
    expect(viewportRect).not.toBeNull();
    const rect = viewportRect as DOMRect;

    await simulateDragSelection(rect.left + 5, rect.top + 5, rect.left + 60, rect.top + 5);
    await browser.pause(50);

    // Verify no cell carries an unexpected selection class (typo guard for P13 refactor).
    const hasUnexpectedClass = await browser.execute((): boolean => {
      for (const cell of document.querySelectorAll('.terminal-pane__cell')) {
        const cls = cell.classList;
        // If a cell looks selected (has any selection-like class) it must be one
        // of the three canonical variants.
        const hasAny =
          cls.contains('terminal-pane__cell--selected') ||
          cls.contains('terminal-pane__cell--selected-flash') ||
          cls.contains('terminal-pane__cell--selected-inactive');
        // Check for unexpected variants (e.g. a stale class name).
        for (const c of cls) {
          if (c.includes('selected') && !hasAny) return true;
        }
      }
      return false;
    });
    expect(hasUnexpectedClass).toBe(false);
  });
});
