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

use serde::{Deserialize, Serialize};

use crate::session::{PaneId, TabId, TabState};
use crate::ssh::SshLifecycleState;

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
}

/// A single updated cell in the screen buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellUpdate {
    pub row: u16,
    pub col: u16,
    pub content: String,
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
    /// Mouse reporting mode: "none", "x10", "normal", "buttonEvent", or "anyEvent".
    pub mouse_reporting: String,
    /// Mouse encoding: "x10", "sgr", or "urxvt".
    pub mouse_encoding: String,
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
}

/// Emitted on first connection or when the host key has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyPromptEvent {
    pub pane_id: PaneId,
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
    ProcessExited { exit_code: i32 },
}
