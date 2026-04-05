// SPDX-License-Identifier: MPL-2.0

/**
 * Test stub for @tauri-apps/api/event.
 *
 * `listen` returns a no-op unsubscribe function.
 * Individual tests can override via vi.mock or vi.spyOn.
 */

export type UnlistenFn = () => void;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function listen<T = any>(
  _event: string,
  _handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  return () => {};
}

export async function once<T = any>(
  _event: string,
  _handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  return () => {};
}

export async function emit(_event: string, _payload?: unknown): Promise<void> {}
