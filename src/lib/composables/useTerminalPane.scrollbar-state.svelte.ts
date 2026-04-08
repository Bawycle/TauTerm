// SPDX-License-Identifier: MPL-2.0

/**
 * useScrollbarState — scrollbar visibility and drag interaction sub-composable.
 *
 * Manages:
 *   - scrollbarVisible: fade timer after scroll events
 *   - scrollbarDragging: pointer-capture drag state
 *   - scrollbarHover: hover state for always-visible scrollbar
 *   - drag position tracking (scrollbarDragStartY, scrollbarDragStartOffset)
 */

export function useScrollbarState() {
  let scrollbarVisible = $state(false);
  let scrollbarFadeTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollbarDragging = $state(false);
  let scrollbarHover = $state(false);
  let scrollbarDragStartY = 0;
  let scrollbarDragStartOffset = 0;

  function showScrollbar(autoHide: boolean) {
    scrollbarVisible = true;
    if (scrollbarFadeTimer) clearTimeout(scrollbarFadeTimer);
    if (autoHide) {
      scrollbarFadeTimer = setTimeout(() => {
        scrollbarVisible = false;
      }, 1500);
    }
  }

  function hideScrollbar() {
    scrollbarVisible = false;
    if (scrollbarFadeTimer) {
      clearTimeout(scrollbarFadeTimer);
      scrollbarFadeTimer = null;
    }
  }

  function startDrag(clientY: number, currentOffset: number) {
    scrollbarDragging = true;
    scrollbarDragStartY = clientY;
    scrollbarDragStartOffset = currentOffset;
  }

  function getDragStartY() {
    return scrollbarDragStartY;
  }

  function getDragStartOffset() {
    return scrollbarDragStartOffset;
  }

  function endDrag() {
    scrollbarDragging = false;
  }

  function cleanup() {
    if (scrollbarFadeTimer) clearTimeout(scrollbarFadeTimer);
  }

  return {
    get scrollbarVisible() {
      return scrollbarVisible;
    },
    get scrollbarDragging() {
      return scrollbarDragging;
    },
    get scrollbarHover() {
      return scrollbarHover;
    },
    set scrollbarHover(v: boolean) {
      scrollbarHover = v;
    },
    showScrollbar,
    hideScrollbar,
    startDrag,
    getDragStartY,
    getDragStartOffset,
    endDrag,
    cleanup,
  };
}
