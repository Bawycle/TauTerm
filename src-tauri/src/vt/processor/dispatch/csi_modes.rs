// SPDX-License-Identifier: MPL-2.0

use crate::vt::modes::{MouseEncoding, MouseReportingMode};
use crate::vt::processor::VtProcessor;

/// DECSET — DEC private mode set (CSI ? Pm h)
pub(super) fn decset(p: &mut VtProcessor, params: &vte::Params) {
    for param in params.iter() {
        let mode = param.first().copied().unwrap_or(0);
        let prev_decckm = p.modes.decckm;
        let prev_mouse_reporting = p.modes.mouse_reporting;
        let prev_mouse_encoding = p.modes.mouse_encoding;
        let prev_focus_events = p.modes.focus_events;
        let prev_bracketed_paste = p.modes.bracketed_paste;
        let prev_cursor_visible = p.modes.cursor_visible;
        match mode {
            1 => p.modes.decckm = true,
            6 => p.modes.decom = true,
            7 => p.modes.decawm = true,
            9 => p.modes.mouse_reporting = MouseReportingMode::X10,
            12 => p.cursor_blink = true,
            25 => p.modes.cursor_visible = true,
            47 => p.enter_alternate(false),
            1000 => p.modes.mouse_reporting = MouseReportingMode::Normal,
            1002 => p.modes.mouse_reporting = MouseReportingMode::ButtonEvent,
            1003 => p.modes.mouse_reporting = MouseReportingMode::AnyEvent,
            1004 => p.modes.focus_events = true,
            1006 => p.modes.mouse_encoding = MouseEncoding::Sgr,
            1015 => p.modes.mouse_encoding = MouseEncoding::Urxvt,
            1047 => p.enter_alternate(false),
            1049 => p.enter_alternate(true),
            2004 => p.modes.bracketed_paste = true,
            _ => {}
        }
        if p.modes.decckm != prev_decckm
            || p.modes.mouse_reporting != prev_mouse_reporting
            || p.modes.mouse_encoding != prev_mouse_encoding
            || p.modes.focus_events != prev_focus_events
            || p.modes.bracketed_paste != prev_bracketed_paste
        {
            p.mode_changed = true;
        }
        if p.modes.cursor_visible != prev_cursor_visible {
            p.pending_dirty.mark_cursor_moved();
        }
    }
}

/// DECRST — DEC private mode reset (CSI ? Pm l)
pub(super) fn decrst(p: &mut VtProcessor, params: &vte::Params) {
    for param in params.iter() {
        let mode = param.first().copied().unwrap_or(0);
        let prev_decckm = p.modes.decckm;
        let prev_mouse_reporting = p.modes.mouse_reporting;
        let prev_mouse_encoding = p.modes.mouse_encoding;
        let prev_focus_events = p.modes.focus_events;
        let prev_bracketed_paste = p.modes.bracketed_paste;
        let prev_cursor_visible = p.modes.cursor_visible;
        match mode {
            1 => p.modes.decckm = false,
            6 => p.modes.decom = false,
            7 => {
                p.modes.decawm = false;
                // Disabling DECAWM cancels any pending wrap immediately.
                p.wrap_pending = false;
            }
            9 | 1000 | 1002 | 1003 => {
                p.modes.mouse_reporting = MouseReportingMode::None
            }
            12 => p.cursor_blink = false,
            25 => p.modes.cursor_visible = false,
            47 => p.leave_alternate(false),
            1004 => p.modes.focus_events = false,
            1006 => p.modes.mouse_encoding = MouseEncoding::X10,
            1015 => p.modes.mouse_encoding = MouseEncoding::X10,
            1047 => p.leave_alternate(false),
            1049 => p.leave_alternate(true),
            2004 => p.modes.bracketed_paste = false,
            _ => {}
        }
        if p.modes.decckm != prev_decckm
            || p.modes.mouse_reporting != prev_mouse_reporting
            || p.modes.mouse_encoding != prev_mouse_encoding
            || p.modes.focus_events != prev_focus_events
            || p.modes.bracketed_paste != prev_bracketed_paste
        {
            p.mode_changed = true;
        }
        if p.modes.cursor_visible != prev_cursor_visible {
            p.pending_dirty.mark_cursor_moved();
        }
    }
}
