// SPDX-License-Identifier: MPL-2.0

//! Terminal mode state — all DECSET/DECRST boolean and enum modes.
//!
//! `ModeState` tracks the complete set of orthogonal terminal modes listed in
//! §5.3 of ARCHITECTURE.md. It is owned by `VtProcessor` and saved/restored on
//! alternate screen buffer switches (mode 1049).

/// The complete terminal mode state for one screen buffer.
///
/// On transition to alternate screen (mode 1049): the current `ModeState` is
/// saved, and a fresh default `ModeState` is applied to the alternate screen.
/// On return: the saved state is restored.
#[derive(Debug, Clone)]
pub struct ModeState {
    /// DECCKM (mode 1): cursor key application mode.
    /// When `true`, arrow keys emit `ESC O A/B/C/D` instead of `ESC [ A/B/C/D`.
    pub decckm: bool,

    /// DECKPAM / DECKPNM (ESC = / ESC >): keypad application mode.
    /// When `true`, numeric keypad sends application sequences.
    pub deckpam: bool,

    /// Mouse reporting mode (DECSET 9, 1000, 1002, 1003).
    pub mouse_reporting: MouseReportingMode,

    /// Mouse encoding format (DECSET 1006 SGR, 1015 URXVT).
    pub mouse_encoding: MouseEncoding,

    /// DECSET 1004: focus events.
    pub focus_events: bool,

    /// DECSET 2004: bracketed paste mode.
    pub bracketed_paste: bool,

    /// DECAWM (mode 7): auto-wrap mode.
    /// When `true` (default), the cursor wraps to the next line when it reaches
    /// the last column. When `false`, subsequent characters overwrite the last column.
    pub decawm: bool,

    /// DECTCEM (mode 25): cursor visible.
    pub cursor_visible: bool,

    /// Scroll region (DECSTBM). Stored as 0-based row indices.
    /// `(0, rows - 1)` = full screen (default).
    pub scroll_region: (u16, u16),

    /// Active character set slot (SI/SO — G0 or G1).
    pub charset_slot: CharsetSlot,

    /// G0 designator.
    pub g0: Charset,

    /// G1 designator.
    pub g1: Charset,
}

impl ModeState {
    /// Create a default mode state for a given terminal size.
    pub fn new(rows: u16) -> Self {
        Self {
            decckm: false,
            deckpam: false,
            mouse_reporting: MouseReportingMode::None,
            mouse_encoding: MouseEncoding::X10,
            focus_events: false,
            bracketed_paste: false,
            decawm: true,
            cursor_visible: true,
            scroll_region: (0, rows.saturating_sub(1)),
            charset_slot: CharsetSlot::G0,
            g0: Charset::Ascii,
            g1: Charset::Ascii,
        }
    }

    /// Reset scroll region to full screen after a resize.
    pub fn reset_scroll_region(&mut self, rows: u16) {
        self.scroll_region = (0, rows.saturating_sub(1));
    }
}

/// Mouse reporting modes (mutually exclusive; higher wins).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseReportingMode {
    /// No mouse reporting.
    #[default]
    None,
    /// X10 compatibility (mode 9): button press only.
    X10,
    /// Normal tracking (mode 1000): press + release.
    Normal,
    /// Button event tracking (mode 1002): press, release, drag.
    ButtonEvent,
    /// Any event tracking (mode 1003): all motion included.
    AnyEvent,
}

/// Mouse event encoding format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseEncoding {
    /// Classic X10 encoding (limited to col/row ≤ 223).
    #[default]
    X10,
    /// SGR encoding (mode 1006) — preferred; no coordinate limit.
    Sgr,
    /// URXVT encoding (mode 1015).
    Urxvt,
}

/// Active charset slot (G0 or G1, switched by SI/SO).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharsetSlot {
    #[default]
    G0,
    G1,
}

/// Character set designator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Charset {
    /// Standard ASCII / ISO 8859-1.
    #[default]
    Ascii,
    /// DEC Special Graphics — line-drawing characters (ESC ( 0).
    DecSpecialGraphics,
}
