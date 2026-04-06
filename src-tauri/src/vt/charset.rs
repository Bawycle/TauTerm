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

#[cfg(test)]
mod tests {
    use super::translate_dec_special;

    // All 27 DEC Special Graphics mappings (bytes 0x60–0x7E).
    // Verified against the DEC Special Graphics character set table.

    #[test]
    fn mapping_0x60_diamond() {
        assert_eq!(translate_dec_special(0x60), '\u{25C6}'); // ◆
    }

    #[test]
    fn mapping_0x61_checkerboard() {
        assert_eq!(translate_dec_special(0x61), '\u{2592}'); // ▒
    }

    #[test]
    fn mapping_0x62_ht_symbol() {
        assert_eq!(translate_dec_special(0x62), '\u{2409}'); // ␉
    }

    #[test]
    fn mapping_0x63_ff_symbol() {
        assert_eq!(translate_dec_special(0x63), '\u{240C}'); // ␌
    }

    #[test]
    fn mapping_0x64_cr_symbol() {
        assert_eq!(translate_dec_special(0x64), '\u{240D}'); // ␍
    }

    #[test]
    fn mapping_0x65_lf_symbol() {
        assert_eq!(translate_dec_special(0x65), '\u{240A}'); // ␊
    }

    #[test]
    fn mapping_0x66_degree_sign() {
        assert_eq!(translate_dec_special(0x66), '\u{00B0}'); // °
    }

    #[test]
    fn mapping_0x67_plus_minus() {
        assert_eq!(translate_dec_special(0x67), '\u{00B1}'); // ±
    }

    #[test]
    fn mapping_0x68_nl_symbol() {
        assert_eq!(translate_dec_special(0x68), '\u{2424}'); // ␤
    }

    #[test]
    fn mapping_0x69_vt_symbol() {
        assert_eq!(translate_dec_special(0x69), '\u{240B}'); // ␋
    }

    #[test]
    fn mapping_0x6a_lower_right_corner() {
        assert_eq!(translate_dec_special(0x6A), '\u{2518}'); // ┘
    }

    #[test]
    fn mapping_0x6b_upper_right_corner() {
        assert_eq!(translate_dec_special(0x6B), '\u{2510}'); // ┐
    }

    #[test]
    fn mapping_0x6c_upper_left_corner() {
        assert_eq!(translate_dec_special(0x6C), '\u{250C}'); // ┌
    }

    #[test]
    fn mapping_0x6d_lower_left_corner() {
        assert_eq!(translate_dec_special(0x6D), '\u{2514}'); // └
    }

    #[test]
    fn mapping_0x6e_cross() {
        assert_eq!(translate_dec_special(0x6E), '\u{253C}'); // ┼
    }

    #[test]
    fn mapping_0x6f_scan_line_1() {
        assert_eq!(translate_dec_special(0x6F), '\u{23BA}'); // ⎺
    }

    #[test]
    fn mapping_0x70_scan_line_3() {
        assert_eq!(translate_dec_special(0x70), '\u{23BB}'); // ⎻
    }

    #[test]
    fn mapping_0x71_horizontal_line() {
        assert_eq!(translate_dec_special(0x71), '\u{2500}'); // ─
    }

    #[test]
    fn mapping_0x72_scan_line_7() {
        assert_eq!(translate_dec_special(0x72), '\u{23BC}'); // ⎼
    }

    #[test]
    fn mapping_0x73_scan_line_9() {
        assert_eq!(translate_dec_special(0x73), '\u{23BD}'); // ⎽
    }

    #[test]
    fn mapping_0x74_left_tee() {
        assert_eq!(translate_dec_special(0x74), '\u{251C}'); // ├
    }

    #[test]
    fn mapping_0x75_right_tee() {
        assert_eq!(translate_dec_special(0x75), '\u{2524}'); // ┤
    }

    #[test]
    fn mapping_0x76_bottom_tee() {
        assert_eq!(translate_dec_special(0x76), '\u{2534}'); // ┴
    }

    #[test]
    fn mapping_0x77_top_tee() {
        assert_eq!(translate_dec_special(0x77), '\u{252C}'); // ┬
    }

    #[test]
    fn mapping_0x78_vertical_line() {
        assert_eq!(translate_dec_special(0x78), '\u{2502}'); // │
    }

    #[test]
    fn mapping_0x79_less_than_or_equal() {
        assert_eq!(translate_dec_special(0x79), '\u{2264}'); // ≤
    }

    #[test]
    fn mapping_0x7a_greater_than_or_equal() {
        assert_eq!(translate_dec_special(0x7A), '\u{2265}'); // ≥
    }

    #[test]
    fn mapping_0x7b_pi() {
        assert_eq!(translate_dec_special(0x7B), '\u{03C0}'); // π
    }

    #[test]
    fn mapping_0x7c_not_equal() {
        assert_eq!(translate_dec_special(0x7C), '\u{2260}'); // ≠
    }

    #[test]
    fn mapping_0x7d_pound_sign() {
        assert_eq!(translate_dec_special(0x7D), '\u{00A3}'); // £
    }

    #[test]
    fn mapping_0x7e_middle_dot() {
        assert_eq!(translate_dec_special(0x7E), '\u{00B7}'); // ·
    }

    // Bytes outside the DEC Special Graphics range pass through unchanged.

    #[test]
    fn passthrough_below_range() {
        // 0x5F is below 0x60 — passes through as-is.
        assert_eq!(translate_dec_special(0x5F), '_');
    }

    #[test]
    fn passthrough_above_range() {
        // 0x7F is above 0x7E — passes through as-is.
        assert_eq!(translate_dec_special(0x7F), '\x7F');
    }

    #[test]
    fn passthrough_printable_ascii_below_range() {
        // Bytes below 0x60 are not in the DEC Special Graphics range — they pass through.
        assert_eq!(translate_dec_special(b'A'), 'A'); // 0x41
        assert_eq!(translate_dec_special(b'Z'), 'Z'); // 0x5A
        assert_eq!(translate_dec_special(b'!'), '!'); // 0x21
        assert_eq!(translate_dec_special(b'0'), '0'); // 0x30
    }
}
