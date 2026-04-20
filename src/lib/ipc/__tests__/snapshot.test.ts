// SPDX-License-Identifier: MPL-2.0

/**
 * Unit tests for the `fetchAndAckSnapshot` helper — the sole supported
 * entry point for pane snapshot consumption (ADR-0027 Addendum 3).
 *
 * Covered:
 *   ACK-FE-009-A — happy path: calls `frame_ack` exactly once with the
 *                  correct pane id; parameterized across multiple ids.
 *   ACK-FE-009-B — error path: binding rejects → helper rejects and
 *                  `frame_ack` is NOT called (no snapshot was consumed).
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as tauriCore from '@tauri-apps/api/core';
import { fetchAndAckSnapshot } from '$lib/ipc';
import type { ScreenSnapshot } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

const VALID_SNAPSHOT: ScreenSnapshot = {
  cols: 4,
  rows: 3,
  cells: [],
  cursorRow: 0,
  cursorCol: 0,
  cursorVisible: true,
  cursorShape: 0,
  scrollbackLines: 0,
  scrollOffset: 0,
};

// ---------------------------------------------------------------------------
// Module-level spies
// ---------------------------------------------------------------------------

let invokeSpy: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  invokeSpy = vi.spyOn(tauriCore, 'invoke');
});

afterEach(() => {
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Mock invoke to emulate the raw Tauri command return values.
 *
 * The generated `commands.*` methods wrap `invoke()` in `typedError()`,
 * which resolves with `{ status: 'ok', data }` on success — so our mock
 * returns the RAW snapshot (not the envelope). The unwrap step lives
 * inside `commands.getPaneScreenSnapshot` → `typedError` → the
 * `getPaneScreenSnapshot` re-export in `./index` that calls `unwrap()`.
 *
 * `frame_ack` is a fire-and-forget that returns `null` raw.
 * Any other command rejects so a missing mock is loud, not silent.
 */
function mockHappyPath(snapshot: ScreenSnapshot): void {
  invokeSpy.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_pane_screen_snapshot') {
      return snapshot as never;
    }
    if (cmd === 'frame_ack') {
      return null as never;
    }
    throw new Error(`unexpected invoke: ${cmd}`);
  });
}

/**
 * Mock snapshot fetch to reject with a non-Error payload (matching the
 * real `typedError` contract: non-`Error` rejections become the `error`
 * field of the envelope, which `unwrap()` then `throw`s). `frame_ack`
 * stays mocked so that if the helper (incorrectly) acked on error, we
 * would still observe the invocation.
 */
function mockErrorPath(error: unknown): void {
  invokeSpy.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_pane_screen_snapshot') {
      throw error;
    }
    if (cmd === 'frame_ack') {
      return null as never;
    }
    throw new Error(`unexpected invoke: ${cmd}`);
  });
}

/** Extract invoke calls for a given command name. */
function invokesOf(cmd: string): unknown[][] {
  return invokeSpy.mock.calls.filter((c: unknown[]) => c[0] === cmd);
}

// ---------------------------------------------------------------------------
// ACK-FE-009-A — happy path
// ---------------------------------------------------------------------------

describe('ACK-FE-009-A: fetchAndAckSnapshot happy path acks exactly once', () => {
  it.each(['pane-1', 'pane-2'])('acks pane %s exactly once with the correct id', async (paneId) => {
    mockHappyPath(VALID_SNAPSHOT);

    const snapshot = await fetchAndAckSnapshot(paneId);

    expect(snapshot).toEqual(VALID_SNAPSHOT);

    const ackCalls = invokesOf('frame_ack');
    expect(ackCalls).toHaveLength(1);
    expect(ackCalls[0]).toEqual(['frame_ack', { paneId }]);

    const fetchCalls = invokesOf('get_pane_screen_snapshot');
    expect(fetchCalls).toHaveLength(1);
    expect(fetchCalls[0]).toEqual(['get_pane_screen_snapshot', { paneId }]);
  });
});

// ---------------------------------------------------------------------------
// ACK-FE-009-B — error path: NO ack when snapshot fetch rejects
// ---------------------------------------------------------------------------

describe('ACK-FE-009-B: fetchAndAckSnapshot error path does NOT ack', () => {
  it('rejects and does not call frame_ack when the snapshot binding rejects', async () => {
    const bindingError = { kind: 'session-not-found', paneId: 'pane-missing' };
    mockErrorPath(bindingError);

    await expect(fetchAndAckSnapshot('pane-missing')).rejects.toEqual(bindingError);

    const ackCalls = invokesOf('frame_ack');
    expect(ackCalls).toHaveLength(0);

    // Sanity: the fetch WAS attempted.
    const fetchCalls = invokesOf('get_pane_screen_snapshot');
    expect(fetchCalls).toHaveLength(1);
  });
});
