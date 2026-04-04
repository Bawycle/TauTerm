// SPDX-License-Identifier: MPL-2.0

/**
 * Test stub for @tauri-apps/api/core.
 *
 * Replaces `invoke` with a no-op that resolves immediately.  Individual test
 * files can override this via `vi.mock` or `vi.spyOn` when they need to
 * control the return value.
 *
 * Do NOT import this file in production code — it is resolved only via the
 * vitest `alias` configuration in vitest.config.ts.
 */

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function invoke<T = void>(_command: string, _args?: Record<string, any>): Promise<T> {
  return undefined as unknown as T;
}
