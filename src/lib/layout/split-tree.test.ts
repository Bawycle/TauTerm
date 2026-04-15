// SPDX-License-Identifier: MPL-2.0

import { describe, it, expect } from 'vitest';
import { computeLayouts, findPane, leafPanes } from './split-tree.js';
import type { PaneNode } from '$lib/ipc';

const ROOT_BOUNDS = { x: 0, y: 0, width: 1200, height: 800 };

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------
function leaf(id: string): PaneNode {
  return {
    type: 'leaf',
    paneId: id,
    state: {
      paneId: id,
      lifecycle: { type: 'running' },
      processTitle: 'bash',
      sshState: null,
      scrollOffset: 0,
      cwd: '/',
    },
  };
}

function horizontal(first: PaneNode, second: PaneNode, ratio = 0.5): PaneNode {
  return { type: 'split', direction: 'horizontal', ratio, first, second };
}

function vertical(first: PaneNode, second: PaneNode, ratio = 0.5): PaneNode {
  return { type: 'split', direction: 'vertical', ratio, first, second };
}

// ---------------------------------------------------------------------------
// computeLayouts
// ---------------------------------------------------------------------------
describe('computeLayouts', () => {
  it('single pane (leaf) → one layout equal to root bounds', () => {
    const root = leaf('pane-1');
    const layouts = computeLayouts(root, ROOT_BOUNDS);
    expect(layouts).toHaveLength(1);
    expect(layouts[0].paneId).toBe('pane-1');
    expect(layouts[0].bounds).toEqual(ROOT_BOUNDS);
  });

  it('two-pane horizontal split (50/50) → two layouts side by side', () => {
    const root = horizontal(leaf('left'), leaf('right'), 0.5);
    const layouts = computeLayouts(root, ROOT_BOUNDS);
    expect(layouts).toHaveLength(2);

    const [l, r] = layouts;
    expect(l.paneId).toBe('left');
    expect(r.paneId).toBe('right');

    // Both are at y=0, full height
    expect(l.bounds.y).toBe(0);
    expect(l.bounds.height).toBe(800);
    expect(r.bounds.y).toBe(0);
    expect(r.bounds.height).toBe(800);

    // Side by side: widths sum to 1200
    expect(l.bounds.x).toBe(0);
    expect(l.bounds.width + r.bounds.width).toBe(1200);
    expect(r.bounds.x).toBe(l.bounds.x + l.bounds.width);
  });

  it('two-pane vertical split (50/50) → two layouts top/bottom', () => {
    const root = vertical(leaf('top'), leaf('bottom'), 0.5);
    const layouts = computeLayouts(root, ROOT_BOUNDS);
    expect(layouts).toHaveLength(2);

    const [t, b] = layouts;
    expect(t.paneId).toBe('top');
    expect(b.paneId).toBe('bottom');

    // Both at x=0, full width
    expect(t.bounds.x).toBe(0);
    expect(t.bounds.width).toBe(1200);
    expect(b.bounds.x).toBe(0);
    expect(b.bounds.width).toBe(1200);

    // Top/bottom: heights sum to 800
    expect(t.bounds.y).toBe(0);
    expect(t.bounds.height + b.bounds.height).toBe(800);
    expect(b.bounds.y).toBe(t.bounds.y + t.bounds.height);
  });

  it('nested split (3 panes): left | (top / bottom)', () => {
    // left pane takes 40% width; right side is split top/bottom 50/50
    const root = horizontal(
      leaf('left'),
      vertical(leaf('top-right'), leaf('bottom-right'), 0.5),
      0.4,
    );
    const layouts = computeLayouts(root, ROOT_BOUNDS);
    expect(layouts).toHaveLength(3);

    const ids = layouts.map((l) => l.paneId);
    expect(ids).toEqual(['left', 'top-right', 'bottom-right']);

    const [l, tr, br] = layouts;

    // Left pane: x=0, width=floor(1200*0.4)=480
    expect(l.bounds.x).toBe(0);
    expect(l.bounds.width).toBe(480);
    expect(l.bounds.height).toBe(800);

    // Right side: x=480, width=720
    expect(tr.bounds.x).toBe(480);
    expect(tr.bounds.width).toBe(720);
    expect(br.bounds.x).toBe(480);
    expect(br.bounds.width).toBe(720);

    // Top-right and bottom-right share 800px of height
    expect(tr.bounds.y).toBe(0);
    expect(tr.bounds.height + br.bounds.height).toBe(800);
    expect(br.bounds.y).toBe(tr.bounds.height);
  });

  it('non-equal ratio allocates correctly', () => {
    const root = horizontal(leaf('a'), leaf('b'), 0.25);
    const layouts = computeLayouts(root, ROOT_BOUNDS);
    const [a, b] = layouts;
    // floor(1200 * 0.25) = 300
    expect(a.bounds.width).toBe(300);
    expect(b.bounds.width).toBe(900);
  });
});

// ---------------------------------------------------------------------------
// findPane
// ---------------------------------------------------------------------------
describe('findPane', () => {
  it('finds a leaf pane by ID (direct leaf)', () => {
    const root = leaf('pane-1');
    const found = findPane(root, 'pane-1');
    expect(found).not.toBeNull();
    expect(found!.type).toBe('leaf');
    if (found!.type === 'leaf') {
      expect(found!.paneId).toBe('pane-1');
    }
  });

  it('finds a pane nested in a split', () => {
    const root = horizontal(leaf('left'), vertical(leaf('top'), leaf('bottom')));
    expect(findPane(root, 'top')).not.toBeNull();
    expect(findPane(root, 'bottom')).not.toBeNull();
    expect(findPane(root, 'left')).not.toBeNull();
  });

  it('returns null when pane ID does not exist', () => {
    const root = horizontal(leaf('a'), leaf('b'));
    expect(findPane(root, 'nonexistent')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// leafPanes
// ---------------------------------------------------------------------------
describe('leafPanes', () => {
  it('single leaf → [leaf]', () => {
    const root = leaf('only');
    const leaves = leafPanes(root);
    expect(leaves).toHaveLength(1);
    expect(leaves[0].type).toBe('leaf');
  });

  it('two-pane split → two leaves in order (first, second)', () => {
    const root = horizontal(leaf('left'), leaf('right'));
    const leaves = leafPanes(root);
    expect(leaves).toHaveLength(2);
    expect(leaves[0].type === 'leaf' && leaves[0].paneId).toBe('left');
    expect(leaves[1].type === 'leaf' && leaves[1].paneId).toBe('right');
  });

  it('nested split → all leaves in depth-first order', () => {
    const root = horizontal(leaf('a'), vertical(leaf('b'), leaf('c')));
    const leaves = leafPanes(root);
    expect(leaves).toHaveLength(3);
    const ids = leaves.map((l) => (l.type === 'leaf' ? l.paneId : ''));
    expect(ids).toEqual(['a', 'b', 'c']);
  });
});
