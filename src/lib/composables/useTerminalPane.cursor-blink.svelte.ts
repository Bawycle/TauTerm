// SPDX-License-Identifier: MPL-2.0

/**
 * useCursorBlink — cursor blink timer sub-composable.
 *
 * Manages the asymmetric 2:1 blink cycle (ON = cursorBlinkMs, OFF = cursorBlinkMs / 2).
 * Returns a reactive getter for cursorVisible, stopCursorBlink, and restartCursorBlink.
 *
 * restartCursorBlink() is the correct helper to call on keydown: it cancels the
 * running cycle and restarts it from scratch so the cursor stays visible for a
 * full ON phase after each keystroke instead of freezing permanently visible.
 * It works by incrementing restartCount, which is tracked by the $effect — the
 * effect teardown cancels the old timer and the re-run schedules a new cycle.
 */

export interface CursorBlinkOptions {
  cursorBlinkMs: () => number;
  /** Whether the cursor should currently blink (cursor.blink && cursorBlinks(cursor.shape)). */
  currentCursorBlinks: () => boolean;
}

export function useCursorBlink(opts: CursorBlinkOptions) {
  let cursorVisible = $state(true);
  let blinkPhaseTimer: ReturnType<typeof setTimeout> | null = null;
  // Incrementing this counter causes the $effect to re-run, restarting the cycle.
  let restartCount = $state(0);

  // Cursor blink timer — restarts when cursorBlinkMs, blink mode, or restartCount changes.
  // Uses asymmetric 2:1 ratio: ON = cursorBlinkMs, OFF = cursorBlinkMs / 2.
  // NOTE: currentCursorBlinks is read here (not inside startCursorBlink) so
  // that this effect re-runs when the blink mode changes, AND to avoid
  // startCursorBlink() becoming an implicit dependency via a nested read.
  $effect(() => {
    const onMs = opts.cursorBlinkMs();
    const blinks = opts.currentCursorBlinks();
    // Read restartCount to register it as a dependency — incrementing it
    // from restartCursorBlink() will re-run this effect.
    restartCount; // eslint-disable-line @typescript-eslint/no-unused-expressions
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

  /** Cancel any running blink cycle and restore cursor to visible. Does not restart. */
  function stopCursorBlink() {
    if (blinkPhaseTimer) {
      clearTimeout(blinkPhaseTimer);
      blinkPhaseTimer = null;
    }
    cursorVisible = true;
  }

  /**
   * Restart the blink cycle from scratch (cursor visible → full ON phase → normal cycle).
   * Call this on keydown so the cursor stays visible after each keystroke and the
   * cycle resumes normally — unlike stopCursorBlink() which leaves blink permanently off.
   */
  function restartCursorBlink() {
    restartCount++;
  }

  return {
    get cursorVisible() {
      return cursorVisible;
    },
    stopCursorBlink,
    restartCursorBlink,
  };
}
