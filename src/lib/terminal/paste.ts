// SPDX-License-Identifier: MPL-2.0

/**
 * Paste dispatch: mode-aware text encoding for PTY input.
 *
 * Implements FS-CLIP-008 (bracketed paste wrapping) and
 * SEC-BLK-012 (strip embedded ESC[201~) and SEC-BLK-014 (strip null bytes).
 */

/**
 * Encode paste text as bytes to send to the PTY.
 *
 * When bracketedPasteActive is true:
 *   1. Strip null bytes (SEC-BLK-014)
 *   2. Strip embedded ESC[201~ sequences (SEC-BLK-012)
 *   3. Wrap with ESC[200~ … ESC[201~
 *
 * When bracketedPasteActive is false:
 *   1. Strip null bytes (SEC-BLK-014)
 *   2. Encode as UTF-8 (no wrapping)
 *
 * Returns null if the resulting text is empty (nothing to send).
 */
export function pasteToBytes(text: string, bracketedPasteActive: boolean): Uint8Array | null {
  // SEC-BLK-014: strip null bytes in both modes
  let sanitized = text.replaceAll('\x00', '');

  if (bracketedPasteActive) {
    // SEC-BLK-012: strip embedded paste-end marker before wrapping
    sanitized = sanitized.replaceAll('\x1b[201~', '');
    if (sanitized.length === 0) return null;
    return new TextEncoder().encode('\x1b[200~' + sanitized + '\x1b[201~');
  }

  if (sanitized.length === 0) return null;
  return new TextEncoder().encode(sanitized);
}
