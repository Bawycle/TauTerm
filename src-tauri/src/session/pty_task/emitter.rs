// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::AppHandle;

use crate::events::{
    emit_bell_triggered, emit_cursor_style_changed, emit_mode_state_changed,
    emit_notification_changed, emit_osc52_write_requested, emit_screen_update,
    emit_session_state_changed,
    types::{
        BellTriggeredEvent, CursorStyleChangedEvent, NotificationChangedEvent,
        Osc52WriteRequestedEvent, PaneNotificationDto, SessionChangeType, SessionStateChangedEvent,
    },
};
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::vt::VtProcessor;

use super::ProcessOutput;
use super::event_builders::{build_mode_state_event, build_screen_update_event};

// ---------------------------------------------------------------------------
// emit_all_pending — flush one coalesced window to the frontend
// ---------------------------------------------------------------------------

/// Emit all pending events accumulated in one debounce window.
///
/// Resets `pending` to `ProcessOutput::default()` on return.
pub(super) fn emit_all_pending(
    app: &AppHandle,
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
    registry: &Arc<SessionRegistry>,
    pending: &mut ProcessOutput,
) {
    // FS-NOTIF-001: background-output notification.
    if !pending.dirty.is_empty()
        && !registry.is_active_pane(pane_id)
        && let Some((_, tab_state)) = registry.get_tab_state_for_pane(pane_id)
    {
        emit_notification_changed(
            app,
            NotificationChangedEvent {
                tab_id: tab_state.id,
                pane_id: pane_id.clone(),
                notification: Some(PaneNotificationDto::BackgroundOutput),
            },
        );
    }

    if !pending.dirty.is_empty() {
        let event = build_screen_update_event(pane_id, vt, &pending.dirty);
        emit_screen_update(app, event);
    }

    if pending.mode_changed {
        let event = build_mode_state_event(pane_id, vt);
        emit_mode_state_changed(app, event);
    }

    if let Some(shape) = pending.new_cursor_shape {
        emit_cursor_style_changed(
            app,
            CursorStyleChangedEvent {
                pane_id: pane_id.clone(),
                shape,
            },
        );
    }

    if pending.bell {
        emit_bell_triggered(
            app,
            BellTriggeredEvent {
                pane_id: pane_id.clone(),
            },
        );
    }

    if let Some(data) = pending.osc52.take() {
        emit_osc52_write_requested(
            app,
            Osc52WriteRequestedEvent {
                pane_id: pane_id.clone(),
                data,
            },
        );
    }

    if let Some(title) = pending.new_title.take()
        && let Some(tab_state) = registry.update_pane_title(pane_id, title)
    {
        emit_session_state_changed(
            app,
            SessionStateChangedEvent {
                change_type: SessionChangeType::PaneMetadataChanged,
                tab: Some(tab_state),
                active_tab_id: None,
                closed_tab_id: None,
            },
        );
    }

    *pending = ProcessOutput::default();
}
