// SPDX-License-Identifier: MPL-2.0

//! Typed event definitions and emit helpers.
//!
//! All backend → frontend events are defined here. Command handlers and background
//! tasks use `emit_*` helpers rather than calling `app_handle.emit()` directly,
//! ensuring the event name and payload type are always in sync.

pub mod types;

pub use types::*;

use tauri::{AppHandle, Emitter};

// Event name constants — single source of truth for string identifiers.
pub const EVENT_SESSION_STATE_CHANGED: &str = "session-state-changed";
pub const EVENT_SSH_STATE_CHANGED: &str = "ssh-state-changed";
pub const EVENT_SCREEN_UPDATE: &str = "screen-update";
pub const EVENT_MODE_STATE_CHANGED: &str = "mode-state-changed";
pub const EVENT_SCROLL_POSITION_CHANGED: &str = "scroll-position-changed";
pub const EVENT_CREDENTIAL_PROMPT: &str = "credential-prompt";
pub const EVENT_HOST_KEY_PROMPT: &str = "host-key-prompt";
pub const EVENT_NOTIFICATION_CHANGED: &str = "notification-changed";

/// Emit a session topology change event.
pub fn emit_session_state_changed(app: &AppHandle, event: SessionStateChangedEvent) {
    if let Err(e) = app.emit(EVENT_SESSION_STATE_CHANGED, event) {
        tracing::error!("Failed to emit session-state-changed: {e}");
    }
}

/// Emit an SSH lifecycle state change event.
pub fn emit_ssh_state_changed(app: &AppHandle, event: SshStateChangedEvent) {
    if let Err(e) = app.emit(EVENT_SSH_STATE_CHANGED, event) {
        tracing::error!("Failed to emit ssh-state-changed: {e}");
    }
}

/// Emit a screen update event (dirty cells from VT processing).
pub fn emit_screen_update(app: &AppHandle, event: ScreenUpdateEvent) {
    if let Err(e) = app.emit(EVENT_SCREEN_UPDATE, event) {
        tracing::error!("Failed to emit screen-update: {e}");
    }
}

/// Emit a terminal mode change event (DECCKM / DECKPAM).
pub fn emit_mode_state_changed(app: &AppHandle, event: ModeStateChangedEvent) {
    if let Err(e) = app.emit(EVENT_MODE_STATE_CHANGED, event) {
        tracing::error!("Failed to emit mode-state-changed: {e}");
    }
}

/// Emit a scrollback position change event.
pub fn emit_scroll_position_changed(app: &AppHandle, event: ScrollPositionChangedEvent) {
    if let Err(e) = app.emit(EVENT_SCROLL_POSITION_CHANGED, event) {
        tracing::error!("Failed to emit scroll-position-changed: {e}");
    }
}

/// Emit a credential prompt event (SSH auth needs user input).
pub fn emit_credential_prompt(app: &AppHandle, event: CredentialPromptEvent) {
    if let Err(e) = app.emit(EVENT_CREDENTIAL_PROMPT, event) {
        tracing::error!("Failed to emit credential-prompt: {e}");
    }
}

/// Emit a host key prompt event (TOFU verification).
pub fn emit_host_key_prompt(app: &AppHandle, event: HostKeyPromptEvent) {
    if let Err(e) = app.emit(EVENT_HOST_KEY_PROMPT, event) {
        tracing::error!("Failed to emit host-key-prompt: {e}");
    }
}

/// Emit a pane notification change event (bell, background output, process exit).
pub fn emit_notification_changed(app: &AppHandle, event: NotificationChangedEvent) {
    if let Err(e) = app.emit(EVENT_NOTIFICATION_CHANGED, event) {
        tracing::error!("Failed to emit notification-changed: {e}");
    }
}
