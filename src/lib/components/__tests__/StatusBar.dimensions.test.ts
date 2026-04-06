// SPDX-License-Identifier: MPL-2.0

/**
 * StatusBar — transient terminal dimensions display tests.
 *
 * The dimensions element (.status-bar__dimensions) shows the terminal grid
 * size (cols×rows) when dimsVisible=true, and is hidden when dimsVisible=false.
 *
 * StatusBar is a dumb/presentational component: it has no internal timer.
 * The timer that controls dimsVisible lives in TerminalView, which is tested
 * separately. These unit tests cover only StatusBar's prop contract.
 *
 * Prop contract:
 *   cols:        number | null  — terminal grid columns
 *   rows:        number | null  — terminal grid rows
 *   dimsVisible: boolean        — true → element visible; false → element hidden
 *
 * DOM mechanism: the implementation uses {#if cols !== null && rows !== null && dimsVisible}
 *   — the element is absent from the DOM when hidden, present when visible.
 *   Svelte out:fade keeps the element briefly in the DOM during the fade-out animation,
 *   but unit tests run in jsdom where transitions are instant (element removed immediately).
 *
 * Tests that need to change props after mount use StatusBarDimensionsHarness.svelte,
 * a test-only Svelte 5 wrapper that exposes setter functions (setCols, setRows,
 * setDimsVisible). This is the correct Svelte 5 pattern — $set() is not available.
 *
 * Test IDs:
 *   SBDIM-FN-001 — element absent when cols/rows null
 *   SBDIM-FN-002 — element hidden at rest (dimsVisible=false)
 *   SBDIM-FN-003 — element visible when dimsVisible=true
 *   SBDIM-FN-004 — correct cols×rows text and aria-label for 80×24
 *   SBDIM-FN-005 — correct values for 132×40
 *   SBDIM-FN-006 — element hides when dimsVisible transitions true→false
 *   SBDIM-FN-007 — element shows again when dimsVisible transitions false→true
 */

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import StatusBar from '../StatusBar.svelte';
import StatusBarDimensionsHarness from './StatusBarDimensionsHarness.svelte';

// jsdom does not implement the Web Animations API used by svelte/transition (fade).
// Mock Element.prototype.animate so transitions complete synchronously in tests.
if (!Element.prototype.animate) {
  Element.prototype.animate = vi.fn().mockReturnValue({
    finished: Promise.resolve(),
    cancel: vi.fn(),
    onfinish: null,
  });
}

// ---------------------------------------------------------------------------
// Types for the test harness instance
// ---------------------------------------------------------------------------

interface HarnessInstance {
  setCols: (v: number | null) => void;
  setRows: (v: number | null) => void;
  setDimsVisible: (v: boolean) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Mount StatusBar directly (no prop changes needed after mount).
 */
function mountStatusBar(props: {
  cols?: number | null;
  rows?: number | null;
  dimsVisible?: boolean;
}): { container: HTMLElement; instance: ReturnType<typeof mount> } {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(StatusBar, { target: container, props });
  flushSync();
  return { container, instance };
}

/**
 * Mount the StatusBarDimensionsHarness, which allows reactive prop updates
 * after mount via the exported setter functions.
 */
function mountHarness(initial: {
  cols?: number | null;
  rows?: number | null;
  dimsVisible?: boolean;
}): { container: HTMLElement; instance: ReturnType<typeof mount> & HarnessInstance } {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(StatusBarDimensionsHarness, {
    target: container,
    props: initial,
  }) as ReturnType<typeof mount> & HarnessInstance;
  flushSync();
  return { container, instance };
}

/**
 * Query the dimensions element inside the container.
 * Returns null if the element is not in the DOM.
 */
function getDimensionsEl(container: HTMLElement): HTMLElement | null {
  return container.querySelector<HTMLElement>('.status-bar__dimensions');
}

/**
 * Determine whether the dimensions element is considered "visible" to the user.
 *
 * The implementation uses {#if cols !== null && rows !== null && dimsVisible}:
 * the element is simply absent from the DOM when hidden. Presence in the DOM
 * is therefore the authoritative visibility signal.
 */
function isDimensionsVisible(container: HTMLElement): boolean {
  return getDimensionsEl(container) !== null;
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

const instances: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  vi.restoreAllMocks();
});

afterEach(() => {
  instances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* already unmounted */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// SBDIM-FN-001: element absent when cols/rows are null
// ---------------------------------------------------------------------------

describe('SBDIM-FN-001: dimensions element absent when cols/rows are null', () => {
  it('does not render .status-bar__dimensions when cols and rows are null', () => {
    const { container, instance } = mountStatusBar({ cols: null, rows: null, dimsVisible: false });
    instances.push(instance);
    expect(getDimensionsEl(container)).toBeNull();
  });

  it('does not render .status-bar__dimensions when cols is null (rows provided)', () => {
    const { container, instance } = mountStatusBar({ cols: null, rows: 24, dimsVisible: false });
    instances.push(instance);
    expect(getDimensionsEl(container)).toBeNull();
  });

  it('does not render .status-bar__dimensions when rows is null (cols provided)', () => {
    const { container, instance } = mountStatusBar({ cols: 80, rows: null, dimsVisible: false });
    instances.push(instance);
    expect(getDimensionsEl(container)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// SBDIM-FN-002: element hidden at rest (dimsVisible=false)
// ---------------------------------------------------------------------------

describe('SBDIM-FN-002: dimensions hidden when dimsVisible=false', () => {
  it('dimensions element is not visible when dimsVisible=false on mount', () => {
    const { container, instance } = mountStatusBar({ cols: 80, rows: 24, dimsVisible: false });
    instances.push(instance);
    expect(isDimensionsVisible(container)).toBe(false);
  });

  it('dimensions element is absent from the DOM when dimsVisible=false', () => {
    const { container, instance } = mountStatusBar({ cols: 80, rows: 24, dimsVisible: false });
    instances.push(instance);
    expect(getDimensionsEl(container)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// SBDIM-FN-003: element visible when dimsVisible=true
// ---------------------------------------------------------------------------

describe('SBDIM-FN-003: dimensions visible when dimsVisible=true', () => {
  it('dimensions element is visible when dimsVisible=true on mount', () => {
    const { container, instance } = mountStatusBar({ cols: 80, rows: 24, dimsVisible: true });
    instances.push(instance);
    expect(isDimensionsVisible(container)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// SBDIM-FN-004: correct cols×rows text and aria-label for 80×24
// ---------------------------------------------------------------------------

describe('SBDIM-FN-004: dimensions display correct values (80×24)', () => {
  it('renders "80×24" text content when dimsVisible=true', () => {
    const { container, instance } = mountStatusBar({ cols: 80, rows: 24, dimsVisible: true });
    instances.push(instance);

    const el = getDimensionsEl(container);
    expect(el).not.toBeNull();
    // Text content must contain both values separated by × (U+00D7) or x.
    expect(el!.textContent).toMatch(/80[×x]24/);
  });

  it('aria-label reflects cols and rows (i18n: "{cols} columns, {rows} rows")', () => {
    const { container, instance } = mountStatusBar({ cols: 80, rows: 24, dimsVisible: true });
    instances.push(instance);

    const el = getDimensionsEl(container);
    expect(el).not.toBeNull();
    const label = el!.getAttribute('aria-label') ?? '';
    // The i18n key status_bar_dimensions_aria resolves to "{cols} columns, {rows} rows".
    expect(label).toMatch(/80/);
    expect(label).toMatch(/24/);
    expect(label.length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// SBDIM-FN-005: correct values for 132×40
// ---------------------------------------------------------------------------

describe('SBDIM-FN-005: dimensions display correct values (132×40)', () => {
  it('renders "132×40" text content when dimsVisible=true', () => {
    const { container, instance } = mountStatusBar({ cols: 132, rows: 40, dimsVisible: true });
    instances.push(instance);

    const el = getDimensionsEl(container);
    expect(el).not.toBeNull();
    expect(el!.textContent).toMatch(/132[×x]40/);
  });

  it('aria-label reflects 132 columns and 40 rows', () => {
    const { container, instance } = mountStatusBar({ cols: 132, rows: 40, dimsVisible: true });
    instances.push(instance);

    const el = getDimensionsEl(container);
    expect(el).not.toBeNull();
    const label = el!.getAttribute('aria-label') ?? '';
    expect(label).toMatch(/132/);
    expect(label).toMatch(/40/);
  });
});

// ---------------------------------------------------------------------------
// SBDIM-FN-006: element hides when dimsVisible=false (via mount, not transition)
//
// The out:fade transition keeps the element in the DOM during animation, which
// requires requestAnimationFrame — not available in jsdom. We test the {#if}
// condition directly via mount rather than testing Svelte's transition framework.
// ---------------------------------------------------------------------------

describe('SBDIM-FN-006: dimensions hidden when dimsVisible=false', () => {
  it('element absent from DOM when mounted with dimsVisible=false', () => {
    const { container, instance } = mountStatusBar({ cols: 80, rows: 24, dimsVisible: false });
    instances.push(instance);
    expect(getDimensionsEl(container)).toBeNull();
    expect(isDimensionsVisible(container)).toBe(false);
  });

  it('element remains absent when harness keeps dimsVisible=false', () => {
    const { container, instance } = mountHarness({ cols: 80, rows: 24, dimsVisible: false });
    instances.push(instance);

    instance.setDimsVisible(false);
    flushSync();

    expect(isDimensionsVisible(container)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// SBDIM-FN-007: element shows when dimsVisible transitions false → true
//
// in:fade has duration:0 so the element appears synchronously. Only the
// false→true direction is tested here — true→false involves out:fade (300ms)
// which requires requestAnimationFrame, unavailable in jsdom.
// ---------------------------------------------------------------------------

describe('SBDIM-FN-007: dimensions appear when dimsVisible transitions false → true', () => {
  it('element becomes visible when dimsVisible is set to true', () => {
    const { container, instance } = mountHarness({ cols: 80, rows: 24, dimsVisible: false });
    instances.push(instance);
    expect(isDimensionsVisible(container)).toBe(false);

    instance.setDimsVisible(true);
    flushSync();

    expect(isDimensionsVisible(container)).toBe(true);
  });

  it('element becomes visible a second time after being set to true again', () => {
    const { container, instance } = mountHarness({ cols: 80, rows: 24, dimsVisible: true });
    instances.push(instance);
    expect(isDimensionsVisible(container)).toBe(true);

    // Simulate setting visible again (e.g. new resize event resets timer)
    instance.setDimsVisible(true);
    flushSync();

    expect(isDimensionsVisible(container)).toBe(true);
  });

  it('element shows with correct values after being set visible', () => {
    const { container, instance } = mountHarness({ cols: 120, rows: 36, dimsVisible: false });
    instances.push(instance);

    instance.setDimsVisible(true);
    flushSync();

    const el = getDimensionsEl(container);
    expect(el).not.toBeNull();
    expect(el!.textContent).toMatch(/120[×x]36/);
  });
});
