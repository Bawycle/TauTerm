// SPDX-License-Identifier: MPL-2.0

/**
 * E2E: Terminal focus restoration after UI panel interactions (FS-UX-013).
 *
 * These tests validate the real-browser focus behaviour that unit tests cannot
 * exercise: Bits UI FocusScope timing, DOM event ordering across async tasks,
 * and WebKitGTK-specific focus semantics.
 *
 * Scenarios:
 *   TEST-E2E-FOCUS-001 — Preferences panel: close → terminal gets focus
 *   TEST-E2E-FOCUS-002 — SSH connection panel: close → terminal gets focus
 *   TEST-E2E-FOCUS-003 — Tab bar arrow navigation: letter key → terminal gets focus
 *
 * Why these are E2E-only:
 *   Unit tests with static source analysis confirmed code presence but could not
 *   catch Bits UI FocusScope restoring focus to the trigger after our onclose
 *   callback (a real-browser async ordering issue). Only running the app in a
 *   real WebKitGTK context can validate the actual focus recipient.
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Focus helpers (mirrors focus-autofocus.spec.ts)
// ---------------------------------------------------------------------------

function isViewportFocused(): Promise<boolean> {
  return browser.execute((): boolean => {
    const el = document.activeElement;
    if (el === null || el === document.body) return false;
    return (
      el.classList.contains('terminal-grid') ||
      el.classList.contains('terminal-pane__viewport') ||
      el.classList.contains('terminal-pane__input')
    );
  }) as Promise<boolean>;
}

async function waitForViewportFocus(timeoutMs = 2_000): Promise<void> {
  await browser.waitUntil(isViewportFocused, {
    timeout: timeoutMs,
    interval: 50,
    timeoutMsg: `Terminal viewport did not receive focus within ${timeoutMs} ms`,
  });
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Terminal focus restoration after panel interactions (FS-UX-013)', () => {
  before(async () => {
    // Dismiss any lingering dialog from a previous spec.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>('[data-testid="close-confirm-cancel"]');
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Close any open panels that might have leaked from another spec.
    await browser.execute((): void => {
      document.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Escape', bubbles: true, cancelable: true,
      }));
    });
    await browser.pause(200);

    // Ensure the terminal viewport is focused as the baseline.
    await waitForViewportFocus(5_000);
  });

  after(async () => {
    // Close any panels opened by the tests (Escape handles both SSH and preferences).
    await browser.execute((): void => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
      );
    });
    await browser.pause(100);

    // Exit fullscreen if any test accidentally left the app in that state.
    // TEST-E2E-FOCUS-003 navigates the tab bar via ArrowRight and presses 'a';
    // if focus lands on the fullscreen button and the key triggers activation,
    // subsequent specs see aria-pressed="true" and fail their initial state check.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-toggle-btn"]',
      );
      if (btn?.getAttribute('aria-pressed') === 'true') {
        btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      }
    });
    await browser.pause(200);
  });

  // -------------------------------------------------------------------------
  // TEST-E2E-FOCUS-001: Preferences panel close → terminal gets focus
  //
  // Regression: Bits UI Dialog.Content FocusScope was restoring focus to the
  // settings button trigger after onclose fired, overriding our focus call.
  // Fix: onCloseAutoFocus={(e) => e.preventDefault()} on Dialog.Content.
  // -------------------------------------------------------------------------

  it('TEST-E2E-FOCUS-001: terminal is focused after closing preferences panel', async () => {
    // Open preferences via keyboard shortcut (Ctrl+,).
    await browser.execute((): void => {
      document.dispatchEvent(new KeyboardEvent('keydown', {
        key: ',', ctrlKey: true, bubbles: true, cancelable: true,
      }));
    });

    // Wait for preferences panel to appear.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.preferencesPanel).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 3_000, timeoutMsg: 'Preferences panel did not open' },
    );

    // Close via Escape (Bits UI Dialog handles this).
    await browser.execute((): void => {
      document.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Escape', bubbles: true, cancelable: true,
      }));
    });

    // Wait for panel to disappear.
    await browser.waitUntil(
      async () => {
        try {
          return !(await $(Selectors.preferencesPanel).isExisting());
        } catch {
          return true;
        }
      },
      { timeout: 3_000, timeoutMsg: 'Preferences panel did not close' },
    );

    // The terminal viewport must now have focus — NOT the settings button.
    await waitForViewportFocus();
    expect(await isViewportFocused()).toBe(true);
  });

  // -------------------------------------------------------------------------
  // TEST-E2E-FOCUS-002: SSH connection panel close → terminal gets focus
  //
  // The SSH panel is a slide-in (not a Bits UI dialog). onclose must explicitly
  // restore focus since there is no FocusScope — focus falls to document.body
  // after the panel unmounts, and the focus guard catches it.
  // -------------------------------------------------------------------------

  it('TEST-E2E-FOCUS-002: terminal is focused after closing SSH connection panel', async () => {
    // Click the SSH toggle button (first .terminal-view__ssh-btn).
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLElement>('.terminal-view__ssh-btn');
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Wait for connection manager to appear.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.connectionManager).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 3_000, timeoutMsg: 'SSH connection manager did not open' },
    );

    // Close via the X button inside the panel header (triggers the onclose callback
    // which is the code path our focus-restoration fix covers).
    await browser.execute((): void => {
      const closeBtn = document.querySelector<HTMLButtonElement>('.connection-manager__header button');
      closeBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Wait for panel to disappear.
    await browser.waitUntil(
      async () => {
        try {
          return !(await $(Selectors.connectionManager).isExisting());
        } catch {
          return true;
        }
      },
      { timeout: 3_000, timeoutMsg: 'SSH connection manager did not close' },
    );

    // Terminal viewport must have focus.
    await waitForViewportFocus();
    expect(await isViewportFocused()).toBe(true);
  });

  // -------------------------------------------------------------------------
  // TEST-E2E-FOCUS-003: Tab bar arrow navigation → printable key returns focus
  //
  // After ArrowRight moves focus to a tab element, pressing a printable key
  // must trigger onEscapeTabBar and return focus to the terminal viewport.
  // This validates the handleTabKeydown printable-key catch-all.
  // -------------------------------------------------------------------------

  it('TEST-E2E-FOCUS-003: terminal is focused after pressing a key while tab bar has focus', async () => {
    // Move focus to the active tab element.
    await browser.execute((): void => {
      const tab = document.querySelector<HTMLElement>('.tab-bar__tab--active');
      tab?.focus();
    });

    // Confirm focus is on the tab, not the terminal.
    const tabHasFocus = await browser.execute((): boolean => {
      const el = document.activeElement;
      return el !== null && el.classList.contains('tab-bar__tab--active');
    });
    expect(tabHasFocus).toBe(true);

    // Press ArrowRight to navigate within the tab bar (stays in tab bar).
    await browser.execute((): void => {
      const tab = document.querySelector<HTMLElement>('.tab-bar__tab--active');
      tab?.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'ArrowRight', bubbles: true, cancelable: true,
      }));
    });
    await browser.pause(50);

    // Press a printable key — handleTabKeydown catch-all must fire onEscapeTabBar.
    await browser.execute((): void => {
      const focused = document.activeElement as HTMLElement | null;
      focused?.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'a', bubbles: true, cancelable: true,
      }));
    });

    // Terminal viewport must receive focus.
    await waitForViewportFocus();
    expect(await isViewportFocused()).toBe(true);
  });
});
