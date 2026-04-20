// SPDX-License-Identifier: MPL-2.0

/**
 * useTerminalResize — ResizeObserver + cell measurement + resize debounce sub-composable.
 *
 * Manages:
 *   - ResizeObserver setup on the viewport element
 *   - cellMeasureProbe: reusable DOM probe for 1lh×1ch measurement (D2-P16)
 *   - scheduleSendResize() / sendResize() debounce logic
 *   - F8 $effect: re-measures when font props (family/size/lineHeight) change
 *
 * Returns { sendResize, scheduleSendResize, cleanup }.
 * The ResizeObserver is started via startObserving(el) called from onMount.
 */

import { measureCellDimensions } from '$lib/terminal/cell-dimensions.js';
import { resizePane } from '$lib/ipc';
import type { PaneId } from '$lib/ipc';

export interface TerminalResizeOptions {
  paneId: () => PaneId;
  viewportEl: () => HTMLDivElement | undefined;
  /** Current column count — used for fallback cell size estimate. */
  getCols: () => number;
  /** Current row count — used for fallback cell size estimate. */
  getRows: () => number;
  /** Callback to notify parent when dimensions are known (before the IPC call). */
  ondimensionschange: () => ((cols: number, rows: number) => void) | undefined;
  /** CSS font-family for terminal text — used by Canvas cell measurement (F8). */
  fontFamily?: () => string | undefined;
  /** Font size in pixels — used by Canvas cell measurement (F8). */
  fontSize?: () => number | undefined;
  /** Line height multiplier — used by Canvas cell measurement (F8). */
  lineHeight?: () => number | undefined;
}

export function useTerminalResize(opts: TerminalResizeOptions) {
  // D2-P16: reusable DOM probe for cell dimension measurement — created once,
  // kept attached to viewportEl for the lifetime of the pane. Avoids the
  // create/append/measure/removeChild cycle on every sendResize() call.
  let cellMeasureProbe: HTMLDivElement | null = null;

  let resizeObserver: ResizeObserver | null = null;
  let resizeDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  // F8 — re-measure and resize when font props change (family, size, line-height).
  // Reading props.fontFamily/fontSize/lineHeight subscribes to them reactively;
  // any change from the preferences panel triggers a new sendResize() call so that
  // cols/rows are recomputed with the new cell dimensions.
  $effect(() => {
    opts.fontFamily?.();
    opts.fontSize?.();
    opts.lineHeight?.();
    scheduleSendResize();
  });

  function ensureCellMeasureProbe(): HTMLDivElement | null {
    const el = opts.viewportEl();
    if (!el) return null;
    if (!cellMeasureProbe) {
      cellMeasureProbe = document.createElement('div');
      cellMeasureProbe.style.cssText =
        'position:absolute;visibility:hidden;pointer-events:none;height:1lh;width:1ch';
      el.appendChild(cellMeasureProbe);
    }
    return cellMeasureProbe;
  }

  function scheduleSendResize() {
    if (resizeDebounceTimer) clearTimeout(resizeDebounceTimer);
    opts.ondimensionschange()?.(opts.getCols(), opts.getRows());
    resizeDebounceTimer = setTimeout(sendResize, 50);
  }

  async function sendResize() {
    const el = opts.viewportEl();
    if (!el) return;
    const rect = el.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;

    // Bug 1a: measure 1lh and 1ch via a DOM probe so dimensions exactly match the
    // CSS units used by the terminal grid cells (height:1lh, width:1ch).
    // Falls back to measureCellDimensions (Canvas 2D) then to a grid-based estimate.
    // D2-P16: the probe is created once and reused across calls (ensureCellMeasureProbe).
    let cellW: number;
    let cellH: number;
    const cols = opts.getCols();
    const rows = opts.getRows();
    try {
      const probe = ensureCellMeasureProbe();
      const probeRect = probe?.getBoundingClientRect();

      if (probeRect && probeRect.height > 0 && probeRect.width > 0) {
        cellH = probeRect.height;
        cellW = probeRect.width;
      } else {
        // Fallback: CSS lh/ch not supported or probe returned zero.
        const family = opts.fontFamily?.() ?? 'monospace';
        const size = opts.fontSize?.() ?? 13;
        const lh = opts.lineHeight?.() ?? 1.2;
        const dims = measureCellDimensions(family, size, lh);
        cellW = dims.width > 0 ? dims.width : Math.max(1, rect.width / cols);
        cellH = dims.height > 0 ? dims.height : Math.max(1, rect.height / rows);
      }
    } catch {
      cellW = Math.max(1, rect.width / cols);
      cellH = Math.max(1, rect.height / rows);
    }

    const newCols = Math.max(1, Math.floor(rect.width / cellW));
    const newRows = Math.max(1, Math.floor(rect.height / cellH));
    const pixelWidth = Math.max(1, Math.floor(rect.width));
    const pixelHeight = Math.max(1, Math.floor(rect.height));

    // cols/rows are now updated from ScreenUpdateEvent.cols/rows in applyScreenUpdate —
    // the event is the authoritative source of truth, eliminating stride mismatch.
    try {
      await resizePane(opts.paneId(), newCols, newRows, pixelWidth, pixelHeight);
    } catch {
      // IPC failure — no cols/rows state to roll back (they update via screen-update events).
      // Log only the generic label, never the path (security constraint).
      console.error('resize_pane IPC failed');
    }
  }

  function startObserving(el: HTMLElement) {
    resizeObserver = new ResizeObserver(() => scheduleSendResize());
    resizeObserver.observe(el);
  }

  function cleanup() {
    resizeObserver?.disconnect();
    if (resizeDebounceTimer) clearTimeout(resizeDebounceTimer);
    cellMeasureProbe?.remove();
    cellMeasureProbe = null;
  }

  return {
    sendResize,
    scheduleSendResize,
    startObserving,
    cleanup,
  };
}
