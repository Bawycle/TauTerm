// SPDX-License-Identifier: MPL-2.0

/**
 * Reactive scroll state — scroll position per pane.
 *
 * Tracks the scroll offset and total scrollback lines for each pane.
 * Updated from scroll-position-changed events and scroll IPC responses.
 *
 * Each TerminalPane reads its own entry from this map to render the
 * scrollbar and scroll-to-bottom indicator.
 */

import type { PaneId, ScrollPositionChangedEvent } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface PaneScrollState {
  /** Lines scrolled from the bottom. 0 = at bottom. */
  offset: number;
  /** Total scrollback lines available. */
  scrollbackLines: number;
}

// ---------------------------------------------------------------------------
// Reactive state — module-level singleton
// ---------------------------------------------------------------------------

/**
 * Scroll state per pane.
 * A pane absent from this map is assumed to be at offset 0 with 0 scrollback.
 */
export const scrollState = $state<Map<PaneId, PaneScrollState>>(new Map());

// ---------------------------------------------------------------------------
// Updaters
// ---------------------------------------------------------------------------

/**
 * Apply a ScrollPositionChangedEvent to the per-pane scroll state.
 */
export function applyScrollPositionChanged(ev: ScrollPositionChangedEvent): void {
  scrollState.set(ev.paneId, {
    offset: ev.offset,
    scrollbackLines: ev.scrollbackLines,
  });
}

/**
 * Update the scroll state for a pane (e.g., after a scroll_pane command returns).
 */
export function setScrollState(paneId: PaneId, offset: number, scrollbackLines: number): void {
  scrollState.set(paneId, { offset, scrollbackLines });
}

/**
 * Remove scroll state for a pane (e.g., when the pane is closed).
 */
export function removeScrollState(paneId: PaneId): void {
  scrollState.delete(paneId);
}

// ---------------------------------------------------------------------------
// Read helpers
// ---------------------------------------------------------------------------

/**
 * Returns the scroll state for a pane, or a default (offset 0, no scrollback).
 */
export function getScrollState(paneId: PaneId): PaneScrollState {
  return scrollState.get(paneId) ?? { offset: 0, scrollbackLines: 0 };
}
