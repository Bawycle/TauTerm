// SPDX-License-Identifier: MPL-2.0

/**
 * useTabBarContextMenu — tab context menu state sub-composable (UXD §7.8.2).
 *
 * Manages:
 *   - contextMenuTabId: ID of the tab for which the context menu is open (null if closed)
 *   - contextMenuX / contextMenuY: screen coordinates of the right-click event
 *   - handleTabContextMenu(), handleContextMenuClose(), handleContextMenuRename()
 */

export interface TabBarContextMenuOptions {
  onRenameRequest: (tabId: string, title: string) => void;
}

export function useTabBarContextMenu(opts: TabBarContextMenuOptions) {
  let contextMenuTabId = $state<string | null>(null);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);

  function handleTabContextMenu(event: MouseEvent, tabId: string) {
    event.preventDefault();
    contextMenuX = event.clientX;
    contextMenuY = event.clientY;
    contextMenuTabId = tabId;
  }

  function handleContextMenuClose() {
    contextMenuTabId = null;
  }

  function handleContextMenuRename(tabId: string, title: string) {
    contextMenuTabId = null;
    opts.onRenameRequest(tabId, title);
  }

  return {
    get contextMenuTabId() {
      return contextMenuTabId;
    },
    get contextMenuX() {
      return contextMenuX;
    },
    get contextMenuY() {
      return contextMenuY;
    },
    handleTabContextMenu,
    handleContextMenuClose,
    handleContextMenuRename,
  };
}
