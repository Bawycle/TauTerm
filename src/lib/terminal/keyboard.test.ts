// SPDX-License-Identifier: MPL-2.0

import { describe, it, expect } from 'vitest';
import { keyEventToVtSequence } from './keyboard.js';

/** Build a minimal KeyboardEvent-like object for testing. */
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
// Printable characters → null (not consumed)
// ---------------------------------------------------------------------------
describe('printable characters → null', () => {
  it('regular letter "a" → null', () => {
    expect(keyEventToVtSequence(key('a'), false)).toBeNull();
  });
  it('digit "5" → null', () => {
    expect(keyEventToVtSequence(key('5'), false)).toBeNull();
  });
  it('space " " → null', () => {
    expect(keyEventToVtSequence(key(' '), false)).toBeNull();
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
