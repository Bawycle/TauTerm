// SPDX-License-Identifier: MPL-2.0
// Shared event registry infrastructure for composable tests

import type { ScreenUpdateEvent, CursorState } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ListenerFn<T = unknown> = (event: { event: string; id: number; payload: T }) => void;
export type ListenerRegistry = Map<string, Array<ListenerFn>>;

// ---------------------------------------------------------------------------
// Factory: create a fresh listener registry
// ---------------------------------------------------------------------------

export function createListenerRegistry(): ListenerRegistry {
  return new Map();
}

// ---------------------------------------------------------------------------
// Factory: create a fireEvent helper bound to a registry
// ---------------------------------------------------------------------------

export function createFireEvent(
  registry: ListenerRegistry,
): <T>(eventName: string, payload: T) => void {
  return function fireEvent<T>(eventName: string, payload: T): void {
    const handlers = registry.get(eventName) ?? [];
    for (const h of handlers) {
      (h as ListenerFn<T>)({ event: eventName, id: 0, payload });
    }
  };
}

// ---------------------------------------------------------------------------
// Helper: build minimal ScreenUpdateEvent fixtures
// ---------------------------------------------------------------------------

export function makeScreenUpdate(
  paneId: string,
  overrides: Partial<ScreenUpdateEvent> = {},
): ScreenUpdateEvent {
  const cursor: CursorState = { row: 0, col: 0, visible: true, shape: 0, blink: true };
  return {
    paneId,
    cells: [],
    cursor,
    scrollbackLines: 0,
    isFullRedraw: false,
    scrollOffset: 0,
    cols: 80,
    rows: 24,
    ...overrides,
  };
}
