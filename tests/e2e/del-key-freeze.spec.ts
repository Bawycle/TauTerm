// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Del key freeze regression — OUTPUT PIPELINE ONLY.
 *
 * User-reported bug: pressing Del freezes the pane; screen-update events
 * stop flowing. Prior diagnostics:
 *   - Frontend events OK (keydown/input fire with correct data)
 *   - `send_input` IPC returns Ok after freeze
 *   - VT state IS updated after freeze (Task 1 alive)
 *   - But screen-update events stop reaching the frontend (Task 2 silent)
 *
 * These tests use the **InjectablePtyBackend** (E2E build) — which means
 * `send_input` cannot be tested here (the backend leaves the pane in a
 * non-Running lifecycle, so input IPC is rejected with PANE_NOT_RUNNING).
 * We focus exclusively on the OUTPUT pipeline: bytes injected via
 * `inject_pty_output` traverse the VT parser and emit path just like
 * real bash output would.
 *
 * If any of these tests fails — if injected bytes stop appearing on the
 * grid after a specific pattern — we've isolated the bug to that pattern.
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

/** Fire-and-forget Tauri invoke (no return value). */
function invokeFire(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as any).__TAURI_INTERNALS__.invoke(cmdArg, argsArg);
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

async function getGridText(): Promise<string> {
  return (await browser.execute((): string => {
    const grid = document.querySelector('.terminal-grid');
    return grid ? (grid.textContent ?? '') : '';
  })) as string;
}

async function waitForGridText(needle: string, timeoutMs: number, hint: string): Promise<void> {
  await browser.waitUntil(async () => (await getGridText()).includes(needle), {
    timeout: timeoutMs,
    timeoutMsg: `Grid never contained "${needle}" — ${hint}`,
  });
}

/** Inject a UTF-8 string as PTY output. */
async function injectText(paneId: string, text: string): Promise<void> {
  await invokeFire('inject_pty_output', {
    paneId,
    data: [...new TextEncoder().encode(text)],
  });
}

/** Inject raw bytes as PTY output. */
async function injectBytes(paneId: string, bytes: number[]): Promise<void> {
  await invokeFire('inject_pty_output', { paneId, data: bytes });
}

describe('TauTerm — Del key freeze regression (output pipeline)', () => {
  let paneId: string;

  before(async () => {
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 15_000, timeoutMsg: 'Active pane did not appear' },
    );
    const id = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(id).toBeTruthy();
    paneId = id!;
  });

  /**
   * DEL-E2E-001 — A bare BEL does NOT break the output pipeline.
   *
   * Injects: BEFORE → BEL → AFTER. Both markers must render.
   * Status: baseline — if this fails, the bug is triggered by BEL alone.
   */
  it('DEL-E2E-001: screen updates continue after a lone BEL byte', async () => {
    await injectText(paneId, '\r\nDEL_E2E_001_BEFORE ');
    await waitForGridText('DEL_E2E_001_BEFORE', 5_000, 'initial output');

    await injectBytes(paneId, [0x07]); // BEL

    await injectText(paneId, 'DEL_E2E_001_AFTER');
    await waitForGridText(
      'DEL_E2E_001_AFTER',
      5_000,
      'post-BEL output — if this times out, BEL breaks the emitter',
    );
  });

  /**
   * DEL-E2E-002 — A CSI 3~ sequence as OUTPUT does not break the pipeline.
   *
   * Unusual scenario: the terminal receives CSI 3~ as output (e.g. if the
   * shell is in raw mode and echoes the Del keycode verbatim). This hits
   * the VT parser's "unknown CSI" branch — the dispatch.rs fall-through
   * `_ => {}` case. Verify the parser doesn't get stuck in a CSI-
   * collecting state after this.
   */
  it('DEL-E2E-002: CSI 3~ as output does not break the pipeline', async () => {
    await injectText(paneId, '\r\nDEL_E2E_002_BEFORE ');
    await waitForGridText('DEL_E2E_002_BEFORE', 5_000, 'initial');

    // ESC [ 3 ~ — the "Delete" keycode as output. Not a standard command.
    await injectBytes(paneId, [0x1b, 0x5b, 0x33, 0x7e]);

    await injectText(paneId, 'DEL_E2E_002_AFTER');
    await waitForGridText(
      'DEL_E2E_002_AFTER',
      5_000,
      'post-CSI output — if this times out, CSI 3~ in output breaks VT parser',
    );
  });

  /**
   * DEL-E2E-003 — BEL + typical bash post-Del redraw does not break.
   *
   * When real bash processes a Del that actually deletes a character, it
   * emits: DCH (delete char) or a line redraw. This test exercises one of
   * the common patterns.
   */
  it('DEL-E2E-003: BEL followed by a redraw sequence survives', async () => {
    await injectText(paneId, '\r\nDEL_E2E_003_BEFORE ');
    await waitForGridText('DEL_E2E_003_BEFORE', 5_000, 'initial');

    // BEL (Del no-op) + CR + EL (erase to EOL) + redraw text.
    // This mirrors what some shell configs emit on Del.
    await injectBytes(paneId, [0x07, 0x0d]); // BEL + CR
    await injectBytes(paneId, [0x1b, 0x5b, 0x4b]); // CSI K (EL)

    await injectText(paneId, 'DEL_E2E_003_AFTER');
    await waitForGridText(
      'DEL_E2E_003_AFTER',
      5_000,
      'post-redraw output — if this times out, the BEL+CR+EL sequence ' + 'breaks the emitter',
    );
  });

  /**
   * DEL-E2E-004 — Many successive BELs don't accumulate into a freeze.
   *
   * Rings the bell 20 times with printable output between each, verifies
   * all markers render. If the emitter slowly degrades with repeated
   * bell events (e.g. emit_bell_triggered dispatch becomes blocking),
   * this test surfaces it.
   */
  it('DEL-E2E-004: repeated BELs do not accumulate into a freeze', async () => {
    for (let i = 0; i < 20; i++) {
      await injectBytes(paneId, [0x07]); // BEL
      await injectText(paneId, `\r\nDEL_E2E_004_${i} `);
      await waitForGridText(
        `DEL_E2E_004_${i}`,
        3_000,
        `iteration ${i} — bell #${i + 1} broke the emitter`,
      );
    }
  });

  /**
   * DEL-E2E-005 — BEL interleaved with CR/LF and wrapping content.
   *
   * Stress test: the user's real-world bash output includes prompts,
   * command lines with wrapping, cursor movements. Combine these with
   * BELs to see if a specific interleaving triggers the bug.
   */
  it('DEL-E2E-005: interleaved BEL + wrapping content stays responsive', async () => {
    // Fill a line close to the right edge.
    const long = 'x'.repeat(60);
    await injectText(paneId, `\r\nDEL_E2E_005_${long}`);
    await waitForGridText('DEL_E2E_005_', 5_000, 'long line');

    // BEL after reaching end of line.
    await injectBytes(paneId, [0x07]);

    // Backspace + more content (user correction pattern).
    await injectBytes(paneId, [0x08, 0x20, 0x08]); // BS SP BS

    await injectText(paneId, 'DEL_E2E_005_TAIL');
    await waitForGridText('DEL_E2E_005_TAIL', 5_000, 'post-BEL + backspace correction');
  });

  /**
   * DEL-E2E-006 — BEL + idle pause >1.2s + text: no freeze (drop-mode guard).
   *
   * This is the critical regression gate for the frame-ack `last_emit_ms`
   * gating fix (ADR-0027 Addendum 2). Before the fix, a lone BEL emission
   * would advance `last_emit_ms` in reader.rs even though `screen-update`
   * was not emitted — which means the frontend never sent `frame_ack`. After
   * a pause >1s, `has_unacked_emits=true` ∧ `ack_age>ACK_DROP_THRESHOLD_MS`
   * would flip the pane into drop-mode, silently discarding the next dirty
   * cells forever (freeze).
   *
   * After the fix: BEL does NOT advance `last_emit_ms` (only `screen-update`
   * emissions do), so no drop-mode arms, and the post-pause text renders.
   *
   * If this test times out, the fix is broken (Del-key freeze is back).
   */
  it('DEL-E2E-006: BEL + 1.2s idle pause + text does not freeze pane', async () => {
    await injectText(paneId, '\r\nDEL_E2E_006_BEFORE ');
    await waitForGridText('DEL_E2E_006_BEFORE', 5_000, 'initial output');

    // Lone BEL — non-visual event, must NOT arm drop-mode / stale-mode.
    await injectBytes(paneId, [0x07]);

    // Idle pause longer than ACK_DROP_THRESHOLD_MS (1000 ms in ADR-0027).
    // Before the fix, this is where the pane enters drop-mode permanently.
    await browser.pause(1200);

    // Post-pause text — before the fix this would be silently dropped.
    await injectText(paneId, 'DEL_E2E_006_AFTER');
    await waitForGridText(
      'DEL_E2E_006_AFTER',
      3_000,
      'post-pause output after lone BEL — if this times out, the ' +
        '`last_emit_ms` gating fix is broken and drop-mode silently ' +
        'discards dirty cells (Del-key freeze regression)',
    );
  });

  /**
   * DEL-E2E-007 — OSC 2 title + idle pause >1.2s + text: no freeze.
   *
   * Real-world scenario: Starship / Powerlevel10k / oh-my-zsh emit an
   * OSC 2 title-change sequence (`ESC ] 2 ; NewTitle BEL`) at every prompt.
   * The OSC terminator is a BEL (0x07) in the common form, which exercises
   * the same non-visual emit path as DEL-E2E-006. If the `last_emit_ms`
   * gating fix is incomplete for OSC emissions, every prompt in these
   * shells would degrade latency and eventually freeze the pane.
   *
   * OSC 2 produces `ProcessOutput { new_title: Some(..), dirty:empty }` —
   * same pattern as bell. Must not arm drop-mode.
   */
  it('DEL-E2E-007: OSC 2 title change + 1.2s pause + text does not freeze', async () => {
    await injectText(paneId, '\r\nDEL_E2E_007_BEFORE ');
    await waitForGridText('DEL_E2E_007_BEFORE', 5_000, 'initial output');

    // ESC ] 2 ; NewTitle BEL — standard xterm title-set sequence.
    const titleSeq = [0x1b, 0x5d, 0x32, 0x3b]; // ESC ] 2 ;
    const titleBody = [...new TextEncoder().encode('NewTitle')];
    const titleEnd = [0x07]; // BEL terminator
    await injectBytes(paneId, [...titleSeq, ...titleBody, ...titleEnd]);

    await browser.pause(1200);

    await injectText(paneId, 'DEL_E2E_007_AFTER');
    await waitForGridText(
      'DEL_E2E_007_AFTER',
      3_000,
      'post-pause output after OSC 2 title — if this times out, ' +
        'Starship/Powerlevel10k prompts will freeze panes',
    );
  });

  /**
   * DEL-E2E-008 — OSC 7 CWD + idle pause >1.2s + text: no freeze.
   *
   * Real-world scenario: many shells emit an OSC 7 sequence
   * (`ESC ] 7 ; file:///path BEL`) to report the current working directory
   * to the terminal (used for "new tab with same CWD"). Like OSC 2, this
   * is a non-visual event that produces `ProcessOutput { new_cwd: Some(..),
   * dirty:empty }` — must not arm drop-mode.
   */
  it('DEL-E2E-008: OSC 7 CWD + 1.2s pause + text does not freeze', async () => {
    await injectText(paneId, '\r\nDEL_E2E_008_BEFORE ');
    await waitForGridText('DEL_E2E_008_BEFORE', 5_000, 'initial output');

    // ESC ] 7 ; file:///tmp BEL
    const cwdSeq = [0x1b, 0x5d, 0x37, 0x3b]; // ESC ] 7 ;
    const cwdBody = [...new TextEncoder().encode('file:///tmp')];
    const cwdEnd = [0x07]; // BEL terminator
    await injectBytes(paneId, [...cwdSeq, ...cwdBody, ...cwdEnd]);

    await browser.pause(1200);

    await injectText(paneId, 'DEL_E2E_008_AFTER');
    await waitForGridText(
      'DEL_E2E_008_AFTER',
      3_000,
      'post-pause output after OSC 7 CWD — if this times out, any ' +
        'shell emitting CWD reports will freeze panes',
    );
  });
});
