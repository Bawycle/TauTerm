// SPDX-License-Identifier: MPL-2.0
/**
 * TerminalPane.altgr.test.ts
 *
 * Tests for AltGr guard fixes in TerminalPane.svelte handleKeydown.
 *
 * Covered:
 *   TEST-TP-ALTGR-001 — AltGr+Shift: guard does NOT block, PTY receives bytes
 *   TEST-TP-ALTGR-002 — Ctrl+Shift (no AltGraph): guard blocks, PTY receives nothing
 *   TEST-TP-ALTGR-003 — AltGr+, (AltGraph): guard does NOT block
 *   TEST-TP-ALTGR-004 — Ctrl+, (no AltGraph): guard blocks
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
import * as ipcCommands from '$lib/ipc/commands';

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
});
