// SPDX-License-Identifier: MPL-2.0

/**
 * E2E: Preferences panel — scrollable preserves scroll position after toggle interaction.
 *
 * Regression: clicking a Toggle inside the Appearance section caused the scrollable
 * to jump back to scrollTop=0. Root cause: sr-only <input> is `position:absolute`
 * without a `position:relative` ancestor, so its containing block is the dialog
 * (position:fixed). WebKitGTK calls scrollIntoView on the focused checkbox, and
 * since the element's effective position is relative to the dialog rather than the
 * scrollable, the nearest scrollable ancestor (the content div) resets to scrollTop=0.
 * Fix: `position:relative` on the Toggle <label> contains the checkbox within the
 * label's bounding box — which is already visible in the scrollable — making
 * scrollIntoView a no-op. Additionally, `margin: 0` overrides sr-only's `margin: -1px`
 * which would place the checkbox 1px above the label edge, making scrollIntoView fire
 * even with position:relative on the label.
 *
 * Note: WebDriver pointer events do not fully replicate WebKitGTK's native
 * focus+scrollIntoView chain triggered by real physical mouse clicks. This test
 * asserts the non-regression invariant (scrollTop is preserved) and documents the
 * scenario, even if it cannot demonstrate the raw regression in automation.
 *
 * Implementation note on creating scroll space:
 * The scrollable content may fully fit within the panel (no overflow) at large monitor
 * resolutions where 80vh is tall. To guarantee overflow without relying on window size,
 * step 3 constrains the scrollable's maxHeight via inline style before scrolling, then
 * removes the constraint before the assertion so layout is restored.
 *
 * Scenarios:
 *   TEST-PREFS-SCROLL-001 — Toggle click in scrolled Appearance section preserves scrollTop
 *
 * Protocol references:
 *   docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §6 (Preferences UI)
 */

import { browser } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Preferences panel scrollable position (regression)', () => {
  before(async () => {
    // Dismiss any lingering dialog from a previous spec.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="close-confirm-cancel"]',
      );
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Close any open panel (Escape).
    await browser.execute((): void => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
      );
    });
    await browser.pause(200);
  });

  after(async () => {
    // Always close the panel after the suite, even if tests failed.
    await browser.execute((): void => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
      );
    });
    await browser.pause(300);
  });

  // -------------------------------------------------------------------------
  // TEST-PREFS-SCROLL-001
  // -------------------------------------------------------------------------

  it('TEST-PREFS-SCROLL-001: toggle click in scrolled Appearance section preserves scrollTop', async () => {
    // --- 1. Open the preferences panel via Ctrl+, ---
    await browser.execute((): void => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: ',',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    });

    await browser.waitUntil(
      async () => {
        try {
          return await (await browser.$(Selectors.preferencesPanel)).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 3_000, timeoutMsg: 'Preferences panel did not open' },
    );

    // --- 2. Navigate to the Appearance section ---
    await browser.execute((): void => {
      const buttons = Array.from(
        document.querySelectorAll<HTMLButtonElement>('.preferences-section-nav__item'),
      );
      const appearanceBtn = buttons.find((b) => /appearance/i.test(b.textContent ?? ''));
      if (appearanceBtn && !appearanceBtn.getAttribute('aria-current')) {
        appearanceBtn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      }
    });
    await browser.pause(150);

    // --- 3. Constrain scrollable height and scroll down ---
    // The scrollable content may fully fit within the panel at large monitor resolutions
    // (80vh can exceed the content height). We force overflow by capping maxHeight to
    // 300 px via inline style, ensuring the toggles at the bottom require scrolling.
    // This constraint is removed after reading scrollBefore so the click target is at
    // its natural position.
    await browser.execute((): void => {
      const panel = document.querySelector('.preferences-panel');
      const scrollable = panel?.querySelector('.overflow-y-auto') as HTMLElement | null;
      if (scrollable) {
        scrollable.style.maxHeight = '300px';
        // Request 600px — will be clamped to scrollable's maximum scrollTop.
        scrollable.scrollTop = 600;
      }
    });
    await browser.pause(100);

    // Confirm we ended up scrolled (clamped value must be > 0).
    const scrollBefore = (await browser.execute((): number => {
      const panel = document.querySelector('.preferences-panel');
      const scrollable = panel?.querySelector('.overflow-y-auto') as HTMLElement | null;
      return scrollable?.scrollTop ?? 0;
    })) as number;
    expect(scrollBefore).toBeGreaterThan(0);

    // Note: the maxHeight constraint is intentionally kept active during step 4 so that
    // scrollTop remains > 0. Removing it before the click would clamp scrollTop to 0
    // (the content no longer overflows the now-taller scrollable). It is removed after
    // the scroll-preservation assertion in step 5.

    // --- 4. Click the "Show pane title bar" toggle via real WebDriver pointer event ---
    // Obtain the center coordinates of the toggle label in viewport space.
    const clickCoords = (await browser.execute((): { x: number; y: number } | null => {
      const panel = document.querySelector('.preferences-panel');
      const labels = Array.from(panel?.querySelectorAll<HTMLLabelElement>('label') ?? []);
      const titleBarLabel = labels.find((l) => /show pane title bar/i.test(l.textContent ?? ''));
      if (!titleBarLabel) return null;
      const rect = titleBarLabel.getBoundingClientRect();
      return {
        x: Math.round(rect.left + rect.width / 2),
        y: Math.round(rect.top + rect.height / 2),
      };
    })) as { x: number; y: number } | null;

    if (clickCoords) {
      await browser
        .action('pointer', { parameters: { pointerType: 'mouse' } })
        .move({ x: clickCoords.x, y: clickCoords.y })
        .down({ button: 0 })
        .up({ button: 0 })
        .perform();
    }

    // Allow WebKitGTK to process the focus, any scrollIntoView, and Svelte re-renders.
    await browser.pause(500);

    // --- 5. Assert scroll position was NOT changed by the toggle click ---
    const scrollAfter = (await browser.execute((): number => {
      const panel = document.querySelector('.preferences-panel');
      const scrollable = panel?.querySelector('.overflow-y-auto') as HTMLElement | null;
      return scrollable?.scrollTop ?? 0;
    })) as number;

    // Remove the maxHeight constraint now that scroll assertions are done.
    await browser.execute((): void => {
      const panel = document.querySelector('.preferences-panel');
      const scrollable = panel?.querySelector('.overflow-y-auto') as HTMLElement | null;
      if (scrollable) scrollable.style.maxHeight = '';
    });

    // Scroll position must not have reset to zero.
    expect(scrollAfter).toBeGreaterThan(0);
    // It must be within a small delta of the pre-click value (no unexpected scroll jump).
    expect(Math.abs(scrollAfter - scrollBefore)).toBeLessThan(50);
  });
});
