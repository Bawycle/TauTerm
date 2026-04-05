// SPDX-License-Identifier: MPL-2.0

/**
 * Security-focused tests for keyboard input pipeline.
 * Covers TUITC-SEC-030, TUITC-SEC-031.
 * Verifies that keyboard.ts produces byte arrays (not strings) and that
 * normal keystrokes do not produce payloads exceeding the 64 KiB limit.
 */

import { describe, it, expect } from 'vitest';
import { keyEventToVtSequence } from './keyboard.js';

const SEND_INPUT_MAX_BYTES = 65_536;

function key(
  k: string,
  mods: { ctrl?: boolean; shift?: boolean; alt?: boolean; meta?: boolean } = {},
): KeyboardEvent {
  return {
    key: k,
    ctrlKey: mods.ctrl ?? false,
    shiftKey: mods.shift ?? false,
    altKey: mods.alt ?? false,
    metaKey: mods.meta ?? false,
  } as KeyboardEvent;
}

// ---------------------------------------------------------------------------
// TUITC-SEC-031: send_input data is always a Uint8Array (byte array)
// ---------------------------------------------------------------------------
describe('TUITC-SEC-031: keyEventToVtSequence returns Uint8Array, never a string', () => {
  const cases = [
    { label: 'ArrowUp', k: key('ArrowUp') },
    { label: 'Enter', k: key('Enter') },
    { label: 'Escape', k: key('Escape') },
    { label: 'F1', k: key('F1') },
    { label: 'Ctrl+C', k: key('c', { ctrl: true }) },
    { label: 'Alt+a', k: key('a', { alt: true }) },
    { label: 'Backspace', k: key('Backspace') },
    { label: 'Tab', k: key('Tab') },
    { label: 'Delete', k: key('Delete') },
  ];

  for (const { label, k } of cases) {
    it(`${label} → result is Uint8Array`, () => {
      const result = keyEventToVtSequence(k, false);
      expect(result).not.toBeNull();
      // In jsdom the VM context may differ, so we check via constructor name
      // rather than instanceof to avoid cross-realm failures.
      expect(result!.constructor.name).toBe('Uint8Array');
      expect(result!.length).toBeGreaterThan(0);
    });
  }

  it('printable character → UTF-8 bytes (sent via keyEventToVtSequence)', () => {
    // Printable chars are encoded directly — the viewport div is not contenteditable
    // so oninput never fires; all input must go through keydown.
    const result = keyEventToVtSequence(key('a'), false);
    expect(result).not.toBeNull();
    expect(result!.constructor.name).toBe('Uint8Array');
    expect(Array.from(result!)).toEqual([0x61]);
  });
});

// ---------------------------------------------------------------------------
// TUITC-SEC-030: Normal keystrokes never produce payloads > 64 KiB
// All escape sequences are at most a few bytes; this test confirms no single
// keystroke produces a payload that would be rejected by the backend guard.
// ---------------------------------------------------------------------------
describe('TUITC-SEC-030: normal keystrokes produce payloads well within 64 KiB', () => {
  const allKeys = [
    key('ArrowUp'),
    key('ArrowDown'),
    key('ArrowLeft'),
    key('ArrowRight'),
    key('F1'),
    key('F2'),
    key('F3'),
    key('F4'),
    key('F5'),
    key('F6'),
    key('F7'),
    key('F8'),
    key('F9'),
    key('F10'),
    key('F11'),
    key('F12'),
    key('Home'),
    key('End'),
    key('Insert'),
    key('Delete'),
    key('PageUp'),
    key('PageDown'),
    key('Enter'),
    key('Escape'),
    key('Tab'),
    key('Backspace'),
    key('c', { ctrl: true }),
    key('d', { ctrl: true }),
    key('z', { ctrl: true }),
    key('a', { alt: true }),
    key('ArrowUp', { ctrl: true }),
  ];

  for (const k of allKeys) {
    it(`key "${k.key}" produces payload < 64 KiB`, () => {
      const result = keyEventToVtSequence(k, false);
      if (result !== null) {
        expect(result.length).toBeLessThan(SEND_INPUT_MAX_BYTES);
      }
    });
  }
});

// ---------------------------------------------------------------------------
// TUITC-SEC-030 corner: application cursor mode doesn't inflate payload size
// ---------------------------------------------------------------------------
describe('TUITC-SEC-030: application cursor mode payloads are also small', () => {
  it('ArrowUp in app mode → 3 bytes (ESC O A)', () => {
    const result = keyEventToVtSequence(key('ArrowUp'), true);
    expect(result).not.toBeNull();
    expect(result!.length).toBe(3);
  });
});
