// SPDX-License-Identifier: MPL-2.0

//! Terminal mode state — all DECSET/DECRST boolean and enum modes.
//!
//! `ModeState` tracks the complete set of orthogonal terminal modes listed in
//! §5.3 of ARCHITECTURE.md. It is owned by `VtProcessor` and saved/restored on
//! alternate screen buffer switches (mode 1049).

use serde::{Deserialize, Serialize};

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

    /// DECOM (DEC origin mode, DECSET/DECRST ?6).
    ///
    /// When `true`, cursor positioning commands (CUP, HVP) are relative to the
    /// top of the active scroll region rather than the top-left corner of the
    /// screen. The cursor is also constrained within the scroll region.
    /// Saved/restored by DECSC/DECRC. Reset to `false` on alt-screen entry.
    pub decom: bool,
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
            decom: false,
        }
    }

    /// Reset scroll region to full screen after a resize.
    pub fn reset_scroll_region(&mut self, rows: u16) {
        self.scroll_region = (0, rows.saturating_sub(1));
    }
}

/// Mouse reporting modes (mutually exclusive; higher wins).
///
/// Serializes to camelCase strings for IPC: `"none"`, `"x10"`, `"normal"`,
/// `"buttonEvent"`, `"anyEvent"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
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
///
/// Serializes to camelCase strings for IPC: `"x10"`, `"sgr"`, `"urxvt"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_reporting_mode_serializes_to_camel_case() {
        assert_eq!(
            serde_json::to_string(&MouseReportingMode::None).unwrap(),
            r#""none""#
        );
        assert_eq!(
            serde_json::to_string(&MouseReportingMode::X10).unwrap(),
            r#""x10""#
        );
        assert_eq!(
            serde_json::to_string(&MouseReportingMode::Normal).unwrap(),
            r#""normal""#
        );
        assert_eq!(
            serde_json::to_string(&MouseReportingMode::ButtonEvent).unwrap(),
            r#""buttonEvent""#
        );
        assert_eq!(
            serde_json::to_string(&MouseReportingMode::AnyEvent).unwrap(),
            r#""anyEvent""#
        );
    }

    #[test]
    fn mouse_encoding_serializes_to_camel_case() {
        assert_eq!(
            serde_json::to_string(&MouseEncoding::X10).unwrap(),
            r#""x10""#
        );
        assert_eq!(
            serde_json::to_string(&MouseEncoding::Sgr).unwrap(),
            r#""sgr""#
        );
        assert_eq!(
            serde_json::to_string(&MouseEncoding::Urxvt).unwrap(),
            r#""urxvt""#
        );
    }

    #[test]
    fn mouse_reporting_mode_roundtrips() {
        for variant in [
            MouseReportingMode::None,
            MouseReportingMode::X10,
            MouseReportingMode::Normal,
            MouseReportingMode::ButtonEvent,
            MouseReportingMode::AnyEvent,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let decoded: MouseReportingMode = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, decoded);
        }
    }

    #[test]
    fn mouse_encoding_roundtrips() {
        for variant in [MouseEncoding::X10, MouseEncoding::Sgr, MouseEncoding::Urxvt] {
            let json = serde_json::to_string(&variant).unwrap();
            let decoded: MouseEncoding = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, decoded);
        }
    }

    // -----------------------------------------------------------------------
    // Default state
    // -----------------------------------------------------------------------

    #[test]
    fn default_state_all_modes_off_except_decawm_and_cursor_visible() {
        let m = ModeState::new(24);
        assert!(!m.decckm, "DECCKM default must be false");
        assert!(!m.deckpam, "DECKPAM default must be false");
        assert_eq!(m.mouse_reporting, MouseReportingMode::None);
        assert_eq!(m.mouse_encoding, MouseEncoding::X10);
        assert!(!m.focus_events);
        assert!(!m.bracketed_paste);
        assert!(m.decawm, "DECAWM default must be true");
        assert!(m.cursor_visible, "cursor_visible default must be true");
        assert_eq!(
            m.scroll_region,
            (0, 23),
            "scroll_region must span full screen (0..23)"
        );
        assert_eq!(m.charset_slot, CharsetSlot::G0);
        assert_eq!(m.g0, Charset::Ascii);
        assert_eq!(m.g1, Charset::Ascii);
    }

    // -----------------------------------------------------------------------
    // scroll_region default for various row counts
    // -----------------------------------------------------------------------

    #[test]
    fn scroll_region_default_spans_full_screen() {
        let m = ModeState::new(12);
        assert_eq!(m.scroll_region, (0, 11));
    }

    #[test]
    fn scroll_region_single_row_terminal() {
        let m = ModeState::new(1);
        assert_eq!(m.scroll_region, (0, 0));
    }

    #[test]
    fn scroll_region_zero_rows_saturates_to_zero() {
        let m = ModeState::new(0);
        // rows.saturating_sub(1) == 0 when rows == 0.
        assert_eq!(m.scroll_region, (0, 0));
    }

    // -----------------------------------------------------------------------
    // reset_scroll_region
    // -----------------------------------------------------------------------

    #[test]
    fn reset_scroll_region_updates_bottom_on_resize() {
        let mut m = ModeState::new(24);
        // Simulate a custom scroll region set by DECSTBM.
        m.scroll_region = (5, 18);
        // Resize: reset_scroll_region must restore full-screen span.
        m.reset_scroll_region(40);
        assert_eq!(m.scroll_region, (0, 39));
    }

    // -----------------------------------------------------------------------
    // Boolean mode set/reset
    // -----------------------------------------------------------------------

    #[test]
    fn decckm_set_and_reset() {
        let mut m = ModeState::new(24);
        m.decckm = true;
        assert!(m.decckm);
        m.decckm = false;
        assert!(!m.decckm);
    }

    #[test]
    fn deckpam_set_and_reset() {
        let mut m = ModeState::new(24);
        m.deckpam = true;
        assert!(m.deckpam);
        m.deckpam = false;
        assert!(!m.deckpam);
    }

    #[test]
    fn focus_events_set_and_reset() {
        let mut m = ModeState::new(24);
        m.focus_events = true;
        assert!(m.focus_events);
        m.focus_events = false;
        assert!(!m.focus_events);
    }

    #[test]
    fn bracketed_paste_set_and_reset() {
        let mut m = ModeState::new(24);
        m.bracketed_paste = true;
        assert!(m.bracketed_paste);
        m.bracketed_paste = false;
        assert!(!m.bracketed_paste);
    }

    #[test]
    fn decawm_can_be_disabled_and_re_enabled() {
        let mut m = ModeState::new(24);
        assert!(m.decawm, "DECAWM starts enabled");
        m.decawm = false;
        assert!(!m.decawm);
        m.decawm = true;
        assert!(m.decawm);
    }

    #[test]
    fn cursor_visible_can_be_hidden_and_shown() {
        let mut m = ModeState::new(24);
        assert!(m.cursor_visible);
        m.cursor_visible = false;
        assert!(!m.cursor_visible);
        m.cursor_visible = true;
        assert!(m.cursor_visible);
    }

    // -----------------------------------------------------------------------
    // Mouse reporting mode transitions
    // -----------------------------------------------------------------------

    #[test]
    fn mouse_reporting_transitions_through_all_modes() {
        let mut m = ModeState::new(24);
        assert_eq!(m.mouse_reporting, MouseReportingMode::None);

        m.mouse_reporting = MouseReportingMode::X10;
        assert_eq!(m.mouse_reporting, MouseReportingMode::X10);

        m.mouse_reporting = MouseReportingMode::Normal;
        assert_eq!(m.mouse_reporting, MouseReportingMode::Normal);

        m.mouse_reporting = MouseReportingMode::ButtonEvent;
        assert_eq!(m.mouse_reporting, MouseReportingMode::ButtonEvent);

        m.mouse_reporting = MouseReportingMode::AnyEvent;
        assert_eq!(m.mouse_reporting, MouseReportingMode::AnyEvent);

        m.mouse_reporting = MouseReportingMode::None;
        assert_eq!(m.mouse_reporting, MouseReportingMode::None);
    }

    // -----------------------------------------------------------------------
    // Mouse encoding transitions
    // -----------------------------------------------------------------------

    #[test]
    fn mouse_encoding_transitions() {
        let mut m = ModeState::new(24);
        assert_eq!(m.mouse_encoding, MouseEncoding::X10);

        m.mouse_encoding = MouseEncoding::Sgr;
        assert_eq!(m.mouse_encoding, MouseEncoding::Sgr);

        m.mouse_encoding = MouseEncoding::Urxvt;
        assert_eq!(m.mouse_encoding, MouseEncoding::Urxvt);

        m.mouse_encoding = MouseEncoding::X10;
        assert_eq!(m.mouse_encoding, MouseEncoding::X10);
    }

    // -----------------------------------------------------------------------
    // Charset slot and designator
    // -----------------------------------------------------------------------

    #[test]
    fn charset_slot_switch_g0_to_g1() {
        let mut m = ModeState::new(24);
        assert_eq!(m.charset_slot, CharsetSlot::G0);
        m.charset_slot = CharsetSlot::G1;
        assert_eq!(m.charset_slot, CharsetSlot::G1);
        m.charset_slot = CharsetSlot::G0;
        assert_eq!(m.charset_slot, CharsetSlot::G0);
    }

    #[test]
    fn g0_can_be_set_to_dec_special_graphics() {
        let mut m = ModeState::new(24);
        m.g0 = Charset::DecSpecialGraphics;
        assert_eq!(m.g0, Charset::DecSpecialGraphics);
        m.g0 = Charset::Ascii;
        assert_eq!(m.g0, Charset::Ascii);
    }

    #[test]
    fn g1_can_be_set_to_dec_special_graphics() {
        let mut m = ModeState::new(24);
        m.g1 = Charset::DecSpecialGraphics;
        assert_eq!(m.g1, Charset::DecSpecialGraphics);
    }

    // -----------------------------------------------------------------------
    // Clone preserves all fields
    // -----------------------------------------------------------------------

    #[test]
    fn clone_preserves_all_fields() {
        let mut m = ModeState::new(24);
        m.decckm = true;
        m.bracketed_paste = true;
        m.mouse_reporting = MouseReportingMode::AnyEvent;
        m.mouse_encoding = MouseEncoding::Sgr;
        m.g0 = Charset::DecSpecialGraphics;
        m.scroll_region = (2, 20);

        let cloned = m.clone();
        assert_eq!(cloned.decckm, m.decckm);
        assert_eq!(cloned.bracketed_paste, m.bracketed_paste);
        assert_eq!(cloned.mouse_reporting, m.mouse_reporting);
        assert_eq!(cloned.mouse_encoding, m.mouse_encoding);
        assert_eq!(cloned.g0, m.g0);
        assert_eq!(cloned.scroll_region, m.scroll_region);
    }
}
