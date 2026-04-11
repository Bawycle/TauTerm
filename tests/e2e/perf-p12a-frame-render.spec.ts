// SPDX-License-Identifier: MPL-2.0
// Build requirement: pnpm tauri build --no-bundle -- --features e2e-testing
// Run: pnpm wdio --spec tests/e2e/perf-p12a-frame-render.spec.ts

/**
 * P12a performance benchmark — tauterm:frameRender
 *
 * Measures the wall-clock time from "start of applyScreenUpdate()" to
 * "Svelte has committed all DOM mutations" using the performance.mark/measure
 * instrumentation added in P12a (tauterm:asu:start → tauterm:render:end).
 *
 * Two workloads:
 *
 *   SCROLL — 30 batches × 12 lines, each batch triggers one screen-update
 *   event. Lines scroll the viewport; most cells in dirty rows change.
 *   This is the "high-churn" scenario (logs, watch output).
 *
 *   CURSOR-UPDATE — 60 individual cursor-addressed updates, each changing
 *   only 4–8 cells in one row via ESC[row;colH. This is the "sparse-update"
 *   scenario (htop, ncurses dashboards) where P12a's cell-level granularity
 *   has the largest effect.
 *
 * Output: avg / p95 / max frame render time printed to the Mocha reporter.
 * Results are non-asserting (no pass/fail threshold) — this file is a
 * measurement tool, not a regression gate.
 *
 * To compare before/after P12a:
 *   1. Run on current branch (post-P12a) → "après" column
 *   2. Temporarily revert the differential else-block in applyScreenUpdate()
 *      to WP3c (Array.from row rebuild), rebuild binary, run again → "avant"
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

/** Clear all tauterm:* performance entries to start fresh. */
async function clearPerfEntries(): Promise<void> {
  await browser.execute((): void => {
    performance.clearMeasures('tauterm:frameRender');
    performance.clearMeasures('tauterm:applyScreenUpdate');
    performance.clearMarks('tauterm:asu:start');
    performance.clearMarks('tauterm:render:end');
  });
}

/** Read tauterm:frameRender durations from the browser performance timeline. */
async function readFrameRenderDurations(): Promise<number[]> {
  return (await browser.execute((): number[] => {
    return performance
      .getEntriesByName('tauterm:frameRender')
      .map((e) => e.duration);
  })) as number[];
}

function stats(durations: number[]): { avg: number; p95: number; max: number; n: number } {
  if (durations.length === 0) return { avg: 0, p95: 0, max: 0, n: 0 };
  const sorted = [...durations].sort((a, b) => a - b);
  const avg = sorted.reduce((s, x) => s + x, 0) / sorted.length;
  const p95 = sorted[Math.floor(sorted.length * 0.95)] ?? sorted[sorted.length - 1];
  const max = sorted[sorted.length - 1];
  return { avg, p95, max, n: sorted.length };
}

function report(label: string, durations: number[]): void {
  const s = stats(durations);
  console.log(
    `[P12a bench] ${label}: n=${s.n}  avg=${s.avg.toFixed(2)} ms  p95=${s.p95.toFixed(2)} ms  max=${s.max.toFixed(2)} ms`,
  );
}

// ---------------------------------------------------------------------------
// Content generators
// ---------------------------------------------------------------------------

/** Generate N lines of colorized log-like output (scroll workload). */
function scrollLines(count: number, offset: number): string {
  let out = '';
  for (let i = 0; i < count; i++) {
    const n = offset + i;
    out +=
      `\x1b[32m${String(n).padStart(4)}\x1b[0m ` +
      `\x1b[33m[INFO]\x1b[0m ` +
      `\x1b[36mcomponent\x1b[0m ` +
      `Processing entry ${n} — status \x1b[32mOK\x1b[0m ` +
      `counter=\x1b[35m${n % 999}\x1b[0m\r\n`;
  }
  return out;
}

/**
 * Build a cursor-addressed update: move cursor to (row, col) and write a
 * short colored value. Simulates ncurses/htop style in-place cell updates.
 */
function cursorUpdate(row: number, col: number, value: string, colorCode: number): string {
  return `\x1b[${row};${col}H\x1b[${colorCode}m${value}\x1b[0m`;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('P12a benchmark — tauterm:frameRender (measurement only, no assertions)', () => {
  let paneId: string;

  before(async () => {
    // Dismiss any lingering confirmation dialogs from previous suites.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="close-confirm-cancel"]',
      );
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
      { timeout: 15_000, timeoutMsg: 'Active terminal pane did not appear' },
    );

    const rawId = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(rawId).toBeTruthy();
    paneId = rawId as string;

    // Warm up: one screen clear + a few lines to initialize the grid state
    // before measuring. This avoids cold-start overhead skewing the results.
    await inject(paneId, '\x1b[2J\x1b[H'); // ED2 + cursor home → isFullRedraw: true
    await browser.pause(50);
    await inject(paneId, scrollLines(5, 0));
    await browser.pause(50);
  });

  // -------------------------------------------------------------------------
  // Workload 1 — SCROLL: many lines scrolling (high-churn, many cells/row)
  //
  // Each batch injects LINES_PER_BATCH lines that scroll into the viewport.
  // The VT pipeline processes the batch in one debounce window, producing
  // one screen-update event with many dirty rows (scrolled content).
  // -------------------------------------------------------------------------

  it('PERF-P12A-001: scroll workload — 30 batches × 12 lines', async () => {
    const BATCHES = 30;
    const LINES_PER_BATCH = 12;
    // 20 ms > 12 ms debounce — each batch produces a separate screen-update event.
    const INTER_BATCH_MS = 20;

    await clearPerfEntries();

    for (let b = 0; b < BATCHES; b++) {
      await inject(paneId, scrollLines(LINES_PER_BATCH, b * LINES_PER_BATCH));
      await browser.pause(INTER_BATCH_MS);
    }

    // Wait for the final batch to be rendered.
    await browser.pause(100);

    const durations = await readFrameRenderDurations();
    report('SCROLL  (30×12 lines)', durations);

    // Non-asserting: print result, never fail.
    expect(durations.length).toBeGreaterThan(0);
  });

  // -------------------------------------------------------------------------
  // Workload 2 — CURSOR-UPDATE: sparse in-place updates (few cells/row)
  //
  // Each update moves the cursor to a specific row/col and writes 4–8 chars.
  // This is the scenario most representative of htop/ncurses TUI apps.
  // P12a's cell-level granularity has the largest effect here because only
  // the updated cells are invalidated, not the entire row.
  // -------------------------------------------------------------------------

  it('PERF-P12A-002: cursor-update workload — 60 sparse in-place updates', async () => {
    const UPDATES = 60;
    const INTER_UPDATE_MS = 20;

    // Initialize a stable screen first (simulates a TUI frame already drawn).
    await inject(paneId, '\x1b[2J\x1b[H');
    await browser.pause(50);
    // Write a static "frame" filling rows 1–15.
    let frame = '';
    for (let r = 1; r <= 15; r++) {
      frame += `\x1b[${r};1H` + `Row ${r}`.padEnd(40, ' ');
    }
    await inject(paneId, frame);
    await browser.pause(50);

    await clearPerfEntries();

    // Now inject sparse updates: move cursor to a cell and change a few chars.
    for (let i = 0; i < UPDATES; i++) {
      const row = (i % 10) + 2; // rows 2–11
      const col = 20 + (i % 6) * 4; // cols 20, 24, 28, 32, 36, 40
      const val = String(i * 7 + 13).padStart(4); // 4-char numeric value
      const color = 31 + (i % 7); // cycle through 7 colors
      await inject(paneId, cursorUpdate(row, col, val, color));
      await browser.pause(INTER_UPDATE_MS);
    }

    await browser.pause(100);

    const durations = await readFrameRenderDurations();
    report('CURSOR  (60 sparse updates)', durations);

    expect(durations.length).toBeGreaterThan(0);
  });

  // -------------------------------------------------------------------------
  // Workload 3 — IDLE: no output for 1 s (verify no spurious renders)
  // -------------------------------------------------------------------------

  it('PERF-P12A-003: idle workload — no output for 1 s (expect 0 renders)', async () => {
    await clearPerfEntries();
    await browser.pause(1_000);
    const durations = await readFrameRenderDurations();
    report('IDLE    (1 s no output)', durations);
    // Idle should not produce any frame renders.
    expect(durations.length).toBe(0);
  });
});
