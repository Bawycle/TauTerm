// SPDX-License-Identifier: MPL-2.0

/**
 * TextInput component tests.
 *
 * Covered:
 *   UIBC-FN-INP-001 — renders label when provided
 *   UIBC-FN-INP-002 — label is associated with input via for/id
 *   UIBC-FN-INP-003 — renders placeholder text
 *   UIBC-FN-INP-004 — reflects value prop
 *   UIBC-FN-INP-005 — error state adds error border class
 *   UIBC-FN-INP-006 — error message is rendered
 *   UIBC-FN-INP-007 — disabled state sets disabled attribute
 *   UIBC-FN-INP-008 — helper text rendered when no error
 *   UIBC-FN-INP-009 — helper text hidden when error is present
 *   UIBC-FN-INP-010 — oninput callback fires with new value
 *   UIBC-FN-INP-011 — onchange callback fires with new value
 *   UIBC-A11Y-INP-001 — aria-invalid=true when error, false otherwise
 *   UIBC-A11Y-INP-002 — aria-describedby points to error element id
 *   UIBC-A11Y-INP-003 — aria-describedby points to helper element id when no error
 *   UIBC-A11Y-INP-004 — input has min-h-[44px] for touch target
 *   UIBC-SEC-002 — XSS via placeholder rendered as attribute value, not markup
 *   UIBC-SEC-010 — maxlength attribute is applied to the input element
 *   UIBC-SEC-011 — special characters in value are rendered verbatim
 *
 * Tests use Svelte 5 mount()/unmount() + jsdom.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount } from 'svelte';
import TextInput from '../TextInput.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type TextInputProps = {
  value?: string;
  placeholder?: string;
  disabled?: boolean;
  error?: string;
  label?: string;
  id?: string;
  helper?: string;
  type?: string;
  maxlength?: number;
  onchange?: (value: string) => void;
  oninput?: (value: string) => void;
};

function mountInput(props: TextInputProps = {}): { container: HTMLElement } {
  const container = document.createElement('div');
  document.body.appendChild(container);
  mount(TextInput, { target: container, props });
  return { container };
}

afterEach(() => {
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// UIBC-FN-INP-001/002 — label rendering and association
// ---------------------------------------------------------------------------

describe('UIBC-FN-INP-001/002 — label', () => {
  it('UIBC-FN-INP-001: renders label element when label prop is provided', () => {
    const { container } = mountInput({ label: 'Username', id: 'username' });
    const label = container.querySelector('label');
    expect(label).not.toBeNull();
    expect(label!.textContent?.trim()).toBe('Username');
  });

  it('UIBC-FN-INP-002: label for attribute matches input id', () => {
    const { container } = mountInput({ label: 'Username', id: 'username' });
    const label = container.querySelector('label');
    const input = container.querySelector('input');
    expect(label!.getAttribute('for')).toBe('username');
    expect(input!.id).toBe('username');
  });

  it('does not render label element when label prop is absent', () => {
    const { container } = mountInput({ id: 'no-label' });
    expect(container.querySelector('label')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-INP-003/004 — placeholder and value
// ---------------------------------------------------------------------------

describe('UIBC-FN-INP-003/004 — placeholder and value', () => {
  it('UIBC-FN-INP-003: renders placeholder attribute', () => {
    const { container } = mountInput({ placeholder: 'Enter name…' });
    const input = container.querySelector('input')!;
    expect(input.getAttribute('placeholder')).toBe('Enter name…');
  });

  it('UIBC-FN-INP-004: reflects value prop', () => {
    const { container } = mountInput({ value: 'hello' });
    const input = container.querySelector('input')!;
    expect(input.value).toBe('hello');
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-INP-005/006 — error state
// ---------------------------------------------------------------------------

describe('UIBC-FN-INP-005/006 — error state', () => {
  it('UIBC-FN-INP-005: error prop adds error border class', () => {
    const { container } = mountInput({ error: 'Required', id: 'f' });
    const input = container.querySelector('input')!;
    expect(input.className).toContain('border-(--color-error)');
  });

  it('UIBC-FN-INP-006: error message is rendered in DOM', () => {
    const { container } = mountInput({ error: 'Required field', id: 'f' });
    const errEl = container.querySelector('[id="f-error"]');
    expect(errEl).not.toBeNull();
    expect(errEl!.textContent?.trim()).toBe('Required field');
  });

  it('no error border when error is absent', () => {
    const { container } = mountInput({ id: 'f' });
    const input = container.querySelector('input')!;
    expect(input.className).not.toContain('border-(--color-error)');
    expect(input.className).toContain('border-(--color-border)');
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-INP-007 — disabled state
// ---------------------------------------------------------------------------

describe('UIBC-FN-INP-007 — disabled state', () => {
  it('sets disabled attribute when disabled=true', () => {
    const { container } = mountInput({ disabled: true });
    expect(container.querySelector('input')!.disabled).toBe(true);
  });

  it('input is enabled by default', () => {
    const { container } = mountInput();
    expect(container.querySelector('input')!.disabled).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-INP-008/009 — helper text
// ---------------------------------------------------------------------------

describe('UIBC-FN-INP-008/009 — helper text', () => {
  it('UIBC-FN-INP-008: helper text rendered when no error', () => {
    const { container } = mountInput({ helper: 'Max 64 chars', id: 'f' });
    const helperEl = container.querySelector('[id="f-helper"]');
    expect(helperEl).not.toBeNull();
    expect(helperEl!.textContent?.trim()).toBe('Max 64 chars');
  });

  it('UIBC-FN-INP-009: helper text hidden when error is present', () => {
    const { container } = mountInput({ helper: 'Max 64 chars', error: 'Too long', id: 'f' });
    expect(container.querySelector('[id="f-helper"]')).toBeNull();
    expect(container.querySelector('[id="f-error"]')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-INP-010/011 — callbacks
// ---------------------------------------------------------------------------

describe('UIBC-FN-INP-010/011 — oninput and onchange callbacks', () => {
  it('UIBC-FN-INP-010: oninput fires with new value', () => {
    const handler = vi.fn();
    const { container } = mountInput({ oninput: handler });
    const input = container.querySelector('input')!;
    Object.defineProperty(input, 'value', { value: 'typed', configurable: true });
    input.dispatchEvent(new Event('input', { bubbles: true }));
    expect(handler).toHaveBeenCalledWith('typed');
  });

  it('UIBC-FN-INP-011: onchange fires with new value', () => {
    const handler = vi.fn();
    const { container } = mountInput({ onchange: handler });
    const input = container.querySelector('input')!;
    Object.defineProperty(input, 'value', { value: 'changed', configurable: true });
    input.dispatchEvent(new Event('change', { bubbles: true }));
    expect(handler).toHaveBeenCalledWith('changed');
  });
});

// ---------------------------------------------------------------------------
// UIBC-A11Y-INP — aria attributes
// ---------------------------------------------------------------------------

describe('UIBC-A11Y-INP — ARIA attributes', () => {
  it('UIBC-A11Y-INP-001: aria-invalid=true when error is set', () => {
    const { container } = mountInput({ error: 'Bad', id: 'f' });
    const input = container.querySelector('input')!;
    expect(input.getAttribute('aria-invalid')).toBe('true');
  });

  it('UIBC-A11Y-INP-001: aria-invalid=false when no error', () => {
    const { container } = mountInput({ id: 'f' });
    const input = container.querySelector('input')!;
    // aria-invalid="false" or attribute absent — both are valid; check it is not "true"
    expect(input.getAttribute('aria-invalid')).not.toBe('true');
  });

  it('UIBC-A11Y-INP-002: aria-describedby references error element', () => {
    const { container } = mountInput({ error: 'Required', id: 'f' });
    const input = container.querySelector('input')!;
    expect(input.getAttribute('aria-describedby')).toBe('f-error');
  });

  it('UIBC-A11Y-INP-003: aria-describedby references helper element when no error', () => {
    const { container } = mountInput({ helper: 'Hint', id: 'f' });
    const input = container.querySelector('input')!;
    expect(input.getAttribute('aria-describedby')).toBe('f-helper');
  });

  it('UIBC-A11Y-INP-004: input carries min-h-[44px] touch target class', () => {
    const { container } = mountInput();
    expect(container.querySelector('input')!.className).toContain('h-[44px]');
  });
});

// ---------------------------------------------------------------------------
// UIBC-SEC-002 — XSS via placeholder
// ---------------------------------------------------------------------------

describe('UIBC-SEC-002 — XSS via placeholder', () => {
  it('script tag in placeholder is stored as attribute value, not executed', () => {
    const xss = '<script>window.__xss_placeholder=true<\/script>';
    const { container } = mountInput({ placeholder: xss });
    expect((window as unknown as Record<string, unknown>).__xss_placeholder).toBeUndefined();
    // The raw attribute value is the xss string; it must not produce a <script> element
    expect(container.querySelector('script')).toBeNull();
  });

  it('img onerror payload in placeholder is not executed', () => {
    const xss = '"><img src=x onerror="window.__xss_ph_img=true">';
    mountInput({ placeholder: xss });
    expect((window as unknown as Record<string, unknown>).__xss_ph_img).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// UIBC-SEC-010 — maxlength enforcement
// ---------------------------------------------------------------------------

describe('UIBC-SEC-010 — maxlength attribute', () => {
  it('maxlength prop is applied to the input element', () => {
    const { container } = mountInput({ maxlength: 64 });
    const input = container.querySelector('input')!;
    expect(input.getAttribute('maxlength')).toBe('64');
  });

  it('maxlength absent when prop is not set', () => {
    const { container } = mountInput();
    expect(container.querySelector('input')!.getAttribute('maxlength')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// UIBC-SEC-011 — special characters rendered verbatim
// ---------------------------------------------------------------------------

describe('UIBC-SEC-011 — special characters in value', () => {
  it('angle brackets in value are stored verbatim in input.value', () => {
    const { container } = mountInput({ value: '<b>bold</b>' });
    expect(container.querySelector('input')!.value).toBe('<b>bold</b>');
  });

  it('ampersands in value are stored verbatim', () => {
    const { container } = mountInput({ value: 'a & b' });
    expect(container.querySelector('input')!.value).toBe('a & b');
  });

  it('null bytes in value are stored verbatim', () => {
    const { container } = mountInput({ value: 'foo\x00bar' });
    expect(container.querySelector('input')!.value).toBe('foo\x00bar');
  });
});
