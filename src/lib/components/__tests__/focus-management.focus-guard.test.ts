// SPDX-License-Identifier: MPL-2.0

/**
 * Focus management — focus guard tests.
 *
 * Covered:
 *   TEST-FOCUS-001 — Focus guard: body focus redirected to activeViewportEl when no modal open
 *   TEST-FOCUS-002 — Focus guard: modal open → guard does NOT redirect focus
 *   TEST-FOCUS-003 — Focus guard: activeViewportEl null → guard does NOT throw
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
// Focus guard pure logic (mirrors onFocusIn in useTerminalView.core.svelte.ts)
//
// The guard is extracted here as a pure function so it can be unit-tested
// without a Svelte component lifecycle. The behaviour contract is:
//   1. If event.target is NOT document.body → return (don't redirect).
//   2. If a [role="dialog"][aria-modal="true"] element exists in the document → return.
//   3. If activeViewportEl is null → return safely (no throw).
//   4. Otherwise → call activeViewportEl.focus({ preventScroll: true }).
// ---------------------------------------------------------------------------

function onFocusIn(
  e: { target: EventTarget | null },
  activeViewportEl: HTMLElement | null,
): void {
  if (e.target !== document.body) return;
  if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
  const el = activeViewportEl;
  if (!el) return;
  el.focus({ preventScroll: true });
}

// ---------------------------------------------------------------------------
// TEST-FOCUS-001: body focus → redirect to activeViewportEl
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-001: focus guard redirects body focus to activeViewportEl', () => {
  it('calls activeViewportEl.focus({ preventScroll: true }) when document.body receives focus', () => {
    const viewport = document.createElement('div');
    document.body.appendChild(viewport);
    const focusSpy = vi.spyOn(viewport, 'focus');

    onFocusIn({ target: document.body }, viewport);

    expect(focusSpy).toHaveBeenCalledOnce();
    expect(focusSpy).toHaveBeenCalledWith({ preventScroll: true });
  });

  it('does NOT call focus when event.target is another element (not body)', () => {
    const viewport = document.createElement('div');
    const other = document.createElement('input');
    document.body.appendChild(viewport);
    document.body.appendChild(other);
    const focusSpy = vi.spyOn(viewport, 'focus');

    onFocusIn({ target: other }, viewport);

    expect(focusSpy).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-002: modal dialog open → guard does NOT redirect focus
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-002: focus guard skips redirect when a modal dialog is open', () => {
  it('does NOT call focus when [role="dialog"][aria-modal="true"] is present', () => {
    const viewport = document.createElement('div');
    document.body.appendChild(viewport);
    const focusSpy = vi.spyOn(viewport, 'focus');

    // Insert an open modal dialog into the document
    const dialog = document.createElement('div');
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    document.body.appendChild(dialog);

    onFocusIn({ target: document.body }, viewport);

    expect(focusSpy).not.toHaveBeenCalled();
  });

  it('DOES redirect when a dialog without aria-modal is present (non-modal dialog)', () => {
    const viewport = document.createElement('div');
    document.body.appendChild(viewport);
    const focusSpy = vi.spyOn(viewport, 'focus');

    // Dialog without aria-modal="true" should NOT block the guard
    const dialog = document.createElement('div');
    dialog.setAttribute('role', 'dialog');
    // No aria-modal attribute
    document.body.appendChild(dialog);

    onFocusIn({ target: document.body }, viewport);

    expect(focusSpy).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-003: activeViewportEl null → guard does NOT throw
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-003: focus guard is safe when activeViewportEl is null', () => {
  it('does not throw when activeViewportEl is null and body receives focus', () => {
    expect(() => {
      onFocusIn({ target: document.body }, null);
    }).not.toThrow();
  });
});
