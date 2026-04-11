// SPDX-License-Identifier: MPL-2.0

/**
 * F3 — Cursor blink ratio 2:1 (ON=cursorBlinkMs, OFF=cursorBlinkMs/2)
 *
 * Tests that the blink cycle uses two distinct phases via setTimeout,
 * not a uniform setInterval. The composable is exercised via TerminalPane.
 *
 * Strategy: use vi.useFakeTimers() to control time precisely and verify
 * that cursorVisible toggles at the correct asymmetric intervals.
 *
 * Covered:
 *   BLINK-001 — cursor starts visible (cursorVisible=true)
 *   BLINK-002 — after ON phase (533ms), cursor becomes invisible
 *   BLINK-003 — after OFF phase (266ms), cursor becomes visible again
 *   BLINK-004 — cycle repeats (second ON phase)
 *   BLINK-005 — setInterval is NOT used (only setTimeout)
 *   BLINK-006 — active=false suspends the blink cycle (cursorVisible does not toggle)
 *   BLINK-007 — restartCursorBlink() resets to visible and restarts the ON phase
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import TerminalPane from '$lib/components/TerminalPane.svelte';
import TerminalPaneActiveHarness from './TerminalPaneActiveHarness.svelte';

// ---------------------------------------------------------------------------
// jsdom polyfills
// ---------------------------------------------------------------------------

if (typeof globalThis.ResizeObserver === 'undefined') {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;
}

if (typeof Element.prototype.animate === 'undefined') {
  Object.defineProperty(Element.prototype, 'animate', {
    value: function () {
      return {
        finished: Promise.resolve(),
        cancel() {},
        finish() {},
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {
          return true;
        },
      };
    },
    writable: true,
    configurable: true,
  });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const instances: ReturnType<typeof mount>[] = [];

async function mountPane(
  cursorBlinkMs = 533,
): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> }> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(TerminalPane, {
    target: container,
    props: { paneId: 'blink-test-pane', tabId: 'blink-test-tab', active: true, cursorBlinkMs },
  });
  instances.push(instance);
  // Drain async chain for onMount
  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();
  return { container, instance };
}

interface HarnessInstance {
  setActive: (v: boolean) => void;
}

async function mountHarness(opts: {
  active: boolean;
  cursorBlinkMs?: number;
}): Promise<{ container: HTMLElement; instance: ReturnType<typeof mount> & HarnessInstance }> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(TerminalPaneActiveHarness, {
    target: container,
    props: {
      paneId: 'blink-harness-pane',
      tabId: 'blink-harness-tab',
      active: opts.active,
      cursorBlinkMs: opts.cursorBlinkMs ?? 533,
    },
  });
  instances.push(instance);
  for (let i = 0; i < 20; i++) await Promise.resolve();
  flushSync();
  return { container, instance: instance as ReturnType<typeof mount> & HarnessInstance };
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  vi.spyOn(tauriEvent, 'listen').mockImplementation(async () => () => {});
  vi.spyOn(tauriCore, 'invoke').mockResolvedValue(undefined as unknown as never);
});

afterEach(() => {
  instances.forEach((inst) => {
    try {
      unmount(inst);
    } catch {
      /* ignore */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  if (vi.isFakeTimers()) vi.useRealTimers();
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('BLINK-001: cursor starts visible', () => {
  it('cursor element is present immediately after mount (cursorVisible=true)', async () => {
    vi.useFakeTimers();
    const { container } = await mountPane(533);
    flushSync();
    // Cursor should be visible initially — the cursor element is rendered
    // when cursor.visible && (cursorVisible || !currentCursorBlinks).
    // cursorVisible starts as true, so the cursor element must be present.
    const cursorEl = container.querySelector('.terminal-pane__cursor');
    expect(cursorEl).not.toBeNull();
  });
});

describe('BLINK-002: after ON phase, cursor becomes invisible', () => {
  it('cursor element disappears after cursorBlinkMs (533ms) elapses', async () => {
    vi.useFakeTimers();
    const { container } = await mountPane(533);
    flushSync();

    // Advance exactly one ON phase, then flush.
    vi.advanceTimersByTime(533);
    flushSync();

    const cursorEl = container.querySelector('.terminal-pane__cursor');
    expect(cursorEl).toBeNull();
  });
});

describe('BLINK-003: OFF phase has a different (shorter) duration than ON phase', () => {
  it('two distinct setTimeout durations are used: onMs and Math.round(onMs/2)', async () => {
    vi.useFakeTimers();
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout');

    await mountPane(533);
    flushSync();

    // Advance one ON phase so scheduleOffPhase is called and creates the OFF timer.
    vi.advanceTimersByTime(533);
    flushSync();

    // Collect all setTimeout calls made during the mount + first ON phase.
    const delays = setTimeoutSpy.mock.calls
      .map((args) => args[1] as number)
      .filter((d) => d === 533 || d === 267); // 267 = Math.round(533/2)

    // Must have at least one call with onMs (533) and one with offMs (267).
    expect(delays).toContain(533);
    expect(delays).toContain(267);
  });
});

describe('BLINK-004: ON and OFF durations differ (2:1 ratio)', () => {
  it('off duration is approximately half of on duration', async () => {
    vi.useFakeTimers();
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout');

    await mountPane(533);
    flushSync();

    vi.advanceTimersByTime(533);
    flushSync();

    const blinkDelays = setTimeoutSpy.mock.calls
      .map((args) => args[1] as number)
      .filter((d) => typeof d === 'number' && d >= 200 && d <= 600);

    const onDelays = blinkDelays.filter((d) => d === 533);
    const offDelays = blinkDelays.filter((d) => d === 267); // Math.round(533/2)

    expect(onDelays.length).toBeGreaterThanOrEqual(1);
    expect(offDelays.length).toBeGreaterThanOrEqual(1);
    // The OFF delay must be half the ON delay (within rounding)
    expect(offDelays[0]).toBe(Math.round(onDelays[0] / 2));
    // ON delay must be strictly greater than OFF delay (2:1 ratio)
    expect(onDelays[0]).toBeGreaterThan(offDelays[0]);
  });
});

describe('BLINK-005: setInterval is NOT used for cursor blink', () => {
  it('does not call setInterval for the blink cycle', async () => {
    vi.useFakeTimers();
    const setIntervalSpy = vi.spyOn(globalThis, 'setInterval');

    await mountPane(533);
    flushSync();

    // setInterval should NOT be called for the cursor blink mechanism.
    // The asymmetric ratio requires setTimeout-based phasing.
    const blinkRelatedCalls = setIntervalSpy.mock.calls.filter(
      // The blink setInterval would use cursorBlinkMs (533) or cursorBlinkMs/2 (266)
      // as the delay. Any call with those values indicates the old implementation.
      (args) => args[1] === 533 || args[1] === 266,
    );
    expect(blinkRelatedCalls.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// BLINK-006 — active=false suspends the blink cycle
// ---------------------------------------------------------------------------

describe('BLINK-006: active=false suspends the blink cycle', () => {
  it('cursor does not toggle after active transitions from true to false', async () => {
    vi.useFakeTimers();
    const { container, instance } = await mountHarness({ active: true, cursorBlinkMs: 533 });

    // Cursor is visible initially.
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();

    // Advance into the OFF phase — cursor should disappear.
    vi.advanceTimersByTime(533);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).toBeNull();

    // Advance back into the ON phase — cursor should reappear.
    vi.advanceTimersByTime(267);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();

    // Now disable the pane: active=false stops the cycle.
    instance.setActive(false);
    flushSync();

    // Record visibility state immediately after deactivation.
    // The $effect teardown resets cursorVisible=true when the cycle stops.
    const visibleAfterDeactivation = container.querySelector('.terminal-pane__cursor') !== null;

    // Advance one full ON + OFF cycle — visibility must not change.
    vi.advanceTimersByTime(533 + 267);
    flushSync();

    const visibleAfterAdvance = container.querySelector('.terminal-pane__cursor') !== null;
    expect(visibleAfterAdvance).toBe(visibleAfterDeactivation);
  });
});

// ---------------------------------------------------------------------------
// BLINK-006b — active=true resumes the blink cycle after active=false
// ---------------------------------------------------------------------------

describe('BLINK-006b: active=true resumes the blink cycle after suspension', () => {
  it('blink cycle restarts from visible after active transitions false → true', async () => {
    vi.useFakeTimers();
    const { container, instance } = await mountHarness({ active: true, cursorBlinkMs: 533 });

    // Step 1: active=true, cursor starts visible.
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();

    // Step 2: advance 533ms → cursor invisible (OFF phase).
    vi.advanceTimersByTime(533);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).toBeNull();

    // Step 3: active=false → blink suspended, cursorVisible resets to true → cursor visible.
    instance.setActive(false);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();

    // Step 4: active=true → blink cycle resumes from visible (ON phase starts).
    instance.setActive(true);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();

    // Step 5: advance 533ms → cursor invisible (ON phase completed, entering OFF phase).
    vi.advanceTimersByTime(533);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).toBeNull();

    // Step 6: advance 267ms → cursor visible again (OFF phase completed).
    vi.advanceTimersByTime(267);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// BLINK-007 — restartCursorBlink() resets to visible and restarts the ON phase
// ---------------------------------------------------------------------------

describe('BLINK-007: restartCursorBlink() resets cursor to visible and restarts the cycle', () => {
  it('cursor is visible immediately after restart and re-enters the OFF phase after onMs', async () => {
    vi.useFakeTimers();
    const { container } = await mountPane(533);
    flushSync();

    // Step 1: advance past the ON phase so cursor is in the OFF phase (invisible).
    vi.advanceTimersByTime(533);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).toBeNull();

    // Step 2: simulate a keydown — TerminalPane calls restartCursorBlink() internally.
    // keydown is now bound to the hidden textarea (.terminal-pane__input), not the
    // viewport div. Use a non-printable key (ArrowDown) so it is not skipped by the
    // bare-printable guard (bare chars are handled via the input event, not keydown).
    const inputEl = container.querySelector('.terminal-pane__input') as HTMLElement | null;
    expect(inputEl).not.toBeNull();
    inputEl!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }),
    );
    flushSync();

    // Step 3: cursor must be immediately visible after keydown (restartCursorBlink).
    expect(container.querySelector('.terminal-pane__cursor')).not.toBeNull();

    // Step 4: advance exactly one ON phase — cursor must enter the OFF phase again.
    vi.advanceTimersByTime(533);
    flushSync();
    expect(container.querySelector('.terminal-pane__cursor')).toBeNull();
  });
});
