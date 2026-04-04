// SPDX-License-Identifier: MPL-2.0

/**
 * Frontend mirror of the Rust `PaneNode` layout tree with pixel-bound computation.
 *
 * Types match the IPC contract in `src/lib/ipc/types.ts` exactly:
 *  - `PaneNode` is a discriminated union with `type: 'leaf' | 'split'`.
 *  - `SplitDirection` matches the `direction` field: 'horizontal' | 'vertical'.
 *
 * Layout computation:
 *  - `horizontal` split divides the width along the X axis (two panes side by side).
 *  - `vertical` split divides the height along the Y axis (two panes top/bottom).
 *  - The `ratio` field allocates `ratio * totalSpace` to `first`, the rest to `second`.
 */

import type { PaneNode, PaneId } from '$lib/ipc/types.js';

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PaneLayout {
  /** The leaf pane ID. */
  paneId: PaneId;
  /** Pixel bounds of this pane within the terminal view. */
  bounds: Rect;
}

/**
 * Recursively compute pixel bounds for every leaf pane in the tree.
 *
 * @param root - The root of the pane layout tree (from IPC).
 * @param bounds - The total pixel area available to `root`.
 * @returns An array of `PaneLayout` entries for every leaf, in depth-first order.
 */
export function computeLayouts(root: PaneNode, bounds: Rect): PaneLayout[] {
  if (root.type === 'leaf') {
    return [{ paneId: root.paneId, bounds: { ...bounds } }];
  }

  // Split node: divide the bounds by direction using ratio
  const { direction, ratio, first, second } = root;

  if (direction === 'horizontal') {
    // Side-by-side: divide width
    const firstWidth = Math.floor(bounds.width * ratio);
    const secondWidth = bounds.width - firstWidth;

    const firstBounds: Rect = {
      x: bounds.x,
      y: bounds.y,
      width: firstWidth,
      height: bounds.height,
    };
    const secondBounds: Rect = {
      x: bounds.x + firstWidth,
      y: bounds.y,
      width: secondWidth,
      height: bounds.height,
    };

    return [...computeLayouts(first, firstBounds), ...computeLayouts(second, secondBounds)];
  } else {
    // Top-bottom: divide height
    const firstHeight = Math.floor(bounds.height * ratio);
    const secondHeight = bounds.height - firstHeight;

    const firstBounds: Rect = {
      x: bounds.x,
      y: bounds.y,
      width: bounds.width,
      height: firstHeight,
    };
    const secondBounds: Rect = {
      x: bounds.x,
      y: bounds.y + firstHeight,
      width: bounds.width,
      height: secondHeight,
    };

    return [...computeLayouts(first, firstBounds), ...computeLayouts(second, secondBounds)];
  }
}

/**
 * Find a pane node by its `paneId` anywhere in the tree.
 * Returns the first matching leaf node, or `null` if not found.
 */
export function findPane(root: PaneNode, paneId: PaneId): PaneNode | null {
  if (root.type === 'leaf') {
    return root.paneId === paneId ? root : null;
  }
  return findPane(root.first, paneId) ?? findPane(root.second, paneId);
}

/**
 * Return all leaf pane nodes in the tree, in depth-first (left-then-right) order.
 */
export function leafPanes(root: PaneNode): PaneNode[] {
  if (root.type === 'leaf') {
    return [root];
  }
  return [...leafPanes(root.first), ...leafPanes(root.second)];
}
