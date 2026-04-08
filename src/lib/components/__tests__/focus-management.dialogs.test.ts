// SPDX-License-Identifier: MPL-2.0

/**
 * Focus management — dialog focus restoration tests.
 *
 * Covered:
 *   TEST-FOCUS-015 — ConnectionManager onclose focus (static check)
 *   TEST-FOCUS-016 — fullscreen toggle focus
 *   TEST-FOCUS-018 — Preferences panel Dialog.Content focus (static check)
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
// (with modal guard). Mounting TerminalView in JSDOM requires prohibitive scaffolding
// so the source file is the authoritative check.
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-015: SSH panel onclose restores focus to activeViewportEl', () => {
  it('TerminalView.svelte ConnectionManager onclose contains activeViewportEl focus call (static check)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(
      process.cwd(),
      'src/lib/components/TerminalView.svelte',
    );
    const source = fs.readFileSync(filePath, 'utf-8');

    // Find the ConnectionManager onclose block
    const oncloseIdx = source.indexOf('connectionManagerOpen = false');
    expect(oncloseIdx).toBeGreaterThan(-1);

    // The surrounding onclose block should restore focus to activeViewportEl
    const oncloseBlock = source.slice(Math.max(0, oncloseIdx - 100), oncloseIdx + 300);
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
    const filePath = path.resolve(
      process.cwd(),
      'src/lib/components/TerminalView.svelte',
    );
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
// Root cause of the bug: Bits UI Dialog.Content's FocusScope restores focus to the
// trigger (settings button) asynchronously after onOpenChange fires. A synchronous
// focus() call in onclose was overridden by this cleanup.
//
// The fix has two parts that must both be present:
//   1. Dialog.Content must carry onCloseAutoFocus={(e) => e.preventDefault()} to
//      disable Bits UI's trigger-restoration. This is the critical property — its
//      absence is what caused the bug.
//   2. TerminalView.svelte onclose must call activeViewportEl.focus() to restore
//      focus to the terminal (now that Bits UI won't fight us).
// ---------------------------------------------------------------------------

describe('TEST-FOCUS-018: Preferences panel close disables Bits UI trigger-restoration', () => {
  it('PreferencesPanel.svelte Dialog.Content has onCloseAutoFocus with preventDefault (critical fix)', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(
      process.cwd(),
      'src/lib/components/PreferencesPanel.svelte',
    );
    const source = fs.readFileSync(filePath, 'utf-8');

    // This is the exact prop that prevents Bits UI FocusScope from returning
    // focus to the settings button trigger. Without it, the bug reappears.
    expect(source).toContain('onCloseAutoFocus');
    expect(source).toContain('preventDefault');
  });

  it('TerminalView.svelte onCloseAutoFocus prop restores focus to activeViewportEl', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const filePath = path.resolve(
      process.cwd(),
      'src/lib/components/TerminalView.svelte',
    );
    const source = fs.readFileSync(filePath, 'utf-8');

    // The focus restoration is in onCloseAutoFocus, not onclose, because
    // onCloseAutoFocus fires at the exact right moment in the Bits UI lifecycle
    // (when FocusScope would restore focus to the trigger). At that point the
    // dialog is still in the DOM, so no modal guard is used.
    const onCloseAutoFocusIdx = source.indexOf('onCloseAutoFocus');
    expect(onCloseAutoFocusIdx).toBeGreaterThan(-1);

    const handlerBlock = source.slice(onCloseAutoFocusIdx, onCloseAutoFocusIdx + 500);
    expect(handlerBlock).toContain('activeViewportEl');
    expect(handlerBlock).toContain('focus');
  });
});
