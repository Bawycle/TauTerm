// SPDX-License-Identifier: MPL-2.0

/**
 * Snapshot consumption helper — `fetchAndAckSnapshot`.
 *
 * The ONLY supported entry point for pane screen snapshot consumption.
 * Calling `commands.getPaneScreenSnapshot` (or the unwrapped
 * `getPaneScreenSnapshot` re-export from `./index`) directly bypasses the
 * frame-ack contract (ADR-0027 Addendum 3) and can trigger false drop-mode
 * activation under adverse timing (dev mode, GC stalls, IO pressure).
 *
 * Two existing call sites rely on this helper, each solving a different
 * root cause:
 *
 *   - `onMount` in `useTerminalPane.svelte.ts` — handles the pre-attach race:
 *     the first backend `screen-update` can be emitted before the frontend's
 *     `onScreenUpdate` listener is attached (reliably reproducible in dev
 *     mode with slow Vite module loading; theoretically present in prod
 *     under GC/IO stalls).
 *
 *   - `triggerSnapshotRefetch` in `useTerminalPane.svelte.ts` — handles
 *     ack-starvation during the async snapshot fetch: the backend's ack
 *     timer keeps aging while the fetch is in flight.
 *
 * Mirrors `flushRafQueue`'s ack-on-paint contract: a snapshot is a
 * coalesced equivalent of the `screen-update` events preceding its
 * capture, so it "owes" an ack just like any `screen-update` would.
 *
 * Returns the snapshot or throws (same contract as the underlying binding).
 * On error (binding reject), NO ack is sent — we did not consume a snapshot.
 */

import { getPaneScreenSnapshot, frameAck } from './index';
import type { PaneId, ScreenSnapshot } from './bindings';

export async function fetchAndAckSnapshot(paneId: PaneId): Promise<ScreenSnapshot> {
  const snapshot = await getPaneScreenSnapshot(paneId);
  // P-HT-6: ack upon snapshot receipt (ADR-0027 Addendum 3).
  frameAck(paneId);
  return snapshot;
}
