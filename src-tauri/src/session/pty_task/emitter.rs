// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tauri::AppHandle;

use crate::events::{
    emit_bell_triggered, emit_cursor_style_changed, emit_mode_state_changed,
    emit_notification_changed, emit_osc52_write_requested, emit_screen_update,
    emit_session_state_changed,
    types::{
        BellTriggeredEvent, CursorStyleChangedEvent, NotificationChangedEvent,
        Osc52WriteRequestedEvent, PaneNotificationDto, SessionStateChangedEvent,
    },
};
use crate::session::ids::PaneId;
use crate::session::registry::SessionRegistry;
use crate::vt::VtProcessor;

use super::ProcessOutput;
use super::event_builders::{build_mode_state_event, build_screen_update_event};

// ---------------------------------------------------------------------------
// EmitOutcome — return type of emit_all_pending
// ---------------------------------------------------------------------------

/// Outcome of `emit_all_pending()` — reports whether a `screen-update` event
/// was actually emitted in this flush.
///
/// Used by Task 2 (the debounce coalescer) to gate `last_emit_ms`: only
/// `screen-update` events are acknowledged by the frontend (via `flushRafQueue`
/// in `useTerminalPane.svelte.ts`). Non-visual events (bell, mode-state,
/// cursor-style, osc52, title, cwd, notification) produce no frontend paint
/// and therefore no `frame_ack`. Advancing `last_emit_ms` for them would
/// create a phantom "unacked emit" that permanently pushes the pane into
/// drop mode after ~1 s of user idle. See ADR-0027 Addendum 2.
#[derive(Debug, Clone, Copy)]
pub(super) struct EmitOutcome {
    /// Wall-clock duration of the emit call (feeds adaptive debounce).
    pub duration: Duration,
    /// `true` iff a `screen-update` event was emitted (i.e. `pending.dirty`
    /// was non-empty on entry).
    pub emitted_screen_update: bool,
}

/// Pure predicate: will flushing `pending` produce a `screen-update` event?
///
/// Extracted so Task 2's gating logic can be tested without an `AppHandle`
/// (which requires a live Tauri runtime / display surface). Accessed by the
/// in-crate unit tests in `pty_task.rs`. The integration test
/// `DEL-ASYNC-PTY-009` in `tests/async_concurrency.rs` mirrors this
/// 1-line predicate inline rather than widening visibility further —
/// `ProcessOutput` itself is `pub(crate)` and should stay so.
///
/// FRONTEND MIRROR (ADR-0027 Addendum 2): the frontend's `frameAck` IPC is
/// invoked only at `src/lib/composables/useTerminalPane.svelte.ts::flushRafQueue`
/// (called after screen-update events) and in the snapshot-refetch recovery
/// path. This predicate must stay in lockstep with that contract: if either
/// side adds a new ack-triggering event path without updating the other, the
/// backpressure mechanism silently desyncs and can produce false drop-mode.
pub(super) fn output_emits_screen_update(pending: &ProcessOutput) -> bool {
    !pending.dirty.is_empty()
}

/// Compile-time signature pin: if the shape of `output_emits_screen_update`
/// ever changes (new input type, returns something other than `bool`), this
/// line fails to compile — forcing a deliberate update of every caller and
/// of the DEL-ASYNC-PTY-009 structural mirror in
/// `tests/async_concurrency.rs`. See ADR-0027 Addendum 2.
const _OUTPUT_EMITS_SIGNATURE_PIN: fn(&ProcessOutput) -> bool = output_emits_screen_update;

// ---------------------------------------------------------------------------
// emit_all_pending — flush one coalesced window to the frontend
// ---------------------------------------------------------------------------

/// Emit all pending events accumulated in one debounce window.
///
/// Resets `pending` to `ProcessOutput::default()` on return.
///
/// Returns an `EmitOutcome` describing both the wall-clock cost of the flush
/// (fed back into adaptive debounce) and whether a `screen-update` event was
/// actually emitted (used to gate `last_emit_ms` in the frame-ack backpressure
/// machinery — see ADR-0027 Addendum 2).
pub(super) fn emit_all_pending(
    app: &AppHandle,
    pane_id: &PaneId,
    vt: &Arc<RwLock<VtProcessor>>,
    registry: &Arc<SessionRegistry>,
    pending: &mut ProcessOutput,
) -> EmitOutcome {
    let t0 = Instant::now();
    // Captured BEFORE any mutation: `pending.dirty` is reset at end of function.
    let emitted_screen_update = output_emits_screen_update(pending);
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
            SessionStateChangedEvent::PaneMetadataChanged { tab: tab_state },
        );
    }

    if let Some(cwd) = pending.new_cwd.take()
        && let Some(tab_state) = registry.update_pane_cwd(pane_id, cwd)
    {
        emit_session_state_changed(
            app,
            SessionStateChangedEvent::PaneMetadataChanged { tab: tab_state },
        );
    }

    *pending = ProcessOutput::default();
    EmitOutcome {
        duration: t0.elapsed(),
        emitted_screen_update,
    }
}
