// SPDX-License-Identifier: MPL-2.0

/**
 * SshReconnectionSeparator component tests.
 *
 * Covered:
 *   UXD-SSH-SEP-001 — renders with timestamp: shows formatted HH:MM:SS
 *   UXD-SSH-SEP-002 — renders without timestamp: shows "reconnected" only
 *   UXD-SSH-SEP-003 — renders with timestampMs=0: treats as unavailable
 *   UXD-SSH-SEP-004 — is aria-hidden (decorative overlay)
 *   UXD-SSH-SEP-005 — has correct CSS class for visual spec
 *   UXD-SSH-SEP-006 — label element is present
 */

import { describe, it, expect, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import SshReconnectionSeparator from '../SshReconnectionSeparator.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountSep(props: { timestampMs?: number }): {
  container: HTMLElement;
  instance: ReturnType<typeof mount>;
} {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(SshReconnectionSeparator, { target: container, props });
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

describe('UXD-SSH-SEP-001: renders with timestamp', () => {
  it('shows formatted time HH:MM:SS when timestampMs is provided', () => {
    // 2024-01-15 14:30:45 UTC → but local time may differ; just check pattern
    const ts = new Date(2024, 0, 15, 14, 30, 45).getTime(); // local time
    const { container, instance } = mountSep({ timestampMs: ts });
    instances.push(instance);
    const label = container.querySelector('.ssh-reconnection-separator__label');
    expect(label).not.toBeNull();
    // Should match HH:MM:SS pattern somewhere in the label
    expect(label!.textContent).toMatch(/\d{2}:\d{2}:\d{2}/);
  });
});

describe('UXD-SSH-SEP-002: renders without timestamp', () => {
  it('shows "reconnected" without time when no timestampMs provided', () => {
    const { container, instance } = mountSep({});
    instances.push(instance);
    const label = container.querySelector('.ssh-reconnection-separator__label');
    expect(label).not.toBeNull();
    expect(label!.textContent).toBeTruthy();
    // Should not contain time pattern (HH:MM:SS)
    expect(label!.textContent).not.toMatch(/\d{2}:\d{2}:\d{2}/);
  });
});

describe('UXD-SSH-SEP-003: timestampMs=0 treated as unavailable', () => {
  it('shows label without time when timestampMs is 0', () => {
    const { container, instance } = mountSep({ timestampMs: 0 });
    instances.push(instance);
    const label = container.querySelector('.ssh-reconnection-separator__label');
    expect(label).not.toBeNull();
    expect(label!.textContent).not.toMatch(/\d{2}:\d{2}:\d{2}/);
  });
});

// ---------------------------------------------------------------------------
// Accessibility tests
// ---------------------------------------------------------------------------

describe('UXD-SSH-SEP-004: separator is aria-hidden', () => {
  it('root element has aria-hidden=true', () => {
    const { container, instance } = mountSep({ timestampMs: Date.now() });
    instances.push(instance);
    const sep = container.querySelector('.ssh-reconnection-separator');
    expect(sep).not.toBeNull();
    expect(sep!.getAttribute('aria-hidden')).toBe('true');
  });
});

// ---------------------------------------------------------------------------
// Visual / CSS tests
// ---------------------------------------------------------------------------

describe('UXD-SSH-SEP-005: correct CSS class', () => {
  it('has ssh-reconnection-separator CSS class', () => {
    const { container, instance } = mountSep({});
    instances.push(instance);
    expect(container.querySelector('.ssh-reconnection-separator')).not.toBeNull();
  });
});

describe('UXD-SSH-SEP-006: label element is present', () => {
  it('has ssh-reconnection-separator__label element', () => {
    const { container, instance } = mountSep({});
    instances.push(instance);
    const label = container.querySelector('.ssh-reconnection-separator__label');
    expect(label).not.toBeNull();
    expect(label!.textContent!.trim().length).toBeGreaterThan(0);
  });
});
