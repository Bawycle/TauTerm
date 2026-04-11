// SPDX-License-Identifier: MPL-2.0

import { describe, it, expect } from 'vitest';
import { keyEventToVtSequence } from './keyboard.js';

/** Build a minimal KeyboardEvent-like object for testing. */
function key(
  k: string,
  mods: {
    ctrl?: boolean;
    shift?: boolean;
    alt?: boolean;
    meta?: boolean;
    altgr?: boolean;
  } = {},
): KeyboardEvent {
  const altgr = mods.altgr ?? false;
  return {
    key: k,
    ctrlKey: mods.ctrl ?? false,
    shiftKey: mods.shift ?? false,
    altKey: mods.alt ?? false,
    metaKey: mods.meta ?? false,
    isComposing: false,
    getModifierState: (state: string) => (state === 'AltGraph' ? altgr : false),
  } as unknown as KeyboardEvent;
}

function bytes(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

// ---------------------------------------------------------------------------
// TEST-KBD-001 — Arrow keys in normal mode (appCursorKeys = false)
// ---------------------------------------------------------------------------
describe('TEST-KBD-001: arrow keys — normal mode', () => {
  it('Up → CSI A', () => {
    expect(keyEventToVtSequence(key('ArrowUp'), false)).toEqual(bytes('\x1b[A'));
  });
  it('Down → CSI B', () => {
    expect(keyEventToVtSequence(key('ArrowDown'), false)).toEqual(bytes('\x1b[B'));
  });
  it('Right → CSI C', () => {
    expect(keyEventToVtSequence(key('ArrowRight'), false)).toEqual(bytes('\x1b[C'));
  });
  it('Left → CSI D', () => {
    expect(keyEventToVtSequence(key('ArrowLeft'), false)).toEqual(bytes('\x1b[D'));
  });
});

// ---------------------------------------------------------------------------
// TEST-KBD-002 — Arrow keys in application cursor mode (appCursorKeys = true)
// ---------------------------------------------------------------------------
describe('TEST-KBD-002: arrow keys — application cursor mode (DECCKM)', () => {
  it('Up → SS3 A', () => {
    expect(keyEventToVtSequence(key('ArrowUp'), true)).toEqual(bytes('\x1bOA'));
  });
  it('Down → SS3 B', () => {
    expect(keyEventToVtSequence(key('ArrowDown'), true)).toEqual(bytes('\x1bOB'));
  });
  it('Right → SS3 C', () => {
    expect(keyEventToVtSequence(key('ArrowRight'), true)).toEqual(bytes('\x1bOC'));
  });
  it('Left → SS3 D', () => {
    expect(keyEventToVtSequence(key('ArrowLeft'), true)).toEqual(bytes('\x1bOD'));
  });
  it('modifier overrides SS3 — Ctrl+Up → CSI 1;5 A', () => {
    expect(keyEventToVtSequence(key('ArrowUp', { ctrl: true }), true)).toEqual(bytes('\x1b[1;5A'));
  });
});

/** Convert result to plain number array for comparison (avoids Uint8Array identity issues). */
function toArr(result: Uint8Array | null): number[] | null {
  return result === null ? null : Array.from(result);
}

// ---------------------------------------------------------------------------
// TEST-KBD-003 — Ctrl+letter → C0 control characters
// ---------------------------------------------------------------------------
describe('TEST-KBD-003: Ctrl+letter → C0 control characters', () => {
  it('Ctrl+C → 0x03', () => {
    expect(toArr(keyEventToVtSequence(key('c', { ctrl: true }), false))).toEqual([0x03]);
  });
  it('Ctrl+D → 0x04', () => {
    expect(toArr(keyEventToVtSequence(key('d', { ctrl: true }), false))).toEqual([0x04]);
  });
  it('Ctrl+Z → 0x1a', () => {
    expect(toArr(keyEventToVtSequence(key('z', { ctrl: true }), false))).toEqual([0x1a]);
  });
  it('Ctrl+A → 0x01', () => {
    expect(toArr(keyEventToVtSequence(key('a', { ctrl: true }), false))).toEqual([0x01]);
  });
  it('Ctrl+[ → ESC (0x1b)', () => {
    expect(toArr(keyEventToVtSequence(key('[', { ctrl: true }), false))).toEqual([0x1b]);
  });
  it('Ctrl+\\ → 0x1c', () => {
    expect(toArr(keyEventToVtSequence(key('\\', { ctrl: true }), false))).toEqual([0x1c]);
  });
});

// ---------------------------------------------------------------------------
// TEST-KBD-004 — Function keys F1–F12
// ---------------------------------------------------------------------------
describe('TEST-KBD-004: function keys', () => {
  it('F1 → ESC O P', () => {
    expect(keyEventToVtSequence(key('F1'), false)).toEqual(bytes('\x1bOP'));
  });
  it('F2 → ESC O Q', () => {
    expect(keyEventToVtSequence(key('F2'), false)).toEqual(bytes('\x1bOQ'));
  });
  it('F3 → ESC O R', () => {
    expect(keyEventToVtSequence(key('F3'), false)).toEqual(bytes('\x1bOR'));
  });
  it('F4 → ESC O S', () => {
    expect(keyEventToVtSequence(key('F4'), false)).toEqual(bytes('\x1bOS'));
  });
  it('F5 → CSI 15~', () => {
    expect(keyEventToVtSequence(key('F5'), false)).toEqual(bytes('\x1b[15~'));
  });
  it('F6 → CSI 17~', () => {
    expect(keyEventToVtSequence(key('F6'), false)).toEqual(bytes('\x1b[17~'));
  });
  it('F7 → CSI 18~', () => {
    expect(keyEventToVtSequence(key('F7'), false)).toEqual(bytes('\x1b[18~'));
  });
  it('F8 → CSI 19~', () => {
    expect(keyEventToVtSequence(key('F8'), false)).toEqual(bytes('\x1b[19~'));
  });
  it('F9 → CSI 20~', () => {
    expect(keyEventToVtSequence(key('F9'), false)).toEqual(bytes('\x1b[20~'));
  });
  it('F10 → CSI 21~', () => {
    expect(keyEventToVtSequence(key('F10'), false)).toEqual(bytes('\x1b[21~'));
  });
  it('F11 → CSI 23~', () => {
    expect(keyEventToVtSequence(key('F11'), false)).toEqual(bytes('\x1b[23~'));
  });
  it('F12 → CSI 24~', () => {
    expect(keyEventToVtSequence(key('F12'), false)).toEqual(bytes('\x1b[24~'));
  });

  // PageUp / PageDown
  it('PageUp → CSI 5~', () => {
    expect(keyEventToVtSequence(key('PageUp'), false)).toEqual(bytes('\x1b[5~'));
  });
  it('PageDown → CSI 6~', () => {
    expect(keyEventToVtSequence(key('PageDown'), false)).toEqual(bytes('\x1b[6~'));
  });

  // Insert / Delete
  it('Insert → CSI 2~', () => {
    expect(keyEventToVtSequence(key('Insert'), false)).toEqual(bytes('\x1b[2~'));
  });
  it('Delete → CSI 3~', () => {
    expect(keyEventToVtSequence(key('Delete'), false)).toEqual(bytes('\x1b[3~'));
  });
});

// ---------------------------------------------------------------------------
// Printable characters → UTF-8 bytes
// ---------------------------------------------------------------------------
describe('printable characters → UTF-8 bytes', () => {
  it('regular letter "a" → [0x61]', () => {
    expect(toArr(keyEventToVtSequence(key('a'), false))).toEqual([0x61]);
  });
  it('digit "5" → [0x35]', () => {
    expect(toArr(keyEventToVtSequence(key('5'), false))).toEqual([0x35]);
  });
  it('space " " → [0x20]', () => {
    expect(toArr(keyEventToVtSequence(key(' '), false))).toEqual([0x20]);
  });
});

// ---------------------------------------------------------------------------
// Alt+key → ESC prefix (FS-KBD-005)
// ---------------------------------------------------------------------------
describe('Alt+key → ESC prefix', () => {
  it('Alt+a → ESC a', () => {
    expect(keyEventToVtSequence(key('a', { alt: true }), false)).toEqual(bytes('\x1ba'));
  });
  it('Alt+f → ESC f', () => {
    expect(keyEventToVtSequence(key('f', { alt: true }), false)).toEqual(bytes('\x1bf'));
  });
});

// ---------------------------------------------------------------------------
// Backspace → DEL (0x7f)
// ---------------------------------------------------------------------------
describe('Backspace', () => {
  it('Backspace → DEL 0x7f', () => {
    expect(toArr(keyEventToVtSequence(key('Backspace'), false))).toEqual([0x7f]);
  });
});

// ---------------------------------------------------------------------------
// Escape, Enter, Tab
// ---------------------------------------------------------------------------
describe('Special control keys', () => {
  it('Escape → 0x1b', () => {
    expect(toArr(keyEventToVtSequence(key('Escape'), false))).toEqual([0x1b]);
  });
  it('Enter → CR (0x0d)', () => {
    expect(toArr(keyEventToVtSequence(key('Enter'), false))).toEqual([0x0d]);
  });
  it('Tab → 0x09', () => {
    expect(toArr(keyEventToVtSequence(key('Tab'), false))).toEqual([0x09]);
  });
  it('Shift+Tab → CSI Z', () => {
    expect(keyEventToVtSequence(key('Tab', { shift: true }), false)).toEqual(bytes('\x1b[Z'));
  });
});

// ---------------------------------------------------------------------------
// Meta key combinations → null (OS shortcuts, do not consume)
// ---------------------------------------------------------------------------
describe('Meta key → null', () => {
  it('Meta+a → null', () => {
    expect(keyEventToVtSequence(key('a', { meta: true }), false)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// AltGr characters (Linux/WebKitGTK)
// ---------------------------------------------------------------------------
describe('AltGr characters (Linux/WebKitGTK)', () => {
  // TEST-KBD-ALTGR-001: Belgian keyboard AltGr+= → ~
  it('TEST-KBD-ALTGR-001: AltGr+= on Belgian keyboard produces ~ (0x7E)', () => {
    // On Belgian keyboard: AltGr+= → '~' (level 3 character)
    // WebKitGTK emits: ctrlKey=true, altKey=true, key='~', getModifierState('AltGraph')=true
    const result = keyEventToVtSequence(key('~', { ctrl: true, alt: true, altgr: true }), false);
    expect(result).not.toBeNull();
    expect(toArr(result!)).toEqual(Array.from(bytes('~')));
  });

  // TEST-KBD-ALTGR-002: Another AltGr combination
  it('TEST-KBD-ALTGR-002: AltGr produces { (0x7B) on keyboards where this is level 3', () => {
    const result = keyEventToVtSequence(key('{', { ctrl: true, alt: true, altgr: true }), false);
    expect(result).not.toBeNull();
    expect(toArr(result!)).toEqual(Array.from(bytes('{')));
  });

  // TEST-KBD-ALTGR-003: Genuine Ctrl+Alt (not AltGr) → null (must NOT be transmitted)
  it('TEST-KBD-ALTGR-003: Ctrl+Alt without AltGraph modifier returns null (not transmitted)', () => {
    // getModifierState('AltGraph') returns false → genuine Ctrl+Alt, not AltGr
    const result = keyEventToVtSequence(key('a', { ctrl: true, alt: true, altgr: false }), false);
    expect(result).toBeNull();
  });

  // TEST-KBD-ALTGR-004: Ctrl alone (no altKey) with a character → not treated as AltGr
  it('TEST-KBD-ALTGR-004: Ctrl+key without altKey is not AltGr and not encoded as printable', () => {
    // Ctrl+~ : ctrlKey only, not AltGr — falls through (not a letter, not printable branch)
    // NOTE: this should return null (no defined mapping for Ctrl+~)
    const result = keyEventToVtSequence(key('~', { ctrl: true, alt: false, altgr: false }), false);
    expect(result).toBeNull();
  });

  // TEST-KBD-ALTGR-005: AltGr+€ (multi-byte UTF-8 character via AltGr)
  it('TEST-KBD-ALTGR-005: AltGr produces € (euro sign, U+20AC, level 3 on some layouts)', () => {
    const result = keyEventToVtSequence(key('€', { ctrl: true, alt: true, altgr: true }), false);
    expect(result).not.toBeNull();
    // € is U+20AC — UTF-8: E2 82 AC
    expect(toArr(result!)).toEqual([0xe2, 0x82, 0xac]);
  });

  // TEST-KBD-ALTGR-006: AltGr+Shift combination (level 4)
  it('TEST-KBD-ALTGR-006: AltGr+Shift produces level-4 character', () => {
    // Example: AltGr+Shift on some layout produces ©
    const result = keyEventToVtSequence(
      key('©', { ctrl: true, alt: true, shift: true, altgr: true }),
      false,
    );
    expect(result).not.toBeNull();
    // © is U+00A9 — UTF-8: C2 A9
    expect(toArr(result!)).toEqual([0xc2, 0xa9]);
  });
});
