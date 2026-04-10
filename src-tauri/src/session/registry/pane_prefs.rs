// SPDX-License-Identifier: MPL-2.0

//! Live preference propagation to existing pane sessions.
//!
//! When the user changes preferences while panes are running, those that map
//! directly to per-pane VT state must be pushed to every active `VtProcessor`
//! immediately. This module owns those propagation methods.
//!
//! Methods here are called by `preferences_cmds::update_preferences` after the
//! `PreferencesStore` has been updated and persisted.

use tauri::AppHandle;

use crate::events::{emit_cursor_style_changed, types::CursorStyleChangedEvent};

use super::SessionRegistry;

impl SessionRegistry {
    /// Push a new cursor shape to every active pane's `VtProcessor`.
    ///
    /// `shape` is a DECSCUSR-encoded value (0–6, use `CursorStyle::to_decscusr()`).
    ///
    /// This overrides any per-pane cursor shape previously set via DECSCUSR: when
    /// the user explicitly resets a preference, the preference takes precedence over
    /// whatever the running application had requested. This is intentional — document
    /// this in the UI if needed ("resets application cursor overrides").
    ///
    /// After updating each VtProcessor a `cursor-style-changed` event is emitted so
    /// the frontend re-renders the cursor immediately (IPC event rules: pane_id in payload).
    pub fn propagate_cursor_shape(&self, app: &AppHandle, shape: u8) {
        let inner = self.inner.read();
        for entry in inner.tabs.values() {
            for (pane_id, pane) in &entry.panes {
                let mut vt = pane.vt.write();
                vt.cursor_shape = shape;
                // Update preferred so that DECSCUSR 0 ("restore default") issued
                // by applications restores the new user-preference shape, not the
                // original initial_cursor_shape (FS-VT-030).
                vt.preferred_cursor_shape = shape;
                // Mark as changed so that in-flight read tasks also pick it up on
                // their next debounce flush (belt-and-suspenders; the event below
                // already informs the frontend).
                vt.cursor_shape_changed = true;
                drop(vt);
                emit_cursor_style_changed(
                    app,
                    CursorStyleChangedEvent {
                        pane_id: pane_id.clone(),
                        shape,
                    },
                );
            }
        }
    }

    /// Push a new `allow_osc52_write` flag to every active pane's `VtProcessor`.
    ///
    /// Panes marked with `osc52_overridden = true` (SSH panes whose per-connection
    /// policy was applied via `apply_pane_osc52_override`) are skipped — their
    /// policy must not be reset by a global preference change (arch §8.2).
    ///
    /// No event is emitted: this is a behavioural gate, not a visual change the
    /// frontend needs to react to immediately.
    pub fn propagate_osc52_allow(&self, allow: bool) {
        let inner = self.inner.read();
        for entry in inner.tabs.values() {
            for pane in entry.panes.values() {
                if pane.osc52_overridden {
                    continue; // SSH pane with per-connection override — do not touch
                }
                pane.vt.write().allow_osc52_write = allow;
            }
        }
    }

    /// Apply the per-connection OSC 52 write policy for an SSH pane and mark it as
    /// overridden so `propagate_osc52_allow` will not reset it (arch §8.2).
    ///
    /// Must be called from `open_ssh_connection_impl` after the VT is created,
    /// before `open_connection` is called.
    pub fn apply_pane_osc52_override(&self, pane_id: &crate::session::ids::PaneId, allow: bool) {
        let mut inner = self.inner.write();
        if let Some(pane) = inner
            .tabs
            .values_mut()
            .find_map(|e| e.panes.get_mut(pane_id))
        {
            pane.vt.write().allow_osc52_write = allow;
            pane.osc52_overridden = true;
        }
    }
}
