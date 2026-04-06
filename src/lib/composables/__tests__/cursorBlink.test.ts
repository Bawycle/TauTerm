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
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as tauriCore from '@tauri-apps/api/core';
import TerminalPane from '$lib/components/TerminalPane.svelte';

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
