// SPDX-License-Identifier: MPL-2.0
/**
 * TerminalPane.altgr.test.ts
 *
 * Tests for input handling in TerminalPane.svelte: AltGr guards, GTK IM
 * composition (dead keys), and Tab key regression.
 *
 * Covered:
 *   TEST-TP-ALTGR-001–004 — AltGr / Ctrl+Shift guards in handleKeydown
 *   TEST-TP-INPUT-001–003  — GTK IM commit via textarea input event
 *   TEST-TP-INPUT-NBSP     — WebKitGTK NBSP normalization
 *   TEST-TP-INPUT-004      — bare printable keydown bypassed to handleInput
 *   TEST-TP-INPUT-005–011  — dead key composition lifecycle (isComposing,
 *                            compositionend, double-send guard, cancelled
 *                            composition, double dead key, incompatible char)
 *   TEST-TP-INPUT-012      — REGRESSION: space after composition (no trailing input)
 *   TEST-TP-INPUT-013      — trailing input matching composed text suppressed
 *   TEST-TP-INPUT-014      — trailing input with different text is sent
 *   TEST-TP-TAB-001–003    — Tab/Shift+Tab key regression
 *
 * Implementation note:
 *   The IPC path is: handleKeydown → tp.sendBytes → sendInput (from $lib/ipc/commands).
 *   vi.spyOn is applied to $lib/ipc/commands.sendInput — the live binding inside the
 *   composable — rather than @tauri-apps/api/core.invoke, which is captured at module
 *   import time and cannot be intercepted via spyOn after the fact.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import TerminalPane from '$lib/components/TerminalPane.svelte';
import * as tauriEvent from '@tauri-apps/api/event';
import * as ipcCommands from '$lib/ipc';

// Stub ResizeObserver (not available in jsdom)
if (!globalThis.ResizeObserver) {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

/** Build a KeyboardEvent with optional AltGraph modifier state. */
function makeKeyEvent(
  key: string,
  opts: {
    ctrlKey?: boolean;
    altKey?: boolean;
    shiftKey?: boolean;
    altGraph?: boolean;
  } = {},
): KeyboardEvent {
  const event = new KeyboardEvent('keydown', {
    key,
    ctrlKey: opts.ctrlKey ?? false,
    altKey: opts.altKey ?? false,
    shiftKey: opts.shiftKey ?? false,
    bubbles: true,
    cancelable: true,
  });
  const altGraph = opts.altGraph ?? false;
  Object.defineProperty(event, 'getModifierState', {
    value: (state: string) => (state === 'AltGraph' ? altGraph : false),
    configurable: true,
  });
  return event;
}

describe('TerminalPane.svelte handleKeydown AltGr guards', () => {
  let container: HTMLDivElement;
  let instance: ReturnType<typeof mount> | null = null;
  let sendInputSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    vi.spyOn(tauriEvent, 'listen').mockResolvedValue(() => {});
    // Stub IPC init commands to avoid backend dependency
    vi.spyOn(ipcCommands, 'getPaneScreenSnapshot').mockResolvedValue(null as unknown as never);
    vi.spyOn(ipcCommands, 'setActivePane').mockResolvedValue(undefined);
    sendInputSpy = vi.spyOn(ipcCommands, 'sendInput').mockResolvedValue(undefined);
  });

  afterEach(async () => {
    if (instance) {
      unmount(instance);
      instance = null;
    }
    container.remove();
    vi.restoreAllMocks();
  });

  async function mountPane() {
    instance = mount(TerminalPane, {
      target: container,
      props: {
        paneId: 'pane-altgr-test',
        tabId: 'tab-altgr-test',
        active: true,
      },
    });
    // Drain effects
    for (let i = 0; i < 30; i++) await Promise.resolve();
    flushSync();
    for (let i = 0; i < 10; i++) await Promise.resolve();
    flushSync();
    // keydown is now bound to the hidden textarea (GTK IM input sink), not the viewport div.
    return container.querySelector('.terminal-pane__input') as HTMLElement | null;
  }

  it('TEST-TP-ALTGR-001: AltGr+Shift does NOT block — PTY receives bytes', async () => {
    const viewport = await mountPane();
    expect(viewport).not.toBeNull();

    sendInputSpy.mockClear();

    // AltGr+Shift+~ : ctrlKey+altKey+shiftKey with AltGraph active.
    // keyboard.ts AltGr branch: ctrlKey && altKey && getModifierState('AltGraph') && key.length === 1
    // → returns encode('~'). The guard in handleKeydown must not return early.
    const event = makeKeyEvent('~', {
      ctrlKey: true,
      altKey: true,
      shiftKey: true,
      altGraph: true,
    });
    viewport!.dispatchEvent(event);
    await Promise.resolve();
    flushSync();

    // sendInput should have been called (guard did not return early)
    expect(sendInputSpy).toHaveBeenCalled();
  });

  it('TEST-TP-ALTGR-002: Ctrl+Shift (no AltGraph) blocks PTY transmission', async () => {
    const viewport = await mountPane();
    expect(viewport).not.toBeNull();

    sendInputSpy.mockClear();

    // Genuine Ctrl+Shift+T (application shortcut) — must NOT reach PTY
    const event = makeKeyEvent('T', { ctrlKey: true, shiftKey: true, altGraph: false });
    viewport!.dispatchEvent(event);
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).not.toHaveBeenCalled();
  });

  it('TEST-TP-ALTGR-003: AltGr+comma does NOT block', async () => {
    const viewport = await mountPane();
    expect(viewport).not.toBeNull();

    sendInputSpy.mockClear();

    // AltGr+, — the Ctrl+, preferences guard must not fire when AltGraph is active
    const event = makeKeyEvent(',', { ctrlKey: true, altKey: true, altGraph: true });
    viewport!.dispatchEvent(event);
    await Promise.resolve();
    flushSync();

    // keyboard.ts AltGr branch: ',' has cp=0x2C >= 0x20 and !== 0x7F → encode(',')
    expect(sendInputSpy).toHaveBeenCalled();
  });

  it('TEST-TP-ALTGR-004: Ctrl+comma (no AltGraph) blocks (preferences shortcut)', async () => {
    const viewport = await mountPane();
    expect(viewport).not.toBeNull();

    sendInputSpy.mockClear();

    // Genuine Ctrl+, — preferences shortcut, must NOT reach PTY
    const event = makeKeyEvent(',', { ctrlKey: true, altGraph: false });
    viewport!.dispatchEvent(event);
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// handleInput — GTK IM / dead key / AltGr characters via textarea input event
// ---------------------------------------------------------------------------

describe('TerminalPane.svelte handleInput (textarea input event)', () => {
  let container: HTMLDivElement;
  let instance: ReturnType<typeof mount> | null = null;
  let sendInputSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    vi.spyOn(tauriEvent, 'listen').mockResolvedValue(() => {});
    vi.spyOn(ipcCommands, 'getPaneScreenSnapshot').mockResolvedValue(null as unknown as never);
    vi.spyOn(ipcCommands, 'setActivePane').mockResolvedValue(undefined);
    sendInputSpy = vi.spyOn(ipcCommands, 'sendInput').mockResolvedValue(undefined);
  });

  afterEach(async () => {
    if (instance) {
      unmount(instance);
      instance = null;
    }
    container.remove();
    vi.restoreAllMocks();
  });

  async function mountPaneAndGetInput() {
    instance = mount(TerminalPane, {
      target: container,
      props: { paneId: 'pane-input-test', tabId: 'tab-input-test', active: true },
    });
    for (let i = 0; i < 30; i++) await Promise.resolve();
    flushSync();
    for (let i = 0; i < 10; i++) await Promise.resolve();
    flushSync();
    return container.querySelector('.terminal-pane__input') as HTMLTextAreaElement | null;
  }

  function dispatchInput(inputEl: HTMLTextAreaElement, data: string | null) {
    // Simulate a GTK IM commit: set textarea.value then fire an input event.
    if (data !== null) inputEl.value = data;
    inputEl.dispatchEvent(new InputEvent('input', { data, bubbles: true, cancelable: true }));
  }

  // TEST-TP-INPUT-001: dead_tilde + space → '~' committed via GTK IM
  it('TEST-TP-INPUT-001: input event data="~" sends 0x7E to PTY', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    dispatchInput(inputEl!, '~');
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).toHaveBeenCalled();
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    expect(Array.from(bytes)).toEqual([0x7e]);
  });

  // TEST-TP-INPUT-002: euro sign (AltGr+E on some layouts) — multi-byte UTF-8
  it('TEST-TP-INPUT-002: input event data="€" sends UTF-8 E2 82 AC to PTY', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    dispatchInput(inputEl!, '€');
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).toHaveBeenCalled();
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    expect(Array.from(bytes)).toEqual([0xe2, 0x82, 0xac]);
  });

  // TEST-TP-INPUT-003: null data (e.g. deleteContentBackward) → nothing sent
  it('TEST-TP-INPUT-003: input event with data=null sends nothing to PTY', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    dispatchInput(inputEl!, null);
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).not.toHaveBeenCalled();
  });

  // TEST-TP-INPUT-NBSP: WebKitGTK textarea inserts NBSP (U+00A0) instead of regular space.
  // The terminal must normalize it to ASCII space (0x20) so the shell receives a proper
  // word separator. Without this, "ls foo" becomes "ls<NBSP>foo" → command not found.
  it('TEST-TP-INPUT-NBSP: NBSP (U+00A0) from textarea is normalized to ASCII space (0x20)', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // Simulate WebKitGTK behaviour: textarea.value contains NBSP
    dispatchInput(inputEl!, '\u00a0');
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).toHaveBeenCalled();
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    // Must be [0x20] (regular space), NOT [0xC2, 0xA0] (UTF-8 NBSP)
    expect(Array.from(bytes)).toEqual([0x20]);
  });

  // TEST-TP-INPUT-004: bare printable keydown is skipped — PTY receives nothing via keydown
  it('TEST-TP-INPUT-004: keydown key="a" (bare printable) is skipped — no PTY bytes via keydown', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    inputEl!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'a', bubbles: true, cancelable: true }),
    );
    await Promise.resolve();
    flushSync();

    // PTY must NOT receive bytes from keydown — character comes via input event instead
    expect(sendInputSpy).not.toHaveBeenCalled();
  });

  // TEST-TP-INPUT-005: input during composition (isComposing=true) is ignored.
  // Dead key ^ → input fires with pre-edit "^" and isComposing=true.
  // This must NOT be sent to the PTY — the composed character will arrive later.
  it('TEST-TP-INPUT-005: input event with isComposing=true is ignored (dead key pre-edit)', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // Simulate dead key pre-edit: IM writes "^" to textarea, fires input with isComposing=true
    inputEl!.value = '^';
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: '^', isComposing: true, bubbles: true, cancelable: true }),
    );
    await Promise.resolve();
    flushSync();

    // Pre-edit text must NOT be sent to PTY
    expect(sendInputSpy).not.toHaveBeenCalled();
    // Textarea value must NOT be cleared (IM needs it for composition)
    expect(inputEl!.value).toBe('^');
  });

  // TEST-TP-INPUT-006: compositionend delivers the final composed character.
  // After dead ^ + i, compositionend fires with data="î". This must be sent to PTY.
  it('TEST-TP-INPUT-006: compositionend data="î" sends composed character to PTY', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // Simulate compositionend: IM commits "î" and textarea holds it
    inputEl!.value = 'î';
    inputEl!.dispatchEvent(
      new CompositionEvent('compositionend', { data: 'î', bubbles: true, cancelable: true }),
    );
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).toHaveBeenCalled();
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    // î = U+00EE → UTF-8: [0xC3, 0xAE]
    expect(Array.from(bytes)).toEqual([0xc3, 0xae]);
    // Textarea must be drained after compositionend
    expect(inputEl!.value).toBe('');
  });

  // TEST-TP-INPUT-007: full dead key sequence (^ then i) produces only "î", not "^î".
  // Simulates the complete sequence: compositionstart → input(isComposing) → compositionend.
  // Also verifies that a space typed after the composition is NOT swallowed (regression guard).
  it('TEST-TP-INPUT-007: dead ^ + i → only "î" sent (no spurious "^"), then space goes through', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // Step 1: compositionstart
    inputEl!.dispatchEvent(new CompositionEvent('compositionstart', { data: '', bubbles: true }));

    // Step 2: input with isComposing=true (dead key pre-edit)
    inputEl!.value = '^';
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: '^', isComposing: true, bubbles: true }),
    );
    await Promise.resolve();
    flushSync();

    // Step 3: compositionend with composed character
    inputEl!.value = 'î';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: 'î', bubbles: true }));
    // Flush microtask (clears compositionEndData if no trailing input)
    await Promise.resolve();
    flushSync();

    // Step 4: space after composition — must NOT be swallowed (REGRESSION GUARD)
    dispatchInput(inputEl!, '\u00a0'); // WebKitGTK NBSP
    await Promise.resolve();
    flushSync();

    // 2 calls: "î" from compositionend + space (NBSP normalized to 0x20)
    expect(sendInputSpy).toHaveBeenCalledTimes(2);
    expect(Array.from(sendInputSpy.mock.calls[0][1] as Uint8Array)).toEqual([0xc3, 0xae]); // î
    expect(Array.from(sendInputSpy.mock.calls[1][1] as Uint8Array)).toEqual([0x20]); // space
  });

  // TEST-TP-INPUT-008: compositionend followed by input(isComposing=false) → no double-send.
  // Some GTK IM backends emit a post-composition input event with isComposing=false and
  // event.data set to the composed character. compositionEndData (set by compositionend)
  // must prevent handleInput from re-sending the character. Both events are dispatched
  // synchronously (same browser task) — no await between them.
  it('TEST-TP-INPUT-008: post-composition input(isComposing=false) does not double-send', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // compositionend + trailing input fire synchronously in the same browser task
    inputEl!.value = 'î';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: 'î', bubbles: true }));
    // Trailing input — dispatched synchronously (same task, no await between)
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: 'î', isComposing: false, bubbles: true }),
    );
    await Promise.resolve();
    flushSync();

    // compositionend sent once; trailing input suppressed by NFC content match
    expect(sendInputSpy).toHaveBeenCalledTimes(1);
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    expect(Array.from(bytes)).toEqual([0xc3, 0xae]);
  });

  // TEST-TP-INPUT-009: cancelled composition (Escape) with empty data → nothing sent.
  // When the user cancels a dead key composition, compositionend fires with data="".
  it('TEST-TP-INPUT-009: cancelled composition (compositionend data="") sends nothing', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // compositionstart + pre-edit
    inputEl!.dispatchEvent(new CompositionEvent('compositionstart', { data: '', bubbles: true }));
    inputEl!.value = '^';
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: '^', isComposing: true, bubbles: true }),
    );
    await Promise.resolve();
    flushSync();

    // Escape cancels → compositionend with empty data, IM clears textarea
    inputEl!.value = '';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: '', bubbles: true }));
    await Promise.resolve();
    flushSync();

    // Nothing should be sent — neither the pre-edit "^" nor the empty string
    expect(sendInputSpy).not.toHaveBeenCalled();
  });

  // TEST-TP-INPUT-010: double dead key (^ + ^ → ^) sends the literal character.
  // On Belgian/French keyboards, pressing the dead key twice produces the accent as a
  // literal character. The IM commits "^" via compositionend.
  it('TEST-TP-INPUT-010: double dead key (^ + ^) sends literal "^" (0x5E)', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // compositionstart + pre-edit for first ^
    inputEl!.dispatchEvent(new CompositionEvent('compositionstart', { data: '', bubbles: true }));
    inputEl!.value = '^';
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: '^', isComposing: true, bubbles: true }),
    );
    await Promise.resolve();
    flushSync();

    // Second ^ commits the literal "^"
    inputEl!.value = '^';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: '^', bubbles: true }));
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).toHaveBeenCalledTimes(1);
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    expect(Array.from(bytes)).toEqual([0x5e]);
  });

  // TEST-TP-INPUT-011: dead key + incompatible char (^ + z → "^z") sends two characters.
  // When the base character is not composable with the dead key, the IM commits both
  // the accent and the character as separate codepoints in a single compositionend.
  it('TEST-TP-INPUT-011: dead ^ + z (incompatible) sends "^z" [0x5E, 0x7A]', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // compositionstart + pre-edit
    inputEl!.dispatchEvent(new CompositionEvent('compositionstart', { data: '', bubbles: true }));
    inputEl!.value = '^';
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: '^', isComposing: true, bubbles: true }),
    );
    await Promise.resolve();
    flushSync();

    // z is incompatible → IM commits "^z" (two chars)
    inputEl!.value = '^z';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: '^z', bubbles: true }));
    await Promise.resolve();
    flushSync();

    // sendBytes is called once per character by sendPrintableText
    expect(sendInputSpy).toHaveBeenCalledTimes(2);
    const bytes0: Uint8Array = sendInputSpy.mock.calls[0][1];
    const bytes1: Uint8Array = sendInputSpy.mock.calls[1][1];
    expect(Array.from(bytes0)).toEqual([0x5e]); // ^
    expect(Array.from(bytes1)).toEqual([0x7a]); // z
  });

  // TEST-TP-INPUT-012: REGRESSION — dead key composition followed by NO trailing
  // input, then space keystroke. The space must NOT be swallowed.
  // Root cause (fixed): compositionJustEnded (boolean) was never cleared when the
  // GTK IM backend omitted the trailing input event. The fix uses compositionEndData
  // (string) + queueMicrotask auto-clearing.
  it('TEST-TP-INPUT-012: space after compositionend (no trailing input) is NOT swallowed', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // Step 1: compositionend for "à" — no trailing input (GTK dead-key compositor path)
    inputEl!.value = 'à';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: 'à', bubbles: true }));
    // NO trailing input dispatched — simulates the missing event

    // Step 2: flush microtask → compositionEndData cleared by queueMicrotask
    await Promise.resolve();
    flushSync();

    // Step 3: space in a new browser task (after microtask cleared the guard)
    dispatchInput(inputEl!, '\u00a0'); // WebKitGTK NBSP
    await Promise.resolve();
    flushSync();

    // 2 calls: à from compositionend + space (NBSP normalized)
    expect(sendInputSpy).toHaveBeenCalledTimes(2);
    expect(Array.from(sendInputSpy.mock.calls[0][1] as Uint8Array)).toEqual([0xc3, 0xa0]); // à
    expect(Array.from(sendInputSpy.mock.calls[1][1] as Uint8Array)).toEqual([0x20]); // space
  });

  // TEST-TP-INPUT-013: compositionend("à") followed immediately by trailing
  // input("à") — the trailing input must be suppressed (duplicate prevention).
  // Both events dispatched synchronously (same browser task).
  it('TEST-TP-INPUT-013: trailing input matching composed text is suppressed', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // Both dispatched synchronously (same browser task — no await between)
    inputEl!.value = 'à';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: 'à', bubbles: true }));
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: 'à', isComposing: false, bubbles: true }),
    );
    await Promise.resolve();
    flushSync();

    // Only 1 call — compositionend sent à; trailing input suppressed
    expect(sendInputSpy).toHaveBeenCalledTimes(1);
    expect(Array.from(sendInputSpy.mock.calls[0][1] as Uint8Array)).toEqual([0xc3, 0xa0]); // à
  });

  // TEST-TP-INPUT-014: compositionend("à") followed by trailing input with
  // DIFFERENT text — the different text must be sent (not suppressed).
  it('TEST-TP-INPUT-014: trailing input with different text from composed is sent', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    // Both dispatched synchronously
    inputEl!.value = 'à';
    inputEl!.dispatchEvent(new CompositionEvent('compositionend', { data: 'à', bubbles: true }));
    inputEl!.value = 'b';
    inputEl!.dispatchEvent(
      new InputEvent('input', { data: 'b', isComposing: false, bubbles: true }),
    );
    await Promise.resolve();
    flushSync();

    // 2 calls: à from compositionend + b from trailing input (text mismatch → sent)
    expect(sendInputSpy).toHaveBeenCalledTimes(2);
    expect(Array.from(sendInputSpy.mock.calls[0][1] as Uint8Array)).toEqual([0xc3, 0xa0]); // à
    expect(Array.from(sendInputSpy.mock.calls[1][1] as Uint8Array)).toEqual([0x62]); // b
  });
});

// ---------------------------------------------------------------------------
// Tab key regression (TEST-TP-TAB)
//
// Two independent regressions broke Tab (shell completion) and space:
//
// 1. tabindex: commit 0a713b7 introduced `tabindex={active ? 0 : -1}` on the
//    hidden textarea. With tabindex>=0, WebKit treats the textarea as a
//    sequential focus tab-stop and can consume Tab for focus navigation before
//    keydown fires in JavaScript.
//    Fix: tabindex={-1} always. Focus is always programmatic.
//
// 2. NBSP (root cause of "command not found" after space): WebKitGTK inserts
//    U+00A0 (NO-BREAK SPACE) instead of U+0020 in textarea elements. The
//    shell received [0xC2, 0xA0] (UTF-8 NBSP) instead of [0x20], treating it
//    as a literal character, not a word separator. This broke both space-after-
//    command ("ls " → command not found) and Tab completion (the shell saw one
//    token "ls\xa0Docum" instead of "ls" + "Docum").
//    Fix: handleInput normalizes U+00A0 → U+0020 (see TEST-TP-INPUT-NBSP).
//
// 3. Capture phase: Svelte's onkeydown={handler} uses bubble phase, but
//    WebKitGTK performs Tab's default action (sequential focus navigation)
//    before bubble-phase handlers run. handleKeydown is now attached via
//    addEventListener with { capture: true } so preventDefault() fires before
//    any default action.
// ---------------------------------------------------------------------------

describe('TerminalPane.svelte Tab key regression (TEST-TP-TAB)', () => {
  let container: HTMLDivElement;
  let instance: ReturnType<typeof mount> | null = null;
  let sendInputSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    vi.spyOn(tauriEvent, 'listen').mockResolvedValue(() => {});
    vi.spyOn(ipcCommands, 'getPaneScreenSnapshot').mockResolvedValue(null as unknown as never);
    vi.spyOn(ipcCommands, 'setActivePane').mockResolvedValue(undefined);
    sendInputSpy = vi.spyOn(ipcCommands, 'sendInput').mockResolvedValue(undefined);
  });

  afterEach(async () => {
    if (instance) {
      unmount(instance);
      instance = null;
    }
    container.remove();
    vi.restoreAllMocks();
  });

  async function mountPaneAndGetInput() {
    instance = mount(TerminalPane, {
      target: container,
      props: { paneId: 'pane-tab-test', tabId: 'tab-tab-test', active: true },
    });
    for (let i = 0; i < 30; i++) await Promise.resolve();
    flushSync();
    for (let i = 0; i < 10; i++) await Promise.resolve();
    flushSync();
    return container.querySelector('.terminal-pane__input') as HTMLTextAreaElement | null;
  }

  // TEST-TP-TAB-001: Tab → PTY receives HT (0x09)
  it('TEST-TP-TAB-001: Tab keydown sends 0x09 to PTY', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    inputEl!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }),
    );
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).toHaveBeenCalled();
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    expect(Array.from(bytes)).toEqual([0x09]);
  });

  // TEST-TP-TAB-002: Shift+Tab → PTY receives CSI Z (backtab)
  it('TEST-TP-TAB-002: Shift+Tab keydown sends CSI Z (0x1B 0x5B 0x5A) to PTY', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();
    sendInputSpy.mockClear();

    inputEl!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true, cancelable: true }),
    );
    await Promise.resolve();
    flushSync();

    expect(sendInputSpy).toHaveBeenCalled();
    const bytes: Uint8Array = sendInputSpy.mock.calls[0][1];
    expect(Array.from(bytes)).toEqual([0x1b, 0x5b, 0x5a]);
  });

  // TEST-TP-TAB-003: Tab keydown must call preventDefault() — this is what prevents
  // WebKit from consuming Tab for focus navigation when tabindex was non-negative.
  it('TEST-TP-TAB-003: Tab keydown calls event.preventDefault()', async () => {
    const inputEl = await mountPaneAndGetInput();
    expect(inputEl).not.toBeNull();

    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true });
    inputEl!.dispatchEvent(event);
    await Promise.resolve();
    flushSync();

    expect(event.defaultPrevented).toBe(true);
  });
});
