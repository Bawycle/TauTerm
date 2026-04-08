// SPDX-License-Identifier: MPL-2.0

/**
 * useTabBarDnd — drag-and-drop tab reorder sub-composable (FS-TAB-005).
 *
 * Manages:
 *   - dragTabId: the tab currently being dragged
 *   - dropIndicatorIndex: position of the drop indicator (null if not dragging over)
 *   - handleDragStart, handleDragOver, handleDragLeave, handleDrop, handleDragEnd
 *   - resetDrag: clears drag state
 */

import { invoke } from '@tauri-apps/api/core';
import type { TabState } from '$lib/ipc/types';

export interface TabBarDndOptions {
  tabs: () => TabState[];
}

export function useTabBarDnd(opts: TabBarDndOptions) {
  let dragTabId = $state<string | null>(null);
  let dropIndicatorIndex = $state<number | null>(null);

  function handleDragStart(event: DragEvent, tabId: string) {
    dragTabId = tabId;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/plain', tabId);
    }
  }

  function handleDragOver(event: DragEvent, index: number) {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    dropIndicatorIndex = index;
  }

  function handleDragLeave(event: DragEvent) {
    const relatedTarget = event.relatedTarget as Node | null;
    const bar = (event.currentTarget as HTMLElement).closest('.tab-bar__tabs');
    if (bar && relatedTarget && bar.contains(relatedTarget)) return;
    dropIndicatorIndex = null;
  }

  async function handleDrop(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    const sourceId = event.dataTransfer?.getData('text/plain') ?? dragTabId;
    if (!sourceId) {
      resetDrag();
      return;
    }
    const sorted = opts.tabs();
    const sourceIdx = sorted.findIndex((t) => t.id === sourceId);
    if (sourceIdx === -1 || sourceIdx === targetIndex || sourceIdx + 1 === targetIndex) {
      resetDrag();
      return;
    }

    const remaining = sorted.filter((t) => t.id !== sourceId);
    let newOrder: number;
    if (targetIndex === 0) {
      newOrder = remaining.length > 0 ? remaining[0].order - 1 : 0;
    } else {
      const insertAfter = sourceIdx < targetIndex ? targetIndex - 1 : targetIndex;
      const clampedInsert = Math.min(insertAfter, remaining.length - 1);
      if (clampedInsert < remaining.length - 1) {
        newOrder = Math.floor(
          (remaining[clampedInsert].order + remaining[clampedInsert + 1].order) / 2,
        );
        if (newOrder === remaining[clampedInsert].order) {
          newOrder = remaining[clampedInsert].order + 1;
        }
      } else {
        newOrder = remaining[clampedInsert].order + 1;
      }
    }

    try {
      await invoke('reorder_tab', { tabId: sourceId, newOrder });
    } catch {
      // Non-fatal; backend is source of truth.
    }
    resetDrag();
  }

  function handleDragEnd() {
    resetDrag();
  }

  function resetDrag() {
    dragTabId = null;
    dropIndicatorIndex = null;
  }

  return {
    get dragTabId() {
      return dragTabId;
    },
    get dropIndicatorIndex() {
      return dropIndicatorIndex;
    },
    handleDragStart,
    handleDragOver,
    handleDragLeave,
    handleDrop,
    handleDragEnd,
    resetDrag,
  };
}
