// SPDX-License-Identifier: MPL-2.0

//! Typed event payload definitions for all events emitted backend → frontend.
//!
//! These types are the canonical Rust definitions (§4.6 of ARCHITECTURE.md).
//! The TypeScript counterparts in `src/lib/ipc/types.ts` must be kept in sync.
//!
//! Event names (string identifiers used in `app_handle.emit()`):
//! - `session-state-changed`
//! - `ssh-state-changed`
//! - `screen-update`
//! - `mode-state-changed`
//! - `scroll-position-changed`
//! - `credential-prompt`
//! - `host-key-prompt`
//! - `notification-changed`
//! - `fullscreen-state-changed`

use serde::{Deserialize, Serialize};

use crate::session::{ConnectionId, PaneId, TabId, TabState};
use crate::ssh::SshLifecycleState;
use crate::vt::modes::{MouseEncoding, MouseReportingMode};

// ---------------------------------------------------------------------------
// Session topology
// ---------------------------------------------------------------------------

/// Emitted when the session topology changes in a way that originates
/// asynchronously (process exit, OSC title, set_active_pane).
///
/// Not emitted for `split_pane` or `close_pane` — those commands return
/// the updated `TabState` directly (§4.5.2 of ARCHITECTURE.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateChangedEvent {
    pub change_type: SessionChangeType,
    /// Present for all change types except `tab-closed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab: Option<TabState>,
    /// Present when `change_type` is `active-tab-changed` or `tab-closed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tab_id: Option<String>,
    /// Present when `change_type` is `tab-closed` — the ID of the tab that was closed.
    /// Required by IPC event rules: every event affecting a specific entity must include
    /// that entity's ID explicitly in the payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_tab_id: Option<TabId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionChangeType {
    TabCreated,
    TabClosed,
    TabReordered,
    ActiveTabChanged,
    ActivePaneChanged,
    PaneMetadataChanged,
}

// ---------------------------------------------------------------------------
// SSH lifecycle
// ---------------------------------------------------------------------------

/// Emitted on every SSH session state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshStateChangedEvent {
    pub pane_id: PaneId,
    pub state: SshLifecycleState,
    /// Optional human-readable reason for `Disconnected` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Screen updates
// ---------------------------------------------------------------------------

/// Emitted by the PTY read task after processing terminal output.
/// Carries either dirty cell diffs or a full snapshot flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenUpdateEvent {
    pub pane_id: PaneId,
    /// Individual cell updates within the dirty region.
    pub cells: Vec<CellUpdate>,
    /// Cursor position after processing.
    pub cursor: CursorState,
    /// Total scrollback lines available after this update.
    ///
    /// Allows the frontend to keep the scrollbar accurate when new lines are
    /// appended to the scrollback buffer while the user is scrolled up
    /// (scroll-freeze policy).
    pub scrollback_lines: usize,
    /// True when the entire screen must be repainted (alternate screen switch,
    /// resize, or ED2). The frontend uses this to reset scroll offset and
    /// rebuild `gridRows` fully rather than applying a partial diff.
    pub is_full_redraw: bool,
    /// Terminal grid dimensions when this event was produced.
    /// Used by the frontend as the authoritative stride for applyUpdates — eliminates
    /// stride mismatch from the optimistic cols/rows update in sendResize().
    pub cols: u16,
    pub rows: u16,
    /// 0 for live PTY events. Equals `pane.scroll_offset` for scroll-triggered viewport redraws.
    pub scroll_offset: i64,
}

/// A single updated cell in the screen buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellUpdate {
    pub row: u16,
    pub col: u16,
    pub content: String,
    /// Visual width of the cell: 1 = normal, 2 = wide (CJK/emoji), 0 = phantom
    /// (continuation slot of a wide character — the frontend must leave it blank).
    pub width: u8,
    pub attrs: CellAttrsDto,
    /// OSC 8 hyperlink URI for this cell, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperlink: Option<String>,
}

/// Serializable cursor state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorState {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
    pub shape: u8,
    pub blink: bool,
}

/// Serializable SGR cell attributes sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellAttrsDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<ColorDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<ColorDto>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: u8,
    pub blink: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_color: Option<ColorDto>,
}

/// Color value — ANSI 16, 256-color index, or 24-bit RGB.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ColorDto {
    Default,
    Ansi { index: u8 },
    Ansi256 { index: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

// ---------------------------------------------------------------------------
// Terminal mode state (keyboard encoding)
// ---------------------------------------------------------------------------

/// Emitted when any terminal mode relevant to the frontend changes.
/// The frontend uses these flags for keyboard encoding, mouse reporting,
/// focus events, and bracketed paste (FS-KBD-007, FS-KBD-010, FS-VT-080–086, FS-CLIP-008).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeStateChangedEvent {
    pub pane_id: PaneId,
    /// DECCKM (mode 1): application cursor keys active.
    pub decckm: bool,
    /// DECKPAM active (ESC =): application keypad mode.
    pub deckpam: bool,
    /// Mouse reporting mode.
    pub mouse_reporting: MouseReportingMode,
    /// Mouse encoding format.
    pub mouse_encoding: MouseEncoding,
    /// DECSET 1004: focus events active.
    pub focus_events: bool,
    /// DECSET 2004: bracketed paste mode active.
    pub bracketed_paste: bool,
}

// ---------------------------------------------------------------------------
// Scrollback
// ---------------------------------------------------------------------------

/// Emitted when the scrollback viewport position changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollPositionChangedEvent {
    pub pane_id: PaneId,
    /// Current scroll offset (lines from bottom; 0 = at bottom).
    pub offset: i64,
    /// Total scrollback lines available.
    pub scrollback_lines: usize,
}

// ---------------------------------------------------------------------------
// SSH credential and host key prompts
// ---------------------------------------------------------------------------

/// Emitted when the SSH authentication flow needs credentials from the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialPromptEvent {
    pub pane_id: PaneId,
    pub host: String,
    pub username: String,
    /// Optional prompt text from the server (keyboard-interactive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// `true` when a previous authentication attempt failed (stale/wrong password).
    /// The frontend uses this to show an error indicator in the credential dialog.
    pub failed: bool,
    /// `true` when the OS keychain is available so the frontend can offer a
    /// "Save in keychain" checkbox (FS-CRED-007).
    pub is_keychain_available: bool,
}

/// Emitted when SSH pubkey auth requires a passphrase for an encrypted private key (FS-SSH-019a).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassphrasePromptEvent {
    pub pane_id: PaneId,
    /// Filename of the private key — NEVER the full path (security: no usernames in logs).
    pub key_path_label: String,
    /// `true` when a previous passphrase attempt failed.
    pub failed: bool,
    /// `true` when the OS keychain is available so the frontend can offer a
    /// "Save in keychain" checkbox (FS-CRED-007).
    pub is_keychain_available: bool,
}

/// Emitted on first connection or when the host key has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyPromptEvent {
    pub pane_id: PaneId,
    /// Connection config ID — frontend uses it to reopen the connection after TOFU acceptance.
    pub connection_id: ConnectionId,
    pub host: String,
    pub key_type: String,
    pub fingerprint: String,
    /// `true` if this is a key change (potential MITM); `false` for first-time TOFU.
    pub is_changed: bool,
}

// ---------------------------------------------------------------------------
// Cursor style (DECSCUSR)
// ---------------------------------------------------------------------------

/// Emitted when the cursor shape changes via DECSCUSR (FS-VT-030).
///
/// `shape` carries the raw DECSCUSR parameter (0–6):
/// - 0/1 = blinking block, 2 = steady block
/// - 3 = blinking underline, 4 = steady underline
/// - 5 = blinking bar, 6 = steady bar
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorStyleChangedEvent {
    pub pane_id: PaneId,
    /// Raw DECSCUSR value (0–6).
    pub shape: u8,
}

// ---------------------------------------------------------------------------
// Bell
// ---------------------------------------------------------------------------

/// Emitted when the terminal produces a BEL character, rate-limited to at most
/// one event per 100 ms per pane (FS-VT-090).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BellTriggeredEvent {
    pub pane_id: PaneId,
}

// ---------------------------------------------------------------------------
// OSC 52 clipboard write forwarding
// ---------------------------------------------------------------------------

/// Emitted when the terminal requests a clipboard write via OSC 52 (FS-VT-075).
/// Only emitted when `allow_osc52_write` is `true` in `VtProcessor`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Osc52WriteRequestedEvent {
    pub pane_id: PaneId,
    /// Decoded UTF-8 clipboard payload.
    pub data: String,
}

// ---------------------------------------------------------------------------
// SSH algorithm warnings (FS-SSH-014)
// ---------------------------------------------------------------------------

/// Emitted when a deprecated SSH algorithm is detected during the connection
/// handshake (`ssh-rsa` SHA-1, `ssh-dss`). Non-blocking — for informational
/// display in the pane or a UI banner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshWarningEvent {
    pub pane_id: PaneId,
    /// The deprecated algorithm name, e.g. `"ssh-rsa"` or `"ssh-dss"`.
    pub algorithm: String,
    /// Human-readable explanation of why this algorithm is deprecated.
    pub reason: String,
}

// ---------------------------------------------------------------------------
// SSH reconnection separator (FS-SSH-042)
// ---------------------------------------------------------------------------

/// Emitted immediately after a successful SSH reconnect.
///
/// The frontend uses this to insert a visual separator in the scrollback so
/// the user can clearly distinguish output from the previous session and the
/// new one (FS-SSH-042, UXD §7.19).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshReconnectedEvent {
    pub pane_id: PaneId,
    /// Unix timestamp in milliseconds at the moment of reconnection.
    pub timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// Full-screen state
// ---------------------------------------------------------------------------

/// Query result for the current window full-screen state.
/// Returned synchronously by `toggle_fullscreen` (FS-FULL-009).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullscreenState {
    pub is_fullscreen: bool,
}

/// Emitted after the window geometry transition to full-screen or windowed.
/// Informational — the frontend ResizeObserver + `resize_pane` handle SIGWINCH.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullscreenStateChangedEvent {
    pub is_fullscreen: bool,
}

// ---------------------------------------------------------------------------
// Pane activity notifications
// ---------------------------------------------------------------------------

/// Emitted when a pane's notification state changes (bell, background output, exit).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChangedEvent {
    pub tab_id: TabId,
    pub pane_id: PaneId,
    /// `None` means the notification was cleared.
    pub notification: Option<PaneNotificationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum PaneNotificationDto {
    Bell,
    BackgroundOutput,
    /// Process exited — clean (exit_code = Some(0)), error (exit_code = Some(n)), or
    /// killed by signal (exit_code = None, signal_name = Some("SIGKILL"), etc.).
    ProcessExited {
        /// `None` if the process was killed by a signal (WIFSIGNALED).
        #[serde(rename = "exitCode")]
        exit_code: Option<i32>,
        /// Signal name (e.g. `"SIGKILL"`, `"SIGHUP"`) when killed by a signal.
        /// `None` for normal exits.
        #[serde(rename = "signalName")]
        signal_name: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// FS-PTY-005: ProcessExited with exit code 0 serialises correctly.
    /// JSON must contain `"exitCode":0` and `"signalName":null`.
    #[test]
    fn process_exited_exit0_serializes_correctly() {
        let dto = PaneNotificationDto::ProcessExited {
            exit_code: Some(0),
            signal_name: None,
        };
        let json = serde_json::to_string(&dto).expect("serialize failed");
        assert!(
            json.contains("\"exitCode\":0"),
            "exitCode must be 0; got: {json}"
        );
        assert!(
            json.contains("\"signalName\":null"),
            "signalName must be null; got: {json}"
        );
        assert!(
            json.contains("\"type\":\"processExited\""),
            "type discriminant must be processExited; got: {json}"
        );
    }

    /// FS-PTY-005: ProcessExited with non-zero exit code serialises correctly.
    #[test]
    fn process_exited_nonzero_serializes_correctly() {
        let dto = PaneNotificationDto::ProcessExited {
            exit_code: Some(1),
            signal_name: None,
        };
        let json = serde_json::to_string(&dto).expect("serialize failed");
        assert!(
            json.contains("\"exitCode\":1"),
            "exitCode must be 1; got: {json}"
        );
        assert!(
            json.contains("\"signalName\":null"),
            "signalName must be null; got: {json}"
        );
    }

    /// FS-PTY-005: ProcessExited when killed by signal — exit_code is None,
    /// signal_name carries the signal name.
    #[test]
    fn process_exited_signal_serializes_correctly() {
        let dto = PaneNotificationDto::ProcessExited {
            exit_code: None,
            signal_name: Some("SIGKILL".to_string()),
        };
        let json = serde_json::to_string(&dto).expect("serialize failed");
        assert!(
            json.contains("\"exitCode\":null"),
            "exitCode must be null; got: {json}"
        );
        assert!(
            json.contains("\"signalName\":\"SIGKILL\""),
            "signalName must be SIGKILL; got: {json}"
        );
    }

    /// Round-trip: serialise then deserialise back to the same value.
    #[test]
    fn process_exited_round_trips() {
        let dto = PaneNotificationDto::ProcessExited {
            exit_code: Some(42),
            signal_name: None,
        };
        let json = serde_json::to_string(&dto).expect("serialize failed");
        let restored: PaneNotificationDto =
            serde_json::from_str(&json).expect("deserialize failed");
        if let PaneNotificationDto::ProcessExited {
            exit_code,
            signal_name,
        } = restored
        {
            assert_eq!(exit_code, Some(42));
            assert_eq!(signal_name, None);
        } else {
            panic!("expected ProcessExited variant; got: {json}");
        }
    }
}
