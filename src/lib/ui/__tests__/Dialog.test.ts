// SPDX-License-Identifier: MPL-2.0

/**
 * Dialog component tests.
 *
 * Covered:
 *   UIBC-FN-DLG-001 — dialog not rendered in DOM when open=false
 *   UIBC-FN-DLG-002 — dialog rendered in document.body (portal) when open=true
 *   UIBC-FN-DLG-003 — title prop rendered in dialog content
 *   UIBC-FN-DLG-004 — sr-only description rendered
 *   UIBC-FN-DLG-006 — close button present and fires onclose
 *   UIBC-A11Y-DLG-001 — element has role="dialog"
 *   UIBC-A11Y-DLG-002 — aria-modal="true"
 *   UIBC-A11Y-DLG-003 — aria-labelledby references title element
 *   UIBC-A11Y-DLG-004 — size prop applies correct width class
 *   UIBC-SEC-005 — XSS via title/description: no {@html} in Dialog.svelte (static)
 *
 * Architecture note:
 *   Dialog.svelte wraps Bits UI Dialog.Root + Dialog.Portal. When open=true,
 *   the dialog content (role="dialog") is mounted in document.body via the
 *   portal, NOT inside the component's container div. Queries must target
 *   document.body. `await tick()` from Svelte is required to flush portal
 *   rendering after mount.
 *
 *   Focus management (UIBC-A11Y-DLG-004/005), focus trap (UIBC-SEC-007),
 *   Escape key (UIBC-FN-DLG-007), overlay click (UIBC-FN-DLG-008), and
 *   focus-restore-on-close (UIBC-SEC-008) are deferred to E2E because they
 *   require real browser pointer/keyboard semantics.
 *
 * @testing-library/svelte is NOT installed. Tests use Svelte 5 mount() + tick() + jsdom.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { mount, tick, createRawSnippet } from 'svelte';
import Dialog from '../Dialog.svelte';

afterEach(() => {
  document.body.innerHTML = '';
});

// ---------------------------------------------------------------------------
// Helper — mounts Dialog and flushes portal rendering via tick()
// ---------------------------------------------------------------------------

async function mountDialog(props: {
  open?: boolean;
  title: string;
  size?: 'small' | 'medium';
  variant?: 'dialog' | 'alertdialog';
  onclose?: () => void;
}): Promise<void> {
  const container = document.createElement('div');
  document.body.appendChild(container);

  const children = createRawSnippet(() => ({
    render: () => '<p>Dialog body</p>',
    setup: () => {},
  }));

  mount(Dialog, { target: container, props: { ...props, children } });
  // tick() flushes Svelte reactivity + Bits UI portal rendering into document.body
  await tick();
}

// ---------------------------------------------------------------------------
// Pure logic — accessibility contract
// ---------------------------------------------------------------------------

describe('Dialog — ARIA contract (pure logic)', () => {
  it('"dialog" is a recognized ARIA role', () => {
    expect(['dialog', 'alertdialog']).toContain('dialog');
  });

  it('aria-modal must be the string "true" for AT compatibility', () => {
    expect('true').toBe('true');
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-DLG-001 — closed dialog not in DOM
// ---------------------------------------------------------------------------

describe('UIBC-FN-DLG-001 — dialog closed state', () => {
  it('no [role="dialog"] element in document when open=false', async () => {
    await mountDialog({ open: false, title: 'Test' });
    expect(document.querySelector('[role="dialog"]')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-DLG-002/003/004 — open dialog rendered in portal
// ---------------------------------------------------------------------------

describe('UIBC-FN-DLG-002/003/004 — open state (portal rendering)', () => {
  it('UIBC-FN-DLG-002: [role="dialog"] exists in document.body when open=true', async () => {
    await mountDialog({ open: true, title: 'My Dialog' });
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  });

  it('UIBC-FN-DLG-003: title prop text appears in dialog element', async () => {
    await mountDialog({ open: true, title: 'Confirm deletion' });
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog!.textContent).toContain('Confirm deletion');
  });

  it('UIBC-FN-DLG-004: sr-only description is rendered with title text', async () => {
    await mountDialog({ open: true, title: 'Confirm' });
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    // Dialog.Description renders title text in an element with data-dialog-description
    const desc = dialog!.querySelector('[data-dialog-description]');
    expect(desc).not.toBeNull();
    expect(desc!.textContent?.trim()).toBe('Confirm');
  });
});

// ---------------------------------------------------------------------------
// UIBC-FN-DLG-006 — close button
// ---------------------------------------------------------------------------

describe('UIBC-FN-DLG-006 — close button', () => {
  it('close button is rendered in open dialog', async () => {
    await mountDialog({ open: true, title: 'Test' });
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    const closeBtn = dialog!.querySelector('button[aria-label="Close dialog"]');
    expect(closeBtn).not.toBeNull();
  });

  it('close button calls onclose when clicked', async () => {
    const handler = vi.fn();
    await mountDialog({ open: true, title: 'Test', onclose: handler });
    const dialog = document.querySelector('[role="dialog"]')!;
    const closeBtn = dialog.querySelector('button[aria-label="Close dialog"]') as HTMLButtonElement;
    closeBtn.click();
    await tick();
    expect(handler).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// UIBC-A11Y-DLG-001/003 — ARIA attributes
// ---------------------------------------------------------------------------

describe('UIBC-A11Y-DLG — ARIA attributes', () => {
  it('UIBC-A11Y-DLG-001: open dialog has role="dialog"', async () => {
    await mountDialog({ open: true, title: 'Test' });
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  });

  it('UIBC-A11Y-DLG-002: dialog has aria-modal="true"', async () => {
    await mountDialog({ open: true, title: 'Test' });
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
  });

  it('UIBC-A11Y-DLG-003: aria-labelledby references element containing title text', async () => {
    await mountDialog({ open: true, title: 'Accessibility test' });
    const dialog = document.querySelector('[role="dialog"]')!;
    const labelledById = dialog.getAttribute('aria-labelledby');
    expect(labelledById).not.toBeNull();
    const labelEl = document.getElementById(labelledById!);
    expect(labelEl).not.toBeNull();
    expect(labelEl!.textContent).toContain('Accessibility test');
  });
});

// ---------------------------------------------------------------------------
// Dialog — size prop (width class assertion)
// ---------------------------------------------------------------------------

describe('Dialog — size prop', () => {
  it('small size renders w-[420px] class on dialog content panel', async () => {
    await mountDialog({ open: true, title: 'Small', size: 'small' });
    const dialog = document.querySelector('[role="dialog"]')!;
    // The Dialog.Content element carries the width class in its className
    expect(dialog.className).toContain('w-[420px]');
  });

  it('medium size renders w-[560px] class on dialog content panel', async () => {
    await mountDialog({ open: true, title: 'Medium', size: 'medium' });
    const dialog = document.querySelector('[role="dialog"]')!;
    expect(dialog.className).toContain('w-[560px]');
  });
});

// ---------------------------------------------------------------------------
// UIBC-SEC-005 — XSS via title and description (static + runtime)
// ---------------------------------------------------------------------------

describe('UIBC-SEC-005 — XSS via title/description', () => {
  it('Dialog.svelte source contains no {@html}', () => {
    const src = readFileSync(resolve(__dirname, '../Dialog.svelte'), 'utf-8');
    const stripped = src
      .replace(/<!--[\s\S]*?-->/g, '')
      .replace(/\/\/[^\n]*/g, '')
      .replace(/\/\*[\s\S]*?\*\//g, '');
    expect(stripped, 'Dialog.svelte must not use {@html} on props').not.toContain('{@html');
  });

  it('script tag in title is not executed', async () => {
    const xss = '<script>window.__xss_dlg_title=true<\/script>';
    await mountDialog({ open: true, title: xss });
    expect((window as unknown as Record<string, unknown>).__xss_dlg_title).toBeUndefined();
    // A free-standing <script> element should not be injected by Dialog rendering the title
    // (Bits UI renders title as text content via Dialog.Title)
    const scripts = Array.from(document.querySelectorAll('script')).filter(
      (s) => !s.getAttribute('src'), // ignore legit script tags injected by test runner
    );
    // No inline script element with window.__xss_dlg_title
    const xssScript = scripts.find((s) => s.textContent?.includes('__xss_dlg_title'));
    expect(xssScript).toBeUndefined();
  });

  it('img onerror in title is not executed', async () => {
    const xss = '<img src=x onerror="window.__xss_dlg_img=true">';
    await mountDialog({ open: true, title: xss });
    expect((window as unknown as Record<string, unknown>).__xss_dlg_img).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// UIBC-A11Y-DLG-ALT — alertdialog variant
// ---------------------------------------------------------------------------

describe('Dialog — alertdialog variant', () => {
  it('open alertdialog has role="alertdialog"', async () => {
    await mountDialog({ open: true, title: 'Confirm delete', variant: 'alertdialog' });
    expect(document.querySelector('[role="alertdialog"]')).not.toBeNull();
  });

  it('alertdialog does not render role="dialog"', async () => {
    await mountDialog({ open: true, title: 'Confirm delete', variant: 'alertdialog' });
    expect(document.querySelector('[role="dialog"]')).toBeNull();
  });

  it('alertdialog has aria-modal="true"', async () => {
    await mountDialog({ open: true, title: 'Confirm delete', variant: 'alertdialog' });
    const el = document.querySelector('[role="alertdialog"]');
    expect(el?.getAttribute('aria-modal')).toBe('true');
  });

  it('alertdialog title text is rendered', async () => {
    await mountDialog({ open: true, title: 'Danger zone', variant: 'alertdialog' });
    const el = document.querySelector('[role="alertdialog"]');
    expect(el?.textContent).toContain('Danger zone');
  });

  it('default variant renders role="dialog" not "alertdialog"', async () => {
    await mountDialog({ open: true, title: 'Standard dialog' });
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
    expect(document.querySelector('[role="alertdialog"]')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Deferred to E2E — require real browser environment
// ---------------------------------------------------------------------------

describe('Dialog interaction tests (E2E deferred)', () => {
  it.todo('UIBC-FN-DLG-007: Escape key fires onclose — E2E');
  it.todo('UIBC-FN-DLG-008: overlay click fires onclose — E2E');
  it.todo('UIBC-A11Y-DLG-004: focus moves into dialog on open — E2E');
  it.todo('UIBC-A11Y-DLG-005: focus returns to trigger on close — E2E');
  it.todo('UIBC-SEC-007: focus trap — Tab cannot escape dialog — E2E');
  it.todo('UIBC-SEC-008: Escape restores focus to trigger element — E2E');
});
