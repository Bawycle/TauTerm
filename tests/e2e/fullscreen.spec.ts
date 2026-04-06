// SPDX-License-Identifier: MPL-2.0
// Build requirement: pnpm tauri build --no-bundle -- --features e2e-testing
// Run: pnpm wdio

/**
 * E2E scenario: Fullscreen mode behavior.
 *
 * Verifies enter/exit transitions, badge visibility, tab bar auto-hide,
 * hover-zone recall, keyboard shortcut non-passthrough to PTY, and rapid
 * toggle state consistency.
 *
 * Protocol references:
 *   - TEST-FS-E2E-001 through TEST-FS-E2E-009
 *   - FS-FULL-009, UXD §7.x (fullscreen chrome)
 *
 * Implementation notes:
 *   - F11 is dispatched via dispatchEvent on the terminal grid to bypass
 *     WebKitGTK/tauri-driver key-delivery quirks that prevent browser.keys(['F11'])
 *     from reaching the Svelte window keydown handler reliably.
 *   - toggle_fullscreen IPC is also called directly (fire-and-forget) for setup
 *     steps where verifying the DOM effect matters more than testing the key path.
 *   - Tab bar visibility is expressed as CSS opacity via the class
 *     `terminal-view__tab-row--hidden` (opacity:0 / pointer-events:none), not
 *     DOM presence — tests assert computed opacity, not element existence.
 *   - The fullscreen state persists in preferences across tests within a run;
 *     afterEach always restores normal mode.
 */

import { browser, $ } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Dispatch an F11 keydown event on the terminal grid element (or document.body
 * as fallback). This reaches the <svelte:window onkeydown> handler without
 * being consumed by the WM layer.
 */
async function dispatchF11(): Promise<void> {
  await browser.execute((): void => {
    const grid = document.querySelector('.terminal-grid') as HTMLElement | null;
    const target = grid ?? document.body;
    target.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'F11',
        code: 'F11',
        bubbles: true,
        cancelable: true,
      }),
    );
  });
}

/**
 * Invoke the toggle_fullscreen Tauri IPC command (fire-and-forget).
 * Returns immediately; DOM effects must be awaited separately.
 */
async function ipcToggleFullscreen(): Promise<void> {
  await browser.execute((): void => {
    (window as any).__TAURI_INTERNALS__.invoke('toggle_fullscreen');
  });
}

/**
 * Query the Tauri window fullscreen state via IPC.
 * Polls the window API directly, bypassing frontend state.
 */
async function getWindowIsFullscreen(): Promise<boolean> {
  return browser.execute(
    (): Promise<boolean> =>
      (window as any).__TAURI_INTERNALS__
        .invoke('toggle_fullscreen')
        .then((r: { is_fullscreen: boolean }) => r.is_fullscreen),
  ) as unknown as Promise<boolean>;
}

/**
 * Wait until the tab row is hidden (opacity 0 via --hidden class).
 * Uses class presence as proxy — class is applied synchronously by Svelte
 * reactive state, although the CSS transition may not have completed.
 */
async function waitForTabRowHidden(timeout = 5_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((): boolean => {
        const el = document.querySelector('.terminal-view__tab-row');
        return el !== null && el.classList.contains('terminal-view__tab-row--hidden');
      }),
    { timeout, timeoutMsg: `Tab row did not become hidden within ${timeout} ms` },
  );
}

/**
 * Wait until the tab row is visible (--hidden class absent).
 */
async function waitForTabRowVisible(timeout = 5_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((): boolean => {
        const el = document.querySelector('.terminal-view__tab-row');
        return el !== null && !el.classList.contains('terminal-view__tab-row--hidden');
      }),
    { timeout, timeoutMsg: `Tab row did not become visible within ${timeout} ms` },
  );
}

/**
 * Wait until the fullscreen exit badge is present in the DOM.
 */
async function waitForBadgePresent(timeout = 5_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((): boolean => {
        return document.querySelector('[data-testid="fullscreen-exit-badge"]') !== null;
      }),
    { timeout, timeoutMsg: `Fullscreen exit badge did not appear within ${timeout} ms` },
  );
}

/**
 * Wait until the fullscreen exit badge is absent from the DOM.
 */
async function waitForBadgeAbsent(timeout = 5_000): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute((): boolean => {
        return document.querySelector('[data-testid="fullscreen-exit-badge"]') === null;
      }),
    { timeout, timeoutMsg: `Fullscreen exit badge did not disappear within ${timeout} ms` },
  );
}

/**
 * Ensure we are in normal (non-fullscreen) mode before each test.
 * Reads the current fullscreen class on the tab row — if fullscreen chrome
 * is active (hover zones present), exit via IPC.
 */
async function ensureNormalMode(): Promise<void> {
  const inFullscreen = await browser.execute((): boolean => {
    // The fullscreen hover zones are only rendered when fullscreenState.value is true.
    return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
  });

  if (inFullscreen) {
    // Exit fullscreen by toggling.  The toggle command flips the current state,
    // so calling it once when we are in fullscreen exits to normal mode.
    await ipcToggleFullscreen();
    // Wait for hover zones to disappear (they are conditionally rendered).
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') === null;
        }),
      { timeout: 5_000, timeoutMsg: 'Could not restore normal mode after test' },
    );
    // Also wait for tab row to be visible again.
    await waitForTabRowVisible(5_000);
  }
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe('TauTerm — Fullscreen mode', () => {
  afterEach(async () => {
    // Always restore normal mode so subsequent tests start from a clean state.
    await ensureNormalMode();
  });

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
  // TEST-FS-E2E-002: F11 enters fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-002: F11 enters fullscreen — hover zones rendered, badge appears, tab bar auto-hides', async () => {
    // Dispatch F11 via dispatchEvent (bypasses WM key capture).
    await dispatchF11();

    // Wait for the fullscreen hover zones to appear — these are conditionally
    // rendered only when fullscreenState.value is true.
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Fullscreen hover zones did not appear after F11' },
    );

    // The tab bar auto-hides after 1.5s in fullscreen. Wait for it.
    await waitForTabRowHidden(6_000);

    // Badge must be present (tab bar is hidden → badge is shown).
    await waitForBadgePresent(3_000);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-E2E-003: Second F11 exits fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-003: second F11 exits fullscreen — tab bar visible, badge absent', async () => {
    // Enter fullscreen first via IPC (setup — not the scenario under test here).
    await ipcToggleFullscreen();
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Could not enter fullscreen for TEST-FS-E2E-003 setup' },
    );

    // Exit via F11 — this is the scenario under test.
    await dispatchF11();

    // Hover zones must disappear.
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') === null;
        }),
      { timeout: 5_000, timeoutMsg: 'Fullscreen hover zones did not disappear after second F11' },
    );

    // Tab row must be visible (no --hidden class).
    await waitForTabRowVisible(3_000);

    // Badge must be absent.
    await waitForBadgeAbsent(3_000);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-E2E-004: F11 does not send any character to the PTY.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-004: F11 does not send characters to the PTY', async () => {
    // Establish a known state: inject a unique sentinel to mark the current
    // end of the terminal buffer, then press F11, and verify no new characters
    // appear beyond the sentinel.

    // Wait for the active pane to be ready.
    await browser.waitUntil(
      async () => {
        try {
          return await $(Selectors.activeTerminalPane).isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 15_000, timeoutMsg: 'Active terminal pane did not appear within 15 s' },
    );

    const paneId = await $(Selectors.activeTerminalPane).getAttribute('data-pane-id');
    expect(paneId).toBeTruthy();

    // Inject a sentinel to mark the current buffer state.
    const sentinel = 'FS-E2E-004-SENTINEL';
    const sentinelBytes = [...new TextEncoder().encode(sentinel)];
    await browser.execute(
      function (cmdArg: string, argsArg: Record<string, unknown>) {
        (window as any).__TAURI_INTERNALS__.invoke(cmdArg, argsArg);
      },
      'inject_pty_output',
      { paneId, data: sentinelBytes },
    );

    // Wait for the sentinel to appear.
    await browser.waitUntil(
      () =>
        browser.execute((s: string): boolean => {
          const grid = document.querySelector('.terminal-grid');
          return grid !== null && (grid.textContent ?? '').includes(s);
        }, sentinel),
      { timeout: 10_000, timeoutMsg: 'Sentinel did not appear before F11 dispatch' },
    );

    // Record grid content before F11.
    const contentBefore = await browser.execute((): string => {
      return (document.querySelector('.terminal-grid')?.textContent ?? '').trim();
    });

    // Dispatch F11.
    await dispatchF11();

    // Wait for fullscreen chrome to appear (confirms F11 was handled).
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Fullscreen chrome did not appear — F11 may not have been handled' },
    );

    // Allow a brief tick for any hypothetical PTY write to propagate.
    // We use a short waitUntil with a condition that's immediately true rather
    // than a sleep, to give the event loop a chance to flush.
    await browser.waitUntil(() => Promise.resolve(true), { timeout: 500 });

    // Grid content must be unchanged (no new characters from F11).
    const contentAfter = await browser.execute((): string => {
      return (document.querySelector('.terminal-grid')?.textContent ?? '').trim();
    });

    expect(contentAfter).toBe(contentBefore);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-E2E-005: Clicking the badge exits fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-005: clicking the fullscreen exit badge exits fullscreen', async () => {
    // Enter fullscreen via IPC.
    await ipcToggleFullscreen();
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Could not enter fullscreen for TEST-FS-E2E-005 setup' },
    );

    // Wait for badge to appear (tab bar must have auto-hidden first).
    await waitForBadgePresent(7_000);

    // Click the badge via a native click event (avoids WebKitGTK click quirks).
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-exit-badge"]',
      );
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Hover zones must disappear.
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') === null;
        }),
      { timeout: 5_000, timeoutMsg: 'Did not exit fullscreen after badge click' },
    );

    // Tab row must be visible.
    await waitForTabRowVisible(3_000);

    // Badge must be gone.
    await waitForBadgeAbsent(3_000);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-E2E-006: Keyboard activation of badge (Tab + Enter) exits fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-006: focusing badge via Tab and pressing Enter exits fullscreen', async () => {
    // Enter fullscreen via IPC.
    await ipcToggleFullscreen();
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Could not enter fullscreen for TEST-FS-E2E-006 setup' },
    );

    // Wait for badge to appear (tab bar auto-hides after 1.5s).
    await waitForBadgePresent(7_000);

    // Focus the badge programmatically and dispatch Enter.
    // browser.keys(['Tab']) could cycle focus but the number of Tab presses
    // depends on the DOM order at runtime. Direct focus + keydown is stable.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-exit-badge"]',
      );
      if (btn) {
        btn.focus();
        btn.dispatchEvent(
          new KeyboardEvent('keydown', {
            key: 'Enter',
            code: 'Enter',
            bubbles: true,
            cancelable: true,
          }),
        );
      }
    });

    // Hover zones must disappear.
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') === null;
        }),
      { timeout: 5_000, timeoutMsg: 'Did not exit fullscreen after Enter on badge' },
    );

    await waitForTabRowVisible(3_000);
    await waitForBadgeAbsent(3_000);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-E2E-007: Hovering the top edge recalls the tab bar.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-007: hovering the 4px top edge recalls the tab bar in fullscreen', async () => {
    // Enter fullscreen and wait for tab bar to auto-hide.
    await ipcToggleFullscreen();
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Could not enter fullscreen for TEST-FS-E2E-007 setup' },
    );
    await waitForTabRowHidden(6_000);

    // Trigger mouseenter on the top hover zone via a synthetic event.
    // WebdriverIO's moveTo cannot reliably target a 4px strip via pixel coords;
    // dispatchEvent on the element is stable and tests the handler directly.
    await browser.execute((): void => {
      const zone = document.querySelector<HTMLElement>(
        '.terminal-view__fullscreen-hover-top',
      );
      zone?.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
    });

    // The tab row --hidden class must be removed (tab bar recalled).
    await waitForTabRowVisible(3_000);

    // Badge must have disappeared (tab bar is now visible).
    await waitForBadgeAbsent(3_000);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-E2E-008: Rapid toggle (×3) leaves state coherent.
  // -------------------------------------------------------------------------
  it('TEST-FS-E2E-008: three rapid F11 toggles leave the state coherent (odd = fullscreen)', async () => {
    // Start from normal mode (ensured by afterEach of previous test).
    // Toggle 3 times rapidly — expected final state: fullscreen (odd count).
    await dispatchF11();
    await dispatchF11();
    await dispatchF11();

    // Wait enough time for all IPC round-trips to settle (each toggle_fullscreen
    // has a 200ms internal delay before emitting the event).
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          // After 3 toggles from normal, expect fullscreen chrome present.
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      {
        timeout: 5_000,
        timeoutMsg: 'After 3 rapid F11 presses, expected to end in fullscreen mode',
      },
    );

    // Confirm tab row ends hidden (auto-hide will trigger) or at least that
    // the DOM is consistent with fullscreen mode (hover zones present).
    const hoverZonePresent = await browser.execute((): boolean => {
      return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
    });
    expect(hoverZonePresent).toBe(true);

    // afterEach will restore normal mode.
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
  // TEST-FS-BTN-002: Button shows Maximize2 icon (enter-fullscreen label) when not in fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-BTN-002: button aria-label is "Enter full screen" when in normal mode', async () => {
    // Ensure normal mode.
    const btn = await $(Selectors.fullscreenToggleBtn);
    const label = await btn.getAttribute('aria-label');
    expect(label).toBe('Enter full screen');
  });

  // -------------------------------------------------------------------------
  // TEST-FS-BTN-003: Clicking the button enters fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-BTN-003: clicking the fullscreen toggle button enters fullscreen', async () => {
    // Click via synthetic event (avoids WebKitGTK click quirks).
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-toggle-btn"]',
      );
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Fullscreen hover zones must appear — reliable indicator that fullscreen is active.
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Fullscreen hover zones did not appear after button click' },
    );
  });

  // -------------------------------------------------------------------------
  // TEST-FS-BTN-004: Button shows Minimize2 icon (exit-fullscreen label) when in fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-BTN-004: button aria-label is "Exit full screen" when tab bar is recalled in fullscreen', async () => {
    // Enter fullscreen via IPC.
    await ipcToggleFullscreen();
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Could not enter fullscreen for TEST-FS-BTN-004 setup' },
    );

    // Recall the tab bar by triggering mouseenter on the top hover zone so the
    // button is visible and its aria-label can be read.
    await browser.execute((): void => {
      const zone = document.querySelector<HTMLElement>(
        '.terminal-view__fullscreen-hover-top',
      );
      zone?.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
    });
    await waitForTabRowVisible(3_000);

    // aria-label must now reflect "Exit full screen".
    const label = await browser.execute((): string | null => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-toggle-btn"]',
      );
      return btn?.getAttribute('aria-label') ?? null;
    });
    expect(label).toBe('Exit full screen');
  });

  // -------------------------------------------------------------------------
  // TEST-FS-BTN-005: Clicking the button from fullscreen exits fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-BTN-005: clicking the fullscreen toggle button exits fullscreen when tab bar is recalled', async () => {
    // Enter fullscreen via IPC.
    await ipcToggleFullscreen();
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Could not enter fullscreen for TEST-FS-BTN-005 setup' },
    );

    // Recall the tab bar so the button is reachable.
    await browser.execute((): void => {
      const zone = document.querySelector<HTMLElement>(
        '.terminal-view__fullscreen-hover-top',
      );
      zone?.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
    });
    await waitForTabRowVisible(3_000);

    // Click the button.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-toggle-btn"]',
      );
      btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Hover zones must disappear (fullscreen exited).
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') === null;
        }),
      { timeout: 5_000, timeoutMsg: 'Did not exit fullscreen after button click from fullscreen' },
    );

    await waitForTabRowVisible(3_000);
    await waitForBadgeAbsent(3_000);
  });

  // -------------------------------------------------------------------------
  // TEST-FS-BTN-006: FS-FULL-004 — button is keyboard-accessible; Enter toggles fullscreen.
  // -------------------------------------------------------------------------
  it('TEST-FS-BTN-006: fullscreen toggle button is in the tab order and Enter activates it', async () => {
    // The button must have a non-negative tabindex (or none, which defaults to 0).
    const tabindex = await browser.execute((): number => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-toggle-btn"]',
      );
      if (!btn) return -999;
      // getAttribute returns the explicit attribute; null means element-default (0 for buttons).
      const attr = btn.getAttribute('tabindex');
      return attr !== null ? parseInt(attr, 10) : 0;
    });
    expect(tabindex).toBeGreaterThanOrEqual(0);

    // Focus the button programmatically and dispatch Enter — must enter fullscreen.
    await browser.execute((): void => {
      const btn = document.querySelector<HTMLButtonElement>(
        '[data-testid="fullscreen-toggle-btn"]',
      );
      if (btn) {
        btn.focus();
        btn.dispatchEvent(
          new KeyboardEvent('keydown', {
            key: 'Enter',
            code: 'Enter',
            bubbles: true,
            cancelable: true,
          }),
        );
        // Also dispatch click — Enter on a <button> fires click in browsers.
        btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      }
    });

    // Fullscreen hover zones must appear, confirming the toggle fired.
    await browser.waitUntil(
      () =>
        browser.execute((): boolean => {
          return document.querySelector('.terminal-view__fullscreen-hover-top') !== null;
        }),
      { timeout: 5_000, timeoutMsg: 'Fullscreen did not activate after Enter on fullscreen toggle button' },
    );
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
