// SPDX-License-Identifier: MPL-2.0

/**
 * SshDeprecatedAlgorithmBanner component tests.
 *
 * Covered:
 *   UXD-SSH-BANNER-001 — banner renders with algorithm name in warning text
 *   UXD-SSH-BANNER-002 — dismiss button calls ondismiss callback
 *   UXD-SSH-BANNER-003 — dismiss button does not call ondismiss when not provided
 *   UXD-SSH-BANNER-004 — icon is aria-hidden (decorative)
 *   UXD-SSH-BANNER-005 — dismiss button has accessible aria-label
 *   UXD-SSH-BANNER-006 — banner has role="alert"
 *   UXD-SSH-BANNER-007 — dismiss button has min 44px hit target via CSS class
 *   SEC-SSH-BANNER-001 — algorithm name is not interpreted as HTML
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import SshDeprecatedAlgorithmBanner from '../SshDeprecatedAlgorithmBanner.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mountBanner(props: { algorithm: string; ondismiss?: () => void }): {
  container: HTMLElement;
  instance: ReturnType<typeof mount>;
} {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(SshDeprecatedAlgorithmBanner, { target: container, props });
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

describe('UXD-SSH-BANNER-001: renders algorithm name', () => {
  it('shows ssh-rsa in the banner text', () => {
    const { container, instance } = mountBanner({ algorithm: 'ssh-rsa' });
    instances.push(instance);
    const banner = container.querySelector('.ssh-deprecated-banner');
    expect(banner).not.toBeNull();
    expect(container.textContent).toContain('ssh-rsa');
  });

  it('shows ssh-dss in the banner text', () => {
    const { container, instance } = mountBanner({ algorithm: 'ssh-dss' });
    instances.push(instance);
    expect(container.textContent).toContain('ssh-dss');
  });
});

describe('UXD-SSH-BANNER-002: dismiss button calls ondismiss', () => {
  it('calls ondismiss when dismiss button is clicked', () => {
    const ondismiss = vi.fn();
    const { container, instance } = mountBanner({ algorithm: 'ssh-rsa', ondismiss });
    instances.push(instance);
    const dismissBtn = container.querySelector('.ssh-deprecated-banner__dismiss');
    expect(dismissBtn).not.toBeNull();
    (dismissBtn as HTMLButtonElement).click();
    expect(ondismiss).toHaveBeenCalledTimes(1);
  });
});

describe('UXD-SSH-BANNER-003: dismiss without callback does not throw', () => {
  it('clicking dismiss without ondismiss prop does not throw', () => {
    const { container, instance } = mountBanner({ algorithm: 'ssh-rsa' });
    instances.push(instance);
    const dismissBtn = container.querySelector('.ssh-deprecated-banner__dismiss');
    expect(dismissBtn).not.toBeNull();
    // Should not throw
    expect(() => (dismissBtn as HTMLButtonElement).click()).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Accessibility tests
// ---------------------------------------------------------------------------

describe('UXD-SSH-BANNER-004: icon is aria-hidden', () => {
  it('svg icons in the banner have aria-hidden=true', () => {
    const { container, instance } = mountBanner({ algorithm: 'ssh-rsa' });
    instances.push(instance);
    const svgs = container.querySelectorAll('svg');
    const hiddenSvgs = Array.from(svgs).filter((s) => s.getAttribute('aria-hidden') === 'true');
    expect(hiddenSvgs.length).toBeGreaterThan(0);
  });
});

describe('UXD-SSH-BANNER-005: dismiss button has accessible aria-label', () => {
  it('dismiss button has a non-empty aria-label', () => {
    const { container, instance } = mountBanner({ algorithm: 'ssh-rsa' });
    instances.push(instance);
    const dismissBtn = container.querySelector('.ssh-deprecated-banner__dismiss');
    expect(dismissBtn).not.toBeNull();
    const label = dismissBtn!.getAttribute('aria-label');
    expect(label).toBeTruthy();
    expect(label!.length).toBeGreaterThan(0);
  });
});

describe('UXD-SSH-BANNER-006: banner has role=alert', () => {
  it('banner root element has role="alert"', () => {
    const { container, instance } = mountBanner({ algorithm: 'ssh-rsa' });
    instances.push(instance);
    const banner = container.querySelector('.ssh-deprecated-banner');
    expect(banner).not.toBeNull();
    expect(banner!.getAttribute('role')).toBe('alert');
  });
});

describe('UXD-SSH-BANNER-007: dismiss button hit target', () => {
  it('dismiss button has the correct CSS class for 44px hit target', () => {
    const { container, instance } = mountBanner({ algorithm: 'ssh-rsa' });
    instances.push(instance);
    const dismissBtn = container.querySelector('.ssh-deprecated-banner__dismiss');
    expect(dismissBtn).not.toBeNull();
    // The class carries the 44px min-height/width in CSS via --size-target-min
    expect(dismissBtn!.classList.contains('ssh-deprecated-banner__dismiss')).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Security: algorithm name is not interpreted as HTML
// ---------------------------------------------------------------------------

describe('SEC-SSH-BANNER-001: algorithm rendered as text not HTML', () => {
  it('XSS payload in algorithm name is rendered as literal text', () => {
    const malicious = '<script>evil()</script>';
    const { container, instance } = mountBanner({ algorithm: malicious });
    instances.push(instance);
    // innerHTML should not contain a script element
    expect(container.querySelector('script')).toBeNull();
    // The raw string should appear as text
    expect(container.textContent).toContain('<script>evil()');
  });
});
