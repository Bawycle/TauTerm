// SPDX-License-Identifier: MPL-2.0

/**
 * E2E: Tab key and space regressions — WebKitGTK-specific input issues.
 *
 * Two independent regressions affected Tab (shell completion) and space:
 *
 * 1. tabindex regression (commit 0a713b7): the hidden textarea had
 *    tabindex={active ? 0 : -1}. With tabindex>=0, WebKit treated it as a
 *    sequential focus tab-stop and consumed Tab for focus navigation BEFORE
 *    keydown fired — silently losing Tab and any input typed after focus moved.
 *    Fix: tabindex={-1} always. Focus is programmatic only.
 *
 * 2. NBSP regression (WebKitGTK textarea behaviour): WebKitGTK inserts
 *    U+00A0 (NO-BREAK SPACE) instead of U+0020 when the user presses Space
 *    in a <textarea>. The PTY received [0xC2, 0xA0] (UTF-8 NBSP) instead of
 *    [0x20]. The shell treated NBSP as a literal character, not a word
 *    separator — breaking both "ls foo" (command not found) and Tab completion
 *    ("ls\xa0Docum" parsed as one token).
 *    Fix: handleInput normalizes U+00A0 → U+0020 before sending to the PTY.
 *
 * 3. Capture phase: Svelte's onkeydown uses bubble phase, but WebKitGTK
 *    performs Tab's default action before bubble handlers run. handleKeydown
 *    is now attached via addEventListener({ capture: true }) so preventDefault
 *    fires before any default action.
 *
 * Why these are E2E-only:
 *   jsdom does not implement WebKit tab-stop focus navigation or the NBSP
 *   textarea insertion. Unit tests dispatch synthetic events that always reach
 *   handlers regardless of tabindex, and can simulate NBSP directly. Only
 *   running in real WebKitGTK validates the end-to-end fix.
 *
 * Scenarios:
 *   TEST-E2E-TAB-001 — textarea tabindex is -1 in the live DOM (tabindex guard)
 *   TEST-E2E-TAB-002 — Tab keydown keeps focus on the terminal input
 *   TEST-E2E-TAB-003 — keystrokes typed after Tab are still received
 *   TEST-E2E-NBSP-001 — NBSP from textarea is normalized to ASCII space [0x20]
 */

import { browser } from '@wdio/globals';
import { Selectors } from './helpers/selectors';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** True iff document.activeElement is the hidden textarea input sink. */
function isInputFocused(): Promise<boolean> {
  return browser.execute((): boolean => {
    const el = document.activeElement;
    return el !== null && el.classList.contains('terminal-pane__input');
  }) as Promise<boolean>;
}

/** Focus the terminal input programmatically (mirrors what mousedown does). */
async function focusTerminalInput(): Promise<void> {
  await browser.execute((): void => {
    const el = document.querySelector<HTMLElement>('.terminal-pane__input');
    el?.focus();
  });
  await browser.waitUntil(isInputFocused, {
    timeout: 2_000,
    interval: 50,
    timeoutMsg: 'terminal-pane__input did not receive focus',
  });
}

/** Dispatch a synthetic keydown on the focused terminal input. */
async function dispatchKey(key: string, opts: { shiftKey?: boolean } = {}): Promise<void> {
  await browser.execute(
    (keyArg: string, shiftArg: boolean): void => {
      const el = document.querySelector<HTMLElement>('.terminal-pane__input');
      el?.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: keyArg,
          shiftKey: shiftArg,
          bubbles: true,
          cancelable: true,
        }),
      );
    },
    key,
    opts.shiftKey ?? false,
  );
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('TauTerm — Tab key not consumed by WebKit (keyboard-tab regression)', () => {
  before(async () => {
    // Ensure the app is in a clean state with the terminal focused.
    await browser.waitUntil(
      async () => {
        try {
          const el = await browser.$(Selectors.activeTerminalPane);
          return await el.isExisting();
        } catch {
          return false;
        }
      },
      { timeout: 15_000, timeoutMsg: 'Active terminal pane did not appear' },
    );
    await focusTerminalInput();
  });

  // -------------------------------------------------------------------------
  // TEST-E2E-TAB-001: tabindex=-1 in the live DOM
  //
  // Direct regression guard: if tabindex is ever changed back to 0 (or
  // active ? 0 : -1), this test fails immediately — no need to exercise
  // WebKit's tab-stop navigation path.
  // -------------------------------------------------------------------------
  it('TEST-E2E-TAB-001: hidden textarea has tabindex=-1 in the live DOM', async () => {
    const tabIndex = await browser.execute((): number => {
      const el = document.querySelector<HTMLElement>('.terminal-pane__input');
      // tabIndex property returns -1 for both attribute "-1" and missing attribute.
      // We check the attribute directly to distinguish "intentionally -1" from "absent".
      const attr = el?.getAttribute('tabindex');
      return attr !== null ? parseInt(attr, 10) : -999;
    });

    expect(tabIndex).toBe(-1);
  });

  // -------------------------------------------------------------------------
  // TEST-E2E-TAB-002: Tab keydown keeps focus on the terminal input
  //
  // Verifies that handleKeydown calls event.preventDefault() for Tab in real
  // WebKit, preventing focus navigation even when a synthetic keydown is used.
  // With the regression (tabindex=0 + native Tab), focus would silently move
  // before keydown fires — this test guards the preventDefault() half.
  // -------------------------------------------------------------------------
  it('TEST-E2E-TAB-002: Tab keydown does not move focus away from the terminal', async () => {
    await focusTerminalInput();

    await dispatchKey('Tab');
    await browser.pause(100);

    expect(await isInputFocused()).toBe(true);
  });

  // -------------------------------------------------------------------------
  // TEST-E2E-TAB-003: keystrokes after Tab are still received
  //
  // Regression: after Tab moved focus away (tabindex=0 case), any keys typed
  // afterwards were silently lost. This test verifies that the terminal input
  // retains focus after Tab + subsequent keystrokes.
  // -------------------------------------------------------------------------
  it('TEST-E2E-TAB-003: terminal input retains focus after Tab followed by other keystrokes', async () => {
    await focusTerminalInput();

    await dispatchKey('Tab');
    await browser.pause(50);
    await dispatchKey('a');
    await dispatchKey('b');

    expect(await isInputFocused()).toBe(true);
  });

  // -------------------------------------------------------------------------
  // TEST-E2E-NBSP-001: NBSP from textarea normalized to ASCII space
  //
  // WebKitGTK inserts U+00A0 (NBSP) instead of U+0020 when the user presses
  // Space in a <textarea>. handleInput must normalize it to 0x20 before
  // sending to the PTY, otherwise the shell receives a literal NBSP which is
  // not a word separator — breaking command parsing and Tab completion.
  //
  // Strategy: spy on TextEncoder.prototype.encode to capture the string that
  // handleInput passes after NBSP normalization. sendBytes calls
  // encoder.encode(char) synchronously within the dispatchEvent call, so the
  // spy captures the value before the microtask for IPC fires.
  // The unit test (TEST-TP-INPUT-NBSP) verifies the byte-level output;
  // this E2E test confirms the pipeline works end-to-end in real WebKitGTK.
  // -------------------------------------------------------------------------
  it('TEST-E2E-NBSP-001: NBSP (U+00A0) from textarea is normalized before encoding', async () => {
    await focusTerminalInput();

    // Single browser.execute: install TextEncoder spy, dispatch NBSP, read
    // the captured string, and restore — all synchronously within one call.
    // handleInput runs synchronously inside dispatchEvent, and calls
    // encoder.encode(char) synchronously, so the spy captures the value
    // before this execute returns.
    const result = (await browser.execute((): {
      textareaCleared: boolean;
      encodedInput: string | null;
    } => {
      const el = document.querySelector<HTMLTextAreaElement>('.terminal-pane__input');
      if (!el) return { textareaCleared: false, encodedInput: null };

      // Spy on TextEncoder.encode to capture the string passed for the space.
      let captured: string | null = null;
      const origEncode = TextEncoder.prototype.encode;
      TextEncoder.prototype.encode = function (input?: string): Uint8Array {
        // Record single-char inputs that are space or NBSP — the character
        // handleInput passes to encoder.encode() after normalization.
        if (input !== undefined && input.length === 1) {
          const cp = input.codePointAt(0);
          if (cp === 0x20 || cp === 0xa0) {
            captured = input;
          }
        }
        return origEncode.call(this, input);
      };

      // Dispatch NBSP as WebKitGTK would.
      el.value = '\u00a0';
      el.dispatchEvent(new InputEvent('input', { data: '\u00a0', bubbles: true }));
      const textareaCleared = el.value === '';

      // Restore immediately.
      TextEncoder.prototype.encode = origEncode;

      return { textareaCleared, encodedInput: captured };
    })) as { textareaCleared: boolean; encodedInput: string | null };

    // handleInput must have run (textarea value drained).
    expect(result.textareaCleared).toBe(true);

    // The string passed to TextEncoder.encode must be regular space (U+0020),
    // NOT the original NBSP (U+00A0).
    expect(result.encodedInput).not.toBeNull();
    expect(result.encodedInput).toBe(' ');             // U+0020
    expect(result.encodedInput).not.toBe('\u00a0');    // NOT U+00A0
  });
});
