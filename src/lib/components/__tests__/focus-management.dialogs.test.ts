// SPDX-License-Identifier: MPL-2.0

/**
 * Focus management — dialog focus restoration tests.
 *
 * Covered:
 *   TEST-FOCUS-015 — ConnectionManager onclose focus (static check)
 *   TEST-FOCUS-016 — fullscreen toggle focus
 *   TEST-FOCUS-018 — Preferences panel Dialog.Content focus (static check)
 *   TEST-FOCUS-019 — Shared Dialog.svelte onCloseAutoFocus (static check)
 *   TEST-FOCUS-020 — onFocusIn safety net deferred re-check (static check)
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
// TEST-FOCUS-015: SSH panel onclose — activeViewportEl.focus() is called
//
// Static source analysis: asserts that the onclose callback of ConnectionManager
// in TerminalView.svelte calls activeViewportEl?.focus({ preventScroll: true })
// (with deferred execution to avoid racing with Bits UI FocusScope teardown).
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-015: SSH panel onclose restores focus to activeViewportEl', () => {
  it('TerminalView.svelte ConnectionManager onclose contains activeViewportEl focus call (static check)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalView.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // Find the ConnectionManager onclose block
    const oncloseIdx = source.indexOf('connectionManagerOpen = false');
    expect(oncloseIdx).toBeGreaterThan(-1);

    // The surrounding onclose block should restore focus to activeViewportEl
    const oncloseBlock = source.slice(Math.max(0, oncloseIdx - 100), oncloseIdx + 700);
    expect(oncloseBlock).toContain('activeViewportEl');
    expect(oncloseBlock).toContain('focus');
    expect(oncloseBlock).toContain('preventScroll');
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-016: Fullscreen state-change event — activeViewportEl.focus() restored
//
// Focus restoration after fullscreen toggle must happen AFTER the WM has
// stabilised the window geometry (the backend emits fullscreen-state-changed
// after a 200 ms delay for this reason). Triggering focus from onclick would
// fire before the geometry is stable and be ignored by some compositors.
//
// The fix: focus is restored inside the onFullscreenStateChanged handler in
// useTerminalView.core.svelte.ts, not in the onclick callback.
// Static source analysis confirms both files carry their respective duties.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-016: Fullscreen state-change handler restores focus to activeViewportEl', () => {
  it('useTerminalView.lifecycle.svelte.ts onFullscreenStateChanged contains activeViewportEl focus call (static check)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(
      process.cwd(),
      'src/lib/composables/useTerminalView.lifecycle.svelte.ts',
    );
    const source = fs.readFileSync(filePath, 'utf-8');

    // Locate the onFullscreenStateChanged call site (second occurrence; first is the import)
    const firstIdx = source.indexOf('onFullscreenStateChanged');
    expect(firstIdx).toBeGreaterThan(-1);
    const handlerIdx = source.indexOf('onFullscreenStateChanged', firstIdx + 1);
    expect(handlerIdx).toBeGreaterThan(-1);

    // The handler block should restore focus after setFullscreen
    const handlerBlock = source.slice(handlerIdx, handlerIdx + 400);
    expect(handlerBlock).toContain('setFullscreen');
    expect(handlerBlock).toContain('activeViewportEl');
    expect(handlerBlock).toContain('focus');
    expect(handlerBlock).toContain('preventScroll');
  });

  it('TerminalView.svelte fullscreen button onclick does NOT inline focus (delegated to event handler)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalView.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // The fullscreen button onclick must stay simple (not async, no inline focus call)
    const fullscreenBtnIdx = source.indexOf('fullscreen-toggle-btn');
    expect(fullscreenBtnIdx).toBeGreaterThan(-1);

    // Extract a window around the fullscreen button (500 chars covers the full element)
    const onclickRegion = source.slice(Math.max(0, fullscreenBtnIdx - 400), fullscreenBtnIdx + 400);
    // Must reference handleToggleFullscreen
    expect(onclickRegion).toContain('handleToggleFullscreen');
    // Must NOT contain an inline focus call (that would race with the WM)
    expect(onclickRegion).not.toContain('activeViewportEl');
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-018: Preferences panel close — Bits UI FocusScope trigger-restoration disabled
//
// Root cause: Bits UI Dialog.Content's FocusScope restores focus to the trigger
// (settings button) asynchronously after onOpenChange fires. Without preventing
// this, a synchronous focus() call is overridden by Bits UI's cleanup.
//
// The fix has two coordinated parts:
//   1. PreferencesPanel.svelte Dialog.Content has onCloseAutoFocus with
//      e.preventDefault() — disables Bits UI's trigger-restoration.
//   2. The focusin safety net in useTerminalView.core.svelte.ts recaptures
//      focus to the terminal textarea when focus lands on document.body
//      after the dialog is fully removed from the DOM.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-018: Preferences panel close disables Bits UI trigger-restoration', () => {
  it('PreferencesPanel.svelte Dialog.Content has onCloseAutoFocus with preventDefault (critical fix)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/PreferencesPanel.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // This is the exact prop that prevents Bits UI FocusScope from returning
    // focus to the settings button trigger. Without it, the bug reappears.
    expect(source).toContain('onCloseAutoFocus');
    expect(source).toContain('preventDefault');
  });

  it('TerminalView.svelte onCloseAutoFocus defers focus to next frame (avoids FocusScope race)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/components/TerminalView.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // The onCloseAutoFocus callback must use requestAnimationFrame to defer
    // focus until after Bits UI's FocusScope has fully torn down.
    const onCloseAutoFocusIdx = source.indexOf('onCloseAutoFocus');
    expect(onCloseAutoFocusIdx).toBeGreaterThan(-1);

    const handlerBlock = source.slice(onCloseAutoFocusIdx, onCloseAutoFocusIdx + 600);
    expect(handlerBlock).toContain('requestAnimationFrame');
    expect(handlerBlock).toContain('activeViewportEl');
    expect(handlerBlock).toContain('focus');
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-019: Shared Dialog.svelte has onCloseAutoFocus on all Content variants
//
// The shared Dialog component wraps Bits UI Dialog and AlertDialog. Both
// Content elements must have onCloseAutoFocus with e.preventDefault() to
// prevent Bits UI's default trigger-restoration. Without this, focus goes
// to the original trigger (often a button or body) instead of the terminal.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-019: Shared Dialog.svelte prevents Bits UI trigger-restoration', () => {
  it('Dialog.svelte Dialog.Content has onCloseAutoFocus with preventDefault', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(process.cwd(), 'src/lib/ui/Dialog.svelte');
    const source = fs.readFileSync(filePath, 'utf-8');

    // Both Dialog.Content and AlertDialog.Content must have onCloseAutoFocus
    const dialogContentIdx = source.indexOf('<Dialog.Content');
    expect(dialogContentIdx).toBeGreaterThan(-1);
    const dialogBlock = source.slice(dialogContentIdx, dialogContentIdx + 600);
    expect(dialogBlock).toContain('onCloseAutoFocus');
    expect(dialogBlock).toContain('preventDefault');

    const alertDialogContentIdx = source.indexOf('<AlertDialog.Content');
    expect(alertDialogContentIdx).toBeGreaterThan(-1);
    const alertBlock = source.slice(alertDialogContentIdx, alertDialogContentIdx + 600);
    expect(alertBlock).toContain('onCloseAutoFocus');
    expect(alertBlock).toContain('preventDefault');
  });
});

// ---------------------------------------------------------------------------
// TEST-FOCUS-020: onFocusIn safety net deferred re-check architecture
//
// When focus lands on document.body while a modal dialog is still in the DOM
// (typical during Bits UI dialog close), the safety net must NOT silently
// discard the event. Instead, it must schedule a deferred re-check
// (requestAnimationFrame) that recaptures focus once the dialog is fully
// removed. Without this, closing any dialog leaves focus on body — the user
// cannot type until they manually click the terminal or toggle another UI
// element.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-020: onFocusIn deferred re-check prevents focus loss on dialog close', () => {
  it('onFocusIn defers to requestAnimationFrame when modal is present (not early return)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(
      process.cwd(),
      'src/lib/composables/useTerminalView.core.svelte.ts',
    );
    const source = fs.readFileSync(filePath, 'utf-8');

    const onFocusInIdx = source.indexOf('function onFocusIn');
    expect(onFocusInIdx).toBeGreaterThan(-1);

    const onFocusInBlock = source.slice(onFocusInIdx, onFocusInIdx + 600);

    // Must use requestAnimationFrame when dialog is present (not just return)
    expect(onFocusInBlock).toContain('requestAnimationFrame');

    // Must call the refocusTerminal function (extracted for reuse)
    expect(onFocusInBlock).toContain('refocusTerminal');

    // The refocusTerminal function must check activeElement is still body
    // before focusing — another dialog may have opened in the meantime
    const refocusIdx = source.indexOf('function refocusTerminal');
    expect(refocusIdx).toBeGreaterThan(-1);
    const refocusBlock = source.slice(refocusIdx, refocusIdx + 400);
    expect(refocusBlock).toContain('document.activeElement');
    expect(refocusBlock).toContain('document.body');
    expect(refocusBlock).toContain('activeViewportEl');
    expect(refocusBlock).toContain('focus');
  });
});
