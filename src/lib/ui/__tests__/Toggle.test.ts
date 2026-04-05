// SPDX-License-Identifier: MPL-2.0

/**
 * Toggle component tests.
 *
 * Covered:
 *   UIBC-FN-TOG-001 — unchecked state: aria-checked="false"
 *   UIBC-FN-TOG-002 — checked state: aria-checked="true"
 *   UIBC-FN-TOG-003 — click on enabled toggle fires onchange with toggled value
 *   UIBC-FN-TOG-004 — disabled prop sets aria-disabled="true"
 *   UIBC-FN-TOG-005 — disabled toggle does not fire onchange
 *   UIBC-A11Y-TOG-001 — element has role="switch"
 *   UIBC-A11Y-TOG-002 — aria-checked reflects checked state
 *   UIBC-A11Y-TOG-003 — aria-disabled reflects disabled state
 *   UIBC-A11Y-TOG-004 — touch target is at least 44×44px (class assertion)
 *   UIBC-A11Y-TOG-005 — label text is accessible
 *   UIBC-SEC-013 — disabled toggle does not fire onchange on click
 *
 * Note: Toggle.svelte is not yet implemented (TDD). Tests for DOM rendering
 * are marked with a runtime skip guard and will become RED→GREEN once the
 * component is created. Logic-only tests (not requiring a component file) pass
 * immediately.
 *
 * @testing-library/svelte is NOT installed. Tests use Svelte 5 mount()/unmount()
 * + jsdom when the component file exists.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { existsSync } from 'fs';
import { resolve } from 'path';
import { mount } from 'svelte';

const COMPONENT_PATH = resolve(__dirname, '../Toggle.svelte');
const COMPONENT_PRESENT = existsSync(COMPONENT_PATH);

afterEach(() => {
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// Lazy import — only loaded if component file exists
// ---------------------------------------------------------------------------

async function mountToggle(props: {
  checked?: boolean;
  disabled?: boolean;
  label?: string;
  onchange?: (value: boolean) => void;
}): Promise<{ container: HTMLElement }> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  // Dynamic import so missing file causes a test failure rather than a module
  // resolution error at collection time.
  const mod = await import('../Toggle.svelte');
  mount(mod.default, { target: container, props });
  return { container };
}

// ---------------------------------------------------------------------------
// UIBC-FN-TOG-001/002 — checked/unchecked state
// ---------------------------------------------------------------------------

describe('UIBC-FN-TOG-001/002 — checked/unchecked state', () => {
  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-FN-TOG-001: unchecked toggle has aria-checked="false"',
    async () => {
      const { container } = await mountToggle({ checked: false });
      const toggle = container.querySelector('[role="switch"]')!;
      expect(toggle.getAttribute('aria-checked')).toBe('false');
    },
  );

  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-FN-TOG-002: checked toggle has aria-checked="true"',
    async () => {
      const { container } = await mountToggle({ checked: true });
      const toggle = container.querySelector('[role="switch"]')!;
      expect(toggle.getAttribute('aria-checked')).toBe('true');
    },
  );
});

// ---------------------------------------------------------------------------
// UIBC-FN-TOG-003 — onchange callback
// ---------------------------------------------------------------------------

describe('UIBC-FN-TOG-003 — onchange callback', () => {
  it.skipIf(!COMPONENT_PRESENT)('click on enabled toggle fires onchange', async () => {
    const handler = vi.fn();
    const { container } = await mountToggle({ checked: false, onchange: handler });
    const toggle = container.querySelector('[role="switch"]') as HTMLElement;
    toggle.click();
    expect(handler).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-TOG-004/005 — disabled state
// ---------------------------------------------------------------------------

describe('UIBC-FN-TOG-004/005 — disabled state', () => {
  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-FN-TOG-004: disabled toggle has aria-disabled="true"',
    async () => {
      const { container } = await mountToggle({ disabled: true });
      const toggle = container.querySelector('[role="switch"]')!;
      expect(toggle.getAttribute('aria-disabled')).toBe('true');
    },
  );

  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-FN-TOG-005 / UIBC-SEC-013: disabled toggle does not fire onchange',
    async () => {
      const handler = vi.fn();
      const { container } = await mountToggle({ disabled: true, onchange: handler });
      const toggle = container.querySelector('[role="switch"]') as HTMLElement;
      toggle.click();
      expect(handler).not.toHaveBeenCalled();
    },
  );
});

// ---------------------------------------------------------------------------
// UIBC-A11Y-TOG-001/005 — ARIA and role
// ---------------------------------------------------------------------------

describe('UIBC-A11Y-TOG — ARIA', () => {
  it.skipIf(!COMPONENT_PRESENT)('UIBC-A11Y-TOG-001: element has role="switch"', async () => {
    const { container } = await mountToggle({});
    expect(container.querySelector('[role="switch"]')).not.toBeNull();
  });

  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-A11Y-TOG-002: aria-checked mirrors checked prop',
    async () => {
      const { container: c1 } = await mountToggle({ checked: true });
      expect(c1.querySelector('[role="switch"]')!.getAttribute('aria-checked')).toBe('true');
      document.body.innerHTML = '';
      const { container: c2 } = await mountToggle({ checked: false });
      expect(c2.querySelector('[role="switch"]')!.getAttribute('aria-checked')).toBe('false');
    },
  );

  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-A11Y-TOG-003: aria-disabled mirrors disabled prop',
    async () => {
      const { container } = await mountToggle({ disabled: true });
      expect(container.querySelector('[role="switch"]')!.getAttribute('aria-disabled')).toBe(
        'true',
      );
    },
  );

  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-A11Y-TOG-004: toggle has 44×44px hit area wrapper (WCAG 2.5.5)',
    async () => {
      const { container } = await mountToggle({});
      // Toggle.svelte wraps the visual track in a <span class="w-[44px] h-[44px]">
      // hit area. The [role="switch"] input itself is sr-only; the touch target
      // is on the visual wrapper span.
      const hitArea = container.querySelector(
        'span.w-\\[44px\\].h-\\[44px\\]',
      ) as HTMLElement | null;
      // Also accept any element with both dimensions in className
      const anyWithDimensions = container.querySelector(
        '[class*="w-[44px]"][class*="h-[44px]"]',
      ) as HTMLElement | null;
      // Or find by querying all spans and checking className
      const spans = Array.from(container.querySelectorAll('span'));
      const has44Target = spans.some(
        (s) => s.className.includes('w-[44px]') && s.className.includes('h-[44px]'),
      );
      expect(
        hitArea !== null || anyWithDimensions !== null || has44Target,
        'Expected a 44×44px hit area wrapper span',
      ).toBe(true);
    },
  );

  it.skipIf(!COMPONENT_PRESENT)(
    'UIBC-A11Y-TOG-005: label text is accessible via aria-label or visible text',
    async () => {
      const { container } = await mountToggle({ label: 'Enable feature' });
      const toggle = container.querySelector('[role="switch"]') as HTMLElement;
      const hasLabel =
        toggle.getAttribute('aria-label') === 'Enable feature' ||
        toggle.getAttribute('aria-labelledby') !== null ||
        (container.textContent ?? '').includes('Enable feature');
      expect(hasLabel).toBe(true);
    },
  );
});

// ---------------------------------------------------------------------------
// Diagnostic: emit a clear message when component is not yet implemented
// ---------------------------------------------------------------------------
describe('Toggle.svelte — implementation status', () => {
  it('Toggle.svelte exists (TDD gate — will be RED until component is created)', () => {
    expect(COMPONENT_PRESENT, 'Toggle.svelte must be created before render tests run').toBe(true);
  });
});
