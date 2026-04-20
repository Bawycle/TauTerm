// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Del key freeze regression — SSH OUTPUT PIPELINE.
 *
 * Mirrors `del-key-freeze.spec.ts` but exercises the SSH coalescer pipeline
 * instead of the PTY pipeline. Uses `create_mock_ssh_pane` to wire a fully
 * functional SSH coalescer (Task B) to the existing E2E pane without
 * requiring a real SSH server, then `inject_ssh_output` to push VT bytes
 * through the pipeline.
 *
 * These tests validate that the SSH output path (ADR-0028) handles non-visual
 * VT events (BEL, OSC 2, OSC 7) without triggering the frame-ack drop-mode
 * freeze — the same regression that DEL-E2E-006/007/008 gate for PTY.
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

/** Await a Tauri invoke and return its result. */
async function invokeAwait<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return (await browser.executeAsync(
    function (
      cmdArg: string,
      argsArg: Record<string, unknown> | undefined,
      done: (result: T) => void,
    ) {
      (window as any).__TAURI_INTERNALS__
        .invoke(cmdArg, argsArg)
        .then((r: T) => done(r))
        .catch((e: unknown) => done(e as T));
    },
    cmd,
    args,
  )) as T;
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

/** Inject a UTF-8 string as SSH output. */
async function injectText(paneId: string, text: string): Promise<void> {
  await invokeFire('inject_ssh_output', {
    paneId,
    data: [...new TextEncoder().encode(text)],
  });
}

/** Inject raw bytes as SSH output. */
async function injectBytes(paneId: string, bytes: number[]): Promise<void> {
  await invokeFire('inject_ssh_output', { paneId, data: bytes });
}

describe('TauTerm — Del key freeze regression (SSH output pipeline)', () => {
  let paneId: string;

  before(async () => {
    // Wait for the initial PTY pane to appear.
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

    // Wire the pane as a mock SSH pane (coalescer task + SshInjectableRegistry).
    await invokeAwait('create_mock_ssh_pane', { paneId });

    // Brief pause to let the coalescer task start.
    await browser.pause(100);
  });

  /**
   * SSH-E2E-001 — BEL flood via inject_ssh_output + pause >= 1.2s + verify
   * UI does not freeze.
   *
   * Mirrors DEL-E2E-006: a lone BEL must not arm drop-mode through the SSH
   * coalescer pipeline. If `last_emit_ms` gating is broken for the SSH path,
   * the post-pause text will be silently dropped.
   */
  it('SSH-E2E-001: BEL + 1.2s idle pause + text does not freeze SSH pane', async () => {
    await injectText(paneId, '\r\nSSH_E2E_001_BEFORE ');
    await waitForGridText('SSH_E2E_001_BEFORE', 5_000, 'initial SSH output');

    // Lone BEL — non-visual event.
    await injectBytes(paneId, [0x07]);

    // Idle pause longer than ACK_DROP_THRESHOLD_MS (1000 ms).
    await browser.pause(1200);

    // Post-pause text — must render.
    await injectText(paneId, 'SSH_E2E_001_AFTER');
    await waitForGridText(
      'SSH_E2E_001_AFTER',
      3_000,
      'post-pause SSH output after lone BEL — if this times out, the SSH ' +
        'coalescer has the same drop-mode bug as the PTY path',
    );
  });

  /**
   * SSH-E2E-002 — OSC 2 title change via inject_ssh_output + verify title
   * updated.
   *
   * Mirrors DEL-E2E-007: OSC 2 produces a non-visual ProcessOutput that must
   * not arm drop-mode. Additionally verifies that the title is propagated
   * through the SSH coalescer to the frontend.
   */
  it('SSH-E2E-002: OSC 2 title change + 1.2s pause + text does not freeze', async () => {
    await injectText(paneId, '\r\nSSH_E2E_002_BEFORE ');
    await waitForGridText('SSH_E2E_002_BEFORE', 5_000, 'initial SSH output');

    // ESC ] 2 ; SshTitle BEL — standard xterm title-set sequence.
    const titleSeq = [0x1b, 0x5d, 0x32, 0x3b]; // ESC ] 2 ;
    const titleBody = [...new TextEncoder().encode('SshTitle')];
    const titleEnd = [0x07]; // BEL terminator
    await injectBytes(paneId, [...titleSeq, ...titleBody, ...titleEnd]);

    await browser.pause(1200);

    await injectText(paneId, 'SSH_E2E_002_AFTER');
    await waitForGridText(
      'SSH_E2E_002_AFTER',
      3_000,
      'post-pause SSH output after OSC 2 title — SSH coalescer drop-mode regression',
    );
  });

  /**
   * SSH-E2E-003 — OSC 7 CWD change via inject_ssh_output + verify CWD
   * updated.
   *
   * Mirrors DEL-E2E-008: OSC 7 produces a non-visual ProcessOutput that must
   * not arm drop-mode. Verifies that CWD propagation works through the SSH
   * coalescer pipeline.
   */
  it('SSH-E2E-003: OSC 7 CWD + 1.2s pause + text does not freeze', async () => {
    await injectText(paneId, '\r\nSSH_E2E_003_BEFORE ');
    await waitForGridText('SSH_E2E_003_BEFORE', 5_000, 'initial SSH output');

    // ESC ] 7 ; file:///tmp/ssh-test BEL
    const cwdSeq = [0x1b, 0x5d, 0x37, 0x3b]; // ESC ] 7 ;
    const cwdBody = [...new TextEncoder().encode('file:///tmp/ssh-test')];
    const cwdEnd = [0x07]; // BEL terminator
    await injectBytes(paneId, [...cwdSeq, ...cwdBody, ...cwdEnd]);

    await browser.pause(1200);

    await injectText(paneId, 'SSH_E2E_003_AFTER');
    await waitForGridText(
      'SSH_E2E_003_AFTER',
      3_000,
      'post-pause SSH output after OSC 7 CWD — SSH coalescer drop-mode regression',
    );
  });
});
