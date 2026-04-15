// SPDX-License-Identifier: MPL-2.0

/**
 * useVisualFx — visual flash effects sub-composable.
 *
 * Manages:
 *   - bellFlashing: 80 ms screen flash on bell (visual/both bell type)
 *   - borderPulse: border pulse indicator for background activity (output/bell/exit)
 *   - selectionFlashing: 80 ms flash after a successful copy
 *
 * Returns state getters and trigger functions.
 */

import type { BellType, NotificationChangedEvent } from '$lib/ipc';
import type { BorderPulse } from './useTerminalPane.bell.js';

export interface VisualFxOptions {
  bellType: () => BellType;
  isActive: () => boolean;
}

export function useVisualFx(opts: VisualFxOptions) {
  // ── Bell flash ──────────────────────────────────────────────────────────────
  let bellFlashing = $state(false);
  let bellFlashTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Border pulse ────────────────────────────────────────────────────────────
  let borderPulse = $state<BorderPulse>(null);
  let borderPulseTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Selection flash ─────────────────────────────────────────────────────────
  let selectionFlashing = $state(false);
  let selectionFlashTimer: ReturnType<typeof setTimeout> | null = null;

  // Clear border pulse when pane becomes active
  $effect(() => {
    if (opts.isActive() && borderPulse !== null) {
      if (borderPulseTimer) clearTimeout(borderPulseTimer);
      borderPulseTimer = null;
      borderPulse = null;
    }
  });

  // ── Bell handler ────────────────────────────────────────────────────────────

  function handleBell() {
    if (opts.bellType() === 'none') return;

    if (opts.bellType() === 'visual' || opts.bellType() === 'both') {
      if (bellFlashTimer) clearTimeout(bellFlashTimer);
      bellFlashing = true;
      bellFlashTimer = setTimeout(() => {
        bellFlashing = false;
        bellFlashTimer = null;
      }, 80);
    }

    if (opts.bellType() === 'audio' || opts.bellType() === 'both') {
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

  // ── Border pulse helpers ────────────────────────────────────────────────────

  function triggerBorderPulse(type: BorderPulse, durationMs: number) {
    if (borderPulse === 'exit' && type !== 'exit') return;
    if (borderPulseTimer) clearTimeout(borderPulseTimer);
    borderPulse = type;
    borderPulseTimer = setTimeout(() => {
      borderPulse = null;
      borderPulseTimer = null;
    }, durationMs);
  }

  function handleNotificationForBorderPulse(ev: NotificationChangedEvent) {
    if (ev.notification === null) {
      if (borderPulse !== 'exit') {
        clearTimeout(borderPulseTimer ?? undefined);
        borderPulseTimer = null;
        borderPulse = null;
      }
      return;
    }
    switch (ev.notification.type) {
      case 'backgroundOutput':
        triggerBorderPulse('output', 800);
        break;
      case 'bell':
        triggerBorderPulse('bell', 800);
        break;
      case 'processExited':
        triggerBorderPulse('exit', 1500);
        break;
    }
  }

  // ── Selection flash ─────────────────────────────────────────────────────────

  function triggerSelectionFlash() {
    if (selectionFlashTimer) clearTimeout(selectionFlashTimer);
    selectionFlashing = true;
    selectionFlashTimer = setTimeout(() => {
      selectionFlashing = false;
      selectionFlashTimer = null;
    }, 80);
  }

  // ── Cleanup ─────────────────────────────────────────────────────────────────

  function cleanup() {
    if (bellFlashTimer) clearTimeout(bellFlashTimer);
    if (borderPulseTimer) clearTimeout(borderPulseTimer);
    if (selectionFlashTimer) clearTimeout(selectionFlashTimer);
  }

  return {
    get bellFlashing() {
      return bellFlashing;
    },
    get borderPulse() {
      return borderPulse;
    },
    get selectionFlashing() {
      return selectionFlashing;
    },
    handleBell,
    handleNotificationForBorderPulse,
    triggerSelectionFlash,
    cleanup,
  };
}
