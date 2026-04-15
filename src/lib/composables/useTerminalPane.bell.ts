// SPDX-License-Identifier: MPL-2.0

/**
 * Bell, border-pulse, and selection-flash logic extracted from
 * useTerminalPane.svelte.ts.
 *
 * All functions here are stateless: they receive mutable state objects as
 * arguments and write back to them. No $state, $derived, or $effect — the
 * reactive wiring remains in useTerminalPane.svelte.ts.
 *
 * The design avoids closures over $state variables so that these helpers can
 * be exercised in plain unit tests without a Svelte compilation context.
 */

import type { BellType, NotificationChangedEvent } from '$lib/ipc';

// ---------------------------------------------------------------------------
// Types for mutable state boxes passed by the composable
// ---------------------------------------------------------------------------

export type BorderPulse = 'output' | 'bell' | 'exit' | null;

/** Mutable state box for bell visual flash. */
export interface BellState {
  bellFlashing: boolean;
  bellFlashTimer: ReturnType<typeof setTimeout> | null;
}

/** Mutable state box for border pulse. */
export interface BorderPulseState {
  borderPulse: BorderPulse;
  borderPulseTimer: ReturnType<typeof setTimeout> | null;
}

/** Mutable state box for selection flash. */
export interface SelectionFlashState {
  selectionFlashing: boolean;
  selectionFlashTimer: ReturnType<typeof setTimeout> | null;
}

// ---------------------------------------------------------------------------
// Bell
// ---------------------------------------------------------------------------

/**
 * Handle a VT BEL character: trigger visual flash and/or audio tone according
 * to `bellType`.
 *
 * Mutates `state.bellFlashing` and `state.bellFlashTimer` in place. The
 * caller's reactive bindings on those fields handle re-rendering.
 *
 * @param bellType  User preference for bell presentation
 * @param state     Mutable bell state box
 */
export function handleBell(bellType: BellType, state: BellState): void {
  if (bellType === 'none') return;

  if (bellType === 'visual' || bellType === 'both') {
    if (state.bellFlashTimer) clearTimeout(state.bellFlashTimer);
    state.bellFlashing = true;
    state.bellFlashTimer = setTimeout(() => {
      state.bellFlashing = false;
      state.bellFlashTimer = null;
    }, 80);
  }

  if (bellType === 'audio' || bellType === 'both') {
    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.value = 440;
      gain.gain.setValueAtTime(0.3, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.08);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(ctx.currentTime);
      osc.stop(ctx.currentTime + 0.08);
      osc.onended = () => ctx.close();
    } catch {
      // AudioContext unavailable in test environments — non-fatal.
    }
  }
}

// ---------------------------------------------------------------------------
// Border pulse
// ---------------------------------------------------------------------------

/**
 * Activate or extend a border pulse animation on the pane border.
 *
 * 'exit' type is sticky — it cannot be overridden by 'output' or 'bell' until
 * the timer expires. Any other type simply restarts the timer.
 *
 * Mutates `state.borderPulse` and `state.borderPulseTimer` in place.
 *
 * @param type        The pulse type to trigger
 * @param durationMs  How long the pulse should remain visible
 * @param state       Mutable border pulse state box
 */
export function triggerBorderPulse(
  type: BorderPulse,
  durationMs: number,
  state: BorderPulseState,
): void {
  if (state.borderPulse === 'exit' && type !== 'exit') return;
  if (state.borderPulseTimer) clearTimeout(state.borderPulseTimer);
  state.borderPulse = type;
  state.borderPulseTimer = setTimeout(() => {
    state.borderPulse = null;
    state.borderPulseTimer = null;
  }, durationMs);
}

/**
 * Handle a `notification-changed` IPC event and map it to a border pulse.
 *
 * Should only be called when the pane is not active (active panes do not
 * receive notification pulses — the composable checks this before calling).
 *
 * @param ev    The notification event payload
 * @param state Mutable border pulse state box
 */
export function handleNotificationForBorderPulse(
  ev: NotificationChangedEvent,
  state: BorderPulseState,
): void {
  if (ev.notification === null) {
    if (state.borderPulse !== 'exit') {
      clearTimeout(state.borderPulseTimer ?? undefined);
      state.borderPulseTimer = null;
      state.borderPulse = null;
    }
    return;
  }
  switch (ev.notification.type) {
    case 'backgroundOutput':
      triggerBorderPulse('output', 800, state);
      break;
    case 'bell':
      triggerBorderPulse('bell', 800, state);
      break;
    case 'processExited':
      triggerBorderPulse('exit', 1500, state);
      break;
  }
}

// ---------------------------------------------------------------------------
// Selection flash
// ---------------------------------------------------------------------------

/**
 * Trigger a brief selection flash animation (copy feedback).
 *
 * Mutates `state.selectionFlashing` and `state.selectionFlashTimer` in place.
 *
 * @param state  Mutable selection flash state box
 */
export function triggerSelectionFlash(state: SelectionFlashState): void {
  if (state.selectionFlashTimer) clearTimeout(state.selectionFlashTimer);
  state.selectionFlashing = true;
  state.selectionFlashTimer = setTimeout(() => {
    state.selectionFlashing = false;
    state.selectionFlashTimer = null;
  }, 80);
}
