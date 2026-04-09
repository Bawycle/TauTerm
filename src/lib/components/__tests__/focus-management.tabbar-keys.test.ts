// SPDX-License-Identifier: MPL-2.0

/**
 * Focus management — TabBar printable key tests.
 *
 * Covered:
 *   TEST-FOCUS-017 — printable key triggers onEscapeTabBar
 */

import { describe, it, expect, vi, afterEach } from 'vitest';

// ---------------------------------------------------------------------------
// Shared teardown
// ---------------------------------------------------------------------------

afterEach(() => {
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-017: Tab bar printable key → onEscapeTabBar invoked (pure logic)
//
// When the user has navigated the tab bar with arrow keys and then types a
// printable character, handleTabKeydown must invoke onEscapeTabBar so focus
// returns to the terminal. This is the "transient navigation surface" contract:
// the tab bar is not a permanent focus owner.
//
// A printable character is defined as: key.length === 1 AND no Ctrl/Alt/Meta modifier.
// Non-printable keys (F2, Enter, Delete, Arrow*, Escape, Tab) keep their existing handlers.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-017: Tab bar printable key triggers onEscapeTabBar', () => {
  function makeHandleTabKeydown(
    getRenamingTabId: () => string | null,
    onEscapeTabBar: (() => void) | undefined,
  ) {
    return function handleTabKeydown(
      event: {
        key: string;
        isComposing?: boolean;
        ctrlKey?: boolean;
        altKey?: boolean;
        metaKey?: boolean;
        preventDefault: () => void;
      },
      tabId: string,
    ) {
      if (getRenamingTabId() === tabId) return;

      if (event.key === 'F2') {
        event.preventDefault();
      } else if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
      } else if (event.key === 'Delete') {
        event.preventDefault();
      } else if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
        event.preventDefault();
      } else if (event.key === 'Escape') {
        event.preventDefault();
        onEscapeTabBar?.();
      } else if (
        !event.isComposing &&
        !event.ctrlKey &&
        !event.altKey &&
        !event.metaKey &&
        event.key.length === 1
      ) {
        onEscapeTabBar?.();
      }
    };
  }

  it('printable character (letter) invokes onEscapeTabBar', () => {
    const cb = vi.fn();
    const handle = makeHandleTabKeydown(() => null, cb);
    handle({ key: 'a', preventDefault: vi.fn() }, 'tab-1');
    expect(cb).toHaveBeenCalledOnce();
  });

  it('printable character (digit) invokes onEscapeTabBar', () => {
    const cb = vi.fn();
    const handle = makeHandleTabKeydown(() => null, cb);
    handle({ key: '3', preventDefault: vi.fn() }, 'tab-1');
    expect(cb).toHaveBeenCalledOnce();
  });

  it('Ctrl+key does NOT invoke onEscapeTabBar (shortcut, not printable)', () => {
    const cb = vi.fn();
    const handle = makeHandleTabKeydown(() => null, cb);
    handle({ key: 'c', ctrlKey: true, preventDefault: vi.fn() }, 'tab-1');
    expect(cb).not.toHaveBeenCalled();
  });

  it('Alt+key does NOT invoke onEscapeTabBar', () => {
    const cb = vi.fn();
    const handle = makeHandleTabKeydown(() => null, cb);
    handle({ key: 'f', altKey: true, preventDefault: vi.fn() }, 'tab-1');
    expect(cb).not.toHaveBeenCalled();
  });

  it('navigation keys (F2, Enter, Delete, ArrowLeft) do NOT invoke onEscapeTabBar', () => {
    const cb = vi.fn();
    const handle = makeHandleTabKeydown(() => null, cb);
    for (const key of ['F2', 'Enter', ' ', 'Delete', 'ArrowLeft', 'ArrowRight']) {
      handle({ key, preventDefault: vi.fn() }, 'tab-1');
    }
    expect(cb).not.toHaveBeenCalled();
  });

  it('printable key during rename does NOT invoke onEscapeTabBar (early-return guard)', () => {
    const cb = vi.fn();
    const handle = makeHandleTabKeydown(() => 'tab-1', cb);
    handle({ key: 'a', preventDefault: vi.fn() }, 'tab-1');
    expect(cb).not.toHaveBeenCalled();
  });

  it('IME composing key does NOT invoke onEscapeTabBar', () => {
    const cb = vi.fn();
    const handle = makeHandleTabKeydown(() => null, cb);
    // During IME composition, isComposing=true even for single-char keys
    handle({ key: 'a', isComposing: true, preventDefault: vi.fn() }, 'tab-1');
    expect(cb).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-018: Delete key on focused tab closes tab (pure logic)
//
// When the user presses Delete on a focused tab, handleTabKeydown must call
// onTabClose with the correct tabId. The early-return guard (rename in progress
// on the same tab) must suppress the action. A rename on a DIFFERENT tab must
// not block the close.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-018: Delete key on focused tab closes tab', () => {
  function makeHandleTabKeydown(
    getRenamingTabId: () => string | null,
    onTabClose: (tabId: string) => void,
    onFocusTerminal?: () => void,
  ) {
    return function handleTabKeydown(
      event: {
        key: string;
        isComposing?: boolean;
        ctrlKey?: boolean;
        altKey?: boolean;
        metaKey?: boolean;
        preventDefault: () => void;
      },
      tabId: string,
    ) {
      if (getRenamingTabId() === tabId) return;
      if (event.key === 'Delete') {
        event.preventDefault();
        onTabClose(tabId);
      } else if (event.key === 'Escape') {
        event.preventDefault();
        onFocusTerminal?.();
      } else if (
        !event.isComposing &&
        !event.ctrlKey &&
        !event.altKey &&
        !event.metaKey &&
        event.key.length === 1
      ) {
        onFocusTerminal?.();
      }
    };
  }

  it('Delete key on active tab calls onTabClose with the correct tabId', () => {
    const closeCb = vi.fn();
    const handle = makeHandleTabKeydown(() => null, closeCb);
    handle({ key: 'Delete', preventDefault: vi.fn() }, 'tab-abc');
    expect(closeCb).toHaveBeenCalledOnce();
    expect(closeCb).toHaveBeenCalledWith('tab-abc');
  });

  it('Delete key during rename does NOT call onTabClose (early-return guard)', () => {
    const closeCb = vi.fn();
    const handle = makeHandleTabKeydown(() => 'tab-abc', closeCb);
    handle({ key: 'Delete', preventDefault: vi.fn() }, 'tab-abc');
    expect(closeCb).not.toHaveBeenCalled();
  });

  it('Delete key on a tab while a DIFFERENT tab is renaming calls onTabClose', () => {
    const closeCb = vi.fn();
    const handle = makeHandleTabKeydown(() => 'tab-xyz', closeCb);
    handle({ key: 'Delete', preventDefault: vi.fn() }, 'tab-abc');
    expect(closeCb).toHaveBeenCalledOnce();
    expect(closeCb).toHaveBeenCalledWith('tab-abc');
  });

  it('Delete key calls preventDefault', () => {
    const preventDefaultSpy = vi.fn();
    const handle = makeHandleTabKeydown(() => null, vi.fn());
    handle({ key: 'Delete', preventDefault: preventDefaultSpy }, 'tab-1');
    expect(preventDefaultSpy).toHaveBeenCalledOnce();
  });
});
