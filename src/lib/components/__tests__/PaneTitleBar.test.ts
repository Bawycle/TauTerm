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

import { describe, it, expect, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import PaneTitleBar from '../PaneTitleBar.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountBar(props: { title: string; isActive: boolean }): {
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
