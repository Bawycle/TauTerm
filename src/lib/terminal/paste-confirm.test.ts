// SPDX-License-Identifier: MPL-2.0

/**
 * FS-CLIP-009 — Multiline paste confirmation dialog logic.
 *
 * Tests the decision function that determines whether a paste operation requires
 * a user confirmation dialog before proceeding.
 *
 * The decision follows: show dialog when ALL of these conditions are met:
 *   - bracketedPasteActive is false (bracketed paste not active)
 *   - text contains at least one newline character
 *   - confirmMultilinePaste preference is true (dialog not disabled by user)
 *
 * This logic is extracted from TerminalPane.svelte (pasteText() function, line ~611)
 * and documented for unit testing purposes. The pure function is tested here
 * independently of the Svelte component runtime.
 *
 * Protocol reference: TP-MIN-015, TP-MIN-016, TP-MIN-017.
 * FS reference: FS-CLIP-009.
 *
 * NOTE: The decision logic is currently inlined in TerminalPane.svelte. These
 * tests document the expected behaviour as a specification. If the logic is
 * extracted into a module, the import below should be updated accordingly.
 * For now, the pure function is re-implemented here to test the logic contract.
 */

import { describe, it, expect } from 'vitest';

// ---------------------------------------------------------------------------
// Pure decision function (mirrors pasteText condition in TerminalPane.svelte)
// ---------------------------------------------------------------------------

/**
 * Determine whether a paste operation should show a confirmation dialog.
 *
 * @param text - The text about to be pasted.
 * @param bracketedPasteActive - True when the terminal has enabled bracketed paste mode.
 * @param confirmMultilinePaste - User preference: whether to show the dialog at all.
 * @returns True if the confirmation dialog should be shown before pasting.
 */
function pasteNeedsConfirmation(
  text: string,
  bracketedPasteActive: boolean,
  confirmMultilinePaste: boolean,
): boolean {
  const hasNewlines = text.includes('\n');
  return !bracketedPasteActive && hasNewlines && confirmMultilinePaste;
}

// ---------------------------------------------------------------------------
// TP-MIN-015: dialog shown when bracketed paste inactive + multiline + pref enabled
// ---------------------------------------------------------------------------

describe('TP-MIN-015: paste confirmation shown for multiline text with bracketed paste inactive', () => {
  it('two-line text + bracketedPaste=false + confirm=true → dialog shown', () => {
    expect(pasteNeedsConfirmation('first line\nsecond line', false, true)).toBe(true);
  });

  it('text with only a trailing newline → dialog shown', () => {
    expect(pasteNeedsConfirmation('command\n', false, true)).toBe(true);
  });

  it('text with multiple newlines → dialog shown', () => {
    expect(pasteNeedsConfirmation('line1\nline2\nline3\n', false, true)).toBe(true);
  });

  it('text with only a newline → dialog shown', () => {
    expect(pasteNeedsConfirmation('\n', false, true)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// TP-MIN-016: no dialog when bracketed paste is active (even with multiline text)
// ---------------------------------------------------------------------------

describe('TP-MIN-016: no confirmation when bracketed paste is active', () => {
  it('multiline text + bracketedPaste=true → dialog NOT shown', () => {
    expect(pasteNeedsConfirmation('line1\nline2', true, true)).toBe(false);
  });

  it('multiline text + bracketedPaste=true + confirm=true → dialog NOT shown', () => {
    // Bracketed paste mode takes priority — text is safely wrapped.
    expect(pasteNeedsConfirmation('rm -rf /\nconfirm', true, true)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TP-MIN-017: no dialog when text is single-line (no newlines)
// ---------------------------------------------------------------------------

describe('TP-MIN-017: no confirmation when text has no newlines', () => {
  it('single-line text + bracketedPaste=false + confirm=true → dialog NOT shown', () => {
    expect(pasteNeedsConfirmation('singlelinecommand', false, true)).toBe(false);
  });

  it('empty text → dialog NOT shown', () => {
    expect(pasteNeedsConfirmation('', false, true)).toBe(false);
  });

  it('text with only spaces → dialog NOT shown', () => {
    expect(pasteNeedsConfirmation('   ', false, true)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// FS-CLIP-009: confirmMultilinePaste=false disables the dialog entirely
// (user has chosen "Don't ask again")
// ---------------------------------------------------------------------------

describe('FS-CLIP-009: dialog disabled when confirmMultilinePaste=false', () => {
  it('multiline + bracketedPaste=false + confirm=false → dialog NOT shown', () => {
    expect(pasteNeedsConfirmation('line1\nline2', false, false)).toBe(false);
  });

  it('multiline + bracketedPaste=true + confirm=false → dialog NOT shown', () => {
    expect(pasteNeedsConfirmation('line1\nline2', true, false)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

describe('FS-CLIP-009: edge cases', () => {
  it('carriage return alone (\\r) does not trigger dialog (only \\n matters)', () => {
    // Windows-style CR without LF — behaviour may vary, but the protocol
    // specifies "contains newlines" as \\n. CR alone is not a newline trigger.
    expect(pasteNeedsConfirmation('line1\rline2', false, true)).toBe(false);
  });

  it('CRLF sequence (\\r\\n) triggers dialog (contains \\n)', () => {
    expect(pasteNeedsConfirmation('line1\r\nline2', false, true)).toBe(true);
  });
});
