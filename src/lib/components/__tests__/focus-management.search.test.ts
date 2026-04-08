// SPDX-License-Identifier: MPL-2.0

/**
 * Focus management — search close tests.
 *
 * Covered:
 *   TEST-FOCUS-005 — handleSearchClose: calls activeViewportEl.focus({ preventScroll: true })
 *
 * Architecture note on TEST-FOCUS-005:
 *   createIoHandlers() returns handleSearchClose as a plain function. It can be
 *   tested by constructing a minimal ViewState mock without touching Svelte lifecycle.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { createIoHandlers } from '$lib/composables/useTerminalView.io-handlers.svelte';
import type { ViewState } from '$lib/composables/useTerminalView.core.svelte';

// ---------------------------------------------------------------------------
// Shared teardown
// ---------------------------------------------------------------------------

afterEach(() => {
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-005: handleSearchClose calls activeViewportEl.focus({ preventScroll: true })
//
// createIoHandlers is a plain factory — it does not require a Svelte lifecycle.
// We construct a minimal ViewState mock and verify the focus call.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-005: handleSearchClose restores focus to activeViewportEl', () => {
  it('calls activeViewportEl.focus({ preventScroll: true }) on handleSearchClose', () => {
    const viewport = document.createElement('div');
    const focusSpy = vi.spyOn(viewport, 'focus');

    // Minimal ViewState mock: only the fields used by handleSearchClose
    const state: Partial<ViewState> = {
      get searchOpen() {
        return true;
      },
      set searchOpen(_v: boolean) {},
      get searchMatches() {
        return [];
      },
      set searchMatches(_v) {},
      get searchCurrentIdx() {
        return 0;
      },
      set searchCurrentIdx(_v: number) {},
      get activeViewportEl() {
        return viewport;
      },
      set activeViewportEl(_v) {},
    };

    const noop = async () => {};
    const noopSync = () => {};
    const noopDir = async (_dir: 'horizontal' | 'vertical') => {};
    const noopNav = async (_dir: 'left' | 'right' | 'up' | 'down') => {};

    const { handleSearchClose } = createIoHandlers(
      state as ViewState,
      noopSync as (delta: 1 | -1) => void,
      noop,
      async (_tabId: string) => {},
      noopDir,
      async (_paneId: string) => {},
      noopNav,
      noop,
    );

    handleSearchClose();

    expect(focusSpy).toHaveBeenCalledOnce();
    expect(focusSpy).toHaveBeenCalledWith({ preventScroll: true });
  });

  it('does NOT throw when activeViewportEl is null on handleSearchClose', () => {
    const state: Partial<ViewState> = {
      get searchOpen() {
        return true;
      },
      set searchOpen(_v: boolean) {},
      get searchMatches() {
        return [];
      },
      set searchMatches(_v) {},
      get searchCurrentIdx() {
        return 0;
      },
      set searchCurrentIdx(_v: number) {},
      get activeViewportEl() {
        return null;
      },
      set activeViewportEl(_v) {},
    };

    const noop = async () => {};
    const noopSync = () => {};
    const noopDir = async (_dir: 'horizontal' | 'vertical') => {};
    const noopNav = async (_dir: 'left' | 'right' | 'up' | 'down') => {};

    const { handleSearchClose } = createIoHandlers(
      state as ViewState,
      noopSync as (delta: 1 | -1) => void,
      noop,
      async (_tabId: string) => {},
      noopDir,
      async (_paneId: string) => {},
      noopNav,
      noop,
    );

    expect(() => handleSearchClose()).not.toThrow();
  });
});
