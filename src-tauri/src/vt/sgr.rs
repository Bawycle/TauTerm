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
