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
}

/// A single updated cell in the screen buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellUpdate {
    pub row: u16,
    pub col: u16,
    pub content: String,
    pub attrs: CellAttrsDto,
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

/// Emitted when DECCKM or DECKPAM/DECKPNM changes.
/// The frontend keyboard encoder (`keyboard.ts`) needs these flags to produce
/// correct escape sequences for arrow keys and keypad (FS-KBD-007, FS-KBD-010).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeStateChangedEvent {
    pub pane_id: PaneId,
    /// DECCKM (mode 1): application cursor keys active.
    pub decckm: bool,
    /// DECKPAM active (ESC =): application keypad mode.
    pub deckpam: bool,
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
