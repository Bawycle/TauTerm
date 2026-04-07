// SPDX-License-Identifier: MPL-2.0

//! Alternate screen helpers for `VtProcessor`.
//!
//! Contains `enter_alternate()` and `leave_alternate()`.

use crate::vt::modes::ModeState;

use super::{CursorPos, VtProcessor};

impl VtProcessor {
    /// Switch to alternate screen (mode 1049 / 47 / 1047).
    pub(super) fn enter_alternate(&mut self, save_cursor: bool) {
        if self.alt_active {
            return;
        }
        if save_cursor {
            self.saved_normal_cursor = Some(self.normal_cursor.clone());
        }
        self.saved_normal_modes = Some(self.modes.clone());
        self.modes = ModeState::new(self.rows);
        self.alt_active = true;
        self.alternate.erase_lines(0, self.rows);
        self.alt_cursor = CursorPos::default();
        self.pending_dirty.mark_full_redraw();
    }

    /// Return to normal screen.
    pub(super) fn leave_alternate(&mut self, restore_cursor: bool) {
        if !self.alt_active {
            return;
        }
        self.alt_active = false;
        if let Some(saved) = self.saved_normal_modes.take() {
            self.modes = saved;
        }
        // FS-VT-086: force mouse reporting off on alt-screen exit.
        // Guards against apps that activate mouse tracking but crash without
        // sending ?1000l (or equivalent reset). This makes mouse capture opt-in
        // per screen, not sticky across screen switches.
        self.modes.mouse_reporting = crate::vt::modes::MouseReportingMode::None;
        if restore_cursor && let Some(saved) = self.saved_normal_cursor.take() {
            self.normal_cursor = saved;
        }
        self.pending_dirty.mark_full_redraw();
    }
}
