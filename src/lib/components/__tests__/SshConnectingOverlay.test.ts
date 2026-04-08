// SPDX-License-Identifier: MPL-2.0

/**
 * SshConnectingOverlay component tests.
 *
 * Covered:
 *   SSH-OVERLAY-001 — state='connecting' renders "Connecting…" text
 *   SSH-OVERLAY-002 — state='connecting' has role="status" and aria-live="polite"
 *   SSH-OVERLAY-003 — state='authenticating' renders "Authenticating…" text
 *   SSH-OVERLAY-004 — icon is decorative (aria-hidden)
 *   SSH-OVERLAY-005 — spin animation class applied for 'connecting'
 *   SSH-OVERLAY-006 — pulse animation class applied for 'authenticating'
 */

import { describe, it, expect, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import SshConnectingOverlay from '../SshConnectingOverlay.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountOverlay(props: { state: 'connecting' | 'authenticating' }): {
  container: HTMLElement;
  instance: ReturnType<typeof mount>;
} {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(SshConnectingOverlay, { target: container, props });
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
// SSH-OVERLAY-001: connecting state shows "Connecting…" text
// ---------------------------------------------------------------------------

describe('SSH-OVERLAY-001: connecting state text', () => {
  it('renders "Connecting…" when state is connecting', () => {
    const { container, instance } = mountOverlay({ state: 'connecting' });
    instances.push(instance);
    expect(container.textContent).toContain('Connecting');
  });
});

// ---------------------------------------------------------------------------
// SSH-OVERLAY-002: role="status" and aria-live="polite" on the overlay root
// ---------------------------------------------------------------------------

describe('SSH-OVERLAY-002: ARIA live region attributes', () => {
  it('has role="status" for non-intrusive screen reader announcement', () => {
    const { container, instance } = mountOverlay({ state: 'connecting' });
    instances.push(instance);
    const overlay = container.querySelector('.ssh-connecting-overlay');
    expect(overlay).not.toBeNull();
    expect(overlay!.getAttribute('role')).toBe('status');
  });

  it('has aria-live="polite"', () => {
    const { container, instance } = mountOverlay({ state: 'connecting' });
    instances.push(instance);
    const overlay = container.querySelector('.ssh-connecting-overlay');
    expect(overlay).not.toBeNull();
    expect(overlay!.getAttribute('aria-live')).toBe('polite');
  });
});

// ---------------------------------------------------------------------------
// SSH-OVERLAY-003: authenticating state shows "Authenticating…" text
// ---------------------------------------------------------------------------

describe('SSH-OVERLAY-003: authenticating state text', () => {
  it('renders "Authenticating…" when state is authenticating', () => {
    const { container, instance } = mountOverlay({ state: 'authenticating' });
    instances.push(instance);
    expect(container.textContent).toContain('Authenticating');
  });

  it('does NOT show "Connecting" text in authenticating state', () => {
    const { container, instance } = mountOverlay({ state: 'authenticating' });
    instances.push(instance);
    // Label is derived — only one string should be visible
    expect(container.textContent).not.toContain('Connecting');
  });
});

// ---------------------------------------------------------------------------
// SSH-OVERLAY-004: icon is decorative (aria-hidden)
// ---------------------------------------------------------------------------

describe('SSH-OVERLAY-004: icon is aria-hidden', () => {
  it('icon wrapper has aria-hidden="true" in connecting state', () => {
    const { container, instance } = mountOverlay({ state: 'connecting' });
    instances.push(instance);
    const icon = container.querySelector('.ssh-connecting-overlay__icon');
    expect(icon).not.toBeNull();
    expect(icon!.getAttribute('aria-hidden')).toBe('true');
  });

  it('icon wrapper has aria-hidden="true" in authenticating state', () => {
    const { container, instance } = mountOverlay({ state: 'authenticating' });
    instances.push(instance);
    const icon = container.querySelector('.ssh-connecting-overlay__icon');
    expect(icon).not.toBeNull();
    expect(icon!.getAttribute('aria-hidden')).toBe('true');
  });
});

// ---------------------------------------------------------------------------
// SSH-OVERLAY-005: spin class applied for 'connecting'
// ---------------------------------------------------------------------------

describe('SSH-OVERLAY-005: spin animation class for connecting state', () => {
  it('icon has --spin class in connecting state', () => {
    const { container, instance } = mountOverlay({ state: 'connecting' });
    instances.push(instance);
    const icon = container.querySelector('.ssh-connecting-overlay__icon');
    expect(icon).not.toBeNull();
    expect(icon!.classList.contains('ssh-connecting-overlay__icon--spin')).toBe(true);
  });

  it('icon does NOT have --pulse class in connecting state', () => {
    const { container, instance } = mountOverlay({ state: 'connecting' });
    instances.push(instance);
    const icon = container.querySelector('.ssh-connecting-overlay__icon');
    expect(icon!.classList.contains('ssh-connecting-overlay__icon--pulse')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// SSH-OVERLAY-006: pulse class applied for 'authenticating'
// ---------------------------------------------------------------------------

describe('SSH-OVERLAY-006: pulse animation class for authenticating state', () => {
  it('icon has --pulse class in authenticating state', () => {
    const { container, instance } = mountOverlay({ state: 'authenticating' });
    instances.push(instance);
    const icon = container.querySelector('.ssh-connecting-overlay__icon');
    expect(icon).not.toBeNull();
    expect(icon!.classList.contains('ssh-connecting-overlay__icon--pulse')).toBe(true);
  });

  it('icon does NOT have --spin class in authenticating state', () => {
    const { container, instance } = mountOverlay({ state: 'authenticating' });
    instances.push(instance);
    const icon = container.querySelector('.ssh-connecting-overlay__icon');
    expect(icon!.classList.contains('ssh-connecting-overlay__icon--spin')).toBe(false);
  });
});
