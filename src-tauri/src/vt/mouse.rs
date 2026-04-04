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
    fn encode_sgr(&self, cb: u8) -> Vec<u8> {
        let trailer = if self.is_press { b'M' } else { b'm' };
        format!("\x1b[<{};{};{}{}", cb, self.col, self.row, trailer as char).into_bytes()
    }

    /// URXVT encoding: `ESC [ cb+32 ; cx ; cy M`
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
