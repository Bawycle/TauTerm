// SPDX-License-Identifier: MPL-2.0

/**
 * useCursorBlink — cursor blink timer sub-composable.
 *
 * Manages the asymmetric 2:1 blink cycle (ON = cursorBlinkMs, OFF = cursorBlinkMs / 2).
 * Returns a reactive getter for cursorVisible and a stopCursorBlink helper.
 */

export interface CursorBlinkOptions {
  cursorBlinkMs: () => number;
  /** Whether the cursor should currently blink (cursor.blink && cursorBlinks(cursor.shape)). */
  currentCursorBlinks: () => boolean;
}

export function useCursorBlink(opts: CursorBlinkOptions) {
  let cursorVisible = $state(true);
  let blinkPhaseTimer: ReturnType<typeof setTimeout> | null = null;

  // Cursor blink timer — restarts when cursorBlinkMs or blink mode changes.
  // Uses asymmetric 2:1 ratio: ON = cursorBlinkMs, OFF = cursorBlinkMs / 2.
  // NOTE: currentCursorBlinks is read here (not inside startCursorBlink) so
  // that this effect re-runs when the blink mode changes, AND to avoid
  // startCursorBlink() becoming an implicit dependency via a nested read.
  $effect(() => {
    const onMs = opts.cursorBlinkMs();
    const blinks = opts.currentCursorBlinks();
    // Cancel any running cycle — this is the only write to cursorVisible inside
    // the effect body; writes to $state inside effects are allowed in Svelte 5.
    if (blinkPhaseTimer) {
      clearTimeout(blinkPhaseTimer);
      blinkPhaseTimer = null;
    }
    cursorVisible = true;
    if (!blinks) return;

    const offMs = Math.round(onMs / 2);

    function scheduleOffPhase() {
      cursorVisible = false;
      blinkPhaseTimer = setTimeout(() => {
        cursorVisible = true;
        blinkPhaseTimer = setTimeout(scheduleOffPhase, onMs);
      }, offMs);
    }

    blinkPhaseTimer = setTimeout(scheduleOffPhase, onMs);

    return () => {
      if (blinkPhaseTimer) {
        clearTimeout(blinkPhaseTimer);
        blinkPhaseTimer = null;
      }
      cursorVisible = true;
    };
  });

  /** Cancel any running blink cycle and restore cursor to visible. */
  function stopCursorBlink() {
    if (blinkPhaseTimer) {
      clearTimeout(blinkPhaseTimer);
      blinkPhaseTimer = null;
    }
    cursorVisible = true;
  }

  return {
    get cursorVisible() {
      return cursorVisible;
    },
    stopCursorBlink,
  };
}
