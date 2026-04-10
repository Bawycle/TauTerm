// SPDX-License-Identifier: MPL-2.0

/**
 * PaneTitleBar component tests.
 *
 * Covered:
 *   PTB-FN-001 — renders the provided title text
 *   PTB-FN-002 — applies pane-title-bar--active class when isActive=true
 *   PTB-FN-003 — does not apply pane-title-bar--active class when isActive=false
 *   PTB-A11Y-001 — root element has aria-hidden="true"
 */

import { describe, it, expect, afterEach, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import PaneTitleBar from '../PaneTitleBar.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountBar(props: {
  title: string;
  isActive: boolean;
  onrename?: (label: string | null) => void;
}): {
  container: HTMLElement;
  instance: ReturnType<typeof mount>;
} {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(PaneTitleBar, { target: container, props });
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
// PTB-FN-001: renders the provided title text
// ---------------------------------------------------------------------------

describe('PTB-FN-001: renders the provided title text', () => {
  it('displays the title string inside the bar', () => {
    const { container, instance } = mountBar({ title: 'vim ~/src/main.rs', isActive: false });
    instances.push(instance);
    expect(container.textContent).toContain('vim ~/src/main.rs');
  });

  it('displays an empty title without error', () => {
    const { container, instance } = mountBar({ title: '', isActive: false });
    instances.push(instance);
    const bar = container.querySelector('.pane-title-bar');
    expect(bar).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// PTB-FN-002: applies pane-title-bar--active class when isActive=true
// ---------------------------------------------------------------------------

describe('PTB-FN-002: pane-title-bar--active class present when isActive=true', () => {
  it('root element has pane-title-bar--active when isActive=true', () => {
    const { container, instance } = mountBar({ title: 'bash', isActive: true });
    instances.push(instance);
    const bar = container.querySelector('.pane-title-bar');
    expect(bar).not.toBeNull();
    expect(bar!.classList.contains('pane-title-bar--active')).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// PTB-FN-003: does not apply pane-title-bar--active class when isActive=false
// ---------------------------------------------------------------------------

describe('PTB-FN-003: pane-title-bar--active class absent when isActive=false', () => {
  it('root element does not have pane-title-bar--active when isActive=false', () => {
    const { container, instance } = mountBar({ title: 'bash', isActive: false });
    instances.push(instance);
    const bar = container.querySelector('.pane-title-bar');
    expect(bar).not.toBeNull();
    expect(bar!.classList.contains('pane-title-bar--active')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// PTB-A11Y-001: root element has aria-hidden="true"
// ---------------------------------------------------------------------------

describe('PTB-A11Y-001: root element has aria-hidden="true"', () => {
  it('pane-title-bar root div carries aria-hidden=true', () => {
    const { container, instance } = mountBar({ title: 'zsh', isActive: false });
    instances.push(instance);
    const bar = container.querySelector('.pane-title-bar');
    expect(bar).not.toBeNull();
    expect(bar!.getAttribute('aria-hidden')).toBe('true');
  });

  it('aria-hidden is true regardless of isActive state', () => {
    const { container, instance } = mountBar({ title: 'zsh', isActive: true });
    instances.push(instance);
    const bar = container.querySelector('.pane-title-bar');
    expect(bar!.getAttribute('aria-hidden')).toBe('true');
  });
});

// ---------------------------------------------------------------------------
// PTB-RN-001: double-click enters rename mode
// ---------------------------------------------------------------------------

describe('PTB-RN-001: double-click enters rename mode', () => {
  it('shows an input with the current title after double-click', () => {
    const { container, instance } = mountBar({ title: 'bash', isActive: false });
    instances.push(instance);

    const bar = container.querySelector('.pane-title-bar')!;
    bar.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    flushSync();

    const input = container.querySelector<HTMLInputElement>('.pane-title-bar__input');
    expect(input).not.toBeNull();
    expect(input!.value).toBe('bash');
  });

  it('hides the title span while the input is shown', () => {
    const { container, instance } = mountBar({ title: 'vim', isActive: true });
    instances.push(instance);

    const bar = container.querySelector('.pane-title-bar')!;
    bar.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    flushSync();

    expect(container.querySelector('.pane-title-bar__title')).toBeNull();
    expect(container.querySelector('.pane-title-bar__input')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// PTB-RN-002: Enter confirms rename
// ---------------------------------------------------------------------------

describe('PTB-RN-002: Enter confirms rename and calls onrename', () => {
  it('calls onrename with trimmed value and hides input on Enter', () => {
    const onrename = vi.fn();
    const { container, instance } = mountBar({ title: 'bash', isActive: false, onrename });
    instances.push(instance);

    container.querySelector('.pane-title-bar')!
      .dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    flushSync();

    const input = container.querySelector<HTMLInputElement>('.pane-title-bar__input')!;
    input.value = '  my-server  ';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    flushSync();

    expect(onrename).toHaveBeenCalledWith('my-server');
    expect(container.querySelector('.pane-title-bar__input')).toBeNull();
  });

  it('calls onrename(null) when input is empty on Enter', () => {
    const onrename = vi.fn();
    const { container, instance } = mountBar({ title: 'bash', isActive: false, onrename });
    instances.push(instance);

    container.querySelector('.pane-title-bar')!
      .dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    flushSync();

    const input = container.querySelector<HTMLInputElement>('.pane-title-bar__input')!;
    input.value = '   ';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    flushSync();

    expect(onrename).toHaveBeenCalledWith(null);
  });
});

// ---------------------------------------------------------------------------
// PTB-RN-003: Escape cancels rename
// ---------------------------------------------------------------------------

describe('PTB-RN-003: Escape cancels rename without calling onrename', () => {
  it('hides the input and does not call onrename on Escape', () => {
    const onrename = vi.fn();
    const { container, instance } = mountBar({ title: 'bash', isActive: false, onrename });
    instances.push(instance);

    container.querySelector('.pane-title-bar')!
      .dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    flushSync();

    container.querySelector<HTMLInputElement>('.pane-title-bar__input')!
      .dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    flushSync();

    expect(onrename).not.toHaveBeenCalled();
    expect(container.querySelector('.pane-title-bar__input')).toBeNull();
    expect(container.querySelector('.pane-title-bar__title')).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// PTB-RN-004: blur confirms rename
// ---------------------------------------------------------------------------

describe('PTB-RN-004: blur confirms rename', () => {
  it('calls onrename with the current value when input loses focus', () => {
    const onrename = vi.fn();
    const { container, instance } = mountBar({ title: 'bash', isActive: false, onrename });
    instances.push(instance);

    container.querySelector('.pane-title-bar')!
      .dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    flushSync();

    const input = container.querySelector<HTMLInputElement>('.pane-title-bar__input')!;
    input.value = 'staging';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new FocusEvent('blur', { bubbles: true }));
    flushSync();

    expect(onrename).toHaveBeenCalledWith('staging');
    expect(container.querySelector('.pane-title-bar__input')).toBeNull();
  });
});
