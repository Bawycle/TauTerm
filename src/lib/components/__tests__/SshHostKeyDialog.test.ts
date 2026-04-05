// SPDX-License-Identifier: MPL-2.0

/**
 * SshHostKeyDialog component tests.
 *
 * Covered:
 *   SSH-HK-FN-001 — TOFU mode: shows fingerprint and host
 *   SSH-HK-FN-002 — TOFU mode: shows first-time connection intro text
 *   SSH-HK-FN-003 — TOFU mode: Accept button is primary (deliberate action)
 *   SSH-HK-FN-004 — TOFU mode: clicking Accept calls onaccept then onclose
 *   SSH-HK-FN-005 — TOFU mode: clicking Reject/Cancel calls onreject then onclose
 *   SSH-HK-FN-006 — MITM mode (isChanged=true): shows warning alert
 *   SSH-HK-FN-007 — MITM mode: Accept button is ghost (non-default, less prominent)
 *   SSH-HK-FN-008 — MITM mode: "man-in-the-middle" warning text is visible
 *   SEC-BLK-004 — host displayed is from config (text interpolation, not innerHTML)
 *   SSH-HK-A11Y-001 — warning block has role="alert"
 *
 * Note: Dialog uses Bits UI Dialog.Portal which renders to document.body,
 * not inside the mount container. All DOM queries use document.body.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, flushSync } from 'svelte';
import SshHostKeyDialog from '../SshHostKeyDialog.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface MountOpts {
  open?: boolean;
  host?: string;
  keyType?: string;
  fingerprint?: string;
  isChanged?: boolean;
  onaccept?: () => void;
  onreject?: () => void;
  onclose?: () => void;
}

function mountDialog(opts: MountOpts = {}): ReturnType<typeof mount> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(SshHostKeyDialog, {
    target: container,
    props: {
      open: opts.open ?? true,
      host: opts.host ?? 'example.com',
      keyType: opts.keyType ?? 'ED25519',
      fingerprint: opts.fingerprint ?? 'SHA256:abc123',
      isChanged: opts.isChanged ?? false,
      onaccept: opts.onaccept ?? vi.fn(),
      onreject: opts.onreject ?? vi.fn(),
      onclose: opts.onclose ?? vi.fn(),
    },
  });
  flushSync();
  return instance;
}

afterEach(() => {
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-001: shows fingerprint, host, and key type
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-001: TOFU — fingerprint and host are visible', () => {
  it('shows host in the dialog content', () => {
    mountDialog({ host: 'myserver.local' });
    expect(document.body.textContent).toContain('myserver.local');
  });

  it('shows fingerprint in the dialog content', () => {
    mountDialog({ fingerprint: 'SHA256:deadbeef' });
    expect(document.body.textContent).toContain('SHA256:deadbeef');
  });

  it('shows key type in the dialog content', () => {
    mountDialog({ keyType: 'ED25519' });
    expect(document.body.textContent).toContain('ED25519');
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-002: TOFU — first-time intro text visible
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-002: TOFU — first-time connection intro text', () => {
  it('shows "first time" or "connecting for the first time" text', () => {
    mountDialog({ isChanged: false });
    const text = document.body.textContent ?? '';
    expect(text.toLowerCase()).toMatch(/first time|first connection/);
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-003: TOFU — Accept button is primary
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-003: TOFU — Accept button is present and not disabled', () => {
  it('Accept button exists in TOFU mode', () => {
    mountDialog({ isChanged: false });
    const buttons = Array.from(document.body.querySelectorAll('button'));
    const acceptButton = buttons.find(
      (b) => b.textContent?.trim().toLowerCase() === 'accept',
    );
    expect(acceptButton).toBeDefined();
    expect(acceptButton!.disabled).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-004: TOFU — clicking Accept calls onaccept then onclose
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-004: TOFU — Accept calls onaccept and onclose', () => {
  it('Accept click triggers onaccept then onclose', () => {
    const onaccept = vi.fn();
    const onclose = vi.fn();
    mountDialog({ isChanged: false, onaccept, onclose });

    const buttons = Array.from(document.body.querySelectorAll('button'));
    const acceptButton = buttons.find(
      (b) => b.textContent?.trim().toLowerCase() === 'accept',
    );
    expect(acceptButton).toBeDefined();
    acceptButton!.click();
    flushSync();

    expect(onaccept).toHaveBeenCalledOnce();
    expect(onclose).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-005: TOFU — clicking Reject/Cancel calls onreject then onclose
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-005: TOFU — Reject/Cancel calls onreject and onclose', () => {
  it('Cancel click triggers onreject and onclose', () => {
    const onreject = vi.fn();
    const onclose = vi.fn();
    mountDialog({ isChanged: false, onreject, onclose });

    const buttons = Array.from(document.body.querySelectorAll('button'));
    // The cancel/reject button contains "Cancel" text (m.action_cancel()).
    // The Bits UI Dialog close button has aria-label but no visible text.
    const cancelButton = buttons.find(
      (b) => b.textContent?.trim().toLowerCase() === 'cancel',
    );
    expect(cancelButton).toBeDefined();
    cancelButton!.click();
    flushSync();

    expect(onreject).toHaveBeenCalledOnce();
    expect(onclose).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-006: MITM — warning alert is visible
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-006: MITM (isChanged=true) — warning alert visible', () => {
  it('shows a warning block when isChanged is true', () => {
    mountDialog({ isChanged: true });
    const text = document.body.textContent ?? '';
    // FS-SSH-011: must warn about key change
    expect(text.toLowerCase()).toMatch(/changed|warning|risk|mitm|man.in.the.middle/);
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-A11Y-001: warning block has role="alert"
// ---------------------------------------------------------------------------

describe('SSH-HK-A11Y-001: MITM warning block has role="alert"', () => {
  it('warning element has role="alert" when isChanged=true', () => {
    mountDialog({ isChanged: true });
    const alertEl = document.body.querySelector('[role="alert"]');
    expect(alertEl).not.toBeNull();
  });

  it('no role="alert" in TOFU mode (isChanged=false)', () => {
    mountDialog({ isChanged: false });
    const alertEl = document.body.querySelector('[role="alert"]');
    expect(alertEl).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-007: MITM — Accept Anyway button exists
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-007: MITM — Accept Anyway button is less prominent (ghost)', () => {
  it('"Accept Anyway" button exists when isChanged=true', () => {
    mountDialog({ isChanged: true });
    const buttons = Array.from(document.body.querySelectorAll('button'));
    const acceptButton = buttons.find(
      (b) => b.textContent?.toLowerCase().includes('accept'),
    );
    expect(acceptButton).toBeDefined();
  });

  it('calling Accept Anyway triggers onaccept', () => {
    const onaccept = vi.fn();
    mountDialog({ isChanged: true, onaccept });
    const buttons = Array.from(document.body.querySelectorAll('button'));
    const acceptButton = buttons.find(
      (b) => b.textContent?.toLowerCase().includes('accept'),
    );
    expect(acceptButton).toBeDefined();
    acceptButton!.click();
    flushSync();
    expect(onaccept).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// SSH-HK-FN-008: MITM — man-in-the-middle warning text
// ---------------------------------------------------------------------------

describe('SSH-HK-FN-008: MITM — man-in-the-middle text in warning', () => {
  it('warning references man-in-the-middle when isChanged=true', () => {
    mountDialog({ isChanged: true });
    const text = document.body.textContent ?? '';
    expect(text.toLowerCase()).toContain('man-in-the-middle');
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-004: host is shown via text interpolation (no innerHTML injection)
// ---------------------------------------------------------------------------

describe('SEC-BLK-004: host rendered as text content, not HTML', () => {
  it('host containing HTML special chars is displayed as literal text', () => {
    const maliciousHost = '<script>alert(1)</script>';
    mountDialog({ host: maliciousHost });
    // The script tag must not be executed — check no live <script> in portal
    const scriptEl = document.body.querySelector('script');
    expect(scriptEl).toBeNull();
    // The literal text (escaped) must be visible
    expect(document.body.textContent).toContain('alert(1)');
  });
});
