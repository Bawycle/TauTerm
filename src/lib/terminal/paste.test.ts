// SPDX-License-Identifier: MPL-2.0

/**
 * Paste mode-aware dispatch unit tests.
 *
 * Covers FS-CLIP-008 (bracketed paste mode) and SEC-BLK-012/014
 * at the paste dispatch layer: given a bracketed-paste mode flag, the
 * `pasteToBytes` function either wraps with ESC[200~/ESC[201~ or sends raw.
 *
 * TDD red phase — `pasteToBytes` does NOT yet exist.
 * These tests will FAIL until `src/lib/terminal/paste.ts` is created.
 *
 * Expected module: `src/lib/terminal/paste.ts`
 * Expected exports:
 *   - pasteToBytes(text: string, bracketedPasteActive: boolean): Uint8Array | null
 *       Returns the bytes to send to the PTY:
 *       - When bracketedPasteActive=true: wraps with ESC[200~ / ESC[201~, strips
 *         embedded ESC[201~ sequences (SEC-BLK-012) and null bytes (SEC-BLK-014).
 *       - When bracketedPasteActive=false: encodes as UTF-8, strips null bytes only.
 *       - Returns null for empty text after stripping (nothing to send).
 *
 * Note: wrapBracketedPaste logic is separately tested in keyboard-extended.test.ts.
 * This test file focuses on the mode dispatch and the null/empty edge cases.
 */

import { describe, it, expect } from 'vitest';

// TDD: import defensively — tests fail until paste.ts exists.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const pasteModule = (await import('./paste.js').catch(() => ({}))) as any;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function bytes(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

function toArr(result: Uint8Array | null): number[] | null {
  return result === null ? null : Array.from(result);
}

// ---------------------------------------------------------------------------
// BPASTE-MODE-001: bracketedPasteActive=true → wrapped output
// ---------------------------------------------------------------------------

describe('BPASTE-MODE-001: bracketed paste active — text wrapped in markers', () => {
  it('plain text is wrapped with ESC[200~ and ESC[201~', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined(); // FAIL until paste.ts exists
    const result = fn_('hello', true);
    expect(result).not.toBeNull();
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[200~hello\x1b[201~')));
  });

  it('multiline text is wrapped as single unit', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('line1\nline2', true);
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[200~line1\nline2\x1b[201~')));
  });
});

// ---------------------------------------------------------------------------
// BPASTE-MODE-002: bracketedPasteActive=false → raw bytes (no wrapping)
// ---------------------------------------------------------------------------

describe('BPASTE-MODE-002: bracketed paste inactive — text sent raw', () => {
  it('plain text sent as raw UTF-8 bytes without markers', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('hello', false);
    expect(toArr(result)).toEqual(toArr(bytes('hello')));
  });

  it('multiline text sent raw', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('line1\nline2', false);
    expect(toArr(result)).toEqual(toArr(bytes('line1\nline2')));
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-012: embedded ESC[201~ stripped when bracketed paste active
// ---------------------------------------------------------------------------

describe('SEC-BLK-012: ESC[201~ stripped from payload before bracketed paste wrapping', () => {
  it('embedded paste-end marker is stripped when mode is active', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('safe\x1b[201~text', true);
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[200~safetext\x1b[201~')));
  });

  it('embedded ESC[201~ is NOT stripped when mode is inactive (raw paste)', () => {
    // In raw mode the text is passed as-is; the terminal handles it.
    // This is correct behavior: we only sanitize inside the bracketed wrapper.
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('safe\x1b[201~text', false);
    expect(toArr(result)).toEqual(toArr(bytes('safe\x1b[201~text')));
  });
});

// ---------------------------------------------------------------------------
// SEC-BLK-014: null bytes stripped from paste payload (both modes)
// ---------------------------------------------------------------------------

describe('SEC-BLK-014: null bytes stripped in both modes', () => {
  it('null bytes stripped when bracketed paste active', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('foo\x00bar', true);
    expect(toArr(result)).toEqual(toArr(bytes('\x1b[200~foobar\x1b[201~')));
  });

  it('null bytes stripped when bracketed paste inactive (raw mode)', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('foo\x00bar', false);
    expect(toArr(result)).toEqual(toArr(bytes('foobar')));
  });
});

// ---------------------------------------------------------------------------
// Edge case: empty paste → null (nothing to send)
// ---------------------------------------------------------------------------

describe('BPASTE-EMPTY: empty paste produces null', () => {
  it('empty text with bracketed paste active returns null', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('', true);
    expect(result).toBeNull();
  });

  it('empty text with bracketed paste inactive returns null', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('', false);
    expect(result).toBeNull();
  });

  it('text that reduces to empty after null-stripping returns null', () => {
    const fn_ = pasteModule.pasteToBytes;
    expect(fn_).toBeDefined();
    const result = fn_('\x00\x00', false);
    expect(result).toBeNull();
  });
});
