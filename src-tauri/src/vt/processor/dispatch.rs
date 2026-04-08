// SPDX-License-Identifier: MPL-2.0

//! `vte::Perform` implementation for `VtPerformBridge`.
//!
//! Dispatches parsed VT/ANSI sequences to the `VtProcessor` state machine:
//! C0 controls (`execute`), printable characters (`print`), CSI, OSC, and ESC sequences.

use vte::Perform;

use crate::vt::sgr::apply_sgr;

use super::VtPerformBridge;

mod csi_cursor;
mod csi_erase;
mod csi_misc;
mod csi_modes;
mod csi_scroll;
mod esc;
mod execute;
mod helpers;
mod osc;
mod print;

impl Perform for VtPerformBridge<'_> {
    fn print(&mut self, c: char) {
        print::handle_print(self.inner, c);
    }

    fn execute(&mut self, byte: u8) {
        execute::handle_execute(self.inner, byte);
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        // DCS hook — no-op for v1. All DCS sequences (including DECRQSS) are ignored.
    }

    fn put(&mut self, _byte: u8) {
        // DCS data byte — no-op for v1.
    }

    fn unhook(&mut self) {
        // DCS unhook — no-op for v1. No DCS sequence is processed on termination.
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        osc::handle_osc(self.inner, params);
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let p = &mut self.inner;
        let param0 = helpers::param0(params);
        let param1 = helpers::param1(params);

        match (intermediates, action) {
            // SGR — CSI Pm m
            ([], 'm') => {
                apply_sgr(params, &mut p.current_attrs);
            }
            // CUU — cursor up
            ([], 'A') => csi_cursor::cuu(p, param0.max(1)),
            // CUD — cursor down
            ([], 'B') => csi_cursor::cud(p, param0.max(1)),
            // CUF — cursor forward
            ([], 'C') => csi_cursor::cuf(p, param0.max(1)),
            // CUB — cursor back
            ([], 'D') => csi_cursor::cub(p, param0.max(1)),
            // CUP / HVP — cursor position
            ([], 'H') | ([], 'f') => csi_cursor::cup(p, param0, param1),
            // ED — erase in display
            ([], 'J') => csi_erase::ed(p, param0),
            // EL — erase in line
            ([], 'K') => csi_erase::el(p, param0),
            // DECSTBM — set scroll region
            ([], 'r') => csi_scroll::decstbm(p, param0, param1),
            // DECSET — DEC private mode set
            ([b'?'], 'h') => csi_modes::decset(p, params),
            // DECRST — DEC private mode reset
            ([b'?'], 'l') => csi_modes::decrst(p, params),
            // DECSC (7) — save cursor (CSI s)
            ([], 's') => csi_misc::decsc(p),
            // DECRC (8) — restore cursor (CSI u)
            ([], 'u') => csi_misc::decrc(p),
            // ICH — Insert Character (CSI Ps @)
            ([], '@') => csi_misc::ich(p, param0.max(1)),
            // DCH — Delete Character (CSI Ps P)
            ([], 'P') => csi_misc::dch(p, param0.max(1)),
            // IL — Insert Line (CSI Ps L)
            ([], 'L') => csi_scroll::il(p, param0.max(1)),
            // DL — Delete Line (CSI Ps M)
            ([], 'M') => csi_scroll::dl(p, param0.max(1)),
            // CSI Ps S — scroll up
            ([], 'S') => csi_scroll::su(p, param0.max(1)),
            // CSI Ps T — scroll down
            ([], 'T') => csi_scroll::sd(p, param0.max(1)),
            // CHA — Cursor Horizontal Absolute
            ([], 'G') => csi_cursor::cha(p, param0),
            // HPA — Horizontal Position Absolute
            ([], '`') => csi_cursor::hpa(p, param0),
            // VPA — Vertical Position Absolute
            ([], 'd') => csi_cursor::vpa(p, param0),
            // ECH — Erase Character
            ([], 'X') => csi_erase::ech(p, param0.max(1)),
            // CNL — Cursor Next Line
            ([], 'E') => csi_cursor::cnl(p, param0.max(1)),
            // CPL — Cursor Previous Line
            ([], 'F') => csi_cursor::cpl(p, param0.max(1)),
            // HPR — Horizontal Position Relative
            ([], 'a') => csi_cursor::hpr(p, param0.max(1)),
            // VPR — Vertical Position Relative
            ([], 'e') => csi_cursor::vpr(p, param0.max(1)),
            // DECSCUSR — set cursor shape
            ([b' '], 'q') => csi_misc::decscusr(p, param0),
            // DSR — Device Status Report
            ([], 'n') => csi_misc::dsr(p, param0),
            // DA — Primary Device Attributes
            ([], 'c') => csi_misc::da(p, param0),

            _ => {} // Unknown CSI sequence — ignore.
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        esc::handle_esc(self.inner, intermediates, byte);
    }
}
