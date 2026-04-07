// SPDX-License-Identifier: MPL-2.0

//! Emoji variation selector helpers for `VtProcessor`.
//!
//! Contains `is_emoji_vs_eligible()` and `flush_pending_emoji()`.

use unicode_width::UnicodeWidthChar;

use crate::vt::cell::Cell;

use super::VtProcessor;

/// Returns `true` if `c` is a codepoint that may appear in Unicode
/// `emoji-variation-sequences.txt` — i.e. one that can be followed by U+FE0F
/// (emoji presentation) or U+FE0E (text presentation) to alter its render width.
///
/// Only these codepoints are buffered for variation-selector look-ahead (R6 /
/// FS-VT-017).  All other codepoints — including box-drawing (U+2500–U+257F),
/// Latin extended, and currency symbols — are written immediately.
///
/// Source: Unicode 15.1 `emoji-variation-sequences.txt` (grouped by block).
pub(super) fn is_emoji_vs_eligible(c: char) -> bool {
    // © and ® (Latin-1 Supplement)
    matches!(c, '\u{00A9}' | '\u{00AE}')
    // General Punctuation and Letterlike Symbols
    || matches!(c,
        '\u{203C}' | '\u{2049}' | '\u{2122}' | '\u{2139}'
    )
    // Arrows block
    || matches!(c,
        '\u{2194}'..='\u{2199}'
        | '\u{21A9}'..='\u{21AA}'
    )
    // Miscellaneous Technical
    || matches!(c,
        '\u{231A}'..='\u{231B}'
        | '\u{2328}'
        | '\u{23CF}'
        | '\u{23E9}'..='\u{23F3}'
        | '\u{23F8}'..='\u{23FA}'
    )
    // Enclosed Alphanumerics
    || matches!(c, '\u{24C2}')
    // Geometric Shapes
    || matches!(c,
        '\u{25AA}'..='\u{25AB}'
        | '\u{25B6}'
        | '\u{25C0}'
        | '\u{25FB}'..='\u{25FE}'
    )
    // Miscellaneous Symbols (U+2600–U+26FF)
    || matches!(c,
        '\u{2600}'..='\u{2604}'
        | '\u{260E}'
        | '\u{2611}'
        | '\u{2614}'..='\u{2615}'
        | '\u{2618}'
        | '\u{261D}'
        | '\u{2620}'
        | '\u{2622}'..='\u{2623}'
        | '\u{2626}'
        | '\u{262A}'
        | '\u{262E}'..='\u{262F}'
        | '\u{2638}'..='\u{263A}'
        | '\u{2640}'
        | '\u{2642}'
        | '\u{2648}'..='\u{2653}'
        | '\u{265F}'..='\u{2660}'
        | '\u{2663}'
        | '\u{2665}'..='\u{2666}'
        | '\u{2668}'
        | '\u{267B}'
        | '\u{267E}'..='\u{267F}'
        | '\u{2692}'..='\u{2697}'
        | '\u{2699}'
        | '\u{269B}'..='\u{269C}'
        | '\u{26A0}'..='\u{26A1}'
        | '\u{26A7}'
        | '\u{26AA}'..='\u{26AB}'
        | '\u{26B0}'..='\u{26B1}'
        | '\u{26BD}'..='\u{26BE}'
        | '\u{26C4}'..='\u{26C5}'
        | '\u{26CE}'..='\u{26CF}'
        | '\u{26D1}'
        | '\u{26D3}'..='\u{26D4}'
        | '\u{26E9}'..='\u{26EA}'
        | '\u{26F0}'..='\u{26F5}'
        | '\u{26F7}'..='\u{26FA}'
        | '\u{26FD}'
        // ☆ (U+2606) and ★ (U+2605) appear in emoji-variation-sequences.txt
        | '\u{2605}'..='\u{2606}'
    )
    // Dingbats (U+2700–U+27BF)
    || matches!(c,
        '\u{2702}'
        | '\u{2705}'
        | '\u{2708}'..='\u{270D}'
        | '\u{270F}'
        | '\u{2712}'
        | '\u{2714}'
        | '\u{2716}'
        | '\u{271D}'
        | '\u{2721}'
        | '\u{2728}'
        | '\u{2733}'..='\u{2734}'
        | '\u{2744}'
        | '\u{2747}'
        | '\u{274C}'
        | '\u{274E}'
        | '\u{2753}'..='\u{2755}'
        | '\u{2757}'
        | '\u{2763}'..='\u{2764}'
        | '\u{2795}'..='\u{2797}'
        | '\u{27A1}'
        | '\u{27B0}'
        | '\u{27BF}'
    )
    // Supplemental Arrows-B and other blocks
    || matches!(c,
        '\u{2934}'..='\u{2935}'
        | '\u{2B05}'..='\u{2B07}'
        | '\u{2B1B}'..='\u{2B1C}'
        | '\u{2B50}'
        | '\u{2B55}'
    )
    // CJK Symbols and Punctuation / Enclosed CJK
    || matches!(c,
        '\u{3030}'
        | '\u{303D}'
        | '\u{3297}'
        | '\u{3299}'
    )
}

impl VtProcessor {
    /// Flush the pending emoji base at the given `forced_width`, or at its natural
    /// `unicode_width` when `forced_width` is `None`.
    ///
    /// A `forced_width` of `Some(2)` comes from FE0F and must only take effect when
    /// the base is a codepoint that *could* be an emoji (non-ASCII, not already
    /// wide).  `Some(1)` from FE0E and `None` always use the natural / forced width
    /// as-is.
    pub(super) fn flush_pending_emoji(&mut self, forced_width: Option<u8>) {
        let Some(pe) = self.pending_emoji.take() else {
            return;
        };
        let width = match forced_width {
            Some(2) => {
                // Only widen if the base is a codepoint that could be emoji.
                // We define "could be emoji" as non-ASCII (already guaranteed by the
                // buffering condition) and outside the 0x0000–0x00FF Latin range.
                // A simple heuristic: anything >= U+00A0 and non-ASCII is eligible.
                if pe.ch as u32 >= 0x00A0 { 2 } else { 1 }
            }
            Some(w) => w,
            None => UnicodeWidthChar::width(pe.ch).unwrap_or(1) as u8,
        };

        // Position the cursor at the stored write position before delegating to the
        // width-aware write.  The wrap was already applied when the base was buffered,
        // so we can set the cursor directly.
        self.active_cursor_mut().col = pe.col;
        self.active_cursor_mut().row = pe.row;

        let cols = self.cols;
        if let Some(cell) = self.active_buf_mut().get_mut(pe.row, pe.col) {
            cell.grapheme = pe.ch.to_string();
            cell.attrs = pe.attrs;
            cell.width = width;
            cell.hyperlink = pe.hyperlink.clone();
        }
        if width == 2
            && pe.col + 1 < cols
            && let Some(cell) = self.active_buf_mut().get_mut(pe.row, pe.col + 1)
        {
            *cell = Cell::phantom();
        }
        let new_col = pe.col + width as u16;
        if new_col >= cols {
            if self.modes.decawm {
                self.wrap_pending = true;
            }
            self.active_cursor_mut().col = cols.saturating_sub(1);
        } else {
            self.active_cursor_mut().col = new_col;
        }
    }
}
