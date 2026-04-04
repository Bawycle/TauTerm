// SPDX-License-Identifier: MPL-2.0

//! DEC Special Graphics character set mapping (SI/SO, ESC ( 0 / ESC ( B).
//!
//! Maps bytes 0x60–0x7E from the DEC Special Graphics set to the
//! corresponding Unicode line-drawing characters.
//! Used by `VtProcessor` when `g0/g1 == Charset::DecSpecialGraphics`.

/// Translate a byte from the DEC Special Graphics character set to its
/// Unicode equivalent. Returns the original character if no mapping exists.
pub fn translate_dec_special(byte: u8) -> char {
    // The DEC Special Graphics set defines substitutions in the range 0x5F–0x7E.
    // The standard line-drawing chars are at 0x60–0x7E.
    match byte {
        0x60 => '\u{25C6}', // ◆  diamond
        0x61 => '\u{2592}', // ▒  checkerboard
        0x62 => '\u{2409}', // ␉  HT symbol
        0x63 => '\u{240C}', // ␌  FF symbol
        0x64 => '\u{240D}', // ␍  CR symbol
        0x65 => '\u{240A}', // ␊  LF symbol
        0x66 => '\u{00B0}', // °  degree sign
        0x67 => '\u{00B1}', // ±  plus/minus
        0x68 => '\u{2424}', // ␤  NL symbol
        0x69 => '\u{240B}', // ␋  VT symbol
        0x6A => '\u{2518}', // ┘  lower-right corner
        0x6B => '\u{2510}', // ┐  upper-right corner
        0x6C => '\u{250C}', // ┌  upper-left corner
        0x6D => '\u{2514}', // └  lower-left corner
        0x6E => '\u{253C}', // ┼  cross
        0x6F => '\u{23BA}', // ⎺  scan line 1
        0x70 => '\u{23BB}', // ⎻  scan line 3
        0x71 => '\u{2500}', // ─  horizontal line
        0x72 => '\u{23BC}', // ⎼  scan line 7
        0x73 => '\u{23BD}', // ⎽  scan line 9
        0x74 => '\u{251C}', // ├  left tee
        0x75 => '\u{2524}', // ┤  right tee
        0x76 => '\u{2534}', // ┴  bottom tee
        0x77 => '\u{252C}', // ┬  top tee
        0x78 => '\u{2502}', // │  vertical line
        0x79 => '\u{2264}', // ≤  less-than-or-equal
        0x7A => '\u{2265}', // ≥  greater-than-or-equal
        0x7B => '\u{03C0}', // π  pi
        0x7C => '\u{2260}', // ≠  not equal
        0x7D => '\u{00A3}', // £  pound sign
        0x7E => '\u{00B7}', // ·  middle dot
        other => other as char,
    }
}
