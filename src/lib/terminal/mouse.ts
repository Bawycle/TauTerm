// SPDX-License-Identifier: MPL-2.0

/**
 * Mouse event encoding for VT mouse reporting protocols.
 *
 * Implements X10 (default) and SGR (1006) encodings per FS-MOUSE-001 to FS-MOUSE-006.
 * Shift+Click/Wheel bypasses reporting for text selection (SEC-BLK-021).
 *
 * References:
 *  - FS-MOUSE-001: mode=none → null
 *  - FS-MOUSE-002: X10 encoding (default)
 *  - FS-MOUSE-003: SGR encoding
 *  - FS-MOUSE-004: Shift bypasses mouse reporting
 *  - FS-MOUSE-005: Wheel events (button 64/65)
 *  - FS-MOUSE-006: Shift+Wheel bypasses mouse reporting
 *  - SEC-BLK-021: Shift forces text selection mode
 */

export type MouseReportingMode = 'none' | 'x10' | 'normal' | 'button-event' | 'any-event';
export type MouseEncoding = 'default' | 'sgr';

export interface MouseModifiers {
  shiftKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
}

/**
 * Encode a mouse event as a VT sequence for the given reporting mode and encoding.
 *
 * @param mode      - Active mouse reporting mode.
 * @param encoding  - Wire encoding: 'default' (X10) or 'sgr' (CSI <).
 * @param button    - Button code: 0=left, 1=middle, 2=right, 3=release(X10),
 *                    64=wheel-up, 65=wheel-down.
 * @param col       - 1-based terminal column.
 * @param row       - 1-based terminal row.
 * @param mods      - Keyboard modifier flags.
 * @param release   - Whether this is a button release event.
 * @returns The byte sequence to send, or null if reporting is inactive or bypassed.
 */
export function encodeMouseEvent(
  mode: MouseReportingMode,
  encoding: MouseEncoding,
  button: number,
  col: number,
  row: number,
  mods: MouseModifiers,
  release: boolean,
): Uint8Array | null {
  // SEC-BLK-021: Shift bypasses mouse reporting for text selection
  if (mods.shiftKey) return null;

  // mode=none → no reporting
  if (mode === 'none') return null;

  // Build modifier bitmask (xterm convention)
  const modBits =
    (mods.shiftKey ? 4 : 0) |
    (mods.altKey ? 8 : 0) |
    (mods.ctrlKey ? 16 : 0);

  const cb = button | modBits;

  if (encoding === 'sgr') {
    const suffix = release ? 'm' : 'M';
    return encode(`\x1b[<${cb};${col};${row}${suffix}`);
  }

  // X10 (default) encoding: ESC [ M <cb+32> <cx+32> <cy+32>
  // For release events in X10 mode, button is encoded as 3 (no specific button info)
  const x10Button = release ? 3 : cb;
  const clamp = (n: number) => Math.min(n + 32, 255);
  return new Uint8Array([0x1b, 0x5b, 0x4d, clamp(x10Button), clamp(col), clamp(row)]);
}

function encode(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}
