// SPDX-License-Identifier: MPL-2.0

/**
 * useTabBarRename — inline tab rename state sub-composable.
 *
 * Manages:
 *   - renamingTabId: the tab currently in rename mode (null if none)
 *   - renameValue: current value of the rename input
 *   - startRename(), confirmRename(), cancelRename() handlers
 *   - $effect that reacts to external requestedRenameTabId prop changes (e.g. F2 global shortcut)
 */

import { invoke } from '@tauri-apps/api/core';
import type { TabState } from '$lib/ipc/types';
import { applySessionDelta } from '$lib/state/session.svelte';

export interface TabBarRenameOptions {
  requestedRenameTabId: () => string | null;
  onRenameHandled: () => void;
  onRenameComplete: () => void;
  /** Resolve a tab's display title given its ID. Used when reacting to external rename requests. */
  getTabDisplayTitle: (tabId: string) => string | null;
}

export function useTabBarRename(opts: TabBarRenameOptions) {
  let renamingTabId = $state<string | null>(null);
  let renameValue = $state('');
  // Note: input focus is handled locally within TabBarItem ($effect on isRenaming).

  /** Enter rename mode for the given tab. */
  function startRename(tabId: string, currentTitle: string) {
    renamingTabId = tabId;
    renameValue = currentTitle;
  }

  /** Confirm rename: send IPC, then exit rename mode. */
  async function confirmRename(tabId: string) {
    if (renamingTabId !== tabId) return;
    const label: string | null = renameValue.trim() === '' ? null : renameValue.trim();
    try {
      const updatedTab = await invoke<TabState>('rename_tab', { tabId, label });
      applySessionDelta({ type: 'paneMetadataChanged', tab: updatedTab });
    } catch {
      // IPC errors are non-fatal; title stays unchanged on next state update.
    }
    renamingTabId = null;
    renameValue = '';
    opts.onRenameComplete();
  }

  /** Cancel rename without saving. */
  function cancelRename() {
    renamingTabId = null;
    renameValue = '';
    opts.onRenameComplete();
  }

  // React to an external rename request (e.g. F2 global shortcut from TerminalView).
  $effect(() => {
    const requested = opts.requestedRenameTabId();
    if (requested === null || requested === undefined) return;
    const title = opts.getTabDisplayTitle(requested);
    if (title === null) return;
    startRename(requested, title);
    opts.onRenameHandled();
  });

  return {
    get renamingTabId() {
      return renamingTabId;
    },
    get renameValue() {
      return renameValue;
    },
    set renameValue(v: string) {
      renameValue = v;
    },
    startRename,
    confirmRename,
    cancelRename,
  };
}
