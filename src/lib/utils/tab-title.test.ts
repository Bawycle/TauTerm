// SPDX-License-Identifier: MPL-2.0

/**
 * Tests for getPaneById and resolveTabTitle utilities.
 *
 * Covered:
 *   getPaneById — leaf matching, split traversal, deep nesting, not-found
 *   resolveTabTitle (single pane, non-regression) — user label, processTitle, empty
 *   resolveTabTitle (multi-pane, FS-PANE-007) — follows activePaneId, not root pane
 */

import { describe, it, expect } from 'vitest';
import type { PaneNode, PaneState, TabState } from '$lib/ipc';
import { getPaneById, resolveTabTitle } from '$lib/utils/tab-title';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makePane(id: string, processTitle: string, label?: string | null): PaneState {
  return {
    paneId: id,
    lifecycle: { type: 'running' },
    processTitle,
    label: label ?? undefined,
    sshState: null,
    scrollOffset: 0,
    cwd: '/home/user',
  };
}

function makeLeaf(paneId: string, processTitle: string): PaneNode {
  return { type: 'leaf', paneId, state: makePane(paneId, processTitle) };
}

function makeSplit(first: PaneNode, second: PaneNode): PaneNode {
  return { type: 'split', direction: 'horizontal', ratio: 0.5, first, second };
}

function makeTabState(
  layout: PaneNode,
  activePaneId: string,
  label: string | null = null,
): TabState {
  return {
    id: 'tab-1',
    label,
    activePaneId,
    order: 0,
    layout,
  };
}

// ---------------------------------------------------------------------------
// describe('getPaneById')
// ---------------------------------------------------------------------------

describe('getPaneById', () => {
  it('returns state for a matching leaf node', () => {
    const leaf = makeLeaf('pane-a', 'bash');
    const result = getPaneById(leaf, 'pane-a');
    expect(result).not.toBeNull();
    expect(result!.paneId).toBe('pane-a');
    expect(result!.processTitle).toBe('bash');
  });

  it('returns null for a non-matching leaf', () => {
    const leaf = makeLeaf('pane-a', 'bash');
    const result = getPaneById(leaf, 'pane-x');
    expect(result).toBeNull();
  });

  it('finds the first child in a split', () => {
    const first = makeLeaf('pane-first', 'vim');
    const second = makeLeaf('pane-second', 'zsh');
    const split = makeSplit(first, second);
    const result = getPaneById(split, 'pane-first');
    expect(result).not.toBeNull();
    expect(result!.paneId).toBe('pane-first');
  });

  it('finds the second child in a split', () => {
    const first = makeLeaf('pane-first', 'vim');
    const second = makeLeaf('pane-second', 'zsh');
    const split = makeSplit(first, second);
    const result = getPaneById(split, 'pane-second');
    expect(result).not.toBeNull();
    expect(result!.paneId).toBe('pane-second');
    expect(result!.processTitle).toBe('zsh');
  });

  it('finds a deeply nested pane (3-level tree)', () => {
    // Layout: split( split( pane-A, pane-B ), pane-C )
    const paneA = makeLeaf('pane-A', 'title-A');
    const paneB = makeLeaf('pane-B', 'title-B');
    const paneC = makeLeaf('pane-C', 'title-C');
    const inner = makeSplit(paneA, paneB);
    const root = makeSplit(inner, paneC);
    expect(getPaneById(root, 'pane-A')?.paneId).toBe('pane-A');
    expect(getPaneById(root, 'pane-B')?.paneId).toBe('pane-B');
    expect(getPaneById(root, 'pane-C')?.paneId).toBe('pane-C');
  });

  it('returns null when paneId is absent from the entire tree', () => {
    const first = makeLeaf('pane-1', 'bash');
    const second = makeLeaf('pane-2', 'zsh');
    const split = makeSplit(first, second);
    expect(getPaneById(split, 'pane-ghost')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// describe('resolveTabTitle — single pane (non-regression)')
// ---------------------------------------------------------------------------

describe('resolveTabTitle — single pane (non-regression)', () => {
  it('returns user label when label !== null', () => {
    const layout = makeLeaf('pane-1', 'bash');
    const tab = makeTabState(layout, 'pane-1', 'My Custom Label');
    expect(resolveTabTitle(tab)).toBe('My Custom Label');
  });

  it('returns processTitle when no label', () => {
    const layout = makeLeaf('pane-1', 'vim ~/project/main.rs');
    const tab = makeTabState(layout, 'pane-1', null);
    expect(resolveTabTitle(tab)).toBe('vim ~/project/main.rs');
  });

  it('returns null when processTitle is empty and no label', () => {
    const layout = makeLeaf('pane-1', '');
    const tab = makeTabState(layout, 'pane-1', null);
    expect(resolveTabTitle(tab)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// describe('resolveTabTitle — multi-pane: follows activePaneId (FS-PANE-007)')
// ---------------------------------------------------------------------------

describe('resolveTabTitle — multi-pane: follows activePaneId (FS-PANE-007)', () => {
  it('returns active pane processTitle, NOT the root pane processTitle', () => {
    // Root (first) pane has title "bash", active pane is the second with "htop"
    const rootPane = makeLeaf('pane-root', 'bash');
    const activePane = makeLeaf('pane-active', 'htop');
    const layout = makeSplit(rootPane, activePane);
    const tab = makeTabState(layout, 'pane-active', null);
    expect(resolveTabTitle(tab)).toBe('htop');
    // Must NOT return the root pane title
    expect(resolveTabTitle(tab)).not.toBe('bash');
  });

  it('when activePaneId is the second child, returns second child title', () => {
    const first = makeLeaf('pane-first', 'zsh');
    const second = makeLeaf('pane-second', 'cargo build');
    const layout = makeSplit(first, second);
    const tab = makeTabState(layout, 'pane-second', null);
    expect(resolveTabTitle(tab)).toBe('cargo build');
  });

  it('user label wins over any pane processTitle in multi-pane layout', () => {
    const rootPane = makeLeaf('pane-root', 'bash');
    const activePane = makeLeaf('pane-active', 'htop');
    const layout = makeSplit(rootPane, activePane);
    const tab = makeTabState(layout, 'pane-active', 'Monitoring');
    expect(resolveTabTitle(tab)).toBe('Monitoring');
  });
});

// ---------------------------------------------------------------------------
// describe('resolveTabTitle — pane label priority (FS-PANE-007)')
// ---------------------------------------------------------------------------

describe('resolveTabTitle — pane label overrides pane processTitle', () => {
  it('returns pane label when pane has a user label set', () => {
    const pane = makePane('pane-1', 'bash', 'my-server');
    const leaf: PaneNode = { type: 'leaf', paneId: 'pane-1', state: pane };
    const tab = makeTabState(leaf, 'pane-1', null);
    expect(resolveTabTitle(tab)).toBe('my-server');
  });

  it('falls back to processTitle when pane label is absent', () => {
    const pane = makePane('pane-1', 'vim');
    const leaf: PaneNode = { type: 'leaf', paneId: 'pane-1', state: pane };
    const tab = makeTabState(leaf, 'pane-1', null);
    expect(resolveTabTitle(tab)).toBe('vim');
  });

  it('falls back to processTitle when pane label is null', () => {
    const pane = makePane('pane-1', 'zsh', null);
    const leaf: PaneNode = { type: 'leaf', paneId: 'pane-1', state: pane };
    const tab = makeTabState(leaf, 'pane-1', null);
    expect(resolveTabTitle(tab)).toBe('zsh');
  });

  it('falls back to null when both pane label and processTitle are empty', () => {
    const pane = makePane('pane-1', '', null);
    const leaf: PaneNode = { type: 'leaf', paneId: 'pane-1', state: pane };
    const tab = makeTabState(leaf, 'pane-1', null);
    expect(resolveTabTitle(tab)).toBeNull();
  });

  it('active pane label wins over root pane processTitle in multi-pane', () => {
    const rootPane = makePane('pane-root', 'bash');
    const activePane = makePane('pane-active', 'htop', 'prod-server');
    const layout = makeSplit(
      { type: 'leaf', paneId: 'pane-root', state: rootPane },
      { type: 'leaf', paneId: 'pane-active', state: activePane },
    );
    const tab = makeTabState(layout, 'pane-active', null);
    expect(resolveTabTitle(tab)).toBe('prod-server');
  });

  it('tab label wins over active pane label', () => {
    const pane = makePane('pane-1', 'bash', 'pane-label');
    const leaf: PaneNode = { type: 'leaf', paneId: 'pane-1', state: pane };
    const tab = makeTabState(leaf, 'pane-1', 'tab-label');
    expect(resolveTabTitle(tab)).toBe('tab-label');
  });
});
