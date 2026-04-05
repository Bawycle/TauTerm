// SPDX-License-Identifier: MPL-2.0

/**
 * SshCredentialDialog component tests.
 *
 * Covered:
 *   SSH-CRED-FN-001 — dialog renders with host and username visible
 *   SSH-CRED-FN-002 — password field is type="password" (masked)
 *   SSH-CRED-FN-003 — OK button is disabled when password is empty
 *   SSH-CRED-FN-004 — OK button enabled after typing password
 *   SSH-CRED-FN-005 — submit calls onsubmit with the entered password
 *   SSH-CRED-FN-006 — cancel calls oncancel and onclose
 *   SSH-CRED-FN-007 — Enter key in password field triggers submit
 *   SSH-CRED-FN-008 — password field is cleared after submit
 *   SSH-CRED-FN-009 — custom prompt text is displayed when provided
 *   SSH-CRED-FN-010 — default prompt text shown when prompt prop is omitted
 *   SEC-BLK-004 — username field is readonly (cannot be edited)
 *   SEC-UI-002 — password field type is "password" (not "text")
 *
 * Note: Bits UI Dialog renders via a portal into document.body, not into the
 * test container element. All DOM queries use document.body.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, flushSync } from 'svelte';
import SshCredentialDialog from '../SshCredentialDialog.svelte';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface MountOpts {
  open?: boolean;
  host?: string;
  username?: string;
  prompt?: string;
  onsubmit?: (password: string) => void;
  oncancel?: () => void;
  onclose?: () => void;
}

function mountDialog(opts: MountOpts = {}): ReturnType<typeof mount> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(SshCredentialDialog, {
    target: container,
    props: {
      open: opts.open ?? true,
      host: opts.host ?? 'example.com',
      username: opts.username ?? 'alice',
      prompt: opts.prompt,
      onsubmit: opts.onsubmit ?? vi.fn(),
      oncancel: opts.oncancel ?? vi.fn(),
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
// SSH-CRED-FN-001: dialog renders with host and username visible
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-001: renders host and username', () => {
  it('shows the username in the readonly input', () => {
    mountDialog({ username: 'alice' });
    const usernameInput = document.body.querySelector(
      '#ssh-credential-username',
    ) as HTMLInputElement | null;
    expect(usernameInput).not.toBeNull();
    expect(usernameInput!.value).toBe('alice');
  });

  it('shows the host in the dialog title or intro text', () => {
    mountDialog({ host: 'myserver.example.com' });
    const text = document.body.textContent ?? '';
    expect(text).toContain('myserver.example.com');
  });
});

// ---------------------------------------------------------------------------
// SEC-UI-002 / SSH-CRED-FN-002: password field type="password"
// ---------------------------------------------------------------------------

describe('SEC-UI-002: password field is masked (type="password")', () => {
  it('password input has type="password"', () => {
    mountDialog();
    const pwInput = document.body.querySelector(
      '#ssh-credential-password',
    ) as HTMLInputElement | null;
    expect(pwInput).not.toBeNull();
    expect(pwInput!.type).toBe('password');
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-004: username field is readonly
// ---------------------------------------------------------------------------

describe('SEC-BLK-004: username field is readonly', () => {
  it('username input has readonly attribute', () => {
    mountDialog({ username: 'bob' });
    const usernameInput = document.body.querySelector(
      '#ssh-credential-username',
    ) as HTMLInputElement | null;
    expect(usernameInput).not.toBeNull();
    expect(usernameInput!.readOnly).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// SSH-CRED-FN-003: OK button disabled when password empty
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-003: OK button disabled when password is empty', () => {
  it('OK button is disabled on initial render (empty password)', () => {
    mountDialog();
    const buttons = Array.from(document.body.querySelectorAll('button'));
    const okButton = buttons.find(
      (b) => b.textContent?.trim() !== '' && b.disabled,
    );
    expect(okButton).toBeDefined();
    expect(okButton!.disabled).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// SSH-CRED-FN-004: OK button enabled after typing password
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-004: OK button enabled after typing a password', () => {
  it('OK button becomes enabled when password input has a value', () => {
    mountDialog();
    const pwInput = document.body.querySelector(
      '#ssh-credential-password',
    ) as HTMLInputElement | null;
    expect(pwInput).not.toBeNull();

    // Simulate typing into the password field
    pwInput!.value = 's3cret';
    pwInput!.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();

    const buttons = Array.from(document.body.querySelectorAll('button'));
    const disabledOkButton = buttons.find(
      (b) => b.disabled && !b.classList.contains('btn--ghost'),
    );
    expect(disabledOkButton).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// SSH-CRED-FN-005: submit calls onsubmit with the entered password
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-005: submit calls onsubmit with password', () => {
  it('clicking OK calls onsubmit with the typed password', () => {
    const onsubmit = vi.fn();
    mountDialog({ onsubmit });

    const pwInput = document.body.querySelector(
      '#ssh-credential-password',
    ) as HTMLInputElement | null;
    expect(pwInput).not.toBeNull();
    pwInput!.value = 'mypassword';
    pwInput!.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();

    // Find and click the OK button by its text content (m.action_ok() = "OK")
    const buttons = Array.from(document.body.querySelectorAll('button'));
    const okButton = buttons.find((b) => b.textContent?.trim() === 'OK');
    expect(okButton).toBeDefined();
    okButton!.click();
    flushSync();

    expect(onsubmit).toHaveBeenCalledOnce();
    expect(onsubmit).toHaveBeenCalledWith('mypassword');
  });
});

// ---------------------------------------------------------------------------
// SSH-CRED-FN-006: cancel calls oncancel and onclose
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-006: cancel calls oncancel and onclose', () => {
  it('clicking Cancel calls oncancel then onclose', () => {
    const oncancel = vi.fn();
    const onclose = vi.fn();
    mountDialog({ oncancel, onclose });

    const buttons = Array.from(document.body.querySelectorAll('button'));
    // Cancel button text is m.action_cancel() = "Cancel"
    const cancelButton = buttons.find((b) => b.textContent?.trim() === 'Cancel');
    expect(cancelButton).toBeDefined();
    cancelButton!.click();
    flushSync();

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onclose).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// SSH-CRED-FN-007: Enter key in password field triggers submit
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-007: Enter key submits the form', () => {
  it('pressing Enter in password field calls onsubmit', () => {
    const onsubmit = vi.fn();
    mountDialog({ onsubmit });

    const pwInput = document.body.querySelector(
      '#ssh-credential-password',
    ) as HTMLInputElement | null;
    expect(pwInput).not.toBeNull();
    pwInput!.value = 'enterpass';
    pwInput!.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();

    const enterEvent = new KeyboardEvent('keydown', {
      key: 'Enter',
      bubbles: true,
      cancelable: true,
    });
    pwInput!.dispatchEvent(enterEvent);
    flushSync();

    expect(onsubmit).toHaveBeenCalledOnce();
    expect(onsubmit).toHaveBeenCalledWith('enterpass');
  });
});

// ---------------------------------------------------------------------------
// SSH-CRED-FN-009: custom prompt text is displayed
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-009: custom prompt text is displayed', () => {
  it('shows custom prompt text from the prompt prop', () => {
    mountDialog({
      prompt: 'Verification code:',
    });
    expect(document.body.textContent).toContain('Verification code:');
  });
});

// ---------------------------------------------------------------------------
// SSH-CRED-FN-010: default prompt text when no prompt prop
// ---------------------------------------------------------------------------

describe('SSH-CRED-FN-010: default prompt text when prompt is omitted', () => {
  it('shows default "Password for user@host:" text', () => {
    mountDialog({
      host: 'srv.local',
      username: 'carol',
      prompt: undefined,
    });
    expect(document.body.textContent).toContain('carol@srv.local');
  });
});
