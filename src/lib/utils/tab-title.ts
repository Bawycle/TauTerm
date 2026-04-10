// SPDX-License-Identifier: MPL-2.0

import type { PaneId, PaneNode, PaneState, TabState } from '$lib/ipc/types';

/**
 * Returns the root (first leaf) pane state from a tab's layout tree.
 * Traverses `first` links at each split node until a leaf is reached.
 * Returns null only if the tree is somehow empty (defensive).
 */
export function getRootPane(tab: TabState): PaneState | null {
  let node: PaneNode = tab.layout;
  while (node.type === 'split') node = node.first;
  return node.type === 'leaf' ? node.state : null;
}

/**
 * Finds a pane by ID in a PaneNode tree. Returns null if not found.
 */
export function getPaneById(node: PaneNode, paneId: PaneId): PaneState | null {
  if (node.type === 'leaf') {
    return node.paneId === paneId ? node.state : null;
  }
  return getPaneById(node.first, paneId) ?? getPaneById(node.second, paneId);
}

/**
 * Resolves the display title for a tab.
 *
 * Priority (mirrors Rust resolution chain):
 *   user label > OSC-driven processTitle of the active pane > null
 *
 * Returns null when no title is available (caller provides the i18n fallback).
 * This keeps the utility free of Paraglide dependency and independently testable.
 */
export function resolveTabTitle(tab: TabState): string | null {
  if (tab.label !== null) return tab.label;
  const pane = getPaneById(tab.layout, tab.activePaneId);
  const title = pane?.label || pane?.processTitle;
  return title != null && title.length > 0 ? title : null;
}
