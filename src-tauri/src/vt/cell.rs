// SPDX-License-Identifier: MPL-2.0

//! Terminal cell and attribute types.
//!
//! A `Cell` is a single character position in the terminal grid. It holds:
//! - A grapheme cluster (the visible character, possibly multi-codepoint)
//! - `CellAttrs`: SGR attributes (colors, bold, italic, etc.)
//! - Width: 1 for normal, 2 for wide (CJK), 0 for the phantom cell that follows a wide char
//!
//! All types are `Copy` + `Clone` + `PartialEq` so they can be diffed efficiently
//! for dirty-cell tracking and screen snapshot generation.

use serde::{Deserialize, Serialize};

/// A single cell in the terminal grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The grapheme cluster displayed in this cell.
    /// Empty string for phantom cells following wide characters.
    pub grapheme: String,
    /// SGR attributes.
    pub attrs: CellAttrs,
    /// Display width: 1 for normal, 2 for wide (CJK/emoji), 0 for phantom.
    pub width: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            grapheme: " ".to_string(),
            attrs: CellAttrs::default(),
            width: 1,
        }
    }
}

impl Cell {
    /// Create an empty cell (space character with default attributes).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a phantom cell that follows a wide character.
    pub fn phantom() -> Self {
        Self {
            grapheme: String::new(),
            attrs: CellAttrs::default(),
            width: 0,
        }
    }

    /// Returns `true` if this cell is the phantom placeholder for a wide char.
    pub fn is_phantom(&self) -> bool {
        self.width == 0
    }
}

/// SGR (Select Graphic Rendition) text attributes for a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellAttrs {
    /// Foreground color. `None` = default terminal foreground.
    pub fg: Option<Color>,
    /// Background color. `None` = default terminal background.
    pub bg: Option<Color>,
    /// SGR 1: bold / increased intensity.
    pub bold: bool,
    /// SGR 2: dim / faint.
    pub dim: bool,
    /// SGR 3: italic.
    pub italic: bool,
    /// Underline style: 0 = none, 1 = single, 2 = double, 3 = curly,
    /// 4 = dotted, 5 = dashed (SGR 4:0–4:5).
    pub underline: u8,
    /// SGR 5 / SGR 6: blink (slow and rapid treated identically per §5.3).
    pub blink: bool,
    /// SGR 7: reverse video.
    pub inverse: bool,
    /// SGR 8: hidden / invisible.
    pub hidden: bool,
    /// SGR 9: strikethrough.
    pub strikethrough: bool,
    /// SGR 58: underline color.
    pub underline_color: Option<Color>,
}

impl CellAttrs {
    /// Reset all attributes to default (SGR 0).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Terminal color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Color {
    /// One of the 16 standard ANSI colors (palette index 0–15).
    /// Colors 0–7 = normal, 8–15 = bright.
    Ansi { index: u8 },
    /// One of the 256 xterm colors (palette index 0–255).
    Ansi256 { index: u8 },
    /// 24-bit truecolor (ITU T.416 / SGR 38;2;R;G;B).
    Rgb { r: u8, g: u8, b: u8 },
}

/// Optional hyperlink attached to a cell (OSC 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperlink {
    /// The URI of the hyperlink. Validated against the scheme whitelist (§8.1).
    pub uri: String,
    /// Optional ID parameter from OSC 8 (for multi-cell hyperlinks).
    pub id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Cell construction ---

    #[test]
    fn default_cell_is_space_width_one() {
        let cell = Cell::default();
        assert_eq!(cell.grapheme, " ");
        assert_eq!(cell.width, 1);
        assert!(!cell.is_phantom());
    }

    #[test]
    fn empty_cell_equals_default() {
        assert_eq!(Cell::empty(), Cell::default());
    }

    #[test]
    fn phantom_cell_has_width_zero_and_empty_grapheme() {
        let cell = Cell::phantom();
        assert_eq!(cell.width, 0);
        assert_eq!(cell.grapheme, "");
        assert!(cell.is_phantom());
    }

    #[test]
    fn non_phantom_cell_is_not_phantom() {
        let cell = Cell::empty();
        assert!(!cell.is_phantom());
    }

    // --- CellAttrs ---

    #[test]
    fn default_attrs_have_no_colors_and_all_flags_false() {
        let attrs = CellAttrs::default();
        assert!(attrs.fg.is_none());
        assert!(attrs.bg.is_none());
        assert!(!attrs.bold);
        assert!(!attrs.dim);
        assert!(!attrs.italic);
        assert_eq!(attrs.underline, 0);
        assert!(!attrs.blink);
        assert!(!attrs.inverse);
        assert!(!attrs.hidden);
        assert!(!attrs.strikethrough);
        assert!(attrs.underline_color.is_none());
    }

    #[test]
    fn attrs_reset_clears_all_fields() {
        let mut attrs = CellAttrs {
            bold: true,
            italic: true,
            fg: Some(Color::Ansi { index: 1 }),
            underline: 2,
            ..CellAttrs::default()
        };
        attrs.reset();
        assert_eq!(attrs, CellAttrs::default());
    }

    // --- Color ---

    #[test]
    fn color_variants_are_distinct() {
        let ansi = Color::Ansi { index: 0 };
        let ansi256 = Color::Ansi256 { index: 0 };
        let rgb = Color::Rgb { r: 0, g: 0, b: 0 };
        assert_ne!(ansi, ansi256);
        assert_ne!(ansi, rgb);
        assert_ne!(ansi256, rgb);
    }

    #[test]
    fn color_ansi_index_boundary_values() {
        // ANSI 0–15: normal/bright palette
        let low = Color::Ansi { index: 0 };
        let high = Color::Ansi { index: 15 };
        assert_ne!(low, high);
    }

    #[test]
    fn color_rgb_round_trips_through_clone() {
        let color = Color::Rgb {
            r: 255,
            g: 128,
            b: 0,
        };
        assert_eq!(color, color.clone());
    }
}
