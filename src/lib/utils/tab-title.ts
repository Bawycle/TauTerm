// SPDX-License-Identifier: MPL-2.0

import type { PaneNode, PaneState, TabState } from '$lib/ipc/types';

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
 * Resolves the display title for a tab.
 *
 * Priority (mirrors Rust resolution chain):
 *   user label > OSC-driven processTitle > null
 *
 * Returns null when no title is available (caller provides the i18n fallback).
 * This keeps the utility free of Paraglide dependency and independently testable.
 */
export function resolveTabTitle(tab: TabState): string | null {
  if (tab.label !== null) return tab.label;
  const processTitle = getRootPane(tab)?.processTitle;
  return processTitle != null && processTitle.length > 0 ? processTitle : null;
}
