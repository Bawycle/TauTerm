// SPDX-License-Identifier: MPL-2.0

/**
 * E2E scenario: Search UI — query → backend → highlight → navigation.
 *
 * Exercises the full search loop that was previously untested at E2E level:
 *   1. Open the search overlay via Ctrl+Shift+F
 *   2. Type a search query → backend search_pane is called
 *   3. Matches are highlighted in the terminal grid
 *   4. Navigate next/prev match
 *   5. Clear the search / close the overlay
 *
 * Protocol references:
 *   - FS-SEARCH-001 (open search), FS-SEARCH-002 (query IPC),
 *     FS-SEARCH-003 (highlight), FS-SEARCH-004 (navigation),
 *     FS-SEARCH-005 (close)
 *   - docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md §4.7
 *
 * Build requirement:
 *   The binary MUST be built with --features e2e-testing so that
 *   inject_pty_output is available for seeding terminal content:
 *     pnpm tauri build --no-bundle -- --features e2e-testing
 *
 * Implementation note on search backend:
 *   search_pane operates on the screen buffer (vt/search.rs). In the E2E
 *   binary, the InjectablePtyBackend sends injected bytes through the real
 *   VT parser and screen buffer — so injected text is genuinely searchable.
 *   The backend returns Vec<SearchMatch> which the frontend uses to render
 *   highlighted cells via searchMatchSet / activeSearchMatchSet in
 *   useTerminalPane.svelte.ts.
 */

import { browser } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Invoke a Tauri IPC command and await its result.
 *
 * Stores the resolved value in `window.__e2e_invoke_result` then polls until
 * it appears — workaround for `browser.executeAsync` unreliability with
 * tauri-driver / WebKitGTK.
 */
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await browser.execute(function () {
    (window as unknown as Record<string, unknown>).__e2e_invoke_result = undefined;
  });

  await browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (
        window as unknown as {
          __TAURI_INTERNALS__: { invoke: (c: string, a?: unknown) => Promise<unknown> };
        }
      ).__TAURI_INTERNALS__
        .invoke(cmdArg, argsArg)
        .then(function (r: unknown) {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result = r ?? '__e2e_null__';
        })
        .catch(function () {
          (window as unknown as Record<string, unknown>).__e2e_invoke_result = '__e2e_error__';
        });
    },
    cmd,
    args,
  );

  await browser.waitUntil(
    () =>
      browser.execute(function (): boolean {
        return (window as unknown as Record<string, unknown>).__e2e_invoke_result !== undefined;
      }),
    { timeout: 5_000, timeoutMsg: `tauriInvoke("${cmd}") did not resolve within 5 s` },
  );

  const raw = await browser.execute(function (): unknown {
    const v = (window as unknown as Record<string, unknown>).__e2e_invoke_result;
    delete (window as unknown as Record<string, unknown>).__e2e_invoke_result;
    return v;
  });

  if (raw === '__e2e_error__') {
    throw new Error(`tauriInvoke("${cmd}") rejected`);
  }
  return (raw === '__e2e_null__' ? null : raw) as T;
}

/**
 * Fire a Tauri IPC command without waiting for the return value.
 * See pty-roundtrip.spec.ts for detailed rationale.
 */
function tauriFireAndForget(cmd: string, args?: Record<string, unknown>): Promise<void> {
  return browser.execute(
    function (cmdArg: string, argsArg: Record<string, unknown> | undefined) {
      (window as unknown as Record<string, { invoke: Function }>).__TAURI_INTERNALS__.invoke(
        cmdArg,
        argsArg,
      );
    },
    cmd,
    args,
  ) as unknown as Promise<void>;
}

/**
 * Dispatch a keyboard shortcut via DOM events, targeting the terminal grid.
 */
async function dispatchShortcut(
  key: string,
  code: string,
  ctrlKey: boolean,
  shiftKey: boolean,
): Promise<void> {
  await browser.execute(
    function (keyArg: string, codeArg: string, ctrlArg: boolean, shiftArg: boolean): void {
      const grid = document.querySelector('.terminal-grid') as HTMLElement | null;
      const target = grid ?? document.body;
      target.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: keyArg,
          code: codeArg,
          ctrlKey: ctrlArg,
          shiftKey: shiftArg,
          bubbles: true,
          cancelable: true,
        }),
      );
    },
    key,
    code,
    ctrlKey,
    shiftKey,
  );
}

/**
 * Determine whether the search overlay is currently visible in the DOM.
 */
async function isSearchOverlayOpen(): Promise<boolean> {
  return browser.execute((): boolean => {
    return document.querySelector('.search-overlay') !== null;
  });
}

/**
 * Type text into the search input field by setting its value and dispatching
 * an 'input' event. The SearchOverlay component debounces at 150ms.
 */
async function typeInSearchInput(text: string): Promise<void> {
  await browser.execute((textArg: string): void => {
    const input = document.querySelector<HTMLInputElement>('.search-overlay input[type="text"]');
    if (!input) return;
    // Set value programmatically and fire input event to trigger the debounced search.
    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      'value',
    )?.set;
    if (nativeInputValueSetter) {
      nativeInputValueSetter.call(input, textArg);
    } else {
      input.value = textArg;
    }
    input.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' }));
  }, text);
}

/**
 * Get the text content of the search match count element.
 */
async function getMatchCountText(): Promise<string> {
  return browser.execute((): string => {
    const el = document.querySelector('.search-overlay__count');
    return el?.textContent?.trim() ?? '';
  });
}

/**
 * Get the active pane ID from the DOM.
 */
async function getActivePaneId(): Promise<string | null> {
  return browser.execute((): string | null => {
    const el = document.querySelector('.terminal-pane[data-active="true"]');
    return el ? el.getAttribute('data-pane-id') : null;
  });
}

/** Seed known text into the active pane via inject_pty_output (fire-and-forget). */
async function seedPaneContent(paneId: string, content: string): Promise<void> {
  const bytes = [...new TextEncoder().encode(content)];
  await tauriFireAndForget('inject_pty_output', { paneId, data: bytes });
}

/** Seed known text into the active pane via inject_pty_output (awaited, throws on error). */
async function seedPaneContentAwaited(paneId: string, content: string): Promise<void> {
  const bytes = [...new TextEncoder().encode(content)];
  await tauriInvoke<void>('inject_pty_output', { paneId, data: bytes });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('TauTerm — Search UI: query → backend → highlight → navigation', () => {
  /**
   * TEST-SEARCH-E2E-001: Ctrl+Shift+F opens the search overlay.
   *
   * Pressing the search shortcut must make the .search-overlay element
   * appear in the DOM.
   */
  it('TEST-SEARCH-E2E-001: Ctrl+Shift+F opens the search overlay', async () => {
    // Give focus to the terminal before sending shortcuts
    await browser.execute((): void => {
      const grid = document.querySelector<HTMLElement>('.terminal-grid');
      grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Dispatch Ctrl+Shift+F
    await dispatchShortcut('F', 'KeyF', true, true);

    await browser.waitUntil(isSearchOverlayOpen, {
      timeout: 5_000,
      timeoutMsg: 'Search overlay did not appear within 5 s after Ctrl+Shift+F',
    });

    expect(await isSearchOverlayOpen()).toBe(true);
  });

  /**
   * TEST-SEARCH-E2E-002: The search input accepts keyboard input.
   *
   * After the overlay opens, the search input must be focusable and
   * accept text input. This validates the component is interactive.
   */
  it('TEST-SEARCH-E2E-002: search input exists and accepts text', async () => {
    // Ensure overlay is open
    if (!(await isSearchOverlayOpen())) {
      await browser.execute((): void => {
        const grid = document.querySelector<HTMLElement>('.terminal-grid');
        grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      });
      await dispatchShortcut('F', 'KeyF', true, true);
      await browser.waitUntil(isSearchOverlayOpen, {
        timeout: 5_000,
        timeoutMsg: 'Search overlay did not open for input test',
      });
    }

    const inputExists = await browser.execute((): boolean => {
      return document.querySelector('.search-overlay input[type="text"]') !== null;
    });
    expect(inputExists).toBe(true);

    // Type a query via the input
    await typeInSearchInput('hello');

    // Verify the value was applied
    const inputValue = await browser.execute((): string => {
      const input = document.querySelector<HTMLInputElement>('.search-overlay input[type="text"]');
      return input?.value ?? '';
    });
    expect(inputValue).toBe('hello');
  });

  /**
   * TEST-SEARCH-E2E-003: Typing a query that matches injected content
   * results in a non-zero match count.
   *
   * Seeds the active pane with a unique token via inject_pty_output, then
   * searches for it. The backend search_pane command (vt/search.rs) runs on
   * the real screen buffer and returns SearchMatch results that update the
   * match count display.
   *
   * Requires --features e2e-testing binary.
   */
  it('TEST-SEARCH-E2E-003: searching for injected content yields matches', async () => {
    // Seed the pane with a distinctive searchable token.
    // Use awaited invoke so errors surface instead of timing out.
    const paneId = await getActivePaneId();
    expect(paneId).toBeTruthy();

    // Reset scroll position in case a previous test left the viewport scrolled.
    await tauriInvoke<void>('scroll_to_bottom', { paneId: paneId! });
    await browser.pause(100);

    const searchToken = 'e2e-search-token-xyz';
    await seedPaneContentAwaited(paneId!, searchToken + '\r\n');

    // Wait for the content to render in the grid (confirms VT parser processed it)
    await browser.waitUntil(
      () =>
        browser.execute((token: string): boolean => {
          const grid = document.querySelector('.terminal-grid');
          return grid !== null && (grid.textContent ?? '').includes(token);
        }, searchToken),
      { timeout: 10_000, timeoutMsg: `Seeded token "${searchToken}" did not render within 10 s` },
    );

    // search_pane searches the scrollback ring buffer, not the visible screen.
    // Inject newlines to scroll the token off the visible area into scrollback.
    await seedPaneContentAwaited(paneId!, '\r\n'.repeat(50));
    await browser.pause(300);

    // Ensure search overlay is open
    if (!(await isSearchOverlayOpen())) {
      await browser.execute((): void => {
        const grid = document.querySelector<HTMLElement>('.terminal-grid');
        grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      });
      await dispatchShortcut('F', 'KeyF', true, true);
      await browser.waitUntil(isSearchOverlayOpen, {
        timeout: 5_000,
        timeoutMsg: 'Search overlay did not open for match-count test',
      });
    }

    // Clear previous query and type the search token
    await typeInSearchInput('');
    // Small pause for debounce reset
    await browser.pause(200);
    await typeInSearchInput(searchToken);

    // Wait for the search to complete and match count to reflect ≥1 match.
    // The debounce is 150ms; the IPC round-trip to search_pane adds latency.
    await browser.waitUntil(
      async () => {
        const text = await getMatchCountText();
        // The count element shows "N of M" format when matches exist,
        // or a "no matches" message when 0. We wait until it contains a digit > 0.
        return /[1-9]/.test(text);
      },
      {
        timeout: 8_000,
        timeoutMsg: `Match count did not show ≥1 match for "${searchToken}" within 8 s`,
      },
    );

    const countText = await getMatchCountText();
    // Should contain a positive integer (e.g. "1 of 1")
    expect(countText).toMatch(/[1-9]/);
  });

  /**
   * TEST-SEARCH-E2E-004: The match count display uses "N of M" format.
   *
   * When there are matches, the count element must show both the current
   * match index and the total. This validates the UI display format (UXD §7.4).
   */
  it('TEST-SEARCH-E2E-004: match count shows "N of M" format', async () => {
    // Ensure search is open with a previous query that yielded matches
    if (!(await isSearchOverlayOpen())) {
      await browser.execute((): void => {
        const grid = document.querySelector<HTMLElement>('.terminal-grid');
        grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      });
      await dispatchShortcut('F', 'KeyF', true, true);
      await browser.waitUntil(isSearchOverlayOpen, {
        timeout: 5_000,
        timeoutMsg: 'Search overlay did not open',
      });
    }

    const countText = await getMatchCountText();

    // If we have matches from the previous test, the count text follows "N of M".
    // If not (isolated run), the test still validates the format when matches exist.
    if (/[1-9]/.test(countText)) {
      // Text should be parseable as "N of M" or similar locale-driven format.
      // We verify it contains at least two digit sequences separated by a non-digit.
      expect(countText).toMatch(/\d+\D+\d+/);
    }
    // If no matches yet, the display shows a "no results" string — acceptable here.
    expect(countText.length).toBeGreaterThan(0);
  });

  /**
   * TEST-SEARCH-E2E-005: The Next button advances the active match index.
   *
   * Clicks the Next (ChevronDown) navigation button and verifies the
   * match count's current index increments.
   */
  it('TEST-SEARCH-E2E-005: Next button advances the active match index', async () => {
    // Close the search overlay first to start from a known state.
    // Previous tests may have left it open with a stale query.
    if (await isSearchOverlayOpen()) {
      await browser.execute((): void => {
        const input = document.querySelector<HTMLInputElement>(
          '.search-overlay input[type="text"]',
        );
        if (input) {
          input.dispatchEvent(
            new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
          );
        }
      });
      await browser.waitUntil(async () => !(await isSearchOverlayOpen()), {
        timeout: 3_000,
        timeoutMsg: 'Search overlay did not close before test 005',
      });
    }

    // Ensure search overlay is open with a seeded pane containing matches
    const paneId = await getActivePaneId();
    expect(paneId).toBeTruthy();

    // Reset scroll to bottom: tests 003/004 search navigation may have scrolled
    // the viewport up to show a match. New inject_pty_output writes to the main
    // screen (bottom), which is outside the viewport when scrolled — waitForTextInGrid
    // reads .terminal-grid textContent (visible rows only) and times out even though
    // the content is present in the buffer. scroll_to_bottom resets scroll_offset to 0.
    await tauriInvoke<void>('scroll_to_bottom', { paneId: paneId! });
    await browser.pause(100); // allow one rAF cycle for DOM re-render at scroll bottom

    // Seed content via awaited invoke so errors surface instead of timing out.
    const searchToken = 'e2e-nav-token';
    await seedPaneContentAwaited(paneId!, `${searchToken}\r\n${searchToken}\r\n`);

    await browser.waitUntil(
      () =>
        browser.execute((token: string): boolean => {
          const grid = document.querySelector('.terminal-grid');
          return (grid?.textContent ?? '').includes(token);
        }, searchToken),
      { timeout: 10_000, timeoutMsg: `Nav token "${searchToken}" did not render` },
    );

    // Push token into scrollback so search_pane can find it
    await seedPaneContentAwaited(paneId!, '\r\n'.repeat(50));
    await browser.pause(300);

    if (!(await isSearchOverlayOpen())) {
      await browser.execute((): void => {
        const grid = document.querySelector<HTMLElement>('.terminal-grid');
        grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      });
      await dispatchShortcut('F', 'KeyF', true, true);
      await browser.waitUntil(isSearchOverlayOpen, {
        timeout: 5_000,
        timeoutMsg: 'Search overlay did not open for nav test',
      });
    }

    await typeInSearchInput(searchToken);

    // Wait for ≥2 matches (we injected the token twice)
    await browser.waitUntil(
      async () => {
        const text = await getMatchCountText();
        // Check for a number ≥2 somewhere in the count text
        const m = text.match(/(\d+)\s*(?:of|\/)\s*(\d+)/i);
        return m !== null && parseInt(m[2], 10) >= 2;
      },
      {
        timeout: 8_000,
        timeoutMsg: `Expected ≥2 matches for "${searchToken}" — got: ${await getMatchCountText()}`,
      },
    );

    const beforeText = await getMatchCountText();
    const beforeMatch = beforeText.match(/(\d+)/);
    const beforeIdx = beforeMatch ? parseInt(beforeMatch[1], 10) : 0;

    // Click Next button (second .search-overlay__nav-btn, ChevronDown)
    await browser.execute((): void => {
      const navBtns = document.querySelectorAll<HTMLButtonElement>('.search-overlay__nav-btn');
      if (navBtns.length >= 2) {
        navBtns[1].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      }
    });

    // Wait for match count to update
    await browser.waitUntil(
      async () => {
        const text = await getMatchCountText();
        const m = text.match(/(\d+)/);
        const idx = m ? parseInt(m[1], 10) : beforeIdx;
        return idx !== beforeIdx;
      },
      {
        timeout: 5_000,
        timeoutMsg: 'Match count index did not change after clicking Next',
      },
    );

    const afterText = await getMatchCountText();
    const afterMatch = afterText.match(/(\d+)/);
    const afterIdx = afterMatch ? parseInt(afterMatch[1], 10) : beforeIdx;

    // Index must have changed (wrapped or incremented)
    expect(afterIdx).not.toBe(beforeIdx);
  });

  /**
   * TEST-SEARCH-E2E-006: The Prev button navigates to the previous match.
   *
   * After Next has been pressed at least once, pressing Prev must move
   * back to the previous match index.
   */
  it('TEST-SEARCH-E2E-006: Prev button moves to previous match', async () => {
    // This test depends on TEST-SEARCH-E2E-005 having set up search with ≥2 matches.
    // If the overlay was closed, re-open it.
    if (!(await isSearchOverlayOpen())) {
      await browser.execute((): void => {
        const grid = document.querySelector<HTMLElement>('.terminal-grid');
        grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      });
      await dispatchShortcut('F', 'KeyF', true, true);
      await browser.waitUntil(isSearchOverlayOpen, {
        timeout: 5_000,
        timeoutMsg: 'Search overlay did not open for Prev button test',
      });
    }

    const beforeText = await getMatchCountText();
    const beforeMatch = beforeText.match(/(\d+)/);
    const beforeIdx = beforeMatch ? parseInt(beforeMatch[1], 10) : 0;

    // Click Prev button (first .search-overlay__nav-btn, ChevronUp)
    await browser.execute((): void => {
      const navBtns = document.querySelectorAll<HTMLButtonElement>('.search-overlay__nav-btn');
      if (navBtns.length >= 1) {
        navBtns[0].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      }
    });

    // Wait for index to change
    await browser.waitUntil(
      async () => {
        const text = await getMatchCountText();
        const m = text.match(/(\d+)/);
        const idx = m ? parseInt(m[1], 10) : beforeIdx;
        return idx !== beforeIdx;
      },
      {
        timeout: 5_000,
        timeoutMsg: 'Match count index did not change after clicking Prev',
      },
    );

    const afterText = await getMatchCountText();
    const afterMatch = afterText.match(/(\d+)/);
    const afterIdx = afterMatch ? parseInt(afterMatch[1], 10) : beforeIdx;
    expect(afterIdx).not.toBe(beforeIdx);
  });

  /**
   * TEST-SEARCH-E2E-007: Escape key closes the search overlay.
   *
   * While the search overlay is open, pressing Escape must close it
   * (onclose handler sets searchOpen=false in TerminalView).
   */
  it('TEST-SEARCH-E2E-007: Escape closes the search overlay', async () => {
    // Ensure overlay is open
    if (!(await isSearchOverlayOpen())) {
      await browser.execute((): void => {
        const grid = document.querySelector<HTMLElement>('.terminal-grid');
        grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      });
      await dispatchShortcut('F', 'KeyF', true, true);
      await browser.waitUntil(isSearchOverlayOpen, {
        timeout: 5_000,
        timeoutMsg: 'Search overlay did not open for Escape test',
      });
    }

    // Dispatch Escape on the search input
    await browser.execute((): void => {
      const input = document.querySelector<HTMLInputElement>('.search-overlay input[type="text"]');
      if (input) {
        input.dispatchEvent(
          new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
        );
      }
    });

    // Wait for overlay to disappear
    await browser.waitUntil(async () => !(await isSearchOverlayOpen()), {
      timeout: 5_000,
      timeoutMsg: 'Search overlay did not close after Escape',
    });

    expect(await isSearchOverlayOpen()).toBe(false);
  });

  /**
   * TEST-SEARCH-E2E-008: The close (X) button closes the search overlay.
   *
   * Clicking the .search-overlay__close-btn must close the overlay.
   */
  it('TEST-SEARCH-E2E-008: X button closes the search overlay', async () => {
    // Re-open search
    await browser.execute((): void => {
      const grid = document.querySelector<HTMLElement>('.terminal-grid');
      grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    await dispatchShortcut('F', 'KeyF', true, true);
    await browser.waitUntil(isSearchOverlayOpen, {
      timeout: 5_000,
      timeoutMsg: 'Search overlay did not open for X-button test',
    });

    // Click the close button
    await browser.execute((): void => {
      const closeBtn = document.querySelector<HTMLButtonElement>('.search-overlay__close-btn');
      closeBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });

    // Wait for overlay to close
    await browser.waitUntil(async () => !(await isSearchOverlayOpen()), {
      timeout: 5_000,
      timeoutMsg: 'Search overlay did not close after clicking X button',
    });

    expect(await isSearchOverlayOpen()).toBe(false);
  });

  /**
   * TEST-SEARCH-E2E-009: Searching for a non-existent term shows no-results indicator.
   *
   * When the query does not match anything in the screen buffer,
   * the match count element must display a "no results" message
   * (i.e., not a positive "N of M" format).
   */
  it('TEST-SEARCH-E2E-009: empty result set shows no-results indicator', async () => {
    // Re-open search
    await browser.execute((): void => {
      const grid = document.querySelector<HTMLElement>('.terminal-grid');
      grid?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    });
    await dispatchShortcut('F', 'KeyF', true, true);
    await browser.waitUntil(isSearchOverlayOpen, {
      timeout: 5_000,
      timeoutMsg: 'Search overlay did not open for no-results test',
    });

    // Type a query that cannot possibly match anything in the buffer
    const unmatchableQuery = 'XYZZY-FROBNICATOR-UNMATCHABLE-8675309';
    await typeInSearchInput(unmatchableQuery);

    // Wait for search to complete (debounce + IPC round-trip)
    await browser.waitUntil(
      async () => {
        const text = await getMatchCountText();
        // We wait until the count display updates to reflect zero matches.
        // A "N of M" pattern with N>0 means search is still from a previous query.
        // The no-results display does not match /[1-9].*of.*[1-9]/.
        return text.length > 0 && !/^1\s/.test(text);
      },
      {
        timeout: 8_000,
        timeoutMsg: 'Match count did not update to no-results state',
      },
    );

    const countText = await getMatchCountText();
    // Must not show a "1 of N" or "N of M" format with positive N
    expect(countText).not.toMatch(/^[1-9]\d*\s+(?:of|\/)\s+[1-9]/i);
  });
});
