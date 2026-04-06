// SPDX-License-Identifier: MPL-2.0

//! SGR (Select Graphic Rendition) attribute parsing.
//!
//! Parses CSI `Pm m` parameter lists into `CellAttrs` deltas.
//! Supports:
//! - SGR 0: reset all
//! - SGR 1–9 / 22–29: basic attributes (bold, dim, italic, underline, blink, inverse, hidden, strikethrough)
//! - SGR 30–37 / 39: ANSI foreground
//! - SGR 40–47 / 49: ANSI background
//! - SGR 90–97: bright foreground
//! - SGR 100–107: bright background
//! - SGR 38;5;N / 48;5;N: 256-color (semicolon form)
//! - SGR 38;2;R;G;B / 48;2;R;G;B: truecolor (semicolon form)
//! - SGR 38:2:R:G:B / 48:2:R:G:B: truecolor (colon sub-parameter form, ITU T.416)
//! - SGR 4:0–4:5: extended underline styles
//! - SGR 58/59: underline color set/reset

use crate::vt::cell::{CellAttrs, Color};

/// Apply a parsed SGR parameter list to `attrs`.
/// `params` is the raw slice as received by `vte::Perform::csi_dispatch`.
pub fn apply_sgr(params: &vte::Params, attrs: &mut CellAttrs) {
    let mut iter = params.iter();

    while let Some(param) = iter.next() {
        // A `vte` param sub-slice can contain colon-separated sub-parameters.
        let first = param.first().copied().unwrap_or(0);

        match first {
            0 => attrs.reset(),
            1 => attrs.bold = true,
            2 => attrs.dim = true,
            3 => attrs.italic = true,
            4 => {
                // Underline style — may have sub-param (colon form): 4:0–4:5
                let style = param.get(1).copied().unwrap_or(1);
                attrs.underline = style as u8;
            }
            5 | 6 => attrs.blink = true, // slow / rapid blink treated identically
            7 => attrs.inverse = true,
            8 => attrs.hidden = true,
            9 => attrs.strikethrough = true,
            22 => {
                attrs.bold = false;
                attrs.dim = false;
            }
            23 => attrs.italic = false,
            24 => attrs.underline = 0,
            25 => attrs.blink = false,
            27 => attrs.inverse = false,
            28 => attrs.hidden = false,
            29 => attrs.strikethrough = false,
            // ANSI foreground colors 30–37.
            30..=37 => {
                attrs.fg = Some(Color::Ansi {
                    index: (first - 30) as u8,
                })
            }
            38 => {
                // Extended foreground color: 38;5;N or 38;2;R;G;B or colon sub-params.
                let color = parse_extended_color(param, &mut iter);
                if let Some(c) = color {
                    attrs.fg = Some(c);
                }
            }
            39 => attrs.fg = None,
            // ANSI background colors 40–47.
            40..=47 => {
                attrs.bg = Some(Color::Ansi {
                    index: (first - 40) as u8,
                })
            }
            48 => {
                let color = parse_extended_color(param, &mut iter);
                if let Some(c) = color {
                    attrs.bg = Some(c);
                }
            }
            49 => attrs.bg = None,
            58 => {
                let color = parse_extended_color(param, &mut iter);
                if let Some(c) = color {
                    attrs.underline_color = Some(c);
                }
            }
            59 => attrs.underline_color = None,
            // Bright foreground colors 90–97.
            90..=97 => {
                attrs.fg = Some(Color::Ansi {
                    index: (first - 90 + 8) as u8,
                })
            }
            // Bright background colors 100–107.
            100..=107 => {
                attrs.bg = Some(Color::Ansi {
                    index: (first - 100 + 8) as u8,
                })
            }
            _ => {} // Unknown SGR parameter — ignore.
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests for apply_sgr — tested indirectly via VtProcessor.
// Direct testing of apply_sgr would require constructing vte::Params, which
// does not implement Clone and has no public constructor. Tests exercise SGR
// behavior end-to-end: feed a CSI sequence to VtProcessor, write a character,
// then inspect the cell attributes in the snapshot.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::vt::{
        VtProcessor,
        cell::{CellAttrs, Color},
    };

    /// Create a standard 80×24 VtProcessor.
    fn make_vt() -> VtProcessor {
        VtProcessor::new(80, 24, 1_000)
    }

    /// Write a CSI SGR sequence followed by a printable character, then return
    /// the `CellAttrs` of the cell at (0, 0).
    fn attrs_after_sgr(sgr_params: &str) -> CellAttrs {
        let mut vt = make_vt();
        // CSI {params} m
        let seq = format!("\x1b[{sgr_params}m");
        vt.process(seq.as_bytes());
        vt.process(b"X");
        vt.get_snapshot()
            .cells
            .first()
            .map(|c| CellAttrs {
                bold: c.bold,
                dim: c.dim,
                italic: c.italic,
                underline: c.underline,
                blink: c.blink,
                inverse: c.inverse,
                hidden: c.hidden,
                strikethrough: c.strikethrough,
                fg: c.fg,
                bg: c.bg,
                underline_color: c.underline_color,
            })
            .unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // SGR 0 — reset
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_0_resets_all_attributes() {
        let mut vt = make_vt();
        // Set bold + italic first.
        vt.process(b"\x1b[1;3m");
        // Then reset.
        vt.process(b"\x1b[0m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        let c = snap.cells.first().unwrap();
        assert!(!c.bold, "SGR 0 must reset bold");
        assert!(!c.italic, "SGR 0 must reset italic");
        assert!(c.fg.is_none(), "SGR 0 must clear fg");
        assert!(c.bg.is_none(), "SGR 0 must clear bg");
    }

    #[test]
    fn sgr_empty_resets_all_attributes() {
        // CSI m (no params) is equivalent to SGR 0.
        let mut vt = make_vt();
        vt.process(b"\x1b[1m");
        vt.process(b"\x1b[m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        let c = snap.cells.first().unwrap();
        assert!(!c.bold, "Empty SGR must reset bold");
    }

    // -----------------------------------------------------------------------
    // Text attributes
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_1_bold() {
        let a = attrs_after_sgr("1");
        assert!(a.bold, "SGR 1 must set bold");
        assert!(!a.dim);
    }

    #[test]
    fn sgr_2_dim() {
        let a = attrs_after_sgr("2");
        assert!(a.dim, "SGR 2 must set dim");
        assert!(!a.bold);
    }

    #[test]
    fn sgr_3_italic() {
        let a = attrs_after_sgr("3");
        assert!(a.italic, "SGR 3 must set italic");
    }

    #[test]
    fn sgr_4_single_underline() {
        let a = attrs_after_sgr("4");
        assert_eq!(a.underline, 1, "SGR 4 must set underline=1 (single)");
    }

    #[test]
    fn sgr_5_blink() {
        let a = attrs_after_sgr("5");
        assert!(a.blink, "SGR 5 must set blink");
    }

    #[test]
    fn sgr_6_rapid_blink_same_as_blink() {
        let a = attrs_after_sgr("6");
        assert!(
            a.blink,
            "SGR 6 must set blink (rapid blink treated identically)"
        );
    }

    #[test]
    fn sgr_7_inverse() {
        let a = attrs_after_sgr("7");
        assert!(a.inverse, "SGR 7 must set inverse");
    }

    #[test]
    fn sgr_8_hidden() {
        let a = attrs_after_sgr("8");
        assert!(a.hidden, "SGR 8 must set hidden");
    }

    #[test]
    fn sgr_9_strikethrough() {
        let a = attrs_after_sgr("9");
        assert!(a.strikethrough, "SGR 9 must set strikethrough");
    }

    // -----------------------------------------------------------------------
    // Attribute reset codes
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_22_resets_bold_and_dim() {
        let mut vt = make_vt();
        vt.process(b"\x1b[1;2m");
        vt.process(b"\x1b[22m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        let c = snap.cells.first().unwrap();
        assert!(!c.bold, "SGR 22 must reset bold");
        assert!(!c.dim, "SGR 22 must reset dim");
    }

    #[test]
    fn sgr_23_resets_italic() {
        let mut vt = make_vt();
        vt.process(b"\x1b[3m");
        vt.process(b"\x1b[23m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert!(
            !snap.cells.first().unwrap().italic,
            "SGR 23 must reset italic"
        );
    }

    #[test]
    fn sgr_24_resets_underline() {
        let mut vt = make_vt();
        vt.process(b"\x1b[4m");
        vt.process(b"\x1b[24m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert_eq!(
            snap.cells.first().unwrap().underline,
            0,
            "SGR 24 must reset underline"
        );
    }

    #[test]
    fn sgr_25_resets_blink() {
        let mut vt = make_vt();
        vt.process(b"\x1b[5m");
        vt.process(b"\x1b[25m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert!(
            !snap.cells.first().unwrap().blink,
            "SGR 25 must reset blink"
        );
    }

    #[test]
    fn sgr_27_resets_inverse() {
        let mut vt = make_vt();
        vt.process(b"\x1b[7m");
        vt.process(b"\x1b[27m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert!(
            !snap.cells.first().unwrap().inverse,
            "SGR 27 must reset inverse"
        );
    }

    #[test]
    fn sgr_28_resets_hidden() {
        let mut vt = make_vt();
        vt.process(b"\x1b[8m");
        vt.process(b"\x1b[28m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert!(
            !snap.cells.first().unwrap().hidden,
            "SGR 28 must reset hidden"
        );
    }

    #[test]
    fn sgr_29_resets_strikethrough() {
        let mut vt = make_vt();
        vt.process(b"\x1b[9m");
        vt.process(b"\x1b[29m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert!(
            !snap.cells.first().unwrap().strikethrough,
            "SGR 29 must reset strikethrough"
        );
    }

    // -----------------------------------------------------------------------
    // Foreground colors — ANSI 3-bit (30–37, 39) and bright (90–97)
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_30_through_37_ansi_foreground() {
        for i in 0u8..8 {
            let a = attrs_after_sgr(&(30 + i as u16).to_string());
            assert_eq!(
                a.fg,
                Some(Color::Ansi { index: i }),
                "SGR {} must set fg to Ansi({})",
                30 + i,
                i
            );
        }
    }

    #[test]
    fn sgr_39_resets_foreground() {
        let a = attrs_after_sgr("31;39");
        assert!(a.fg.is_none(), "SGR 39 must clear foreground color");
    }

    #[test]
    fn sgr_90_through_97_bright_foreground() {
        for i in 0u8..8 {
            let a = attrs_after_sgr(&(90 + i as u16).to_string());
            assert_eq!(
                a.fg,
                Some(Color::Ansi { index: 8 + i }),
                "SGR {} must set fg to Ansi({})",
                90 + i,
                8 + i
            );
        }
    }

    // -----------------------------------------------------------------------
    // Background colors — ANSI 3-bit (40–47, 49) and bright (100–107)
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_40_through_47_ansi_background() {
        for i in 0u8..8 {
            let a = attrs_after_sgr(&(40 + i as u16).to_string());
            assert_eq!(
                a.bg,
                Some(Color::Ansi { index: i }),
                "SGR {} must set bg to Ansi({})",
                40 + i,
                i
            );
        }
    }

    #[test]
    fn sgr_49_resets_background() {
        let a = attrs_after_sgr("41;49");
        assert!(a.bg.is_none(), "SGR 49 must clear background color");
    }

    #[test]
    fn sgr_100_through_107_bright_background() {
        for i in 0u8..8 {
            let a = attrs_after_sgr(&(100 + i as u16).to_string());
            assert_eq!(
                a.bg,
                Some(Color::Ansi { index: 8 + i }),
                "SGR {} must set bg to Ansi({})",
                100 + i,
                8 + i
            );
        }
    }

    // -----------------------------------------------------------------------
    // 8-bit (256-color) — semicolon form: 38;5;N and 48;5;N
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_38_5_n_foreground_256_color() {
        let a = attrs_after_sgr("38;5;200");
        assert_eq!(
            a.fg,
            Some(Color::Ansi256 { index: 200 }),
            "SGR 38;5;200 must set fg Ansi256(200)"
        );
    }

    #[test]
    fn sgr_48_5_n_background_256_color() {
        let a = attrs_after_sgr("48;5;42");
        assert_eq!(
            a.bg,
            Some(Color::Ansi256 { index: 42 }),
            "SGR 48;5;42 must set bg Ansi256(42)"
        );
    }

    #[test]
    fn sgr_38_5_boundary_indices() {
        let a0 = attrs_after_sgr("38;5;0");
        assert_eq!(a0.fg, Some(Color::Ansi256 { index: 0 }));
        let a255 = attrs_after_sgr("38;5;255");
        assert_eq!(a255.fg, Some(Color::Ansi256 { index: 255 }));
    }

    // -----------------------------------------------------------------------
    // 24-bit truecolor — semicolon form: 38;2;R;G;B and 48;2;R;G;B
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_38_2_rgb_foreground_truecolor() {
        let a = attrs_after_sgr("38;2;255;128;0");
        assert_eq!(
            a.fg,
            Some(Color::Rgb {
                r: 255,
                g: 128,
                b: 0
            }),
            "SGR 38;2;255;128;0 must set fg Rgb(255,128,0)"
        );
    }

    #[test]
    fn sgr_48_2_rgb_background_truecolor() {
        let a = attrs_after_sgr("48;2;10;20;30");
        assert_eq!(
            a.bg,
            Some(Color::Rgb {
                r: 10,
                g: 20,
                b: 30
            }),
            "SGR 48;2;10;20;30 must set bg Rgb(10,20,30)"
        );
    }

    // -----------------------------------------------------------------------
    // Extended underline styles (SGR 4:N colon sub-param form)
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_4_colon_0_no_underline() {
        // SGR 4:0 = no underline (explicit off).
        let mut vt = make_vt();
        vt.process(b"\x1b[4m"); // single underline on
        // Send 4:0 as colon sub-param (encoded as a single param with sub-param).
        // The vte crate accepts colon sub-params; we encode them with `:`.
        vt.process(b"\x1b[4:0m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert_eq!(
            snap.cells.first().unwrap().underline,
            0,
            "SGR 4:0 must set underline=0"
        );
    }

    #[test]
    fn sgr_4_colon_1_single_underline() {
        let mut vt = make_vt();
        vt.process(b"\x1b[4:1m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert_eq!(
            snap.cells.first().unwrap().underline,
            1,
            "SGR 4:1 must set underline=1"
        );
    }

    #[test]
    fn sgr_4_colon_2_double_underline() {
        let mut vt = make_vt();
        vt.process(b"\x1b[4:2m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert_eq!(
            snap.cells.first().unwrap().underline,
            2,
            "SGR 4:2 must set underline=2"
        );
    }

    #[test]
    fn sgr_4_colon_3_curly_underline() {
        let mut vt = make_vt();
        vt.process(b"\x1b[4:3m");
        vt.process(b"X");
        let snap = vt.get_snapshot();
        assert_eq!(
            snap.cells.first().unwrap().underline,
            3,
            "SGR 4:3 must set underline=3"
        );
    }

    // -----------------------------------------------------------------------
    // Underline color (SGR 58/59)
    // -----------------------------------------------------------------------

    #[test]
    fn sgr_58_sets_underline_color() {
        let a = attrs_after_sgr("58;2;255;0;128");
        assert_eq!(
            a.underline_color,
            Some(Color::Rgb {
                r: 255,
                g: 0,
                b: 128
            }),
            "SGR 58;2;R;G;B must set underline_color"
        );
    }

    #[test]
    fn sgr_59_resets_underline_color() {
        let a = attrs_after_sgr("58;2;255;0;128;59");
        assert!(
            a.underline_color.is_none(),
            "SGR 59 must clear underline_color"
        );
    }

    // -----------------------------------------------------------------------
    // Unknown SGR codes — silently ignored, no change to attributes
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_sgr_code_does_not_change_attributes() {
        // SGR 255 is unrecognised. Attributes must remain at default.
        let a = attrs_after_sgr("255");
        assert_eq!(
            a,
            CellAttrs::default(),
            "Unknown SGR code must not change attributes"
        );
    }

    #[test]
    fn unknown_sgr_code_does_not_corrupt_preceding_attribute() {
        let a = attrs_after_sgr("1;255"); // bold + unknown
        assert!(a.bold, "Known attribute before unknown SGR must survive");
    }
}

/// Parse an extended color value from the current parameter (colon sub-params)
/// or from subsequent parameters (semicolon form).
fn parse_extended_color<'a>(
    param: &'a [u16],
    iter: &mut impl Iterator<Item = &'a [u16]>,
) -> Option<Color> {
    if param.len() >= 2 {
        // Colon sub-parameter form: 38:5:N or 38:2:R:G:B
        match param.get(1).copied() {
            Some(5) => {
                let index = param.get(2).copied()? as u8;
                return Some(Color::Ansi256 { index });
            }
            Some(2) => {
                let r = param.get(2).copied()? as u8;
                let g = param.get(3).copied()? as u8;
                let b = param.get(4).copied()? as u8;
                return Some(Color::Rgb { r, g, b });
            }
            _ => return None,
        }
    }

    // Semicolon form: 38 ; 5 ; N or 38 ; 2 ; R ; G ; B
    let kind = iter.next()?.first().copied()?;
    match kind {
        5 => {
            let index = iter.next()?.first().copied()? as u8;
            Some(Color::Ansi256 { index })
        }
        2 => {
            let r = iter.next()?.first().copied()? as u8;
            let g = iter.next()?.first().copied()? as u8;
            let b = iter.next()?.first().copied()? as u8;
            Some(Color::Rgb { r, g, b })
        }
        _ => None,
    }
}
