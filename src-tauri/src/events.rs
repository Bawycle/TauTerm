// SPDX-License-Identifier: MPL-2.0

//! Typed event definitions and emit helpers.
//!
//! All backend → frontend events are defined here. Command handlers and background
//! tasks use `emit_*` helpers rather than calling `app_handle.emit()` directly,
//! ensuring the event name and payload type are always in sync.
//!
//! Event names are defined by the `#[tauri_specta(event_name = "...")]` attribute
//! on each event struct. The helpers below use `tauri_specta::Event::emit()`.

pub mod types;

pub use types::*;

use tauri::AppHandle;
use tauri_specta::Event;

/// Emit a session topology change event.
pub fn emit_session_state_changed(app: &AppHandle, event: SessionStateChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit session-state-changed: {e}");
    }
}

/// Emit an SSH lifecycle state change event.
pub fn emit_ssh_state_changed(app: &AppHandle, event: SshStateChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit ssh-state-changed: {e}");
    }
}

/// Emit a screen update event (dirty cells from VT processing).
pub fn emit_screen_update(app: &AppHandle, event: ScreenUpdateEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit screen-update: {e}");
    }
}

/// Emit a terminal mode change event (DECCKM / DECKPAM).
pub fn emit_mode_state_changed(app: &AppHandle, event: ModeStateChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit mode-state-changed: {e}");
    }
}

/// Emit a scrollback position change event.
pub fn emit_scroll_position_changed(app: &AppHandle, event: ScrollPositionChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit scroll-position-changed: {e}");
    }
}

/// Emit a credential prompt event (SSH auth needs user input).
pub fn emit_credential_prompt(app: &AppHandle, event: CredentialPromptEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit credential-prompt: {e}");
    }
}

/// Emit a passphrase prompt event (SSH pubkey auth — encrypted private key, FS-SSH-019a).
pub fn emit_passphrase_prompt(app: &AppHandle, event: PassphrasePromptEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit passphrase-prompt: {e}");
    }
}

/// Emit a host key prompt event (TOFU verification).
pub fn emit_host_key_prompt(app: &AppHandle, event: HostKeyPromptEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit host-key-prompt: {e}");
    }
}

/// Emit a pane notification change event (bell, background output, process exit).
pub fn emit_notification_changed(app: &AppHandle, event: NotificationChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit notification-changed: {e}");
    }
}

/// Emit a cursor style change event (DECSCUSR).
pub fn emit_cursor_style_changed(app: &AppHandle, event: CursorStyleChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit cursor-style-changed: {e}");
    }
}

/// Emit a bell triggered event (rate-limited, at most 1 per 100 ms per pane).
pub fn emit_bell_triggered(app: &AppHandle, event: BellTriggeredEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit bell-triggered: {e}");
    }
}

/// Emit an OSC 52 clipboard write request event (FS-VT-075).
pub fn emit_osc52_write_requested(app: &AppHandle, event: Osc52WriteRequestedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit osc52-write-requested: {e}");
    }
}

/// Emit a deprecated SSH algorithm warning event (FS-SSH-014).
pub fn emit_ssh_warning(app: &AppHandle, event: SshWarningEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit ssh-warning: {e}");
    }
}

/// Emit an SSH reconnected separator event (FS-SSH-042).
pub fn emit_ssh_reconnected(app: &AppHandle, event: SshReconnectedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit ssh-reconnected: {e}");
    }
}

/// Emit a full-screen state change event (FS-FULL-009).
pub fn emit_fullscreen_state_changed(app: &AppHandle, event: FullscreenStateChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit fullscreen-state-changed: {e}");
    }
}

/// Emit a preferences-changed event (cross-instance sync).
pub fn emit_preferences_changed(app: &AppHandle, event: PreferencesChangedEvent) {
    if let Err(e) = event.emit(app) {
        tracing::error!("Failed to emit preferences-changed: {e}");
    }
}
