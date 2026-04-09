// SPDX-License-Identifier: MPL-2.0

//! Mouse event encoding for terminal applications.
//!
//! Encodes mouse events (button, position, modifiers) into the byte sequences
//! expected by the application running in the PTY, according to the active mode:
//!
//! - X10 encoding (default): `ESC [ M <cb+32> <cx+32> <cy+32>` (limited to col/row ≤ 223)
//! - SGR encoding (mode 1006): `ESC [ < <cb> ; <cx> ; <cy> M|m`
//! - URXVT encoding (mode 1015): `ESC [ <cb+32> ; <cx> ; <cy> M`
//!
//! Mode arbitration (FS-VT-081, §3.2):
//! If SGR (1006) is active → encode as SGR regardless of other modes.
//! Else if URXVT (1015) active → encode as URXVT.
//! Else encode as X10 (limited to col/row ≤ 223).

use crate::vt::modes::MouseEncoding;

/// A mouse event from the frontend, ready to be encoded for the PTY.
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    /// X position (1-based column).
    pub col: u32,
    /// Y position (1-based row).
    pub row: u32,
    /// Mouse button (0 = left, 1 = middle, 2 = right, 64/65 = wheel up/down).
    pub button: u8,
    /// `true` for button press, `false` for release.
    pub is_press: bool,
    /// Shift modifier held.
    pub shift: bool,
    /// Alt/Meta modifier held.
    pub alt: bool,
    /// Control modifier held.
    pub ctrl: bool,
    /// `true` if this is a motion event (no button state change).
    pub is_motion: bool,
}

impl MouseEvent {
    /// Encode this event into the bytes sequence to write to the PTY,
    /// using the given encoding mode.
    pub fn encode(&self, encoding: MouseEncoding) -> Vec<u8> {
        let cb = self.control_byte();

        match encoding {
            MouseEncoding::Sgr => self.encode_sgr(cb),
            MouseEncoding::Urxvt => self.encode_urxvt(cb),
            MouseEncoding::X10 => self.encode_x10(cb),
        }
    }

    fn control_byte(&self) -> u8 {
        let button_bits: u8 = match self.button {
            0 => 0,
            1 => 1,
            2 => 2,
            64 => 64, // wheel up
            65 => 65, // wheel down
            _ => 3,   // release or unknown
        };
        let modifier_bits = (if self.shift { 4 } else { 0 })
            | (if self.alt { 8 } else { 0 })
            | (if self.ctrl { 16 } else { 0 })
            | (if self.is_motion { 32 } else { 0 });
        button_bits | modifier_bits
    }

    /// SGR encoding: `ESC [ < cb ; cx ; cy M|m`
    ///
    /// # Invariant
    /// The result is always valid UTF-8: `format!` on integer fields (`u8`, `u32`)
    /// and ASCII byte literals cast to `char` (`b'M'`/`b'm'`) produce only ASCII
    /// codepoints, which are a strict subset of valid UTF-8.
    fn encode_sgr(&self, cb: u8) -> Vec<u8> {
        let trailer = if self.is_press { b'M' } else { b'm' };
        format!("\x1b[<{};{};{}{}", cb, self.col, self.row, trailer as char).into_bytes()
    }

    /// URXVT encoding: `ESC [ cb+32 ; cx ; cy M`
    ///
    /// # Invariant
    /// The result is always valid UTF-8: `format!` on integer fields produces only
    /// ASCII codepoints, which are a strict subset of valid UTF-8.
    fn encode_urxvt(&self, cb: u8) -> Vec<u8> {
        format!("\x1b[{};{};{}M", cb as u32 + 32, self.col, self.row).into_bytes()
    }

    /// X10 encoding: `ESC [ M <cb+32> <cx+32> <cy+32>` (limited to ≤ 223)
    fn encode_x10(&self, cb: u8) -> Vec<u8> {
        // Clamp coordinates to the X10 limit.
        let cx = self.col.min(223) as u8 + 32;
        let cy = self.row.min(223) as u8 + 32;
        vec![0x1b, b'[', b'M', cb + 32, cx, cy]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left_press(col: u32, row: u32) -> MouseEvent {
        MouseEvent {
            col,
            row,
            button: 0,
            is_press: true,
            shift: false,
            alt: false,
            ctrl: false,
            is_motion: false,
        }
    }

    // ---------------------------------------------------------------------------
    // TEST-VT-016 (partial) — mouse encoding
    // FS-VT-080, FS-VT-082, FS-VT-083
    // ---------------------------------------------------------------------------

    #[test]
    fn x10_encoding_produces_correct_bytes() {
        // TEST-VT-016 step 3: left-click at (10, 5) encoded as X10.
        let ev = left_press(10, 5);
        let bytes = ev.encode(MouseEncoding::X10);
        // ESC [ M <cb+32> <cx+32> <cy+32>
        // cb = 0 (left button, press), cx = 10+32=42, cy = 5+32=37
        assert_eq!(bytes, vec![0x1b, b'[', b'M', 32, 42, 37]);
    }

    #[test]
    fn sgr_encoding_left_press_correct() {
        let ev = left_press(10, 5);
        // SAFETY (UTF-8): encode_sgr produces ASCII-only bytes via format! on integer
        // fields and ASCII byte literals; ASCII is always valid UTF-8.
        let s = String::from_utf8(ev.encode(MouseEncoding::Sgr))
            .expect("encode_sgr always produces valid UTF-8 (ASCII-only)");
        // ESC [ < 0 ; 10 ; 5 M
        assert_eq!(s, "\x1b[<0;10;5M");
    }

    #[test]
    fn sgr_encoding_left_release_uses_lowercase_m() {
        let ev = MouseEvent {
            col: 10,
            row: 5,
            button: 0,
            is_press: false,
            shift: false,
            alt: false,
            ctrl: false,
            is_motion: false,
        };
        // SAFETY (UTF-8): encode_sgr produces ASCII-only bytes; ASCII is always valid UTF-8.
        let s = String::from_utf8(ev.encode(MouseEncoding::Sgr))
            .expect("encode_sgr always produces valid UTF-8 (ASCII-only)");
        assert!(s.ends_with('m'), "release must use lowercase 'm'");
    }

    #[test]
    fn x10_coordinates_clamped_to_223() {
        // Coordinates > 223 must be clamped.
        let ev = left_press(300, 250);
        let bytes = ev.encode(MouseEncoding::X10);
        // cx = min(300, 223) + 32 = 255, cy = min(250, 223) + 32 = 255
        assert_eq!(bytes[4], 255); // cx clamped
        assert_eq!(bytes[5], 255); // cy clamped
    }

    #[test]
    fn urxvt_encoding_correct_format() {
        let ev = left_press(10, 5);
        // SAFETY (UTF-8): encode_urxvt produces ASCII-only bytes via format! on integer
        // fields; ASCII is always valid UTF-8.
        let s = String::from_utf8(ev.encode(MouseEncoding::Urxvt))
            .expect("encode_urxvt always produces valid UTF-8 (ASCII-only)");
        // ESC [ 32 ; 10 ; 5 M  (cb=0, 0+32=32)
        assert_eq!(s, "\x1b[32;10;5M");
    }

    #[test]
    fn shift_modifier_sets_bit4_in_control_byte() {
        let ev = MouseEvent {
            col: 1,
            row: 1,
            button: 0,
            is_press: true,
            shift: true,
            alt: false,
            ctrl: false,
            is_motion: false,
        };
        // SAFETY (UTF-8): encode_sgr produces ASCII-only bytes; ASCII is always valid UTF-8.
        let s = String::from_utf8(ev.encode(MouseEncoding::Sgr))
            .expect("encode_sgr always produces valid UTF-8 (ASCII-only)");
        // cb = 0 | 4 (shift) = 4
        assert!(
            s.contains("<4;"),
            "shift bit must be set in SGR cb (got: {s})"
        );
    }
}
