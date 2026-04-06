// SPDX-License-Identifier: MPL-2.0

/**
 * IPC type contract between the Rust backend and the Svelte frontend.
 * These types mirror the Rust serde structures exactly — never use `any`.
 * Source of truth: src-tauri/src/events/types.rs (Rust) — see docs/ARCHITECTURE.md §4.6.
 *
 * IMPORTANT: Any change to the Rust types requires a corresponding update here.
 * Rust serde config: #[serde(rename_all = "camelCase")] on structs,
 *                    #[serde(tag = "type", rename_all = "camelCase")] on tagged enums.
 */

// ---------------------------------------------------------------------------
// Shared primitives
// ---------------------------------------------------------------------------

export type PaneId = string;
export type TabId = string;

// ---------------------------------------------------------------------------
// Session state (ARCHITECTURE §4.2)
// ---------------------------------------------------------------------------

/** Full snapshot of all tabs returned by `invoke('get_session_state')`. */
export interface SessionState {
  tabs: TabState[];
  activeTabId: TabId;
}

/**
 * Recursive pane layout tree (ARCHITECTURE §4.5.1).
 * A leaf holds a terminal pane; a split holds two child nodes.
 *
 * Mirrors Rust PaneNode enum: #[serde(tag = "type", rename_all = "camelCase")]
 */
export type PaneNode =
  | { type: 'leaf'; paneId: PaneId; state: PaneState }
  | {
      type: 'split';
      direction: 'horizontal' | 'vertical';
      /** Fraction of space allocated to `first` (0–1). */
      ratio: number;
      first: PaneNode;
      second: PaneNode;
    };

export interface TabState {
  id: TabId;
  /** User-defined label. `null` means "use the process title". */
  label: string | null;
  activePaneId: PaneId;
  /** Zero-indexed position in the tab bar. */
  order: number;
  /** Full layout tree for the tab. */
  layout: PaneNode;
}

export interface PaneState {
  id: PaneId;
  sessionType: 'local' | 'ssh';
  /** OSC-driven title or shell name. */
  processTitle: string;
  cwd: string;
  /** Reference to a saved SSH connection; `null` for local sessions. */
  sshConnectionId: string | null;
  /** `null` for local sessions. */
  sshState: SshLifecycleState | null;
  /** Active notification indicator; `null` when none. */
  notification: PaneNotification | null;
}

/**
 * Mirrors Rust SshLifecycleState: #[serde(rename_all = "camelCase", tag = "type")]
 * Note: the Rust enum has no payload fields — all state context is carried in
 * SshStateChangedEvent (paneId, reason) or in PaneState (sshConnectionId).
 */
export type SshLifecycleState =
  | { type: 'connecting' }
  | { type: 'authenticating' }
  | { type: 'connected' }
  | { type: 'disconnected' }
  | { type: 'closed' };

/**
 * Pane notification state.
 *
 * Mirrors Rust PaneNotificationDto: #[serde(tag = "type", rename_all = "camelCase")]
 * - Bell          → { type: 'bell' }
 * - BackgroundOutput → { type: 'backgroundOutput' }
 * - ProcessExited → { type: 'processExited', exitCode: number }
 */
export type PaneNotification =
  | { type: 'bell' }
  | { type: 'backgroundOutput' }
  | { type: 'processExited'; exitCode: number };

/**
 * Delta event payload emitted by `session-state-changed` (ARCHITECTURE §4.5.2).
 * Carries the full `TabState` of the affected tab — no partial merge needed.
 *
 * Mirrors Rust SessionStateChangedEvent (camelCase) with SessionChangeType
 * serialized as kebab-case via #[serde(rename_all = "kebab-case")].
 */
export interface SessionStateChangedEvent {
  changeType:
    | 'tab-created'
    | 'tab-closed'
    | 'tab-reordered'
    | 'active-tab-changed'
    | 'active-pane-changed'
    | 'pane-metadata-changed';
  /**
   * Complete updated `TabState` of the affected tab.
   * Absent when `changeType === 'tab-closed'`.
   */
  tab?: TabState;
  /**
   * Present when `changeType === 'active-tab-changed'` or `'tab-closed'`.
   */
  activeTabId?: TabId;
  /**
   * Present when `changeType === 'tab-closed'` — the ID of the closed tab.
   * Used to reliably identify which tab to remove from the local list.
   */
  closedTabId?: TabId;
}

// ---------------------------------------------------------------------------
// SSH lifecycle state events (ARCHITECTURE §4.3)
// ---------------------------------------------------------------------------

/**
 * Emitted on every SSH session state transition.
 *
 * Mirrors Rust SshStateChangedEvent:
 *   pane_id: PaneId  → paneId
 *   state: SshLifecycleState → state
 *   reason: Option<String>   → reason? (skip_serializing_if = "Option::is_none")
 */
export interface SshStateChangedEvent {
  paneId: PaneId;
  state: SshLifecycleState;
  /** Human-readable reason for `Disconnected` state, if provided. */
  reason?: string;
}

// ---------------------------------------------------------------------------
// Screen buffer update events (ARCHITECTURE §4.3)
// ---------------------------------------------------------------------------

/**
 * Emitted by the PTY read task after processing terminal output.
 *
 * Mirrors Rust ScreenUpdateEvent:
 *   pane_id: PaneId        → paneId
 *   cells: Vec<CellUpdate> → cells
 *   cursor: CursorState    → cursor
 *   scrollback_lines: usize → scrollbackLines
 */
export interface ScreenUpdateEvent {
  paneId: PaneId;
  cells: CellUpdate[];
  cursor: CursorState;
  /** Total scrollback lines available — kept in sync on every screen update. */
  scrollbackLines: number;
}

/**
 * A single updated cell in the screen buffer.
 *
 * Mirrors Rust CellUpdate:
 *   content: String   → content  (single char or empty string)
 *   attrs: CellAttrsDto → attrs
 */
export interface CellUpdate {
  /** Row 0 = top of visible viewport. */
  row: number;
  /** Column 0 = leftmost position. */
  col: number;
  /** Single character, or empty string for a blank cell. */
  content: string;
  attrs: CellAttrsDto;
  /**
   * OSC 8 hyperlink URI for this cell, if any (FS-VT-070–073).
   * Absent when no active hyperlink.
   * Mirrors Rust CellUpdate.hyperlink (skip_serializing_if = "Option::is_none").
   */
  hyperlink?: string;
}

/**
 * Serializable cursor state.
 *
 * Mirrors Rust CursorState.
 * `shape` is a u8 cursor shape code (0 = default block, 1 = block, 2 = underline,
 * 3 = bar — matches DECSCUSR values).
 */
export interface CursorState {
  row: number;
  col: number;
  visible: boolean;
  shape: number;
  blink: boolean;
}

/**
 * SGR cell attributes.
 *
 * Mirrors Rust CellAttrsDto. `fg`/`bg`/`underlineColor` are absent when not
 * set (skip_serializing_if = "Option::is_none").
 * `underline` is a u8: 0 = none, 1 = single, 2 = double, 3 = curly, etc.
 */
export interface CellAttrsDto {
  fg?: ColorDto;
  bg?: ColorDto;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  /** Underline style: 0 = none, 1 = single, 2 = double, 3 = curly, 4 = dotted, 5 = dashed. */
  underline: number;
  blink: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
  underlineColor?: ColorDto;
}

/**
 * Color value — ANSI 16, 256-color index, or 24-bit RGB.
 *
 * Mirrors Rust ColorDto: #[serde(tag = "type", rename_all = "camelCase")]
 * - Default  → { type: 'default' }
 * - Ansi     → { type: 'ansi', index: number }
 * - Ansi256  → { type: 'ansi256', index: number }
 * - Rgb      → { type: 'rgb', r: number, g: number, b: number }
 */
export type ColorDto =
  | { type: 'default' }
  | { type: 'ansi'; index: number }
  | { type: 'ansi256'; index: number }
  | { type: 'rgb'; r: number; g: number; b: number };

// ---------------------------------------------------------------------------
// Terminal mode state event (ARCHITECTURE §4.3)
// ---------------------------------------------------------------------------

/**
 * Emitted when any terminal mode relevant to the frontend changes.
 *
 * Mirrors Rust ModeStateChangedEvent.
 */
export interface ModeStateChangedEvent {
  paneId: PaneId;
  /** DECCKM (mode 1): application cursor keys active. */
  decckm: boolean;
  /** DECKPAM active (ESC =): application keypad mode. */
  deckpam: boolean;
  /** Mouse reporting mode. */
  mouseReporting: 'none' | 'x10' | 'normal' | 'buttonEvent' | 'anyEvent';
  /** Mouse encoding format. */
  mouseEncoding: 'x10' | 'sgr' | 'urxvt';
  /** DECSET 1004: focus events active. */
  focusEvents: boolean;
  /** DECSET 2004: bracketed paste mode active. */
  bracketedPaste: boolean;
}

// ---------------------------------------------------------------------------
// Scroll position changed event (ARCHITECTURE §4.3)
// ---------------------------------------------------------------------------

/**
 * Emitted when the scrollback viewport position changes.
 *
 * Mirrors Rust ScrollPositionChangedEvent:
 *   offset: i64          → offset
 *   scrollback_lines: usize → scrollbackLines
 */
export interface ScrollPositionChangedEvent {
  paneId: PaneId;
  /** Lines scrolled from the bottom. 0 = at bottom. */
  offset: number;
  /** Total scrollback lines available. */
  scrollbackLines: number;
}

// ---------------------------------------------------------------------------
// SSH credential and host key prompt events (ARCHITECTURE §4.3)
// ---------------------------------------------------------------------------

/**
 * Emitted when the SSH authentication flow needs credentials from the user.
 *
 * Mirrors Rust CredentialPromptEvent.
 */
export interface CredentialPromptEvent {
  paneId: PaneId;
  host: string;
  username: string;
  /** Optional prompt text from the server (keyboard-interactive). */
  prompt?: string;
}

/**
 * Emitted on first connection or when the host key has changed.
 *
 * Mirrors Rust HostKeyPromptEvent.
 */
export interface HostKeyPromptEvent {
  paneId: PaneId;
  host: string;
  keyType: string;
  fingerprint: string;
  /** `true` if this is a key change (potential MITM); `false` for first-time TOFU. */
  isChanged: boolean;
}

// ---------------------------------------------------------------------------
// Notification changed event (ARCHITECTURE §4.5.4)
// ---------------------------------------------------------------------------

/**
 * Emitted when a pane's notification state changes (bell, background output, exit).
 *
 * Mirrors Rust NotificationChangedEvent.
 * `notification: null` means the notification was cleared.
 */
export interface NotificationChangedEvent {
  tabId: TabId;
  paneId: PaneId;
  notification: PaneNotification | null;
}

/**
 * Emitted when a DECSCUSR escape changes the cursor shape for a pane.
 * Mirrors Rust CursorStyleChangedEvent (events/types.rs).
 * `shape` is the raw DECSCUSR parameter (0–6):
 *   0/1 = blinking block, 2 = steady block,
 *   3 = blinking underline, 4 = steady underline,
 *   5 = blinking bar, 6 = steady bar.
 * Event name: "cursor-style-changed"
 */
export interface CursorStyleChangedEvent {
  paneId: PaneId;
  /** Raw DECSCUSR value 0–6. */
  shape: number;
}

/**
 * Emitted when the terminal produces a BEL character (rate-limited, ≤1/100 ms per pane).
 * Mirrors Rust BellTriggeredEvent (events/types.rs).
 * Event name: "bell-triggered"
 */
export interface BellTriggeredEvent {
  paneId: PaneId;
}

// ---------------------------------------------------------------------------
// Tauri command signatures
// Frontend→Backend commands via `invoke()`.
// Mirrors registered handlers in src-tauri/src/lib.rs.
// ---------------------------------------------------------------------------

/**
 * Retrieve a full session snapshot on mount.
 * @command get_session_state
 */
export type GetSessionStateCommand = () => Promise<SessionState>;

/**
 * Create a new local tab. Returns the new TabState.
 * @command create_tab
 */
export type CreateTabCommand = (args: { config: CreateTabConfig }) => Promise<TabState>;

/**
 * Configuration for creating a new tab.
 * Mirrors Rust CreateTabConfig from session/registry.rs.
 */
export interface CreateTabConfig {
  label?: string;
  /** Initial terminal width in columns. */
  cols: number;
  /** Initial terminal height in rows. */
  rows: number;
}

/**
 * Close a tab by ID.
 * @command close_tab
 */
export type CloseTabCommand = (args: { tabId: TabId }) => Promise<void>;

/**
 * Rename a tab (set or clear user label). Returns the updated TabState.
 * @command rename_tab
 */
export type RenameTabCommand = (args: { tabId: TabId; label: string | null }) => Promise<TabState>;

/**
 * Move a tab to a new position.
 * @command reorder_tab
 */
export type ReorderTabCommand = (args: { tabId: TabId; newOrder: number }) => Promise<void>;

/**
 * Split a pane. Returns the complete updated `TabState`.
 * @command split_pane
 */
export type SplitPaneCommand = (args: {
  paneId: PaneId;
  direction: 'horizontal' | 'vertical';
}) => Promise<TabState>;

/**
 * Close a pane. Returns the updated `TabState`, or null if no panes remain (tab removed).
 * See ARCHITECTURE §4.5.3.
 * @command close_pane
 */
export type ClosePaneCommand = (args: { paneId: PaneId }) => Promise<TabState | null>;

/**
 * Set the active pane within a tab.
 * @command set_active_pane
 */
export type SetActivePaneCommand = (args: { paneId: PaneId }) => Promise<void>;

/**
 * Send raw input bytes to a PTY session.
 * `data` must be a JSON array of u8 values (matches Rust Vec<u8> serde representation).
 * @command send_input
 */
export type SendInputCommand = (args: { paneId: PaneId; data: number[] }) => Promise<void>;

/**
 * Scroll a pane's viewport. Positive offset = scroll up into scrollback; 0 = go to bottom.
 * Returns the updated scroll position.
 * @command scroll_pane
 */
export type ScrollPaneCommand = (args: {
  paneId: PaneId;
  offset: number;
}) => Promise<ScrollPositionState>;

/**
 * Scroll position returned by scroll_pane.
 * Mirrors Rust ScrollPositionState.
 */
export interface ScrollPositionState {
  offset: number;
  scrollbackLines: number;
}

/**
 * Jump to the bottom of scrollback.
 * @command scroll_to_bottom
 */
export type ScrollToBottomCommand = (args: { paneId: PaneId }) => Promise<void>;

/**
 * Search the scrollback buffer.
 * @command search_pane
 */
export type SearchPaneCommand = (args: {
  paneId: PaneId;
  query: SearchQuery;
}) => Promise<SearchMatch[]>;

/**
 * Mirrors Rust SearchQuery from vt/search.rs.
 */
export interface SearchQuery {
  text: string;
  caseSensitive: boolean;
  regex: boolean;
}

/**
 * Mirrors Rust SearchMatch from vt/search.rs.
 * `scrollbackRow` is 0-based from the oldest scrollback line.
 */
export interface SearchMatch {
  scrollbackRow: number;
  colStart: number;
  colEnd: number;
}

/**
 * Report a terminal resize (triggered by viewport observer).
 * `pixelWidth` and `pixelHeight` are required for complete TIOCSWINSZ (ARCHITECTURE §4.2).
 * @command resize_pane
 */
export type ResizePaneCommand = (args: {
  paneId: PaneId;
  cols: number;
  rows: number;
  pixelWidth: number;
  pixelHeight: number;
}) => Promise<void>;

/**
 * Get the full screen state for a pane (initial render).
 * @command get_pane_screen_snapshot
 */
export type GetPaneScreenSnapshotCommand = (args: { paneId: PaneId }) => Promise<ScreenSnapshot>;

/**
 * Mirrors Rust ScreenSnapshot from vt/screen_buffer.rs.
 * Cells are row-major: rows × cols SnapshotCell entries.
 */
export interface ScreenSnapshot {
  cols: number;
  rows: number;
  cells: SnapshotCell[];
  cursorRow: number;
  cursorCol: number;
  cursorVisible: boolean;
  cursorShape: number;
  scrollbackLines: number;
  scrollOffset: number;
}

/**
 * A single cell in a full screen snapshot.
 * Mirrors Rust SnapshotCell from vt/screen_buffer.rs.
 * `fg`/`bg`/`underlineColor` are absent when default (not set by SGR).
 *
 * Note: SnapshotCell uses Rust vt::cell::Color (no Default variant) —
 * absent fields mean "use terminal default color".
 */
export interface SnapshotCell {
  content: string;
  /** Cell display width: 1 for normal, 2 for wide (CJK), 0 for combining. */
  width: number;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  /** Underline style: 0 = none, 1 = single, 2 = double, 3 = curly, 4 = dotted, 5 = dashed. */
  underline: number;
  blink: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
  fg?: Color;
  bg?: Color;
  underlineColor?: Color;
  /**
   * OSC 8 hyperlink URI for this cell, if any (FS-VT-070–073).
   * Absent when no active hyperlink.
   * Mirrors Rust SnapshotCell.hyperlink (skip_serializing_if = "Option::is_none").
   */
  hyperlink?: string;
}

/**
 * Color value used in SnapshotCell (vt::cell::Color — no Default variant).
 * Absent fg/bg means "use terminal default".
 *
 * Mirrors Rust Color enum: #[serde(tag = "type", rename_all = "camelCase")]
 */
export type Color =
  | { type: 'ansi'; index: number }
  | { type: 'ansi256'; index: number }
  | { type: 'rgb'; r: number; g: number; b: number };

/**
 * Begin SSH connect flow on a pane.
 * @command open_ssh_connection
 */
export type OpenSshConnectionCommand = (args: {
  paneId: PaneId;
  connectionId: string;
}) => Promise<void>;

/**
 * Close SSH session on a pane.
 * @command close_ssh_connection
 */
export type CloseSshConnectionCommand = (args: { paneId: PaneId }) => Promise<void>;

/**
 * Reconnect after Disconnected state.
 * @command reconnect_ssh
 */
export type ReconnectSshCommand = (args: { paneId: PaneId }) => Promise<void>;

/**
 * List saved SSH connections.
 * @command get_connections
 */
export type GetConnectionsCommand = () => Promise<SshConnectionConfig[]>;

/**
 * Mirrors Rust SshConnectionConfig from ssh.rs.
 * Authentication method is determined by presence of `identityFile`:
 * if set, uses public key auth; otherwise password/agent.
 */
export interface SshConnectionConfig {
  id: string;
  label: string;
  host: string;
  port: number;
  username: string;
  /** Path to a private key file. Absent means password/agent auth. */
  identityFile?: string;
  /** Per-connection OSC 52 write policy override. */
  allowOsc52Write: boolean;
}

/**
 * Create or update a saved connection. Returns the ConnectionId.
 * @command save_connection
 */
export type SaveConnectionCommand = (args: { config: SshConnectionConfig }) => Promise<string>;

/**
 * Delete a saved connection.
 * @command delete_connection
 */
export type DeleteConnectionCommand = (args: { connectionId: string }) => Promise<void>;

/**
 * Read current preferences.
 * @command get_preferences
 */
export type GetPreferencesCommand = () => Promise<Preferences>;

/**
 * Write preferences (immediate apply). Returns the updated Preferences.
 * @command update_preferences
 */
export type UpdatePreferencesCommand = (args: { patch: PreferencesPatch }) => Promise<Preferences>;

// ---------------------------------------------------------------------------
// Preferences types
// Mirrors src-tauri/src/preferences/schema.rs — #[serde(rename_all = "camelCase")]
// ---------------------------------------------------------------------------

/**
 * UI language enum.
 * Mirrors Rust Language: #[serde(rename_all = "camelCase")]
 * Language::En → "en", Language::Fr → "fr"
 * MUST NOT be a free string (FS-I18N-006, CLAUDE.md constraint).
 */
export type Language = 'en' | 'fr';

/**
 * Cursor shape.
 * Mirrors Rust CursorStyle: #[serde(rename_all = "camelCase")]
 * CursorStyle::Block → "block", CursorStyle::Underline → "underline", CursorStyle::Bar → "bar"
 */
export type CursorStyle = 'block' | 'underline' | 'bar';

/**
 * Bell notification type.
 * Mirrors Rust BellType: #[serde(rename_all = "camelCase")]
 * BellType::None → "none", BellType::Visual → "visual",
 * BellType::Audio → "audio", BellType::Both → "both"
 */
export type BellType = 'none' | 'visual' | 'audio' | 'both';

/** Mirrors Rust Preferences. */
export interface Preferences {
  appearance: AppearancePrefs;
  terminal: TerminalPrefs;
  keyboard: KeyboardPrefs;
  connections: SshConnectionConfig[];
  /** User-defined themes. */
  themes: UserTheme[];
}

/** Mirrors Rust AppearancePrefs. */
export interface AppearancePrefs {
  fontFamily: string;
  fontSize: number;
  cursorStyle: CursorStyle;
  /** Cursor blink period in milliseconds. Default: 530. */
  cursorBlinkMs: number;
  /** Name of the active theme. */
  themeName: string;
  /** Background opacity (0.0–1.0). */
  opacity: number;
  /**
   * Language enum. MUST be 'en' or 'fr' — never a free string (FS-I18N-006).
   * Mirrors Rust Language::En → "en", Language::Fr → "fr"
   */
  language: Language;
  /** Whether the context menu hint has been shown at least once. */
  contextMenuHintShown: boolean;
}

/** Mirrors Rust TerminalPrefs. */
export interface TerminalPrefs {
  scrollbackLines: number;
  allowOsc52Write: boolean;
  wordDelimiters: string;
  bellType: BellType;
  /** Show confirmation dialog before pasting multi-line text without bracketed paste (FS-CLIP-009). */
  confirmMultilinePaste: boolean;
}

/** Mirrors Rust KeyboardPrefs. */
export interface KeyboardPrefs {
  /** Keybinding overrides: action name → key combo string. */
  bindings: Record<string, string>;
}

/** Mirrors Rust PreferencesPatch — all fields optional. */
export interface PreferencesPatch {
  appearance?: Partial<AppearancePrefs>;
  terminal?: Partial<TerminalPrefs>;
  keyboard?: Partial<KeyboardPrefs>;
}

/**
 * List all user themes.
 * @command get_themes
 */
export type GetThemesCommand = () => Promise<UserTheme[]>;

/**
 * A user-defined color theme.
 * Mirrors Rust UserTheme: #[serde(rename_all = "camelCase")]
 */
export interface UserTheme {
  name: string;
  /** ANSI palette: 16 colors (0–15). Each entry is an RGB hex string (e.g., "#1e1e2e"). */
  palette: [
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
    string,
  ];
  foreground: string;
  background: string;
  cursorColor: string;
  selectionBg: string;
  /**
   * Terminal line height multiplier (FS-THEME-010). Range: 1.0–2.0.
   * `undefined` means use the global default (`--line-height-terminal` token).
   */
  lineHeight?: number;
}

/**
 * Create or update a theme.
 * @command save_theme
 */
export type SaveThemeCommand = (args: { theme: UserTheme }) => Promise<void>;

/**
 * Delete a user theme.
 * @command delete_theme
 */
export type DeleteThemeCommand = (args: { name: string }) => Promise<void>;

/**
 * Respond to SSH credential prompt.
 * @command provide_credentials
 */
export type ProvideCredentialsCommand = (args: {
  paneId: PaneId;
  credentials: Credentials;
}) => Promise<void>;

/**
 * Mirrors Rust Credentials from ssh/manager.rs.
 * `username` is required; `password` and `privateKeyPath` are optional
 * depending on the authentication method.
 */
export interface Credentials {
  username: string;
  password?: string;
  privateKeyPath?: string;
}

/**
 * Accept new/changed SSH host key.
 * @command accept_host_key
 */
export type AcceptHostKeyCommand = (args: { paneId: PaneId }) => Promise<void>;

/**
 * Reject SSH host key (abort connection).
 * @command reject_host_key
 */
export type RejectHostKeyCommand = (args: { paneId: PaneId }) => Promise<void>;

/**
 * Dismiss deprecated-algorithm banner.
 * @command dismiss_ssh_algorithm_warning
 */
export type DismissSshAlgorithmWarningCommand = (args: { paneId: PaneId }) => Promise<void>;

/**
 * Copy text to CLIPBOARD selection.
 * @command copy_to_clipboard
 */
export type CopyToClipboardCommand = (args: { text: string }) => Promise<void>;

/**
 * Read CLIPBOARD content.
 * @command get_clipboard
 */
export type GetClipboardCommand = () => Promise<string>;

/**
 * Open a validated URL in the system browser.
 * `paneId` — the pane from which the link was activated. When provided and the
 * pane is a local PTY session, the `file://` scheme is accepted. When absent
 * or the pane is an SSH session, `file://` is rejected (FS-VT-073).
 * @command open_url
 */
export type OpenUrlCommand = (args: { url: string; paneId?: string }) => Promise<void>;

/**
 * Clear first-launch context menu hint.
 * @command mark_context_menu_used
 */
export type MarkContextMenuUsedCommand = () => Promise<void>;
