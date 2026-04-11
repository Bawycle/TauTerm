// SPDX-License-Identifier: MPL-2.0

/**
 * VT escape sequence mapping for keyboard input.
 *
 * Implements xterm key encoding per FS-KBD-004 through FS-KBD-009.
 * Handles all key events including printable characters — the viewport
 * element is a non-contenteditable <div> so the browser's oninput pipeline
 * never fires for it; every keystroke must go through keydown.
 *
 * References:
 *  - FS-KBD-004: Ctrl+letter → C0 control character
 *  - FS-KBD-005: Alt+key → ESC prefix
 *  - FS-KBD-006: F1–F12 → standard xterm sequences
 *  - FS-KBD-007: Arrow keys mode-dependent (DECCKM)
 *  - FS-KBD-008: Home/End/Insert/Delete/PageUp/PageDown
 *  - FS-KBD-009: Modified keys → CSI 1;Mod X
 */

/** Modifier bitmask values per FS-KBD-009 (xterm convention: 1-based, so +1 to bitfield). */
const MOD_SHIFT = 2;
const MOD_ALT = 3;
const MOD_SHIFT_ALT = 4;
const MOD_CTRL = 5;
const MOD_CTRL_SHIFT = 6;
const MOD_CTRL_ALT = 7;
const MOD_CTRL_SHIFT_ALT = 8;

function modifierCode(event: KeyboardEvent): number {
  const shift = event.shiftKey ? 1 : 0;
  const alt = event.altKey ? 1 : 0;
  const ctrl = event.ctrlKey ? 1 : 0;
  // Table: (ctrl*4 + alt*2 + shift*1) + 1 → matches xterm's 1-based modifier encoding
  const bits = ctrl * 4 + alt * 2 + shift;
  // bits 0 = no modifier, 1 = Shift, 2 = Alt, 3 = Shift+Alt, 4 = Ctrl, 5 = Ctrl+Shift,
  //      6 = Ctrl+Alt, 7 = Ctrl+Shift+Alt
  const table = [
    0,
    MOD_SHIFT,
    MOD_ALT,
    MOD_SHIFT_ALT,
    MOD_CTRL,
    MOD_CTRL_SHIFT,
    MOD_CTRL_ALT,
    MOD_CTRL_SHIFT_ALT,
  ];
  return table[bits] ?? 0;
}

function encode(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

/**
 * Build a CSI sequence with optional modifier parameter.
 * e.g. arrowWithMod('A', mod) → CSI 1;mod A when mod != 0, else CSI A.
 */
function csiArrow(letter: string, mod: number): Uint8Array {
  if (mod === 0) return encode(`\x1b[${letter}`);
  return encode(`\x1b[1;${mod}${letter}`);
}

/**
 * Build a CSI tilde sequence with optional modifier parameter.
 * e.g. tildeWithMod(5, mod) → CSI 5;mod ~ when mod != 0, else CSI 5~.
 */
function csiTilde(code: number, mod: number): Uint8Array {
  if (mod === 0) return encode(`\x1b[${code}~`);
  return encode(`\x1b[${code};${mod}~`);
}

/**
 * Map a KeyboardEvent to its VT escape sequence bytes.
 *
 * @param event - The browser KeyboardEvent (keydown).
 * @param appCursorKeys - Whether DECCKM (application cursor mode) is active.
 * @param appKeypad - Whether DECKPAM (application keypad mode) is active (FS-KBD-010).
 * @returns The VT sequence bytes to send to the PTY, or `null` if the key
 *          should not be consumed (unrecognised combinations, Meta key, etc.).
 */
export function keyEventToVtSequence(
  event: KeyboardEvent,
  appCursorKeys: boolean,
  appKeypad: boolean = false,
): Uint8Array | null {
  const { key, ctrlKey, altKey, shiftKey, metaKey } = event;

  // Meta key combinations are OS-level shortcuts — never send to PTY
  if (metaKey) return null;

  const mod = modifierCode(event);

  // -------------------------------------------------------------------------
  // Arrow keys (FS-KBD-007)
  // Normal mode: CSI A/B/C/D; Application mode (DECCKM): SS3 A/B/C/D
  // With modifiers: always CSI 1;Mod A/B/C/D
  // -------------------------------------------------------------------------
  if (key === 'ArrowUp') {
    if (mod !== 0) return csiArrow('A', mod);
    return appCursorKeys ? encode('\x1bOA') : encode('\x1b[A');
  }
  if (key === 'ArrowDown') {
    if (mod !== 0) return csiArrow('B', mod);
    return appCursorKeys ? encode('\x1bOB') : encode('\x1b[B');
  }
  if (key === 'ArrowRight') {
    if (mod !== 0) return csiArrow('C', mod);
    return appCursorKeys ? encode('\x1bOC') : encode('\x1b[C');
  }
  if (key === 'ArrowLeft') {
    if (mod !== 0) return csiArrow('D', mod);
    return appCursorKeys ? encode('\x1bOD') : encode('\x1b[D');
  }

  // -------------------------------------------------------------------------
  // Home / End (FS-KBD-008)
  // -------------------------------------------------------------------------
  if (key === 'Home') {
    if (mod !== 0) return encode(`\x1b[1;${mod}H`);
    return encode('\x1b[H');
  }
  if (key === 'End') {
    if (mod !== 0) return encode(`\x1b[1;${mod}F`);
    return encode('\x1b[F');
  }

  // -------------------------------------------------------------------------
  // Insert / Delete / PageUp / PageDown (FS-KBD-008)
  // -------------------------------------------------------------------------
  if (key === 'Insert') return csiTilde(2, mod);
  if (key === 'Delete') return csiTilde(3, mod);
  if (key === 'PageUp') return csiTilde(5, mod);
  if (key === 'PageDown') return csiTilde(6, mod);

  // -------------------------------------------------------------------------
  // Function keys F1–F12 (FS-KBD-006)
  // F1–F4: SS3 P/Q/R/S (or CSI 1;Mod P/Q/R/S with modifiers)
  // F5–F12: CSI tilde sequences
  // -------------------------------------------------------------------------
  if (key === 'F1') {
    if (mod !== 0) return encode(`\x1b[1;${mod}P`);
    return encode('\x1bOP');
  }
  if (key === 'F2') {
    if (mod !== 0) return encode(`\x1b[1;${mod}Q`);
    return encode('\x1bOQ');
  }
  if (key === 'F3') {
    if (mod !== 0) return encode(`\x1b[1;${mod}R`);
    return encode('\x1bOR');
  }
  if (key === 'F4') {
    if (mod !== 0) return encode(`\x1b[1;${mod}S`);
    return encode('\x1bOS');
  }
  if (key === 'F5') return csiTilde(15, mod);
  if (key === 'F6') return csiTilde(17, mod);
  if (key === 'F7') return csiTilde(18, mod);
  if (key === 'F8') return csiTilde(19, mod);
  if (key === 'F9') return csiTilde(20, mod);
  if (key === 'F10') return csiTilde(21, mod);
  if (key === 'F11') return csiTilde(23, mod);
  if (key === 'F12') return csiTilde(24, mod);

  // -------------------------------------------------------------------------
  // Backspace: DEL (0x7f) — standard xterm behavior
  // -------------------------------------------------------------------------
  if (key === 'Backspace') {
    if (ctrlKey) return encode('\x08'); // Ctrl+Backspace → BS
    return encode('\x7f');
  }

  // -------------------------------------------------------------------------
  // Tab
  // -------------------------------------------------------------------------
  if (key === 'Tab') {
    if (shiftKey) return encode('\x1b[Z'); // Shift+Tab → CSI Z (backtab)
    return encode('\t');
  }

  // -------------------------------------------------------------------------
  // Enter: CR (0x0d)
  // -------------------------------------------------------------------------
  if (key === 'Enter') {
    return encode('\r');
  }

  // -------------------------------------------------------------------------
  // Escape
  // -------------------------------------------------------------------------
  if (key === 'Escape') {
    return encode('\x1b');
  }

  // -------------------------------------------------------------------------
  // Ctrl+letter → C0 control characters (FS-KBD-004)
  // Ctrl+A (0x01) through Ctrl+Z (0x1A)
  // Ctrl+[ (0x1B), Ctrl+\ (0x1C), Ctrl+] (0x1D), Ctrl+^ (0x1E), Ctrl+_ (0x1F)
  // -------------------------------------------------------------------------
  // Ctrl+Shift+letter combinations are TauTerm application shortcuts
  // (FS-KBD-003: Ctrl+Shift+T, W, F, V). Never send them to the PTY.
  if (ctrlKey && !altKey && !shiftKey) {
    const code = key.toUpperCase().codePointAt(0);
    // A–Z: codes 65–90 → control chars 0x01–0x1A
    if (code !== undefined && code >= 65 && code <= 90) {
      // Ctrl+C=3, Ctrl+D=4, Ctrl+Z=26, etc.
      return new Uint8Array([code - 64]);
    }
    // Special control characters
    if (key === '[') return encode('\x1b');
    if (key === '\\') return encode('\x1c');
    if (key === ']') return encode('\x1d');
    if (key === '^') return encode('\x1e');
    if (key === '_') return encode('\x1f');
    if (key === '@') return encode('\x00'); // Ctrl+@ = NUL
    if (key === ' ') return encode('\x00'); // Ctrl+Space = NUL
  }

  // -------------------------------------------------------------------------
  // Alt+key → ESC prefix (FS-KBD-005)
  // Only for single printable characters (no ctrl combos with alt here;
  // those were handled above or are left to the platform).
  // -------------------------------------------------------------------------
  if (altKey && !ctrlKey && key.length === 1) {
    return encode('\x1b' + key);
  }

  // -------------------------------------------------------------------------
  // Keypad application mode (DECKPAM, FS-KBD-010)
  // When active, numeric keypad keys send SS3 sequences instead of digits.
  // -------------------------------------------------------------------------
  if (appKeypad && !ctrlKey && !altKey && !shiftKey) {
    switch (key) {
      case '0':
        return encode('\x1bOp');
      case '1':
        return encode('\x1bOq');
      case '2':
        return encode('\x1bOr');
      case '3':
        return encode('\x1bOs');
      case '4':
        return encode('\x1bOt');
      case '5':
        return encode('\x1bOu');
      case '6':
        return encode('\x1bOv');
      case '7':
        return encode('\x1bOw');
      case '8':
        return encode('\x1bOx');
      case '9':
        return encode('\x1bOy');
      case '.':
        return encode('\x1bOn');
      case '+':
        return encode('\x1bOl');
      case '-':
        return encode('\x1bOm');
      case '*':
        return encode('\x1bOj');
      case '/':
        return encode('\x1bOo');
    }
  }

  // -------------------------------------------------------------------------
  // Printable single characters — send UTF-8 bytes directly to PTY.
  // Placed after DECKPAM so keypad application sequences take priority.
  // -------------------------------------------------------------------------
  if (key.length === 1 && !ctrlKey && !altKey) {
    return encode(key);
  }

  // ---------------------------------------------------------------------------
  // AltGr characters (Linux/WebKitGTK)
  // AltGr is emitted as ctrlKey=true AND altKey=true simultaneously.
  // event.key already holds the fully resolved level-3 (or level-4) character
  // from XKB. Detect via getModifierState('AltGraph') to distinguish AltGr from
  // a genuine Ctrl+Alt combination, then transmit the character directly to the PTY.
  // ---------------------------------------------------------------------------
  if (key.length === 1 && ctrlKey && altKey && event.getModifierState('AltGraph')) {
    const cp = key.codePointAt(0);
    if (cp !== undefined && cp >= 0x20 && cp !== 0x7f) {
      return encode(key);
    }
  }

  // Unrecognised key — do not consume
  return null;
}

// ---------------------------------------------------------------------------
// Standalone helper exports (used by tests and higher-level components)
// ---------------------------------------------------------------------------

/**
 * Encode a keypad key event to a VT sequence when DECKPAM is active.
 * Only fires for physical numpad keys (event.code starts with 'Numpad').
 * Returns null in numeric mode or for non-numpad keys.
 */
export function keypadToVtSequence(event: KeyboardEvent, deckpam: boolean): Uint8Array | null {
  if (!deckpam) return null;
  if (!event.code.startsWith('Numpad')) return null;
  if (event.ctrlKey || event.altKey || event.shiftKey) return null;
  switch (event.key) {
    case '0':
      return encode('\x1bOp');
    case '1':
      return encode('\x1bOq');
    case '2':
      return encode('\x1bOr');
    case '3':
      return encode('\x1bOs');
    case '4':
      return encode('\x1bOt');
    case '5':
      return encode('\x1bOu');
    case '6':
      return encode('\x1bOv');
    case '7':
      return encode('\x1bOw');
    case '8':
      return encode('\x1bOx');
    case '9':
      return encode('\x1bOy');
    case '.':
      return encode('\x1bOn');
    case '+':
      return encode('\x1bOl');
    case '-':
      return encode('\x1bOm');
    case '*':
      return encode('\x1bOj');
    case '/':
      return encode('\x1bOo');
    case 'Enter':
      return encode('\x1bOM');
    default:
      return null;
  }
}

/** Focus-in sequence (DECSET 1004): ESC [ I */
export function encodeFocusIn(): Uint8Array {
  return encode('\x1b[I');
}

/** Focus-out sequence (DECSET 1004): ESC [ O */
export function encodeFocusOut(): Uint8Array {
  return encode('\x1b[O');
}

/**
 * Returns true if the event is Ctrl+Shift+V (paste shortcut).
 */
export function isCtrlShiftV(event: KeyboardEvent): boolean {
  return event.ctrlKey && event.shiftKey && (event.key === 'v' || event.key === 'V');
}

/**
 * Wrap text in bracketed paste markers (ESC[200~ … ESC[201~).
 * Strips embedded ESC[201~ (SEC-BLK-012) and null bytes (SEC-BLK-014) from the payload.
 */
export function wrapBracketedPaste(text: string): string {
  const sanitized = text.replaceAll('\x00', '').replaceAll('\x1b[201~', '');
  return '\x1b[200~' + sanitized + '\x1b[201~';
}
