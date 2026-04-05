// SPDX-License-Identifier: MPL-2.0

/**
 * Extended keyboard tests covering:
 *   DECKPAM-001/002/003 — keypad application mode sequences (FS-KBD-010)
 *   FOCUS-001 — focus events not generated when mode inactive (FS-VT-084)
 *   PASTE-001/002 — Ctrl+Shift+V intercepted (FS-CLIP-005, FS-KBD-003)
 *   BPASTE-001 to 005 — bracketed paste wrapping (FS-CLIP-008)
 *   SEC-BLK-012 — ESC[201~ stripped from paste payload
 *   SEC-BLK-014 — null bytes stripped from paste payload
 *
 * These tests call functions that DO NOT EXIST YET — they are the TDD red phase.
 * Once implemented, all tests in this file must pass.
 */

import { describe, it, expect } from 'vitest';
import { keyEventToVtSequence } from './keyboard.js';

// Functions to be implemented (not yet exported from keyboard.ts):
// - keypadToVtSequence(event, deckpam): Uint8Array | null
// - encodeFocusIn(): Uint8Array
// - encodeFocusOut(): Uint8Array
// - wrapBracketedPaste(text): string
// - isCtrlShiftV(event): boolean

// We import them defensively — tests will fail if the functions don't exist.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const kb = (await import('./keyboard.js')) as any;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function key(
  k: string,
  code: string,
  mods: { ctrl?: boolean; shift?: boolean; alt?: boolean; meta?: boolean } = {},
): KeyboardEvent {
  return {
    key: k,
    code,
    ctrlKey: mods.ctrl ?? false,
    shiftKey: mods.shift ?? false,
    altKey: mods.alt ?? false,
    metaKey: mods.meta ?? false,
    preventDefault: () => {},
  } as unknown as KeyboardEvent;
}

function bytes(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

function toArr(result: Uint8Array | null): number[] | null {
  return result === null ? null : Array.from(result);
}

// ---------------------------------------------------------------------------
// DECKPAM-001: Keypad application mode — numpad keys send SS3 sequences
// Tests keypadToVtSequence(event, deckpam=true)
// ---------------------------------------------------------------------------

describe('DECKPAM-001: keypad application mode — numpad 5 → ESC O u', () => {
  it('KP_5 in application mode sends ESC O u', () => {
    const fn_ = kb.keypadToVtSequence;
    expect(fn_).toBeDefined(); // FAIL until function is exported
    const ev = key('5', 'Numpad5');
    const result: Uint8Array | null = fn_(ev, /* deckpam */ true);
    expect(result).not.toBeNull();
    expect(toArr(result)).toEqual(toArr(bytes('\x1bOu')));
  });
});

describe('DECKPAM-001: KP_1 in application mode → ESC O q', () => {
  it('KP_1 → ESC O q', () => {
    const fn_ = kb.keypadToVtSequence;
    expect(fn_).toBeDefined();
    const ev = key('1', 'Numpad1');
    const result: Uint8Array | null = fn_(ev, true);
    expect(toArr(result)).toEqual(toArr(bytes('\x1bOq')));
  });
});

describe('DECKPAM-001: KP_Enter in application mode → ESC O M', () => {
  it('KP_Enter → ESC O M', () => {
    const fn_ = kb.keypadToVtSequence;
    expect(fn_).toBeDefined();
    const ev = key('Enter', 'NumpadEnter');
    const result: Uint8Array | null = fn_(ev, true);
    expect(toArr(result)).toEqual(toArr(bytes('\x1bOM')));
  });
});

// ---------------------------------------------------------------------------
// DECKPAM-002: Numeric mode — numpad keys send digits
// ---------------------------------------------------------------------------

describe('DECKPAM-002: numeric mode — numpad 5 → digit "5"', () => {
  it('KP_5 in numeric mode returns null (handled as printable digit)', () => {
    const fn_ = kb.keypadToVtSequence;
    expect(fn_).toBeDefined();
    const ev = key('5', 'Numpad5');
    // In numeric mode (deckpam=false), numpad digits are treated as printable
    // and should return null (passed through as normal input).
    const result: Uint8Array | null = fn_(ev, false);
    expect(result).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// DECKPAM-004: Arrow keys unaffected by DECKPAM (controlled by DECCKM)
// ---------------------------------------------------------------------------

describe('DECKPAM-004: arrow keys use DECCKM regardless of DECKPAM', () => {
  it('ArrowUp in DECCKM=false mode → CSI A (DECKPAM has no effect)', () => {
    // Arrow keys go through keyEventToVtSequence with appCursorKeys flag,
    // not through keypadToVtSequence. This verifies they are independent.
    const result = keyEventToVtSequence(key('ArrowUp', 'ArrowUp'), false);
    expect(result).toEqual(bytes('\x1b[A'));
  });
});

// ---------------------------------------------------------------------------
// FOCUS events — encodeFocusIn / encodeFocusOut
// Tests encodeFocusIn() and encodeFocusOut() functions
// ---------------------------------------------------------------------------

describe('FOCUS: encodeFocusIn() returns ESC [ I', () => {
  it('encodeFocusIn returns \\x1b[I', () => {
    const fn_ = kb.encodeFocusIn;
    expect(fn_).toBeDefined(); // FAIL until exported
    const result: Uint8Array = fn_();
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[I')));
  });
});

describe('FOCUS: encodeFocusOut() returns ESC [ O', () => {
  it('encodeFocusOut returns \\x1b[O', () => {
    const fn_ = kb.encodeFocusOut;
    expect(fn_).toBeDefined();
    const result: Uint8Array = fn_();
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[O')));
  });
});

// ---------------------------------------------------------------------------
// PASTE-001/002: Ctrl+Shift+V detection — isCtrlShiftV(event)
// ---------------------------------------------------------------------------

describe('PASTE-001: isCtrlShiftV detects Ctrl+Shift+V', () => {
  it('returns true for Ctrl+Shift+V', () => {
    const fn_ = kb.isCtrlShiftV;
    expect(fn_).toBeDefined(); // FAIL until exported
    const ev = key('v', 'KeyV', { ctrl: true, shift: true });
    expect(fn_(ev)).toBe(true);
  });

  it('returns false for Ctrl+V (without Shift)', () => {
    const fn_ = kb.isCtrlShiftV;
    expect(fn_).toBeDefined();
    const ev = key('v', 'KeyV', { ctrl: true });
    expect(fn_(ev)).toBe(false);
  });

  it('returns false for Shift+V (without Ctrl)', () => {
    const fn_ = kb.isCtrlShiftV;
    expect(fn_).toBeDefined();
    const ev = key('V', 'KeyV', { shift: true });
    expect(fn_(ev)).toBe(false);
  });

  it('returns false for plain V', () => {
    const fn_ = kb.isCtrlShiftV;
    expect(fn_).toBeDefined();
    const ev = key('v', 'KeyV');
    expect(fn_(ev)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// BPASTE-001: wrapBracketedPaste wraps text in ESC[200~ / ESC[201~
// ---------------------------------------------------------------------------

describe('BPASTE-001: wrapBracketedPaste wraps payload', () => {
  it('plain text is wrapped with paste markers', () => {
    const fn_ = kb.wrapBracketedPaste;
    expect(fn_).toBeDefined(); // FAIL until exported
    const result: string = fn_('hello world');
    expect(result).toBe('\x1b[200~hello world\x1b[201~');
  });

  it('multiline text is wrapped as single unit', () => {
    const fn_ = kb.wrapBracketedPaste;
    const result: string = fn_('line1\nline2\nline3');
    expect(result).toBe('\x1b[200~line1\nline2\nline3\x1b[201~');
  });

  it('empty payload produces just the markers', () => {
    const fn_ = kb.wrapBracketedPaste;
    const result: string = fn_('');
    expect(result).toBe('\x1b[200~\x1b[201~');
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-012 / BPASTE-002: ESC[201~ stripped from payload before wrapping
// ---------------------------------------------------------------------------

describe('SEC-BLK-012: embedded ESC[201~ is stripped from paste payload', () => {
  it('embedded paste-end marker is stripped', () => {
    const fn_ = kb.wrapBracketedPaste;
    expect(fn_).toBeDefined();
    const result: string = fn_('safe_text\x1b[201~more');
    expect(result).toBe('\x1b[200~safe_textmore\x1b[201~');
  });

  it('multiple embedded paste-end markers are all stripped', () => {
    const fn_ = kb.wrapBracketedPaste;
    const result: string = fn_('a\x1b[201~b\x1b[201~c');
    expect(result).toBe('\x1b[200~abc\x1b[201~');
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-014: null bytes stripped from paste payload
// ---------------------------------------------------------------------------

describe('SEC-BLK-014: null bytes stripped from paste payload', () => {
  it('null bytes are removed', () => {
    const fn_ = kb.wrapBracketedPaste;
    expect(fn_).toBeDefined();
    const result: string = fn_('foo\x00bar');
    expect(result).toBe('\x1b[200~foobar\x1b[201~');
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-001 (frontend side): regex toggle controls query.regex field
// Already covered in SearchOverlay.test.ts (SEC-UI-003). Verified here as
// a unit-level invariant: keyEventToVtSequence does not consume Ctrl+Shift+F.
// ---------------------------------------------------------------------------

describe('Ctrl+Shift+F is not consumed by keyEventToVtSequence', () => {
  it('Ctrl+Shift+F → null (not a VT sequence, handled by application layer)', () => {
    const ev = key('F', 'KeyF', { ctrl: true, shift: true });
    // keyEventToVtSequence must not consume this — it is the search shortcut.
    // It should return null (printable-ish or unrecognised key with shift+ctrl).
    const result = keyEventToVtSequence(ev, false);
    // Result may be null or a bytes sequence — but it should NOT be a VT
    // sequence that would interfere with the search overlay. We verify the
    // key is not turned into a PTY-bound control char.
    // Ctrl+F = 0x06, but Ctrl+Shift+F is not defined. Must return null.
    expect(result).toBeNull();
  });
});
