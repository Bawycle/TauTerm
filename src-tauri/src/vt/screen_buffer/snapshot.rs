// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use crate::vt::cell::Cell;

/// A snapshot of the visible screen content, used for `get_pane_screen_snapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenSnapshot {
    pub cols: u16,
    pub rows: u16,
    /// Row-major flat array: rows × cols cells.
    pub cells: Vec<SnapshotCell>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_visible: bool,
    pub cursor_shape: u8,
    pub scrollback_lines: usize,
    pub scroll_offset: i64,
}

/// A single cell in a screen snapshot, serialized to the frontend for rendering.
///
/// # Frontend rendering contracts
///
/// The frontend MUST respect the following rules when interpreting a `SnapshotCell`:
///
/// ## Bold color promotion
/// When `bold == true` and `fg` is `Color::Ansi { index }` with `index` in `[1, 7]`
/// (the 7 non-black standard colors), the frontend MUST resolve the displayed color
/// using `index + 8` (the bright variant). Index 0 (black) is **excluded** from
/// promotion — bold black renders as ordinary black.
///
/// ## Dim (faint)
/// When `dim == true`, the frontend MUST apply `opacity: var(--term-dim-opacity)`
/// (design token value: 0.5) to the foreground color. Dim is independent of `bold`
/// and applies after bold color promotion.
///
/// ## Reverse video
/// When `inverse == true`, the frontend MUST swap the resolved foreground and
/// background colors. The swap operates on the **resolved** values — i.e. after
/// bold color promotion has been applied to `fg`, and after substituting terminal
/// defaults for any `None` color.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotCell {
    pub content: String,
    pub width: u8,
    // Attributes encoded as per CellAttrsDto (re-encoded at snapshot time).
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: u8,
    pub blink: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<crate::vt::cell::Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<crate::vt::cell::Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_color: Option<crate::vt::cell::Color>,
    /// OSC 8 hyperlink URI for this cell, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperlink: Option<String>,
}

impl From<&Cell> for SnapshotCell {
    fn from(cell: &Cell) -> Self {
        Self {
            content: cell.grapheme.to_string(),
            width: cell.width,
            bold: cell.attrs.bold,
            dim: cell.attrs.dim,
            italic: cell.attrs.italic,
            underline: cell.attrs.underline,
            blink: cell.attrs.blink,
            inverse: cell.attrs.inverse,
            hidden: cell.attrs.hidden,
            strikethrough: cell.attrs.strikethrough,
            fg: cell.attrs.fg,
            bg: cell.attrs.bg,
            underline_color: cell.attrs.underline_color,
            hyperlink: cell.hyperlink.as_ref().map(|h| h.as_ref().to_owned()),
        }
    }
}
