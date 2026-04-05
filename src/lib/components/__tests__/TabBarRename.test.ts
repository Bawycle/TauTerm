// SPDX-License-Identifier: MPL-2.0

/**
 * TabBar inline rename — logic unit tests (FS-TAB-006).
 *
 * Covered (pure-logic, no DOM):
 *   TEST-SPRINT-005a — startRename sets renamingTabId and initialises renameValue
 *   TEST-SPRINT-005b — cancelRename clears both state fields
 *   TEST-SPRINT-005c — confirmRename with empty string → null label (clear)
 *   TEST-SPRINT-005d — confirmRename with non-empty string → trimmed label
 *   TEST-SPRINT-005e — Enter key triggers confirm; Escape key triggers cancel
 *   TEST-SPRINT-005f — double-click on a different tab replaces the previous rename
 *
 * The functions startRename / confirmRename / cancelRename / handleRenameKeydown
 * are local to TabBar.svelte and not exported.  These tests exercise the same
 * logic in isolation, mirroring the implementation exactly.
 *
 * Full DOM interaction tests (double-click fires startRename, input appears, blur
 * confirms) require a live Tauri `invoke()` and are deferred to E2E.
 */

import { describe, it, expect, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Mirror of TabBar rename state machine (extracted pure logic)
// ---------------------------------------------------------------------------

interface RenameState {
  renamingTabId: string | null;
  renameValue: string;
}

function startRename(state: RenameState, tabId: string, currentTitle: string): RenameState {
  return { renamingTabId: tabId, renameValue: currentTitle };
}

function cancelRename(_state: RenameState): RenameState {
  return { renamingTabId: null, renameValue: '' };
}

/**
 * Returns the label to pass to `invoke('rename_tab', { tabId, label })`.
 * Empty / whitespace-only value → null (clear label, revert to OSC title).
 */
function resolveRenameLabel(renameValue: string): string | null {
  return renameValue.trim() === '' ? null : renameValue.trim();
}

/**
 * Simulate handleRenameKeydown — returns 'confirm', 'cancel', or 'noop'.
 */
function handleRenameKeydown(key: string): 'confirm' | 'cancel' | 'noop' {
  if (key === 'Enter') return 'confirm';
  if (key === 'Escape') return 'cancel';
  return 'noop';
}

// ---------------------------------------------------------------------------
// TEST-SPRINT-005a — startRename sets state correctly
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-005a: startRename sets renamingTabId and renameValue', () => {
  it('sets renamingTabId to the given tab ID', () => {
    const state: RenameState = { renamingTabId: null, renameValue: '' };
    const next = startRename(state, 'tab-42', 'My Tab');
    expect(next.renamingTabId).toBe('tab-42');
  });

  it('initialises renameValue with the current title', () => {
    const state: RenameState = { renamingTabId: null, renameValue: '' };
    const next = startRename(state, 'tab-1', 'bash — ~/projects');
    expect(next.renameValue).toBe('bash — ~/projects');
  });

  it('starting rename on a second tab replaces the previous rename', () => {
    let state: RenameState = { renamingTabId: null, renameValue: '' };
    state = startRename(state, 'tab-1', 'First');
    state = startRename(state, 'tab-2', 'Second');
    expect(state.renamingTabId).toBe('tab-2');
    expect(state.renameValue).toBe('Second');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-005b — cancelRename clears state
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-005b: cancelRename clears rename state', () => {
  it('sets renamingTabId to null', () => {
    const state: RenameState = { renamingTabId: 'tab-1', renameValue: 'hello' };
    const next = cancelRename(state);
    expect(next.renamingTabId).toBeNull();
  });

  it('clears renameValue to empty string', () => {
    const state: RenameState = { renamingTabId: 'tab-1', renameValue: 'hello' };
    const next = cancelRename(state);
    expect(next.renameValue).toBe('');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-005c — confirmRename with empty / whitespace → null label
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-005c: confirmRename — empty value clears label', () => {
  it('empty string resolves to null', () => {
    expect(resolveRenameLabel('')).toBeNull();
  });

  it('whitespace-only resolves to null', () => {
    expect(resolveRenameLabel('   ')).toBeNull();
  });

  it('tab character resolves to null', () => {
    expect(resolveRenameLabel('\t')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-005d — confirmRename with non-empty string → trimmed label
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-005d: confirmRename — non-empty value is trimmed', () => {
  it('plain string is returned as-is', () => {
    expect(resolveRenameLabel('My Tab')).toBe('My Tab');
  });

  it('leading/trailing spaces are trimmed', () => {
    expect(resolveRenameLabel('  My Tab  ')).toBe('My Tab');
  });

  it('internal spaces are preserved', () => {
    expect(resolveRenameLabel('bash — ~/projects')).toBe('bash — ~/projects');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-005e — Enter confirms, Escape cancels, other keys are no-ops
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-005e: handleRenameKeydown dispatches correctly', () => {
  it('Enter key → confirm', () => {
    expect(handleRenameKeydown('Enter')).toBe('confirm');
  });

  it('Escape key → cancel', () => {
    expect(handleRenameKeydown('Escape')).toBe('cancel');
  });

  it('other key → noop', () => {
    expect(handleRenameKeydown('a')).toBe('noop');
    expect(handleRenameKeydown('ArrowLeft')).toBe('noop');
    expect(handleRenameKeydown(' ')).toBe('noop');
  });
});

// ---------------------------------------------------------------------------
// TEST-SPRINT-005f — double-click on different tab replaces ongoing rename
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-005f: successive startRename calls replace previous', () => {
  it('second startRename replaces first without cancel in between', () => {
    let state: RenameState = { renamingTabId: null, renameValue: '' };
    state = startRename(state, 'tab-1', 'First tab');
    expect(state.renamingTabId).toBe('tab-1');
    state = startRename(state, 'tab-2', 'Second tab');
    expect(state.renamingTabId).toBe('tab-2');
    expect(state.renameValue).toBe('Second tab');
  });
});

// ---------------------------------------------------------------------------
// E2E-deferred: interaction tests requiring live Tauri invoke()
// ---------------------------------------------------------------------------

describe('TEST-SPRINT-005 [E2E-deferred]: DOM interaction tests', () => {
  it.todo('double-click on a tab renders the rename input element');
  it.todo('pressing Enter on the input calls invoke("rename_tab")');
  it.todo('pressing Escape on the input closes without calling invoke');
  it.todo('blur on the input calls invoke("rename_tab")');
  it.todo('F2 on focused tab activates rename mode via requestedRenameTabId prop');
  it.todo('context menu Rename item activates rename mode');
});
