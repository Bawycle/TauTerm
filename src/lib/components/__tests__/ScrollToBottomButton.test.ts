// SPDX-License-Identifier: MPL-2.0

/**
 * ScrollToBottomButton component tests.
 *
 * Covered:
 *   STBB-FN-001 — renders button with correct aria-label
 *   STBB-FN-002 — onclick callback called on click
 *   STBB-FN-003 — onclick callback called on Enter key
 *   STBB-FN-004 — onclick callback called on Space key
 *   STBB-A11Y-001 — role="button" and tabindex="0"
 *   STBB-A11Y-002 — ArrowDown icon is aria-hidden
 *   STBB-UX-001  — button has expected CSS class
 *   STBB-UX-002  — button uses CSS design tokens (no hardcoded colors)
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import { flushSync } from 'svelte';
import ScrollToBottomButton from '../ScrollToBottomButton.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountButton(props: { onclick: () => void }): {
  container: HTMLElement;
  instance: ReturnType<typeof mount>;
} {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(ScrollToBottomButton, { target: container, props });
  return { container, instance };
}

const instances: ReturnType<typeof mount>[] = [];

afterEach(() => {
  instances.forEach((i) => {
    try {
      unmount(i);
    } catch {
      /* ignore */
    }
  });
  instances.length = 0;
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// Functional tests
// ---------------------------------------------------------------------------

describe('STBB-FN-001: renders button with correct aria-label', () => {
  it('has aria-label set from i18n key scroll_to_bottom', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('[role="button"]');
    expect(btn).not.toBeNull();
    // The i18n key resolves to "Go to bottom" (en) or "Aller en bas" (fr)
    const label = btn!.getAttribute('aria-label');
    expect(label).toBeTruthy();
    expect(label!.length).toBeGreaterThan(0);
  });
});

describe('STBB-FN-002: onclick callback called on click', () => {
  it('calls onclick once when button is clicked', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('[role="button"]') as HTMLElement;
    expect(btn).not.toBeNull();
    btn.click();
    expect(onclick).toHaveBeenCalledTimes(1);
  });
});

describe('STBB-FN-003: onclick callback called on Enter key', () => {
  it('calls onclick when Enter is pressed on the button', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('[role="button"]') as HTMLElement;
    expect(btn).not.toBeNull();
    const event = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true });
    btn.dispatchEvent(event);
    flushSync();
    expect(onclick).toHaveBeenCalledTimes(1);
  });
});

describe('STBB-FN-004: onclick callback called on Space key', () => {
  it('calls onclick when Space is pressed on the button', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('[role="button"]') as HTMLElement;
    expect(btn).not.toBeNull();
    const event = new KeyboardEvent('keydown', { key: ' ', bubbles: true });
    btn.dispatchEvent(event);
    flushSync();
    expect(onclick).toHaveBeenCalledTimes(1);
  });
});

describe('STBB-FN-005: other keys do not trigger onclick', () => {
  it('does not call onclick on Escape', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('[role="button"]') as HTMLElement;
    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
    btn.dispatchEvent(event);
    flushSync();
    expect(onclick).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Accessibility tests
// ---------------------------------------------------------------------------

describe('STBB-A11Y-001: role="button" and tabindex="0"', () => {
  it('has role=button and tabindex=0', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('[role="button"]');
    expect(btn).not.toBeNull();
    expect(btn!.getAttribute('tabindex')).toBe('0');
  });
});

describe('STBB-A11Y-002: ArrowDown icon is aria-hidden', () => {
  it('svg icon inside button has aria-hidden=true', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute('aria-hidden')).toBe('true');
  });
});

// ---------------------------------------------------------------------------
// UX / Visual tests
// ---------------------------------------------------------------------------

describe('STBB-UX-001: button has expected CSS class', () => {
  it('button element has scroll-to-bottom-btn class', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('.scroll-to-bottom-btn');
    expect(btn).not.toBeNull();
  });
});

describe('STBB-UX-002: button uses design tokens (no hardcoded colors)', () => {
  it('inline style is absent (colors come from CSS class variables)', () => {
    const onclick = vi.fn();
    const { container, instance } = mountButton({ onclick });
    instances.push(instance);
    const btn = container.querySelector('.scroll-to-bottom-btn') as HTMLElement;
    expect(btn).not.toBeNull();
    // No inline color overrides — all colors via CSS custom properties in the class
    const style = btn.getAttribute('style') ?? '';
    expect(style).not.toMatch(/color\s*:/);
    expect(style).not.toMatch(/background/);
  });
});
