// SPDX-License-Identifier: MPL-2.0
// Build requirement: pnpm tauri build --no-bundle -- --features e2e-testing
// Run: pnpm wdio

/**
 * E2E scenario: Fullscreen mode behavior (non-WM-dependent tests only).
 *
 * Tests that require the window manager to actually toggle fullscreen
 * (F11, hover-zone recall, badge click, etc.) cannot run under
 * tauri-driver / WebKitGTK because `window.set_fullscreen()` is a
 * no-op in headless / virtual-framebuffer environments. Those behaviors
 * are specified in FS-FULL-001 – FS-FULL-011 and UXD §7.22; they must
 * be verified via manual testing or a runner with a compositing WM.
 *
 * The tests retained here verify properties that are observable without
 * an actual fullscreen transition: initial windowed state, button
 * presence, and CSS rule compilation.
 *
 * Protocol references:
 *   - TEST-FS-E2E-001, TEST-FS-BTN-001, TEST-FS-E2E-009
 *   - FS-FULL-001, FS-FULL-004, FS-FULL-009
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe('TauTerm — Fullscreen mode', () => {
  // -------------------------------------------------------------------------
  // TEST-FS-E2E-001: Application starts in normal mode.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-001: starts in normal mode — no fullscreen chrome, tab bar visible, badge absent', async () => {
    // Tab row must be present and not hidden.
    const tabRow = await $(Selectors.tabRow);
    await expect(tabRow).toExist();

    const isHidden = await browser.execute((): boolean => {
      const el = document.querySelector('.terminal-view__tab-row');
      return el !== null && el.classList.contains('terminal-view__tab-row--hidden');
    });
    expect(isHidden).toBe(false);

    // Tab bar child must be rendered.
    const tabBar = await $(Selectors.tabBar);
    await expect(tabBar).toExist();

    // Fullscreen hover zones must not be present (only rendered in fullscreen).
    const hoverTopPresent = await browser.execute((): boolean => {
      return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
    });
    expect(hoverTopPresent).toBe(false);

    // Fullscreen exit badge must not be present.
    const badgePresent = await browser.execute((): boolean => {
      return document.querySelector('[data-testid="fullscreen-exit-badge"]') !== null;
    });
    expect(badgePresent).toBe(false);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-BTN-001: Fullscreen toggle button is visible at startup.
  // -------------------------------------------------------------------------
  it('TEST-FS-BTN-001: fullscreen toggle button is present and visible in the tab bar on startup', async () => {
    const btn = await $(Selectors.fullscreenToggleBtn);
    await expect(btn).toExist();
    await expect(btn).toBeDisplayed();

    // The button must be inside the tab row.
    const inTabRow = await browser.execute((): boolean => {
      const tabRow = document.querySelector('.terminal-view__tab-row');
      const toggleBtn = document.querySelector('[data-testid="fullscreen-toggle-btn"]');
      return tabRow !== null && toggleBtn !== null && tabRow.contains(toggleBtn);
    });
    expect(inTabRow).toBe(true);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-E2E-009: prefers-reduced-motion — tab row has no CSS transition.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-009: with prefers-reduced-motion:reduce, tab row transition is disabled', async () => {
    // This test verifies that the CSS rule
    //   @media (prefers-reduced-motion: reduce) { .terminal-view__tab-row { transition: none; } }
    // is present and correctly expressed. We cannot force the media query to
    // match at runtime via WebDriver; instead we verify the stylesheet contains
    // the rule by inspecting CSSStyleSheet rules via document.styleSheets.
    //
    // This approach is valid: it asserts the rule is compiled into the CSS
    // bundle (not removed by tree-shaking or PostCSS), which is the observable
    // E2E guarantee we can make without OS-level media query control.

    const hasReducedMotionRule = await browser.execute((): boolean => {
      try {
        for (const sheet of Array.from(document.styleSheets)) {
          let rules: CSSRuleList;
          try {
            rules = sheet.cssRules;
          } catch {
            // Cross-origin sheets throw SecurityError — skip.
            continue;
          }
          for (const rule of Array.from(rules)) {
            if (rule instanceof CSSMediaRule) {
              const media = rule.conditionText ?? rule.media?.mediaText ?? '';
              if (!media.includes('prefers-reduced-motion')) continue;
              // Look for a nested rule targeting .terminal-view__tab-row with transition:none.
              for (const inner of Array.from(rule.cssRules)) {
                if (!(inner instanceof CSSStyleRule)) continue;
                if (!inner.selectorText.includes('terminal-view__tab-row')) continue;
                const transition = inner.style.getPropertyValue('transition');
                if (transition === 'none') return true;
              }
            }
          }
        }
        return false;
      } catch {
        return false;
      }
    });

    expect(hasReducedMotionRule).toBe(true);
  });
});
