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
  /** User-defined label. Null/absent until the user sets one. */
  label?: string | null;
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
  | { type: 'disconnected'; reason?: string }
  | { type: 'closed' };

/**
 * Pane notification state.
 *
 * Mirrors Rust PaneNotificationDto: #[serde(tag = "type", rename_all = "camelCase")]
 * - Bell            → { type: 'bell' }
 * - BackgroundOutput → { type: 'backgroundOutput' }
 * - ProcessExited   → { type: 'processExited'; exitCode: number | null; signalName: string | null }
 *   - exitCode null  → process killed by signal (WIFSIGNALED)
 *   - signalName non-null → e.g. "SIGKILL", "SIGHUP"
 *   - signalName null → normal exit (exit code is authoritative)
 */
export type PaneNotification =
  | { type: 'bell' }
  | { type: 'backgroundOutput' }
  | { type: 'processExited'; exitCode: number | null; signalName: string | null };

/**
 * Delta event payload emitted by `session-state-changed` (ARCHITECTURE §4.5.2).
 * Carries the full `TabState` of the affected tab — no partial merge needed.
 *
 * Mirrors Rust SessionStateChangedEvent:
 *   #[serde(tag = "type", rename_all = "camelCase")]
 *   enum SessionStateChangedEvent { TabCreated { tab }, TabClosed { closedTabId, activeTabId? }, … }
 *
 * The `type` discriminant is the camelCase variant name. Switch on `event.type`
 * for exhaustive handling — TypeScript narrows each branch automatically.
 */
export type SessionStateChangedEvent =
  | { type: 'tabCreated'; tab: TabState }
  | { type: 'tabClosed'; closedTabId: TabId; activeTabId?: TabId }
  | { type: 'tabReordered'; tab: TabState }
  | { type: 'activeTabChanged'; tab: TabState; activeTabId: TabId }
  | { type: 'activePaneChanged'; tab: TabState }
  | { type: 'paneMetadataChanged'; tab: TabState };

// ---------------------------------------------------------------------------
// SSH lifecycle state events (ARCHITECTURE §4.3)
// ---------------------------------------------------------------------------

/**
 * Emitted on every SSH session state transition.
 *
 * Mirrors Rust SshStateChangedEvent:
 *   pane_id: PaneId          → paneId
 *   state: SshLifecycleState → state
 *
 * The disconnect reason is carried inside `state` when `state.type === 'disconnected'`:
 *   `(event.state as { type: 'disconnected'; reason?: string }).reason`
 */
export interface SshStateChangedEvent {
  paneId: PaneId;
  state: SshLifecycleState;
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
 *   is_full_redraw: bool   → isFullRedraw
 *   cols: u16              → cols
 *   rows: u16              → rows
 */
export interface ScreenUpdateEvent {
  paneId: PaneId;
  cells: CellUpdate[];
  cursor: CursorState;
  /** Total scrollback lines available — kept in sync on every screen update. */
  scrollbackLines: number;
  /**
   * True when this update represents a full screen repaint (e.g. alternate screen
   * entry/exit, terminal resize). Used to reset scroll offset to 0.
   * Mirrors Rust ScreenUpdateEvent.is_full_redraw.
   */
  isFullRedraw: boolean;
  /**
   * Scroll offset active when this event was produced.
   * 0 = live PTY event; > 0 = scroll-triggered viewport from scrollback.
   * The frontend uses this to decide whether to apply cell updates or freeze
   * the viewport (FS-SB-009).
   */
  scrollOffset: number;
  /** Terminal grid width when this event was produced — authoritative stride for applyUpdates. */
  cols: number;
  /** Terminal grid height when this event was produced. */
  rows: number;
}

/**
 * A single updated cell in the screen buffer.
 *
 * Mirrors Rust CellUpdate:
 *   content: String   → content  (single char or empty string)
 *   width: u8         → width    (1 = normal, 2 = wide/CJK, 0 = phantom continuation)
 *   attrs: CellAttrsDto → attrs
 */
export interface CellUpdate {
  /** Row 0 = top of visible viewport. */
  row: number;
  /** Column 0 = leftmost position. */
  col: number;
  /** Single character, or empty string for a blank cell. */
  content: string;
  /**
   * Cell display width: 1 for normal, 2 for wide (CJK), 0 for phantom continuation cell.
   * Mirrors Rust CellUpdate.width.
   */
  width: number;
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
  /**
   * Omitted from JSON when false (Rust: #[serde(skip_serializing_if = "is_false")]).
   * Defaults to false when absent.
   */
  bold?: boolean;
  /** Omitted from JSON when false. Defaults to false when absent. */
  dim?: boolean;
  /** Omitted from JSON when false. Defaults to false when absent. */
  italic?: boolean;
  /**
   * Underline style: 0 = none, 1 = single, 2 = double, 3 = curly, 4 = dotted, 5 = dashed.
   * Omitted from JSON when 0 (Rust: #[serde(skip_serializing_if = "is_zero")]).
   * Defaults to 0 when absent.
   */
  underline?: number;
  /** Omitted from JSON when false. Defaults to false when absent. */
  blink?: boolean;
  /** Omitted from JSON when false. Defaults to false when absent. */
  inverse?: boolean;
  /** Omitted from JSON when false. Defaults to false when absent. */
  hidden?: boolean;
  /** Omitted from JSON when false. Defaults to false when absent. */
  strikethrough?: boolean;
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
  /** `true` when a previous attempt failed — frontend shows an error indicator. */
  failed: boolean;
  /** `true` when the OS keychain is available — frontend shows "Save in keychain" checkbox. */
  isKeychainAvailable: boolean;
}

/**
 * Emitted when SSH pubkey auth needs a passphrase for an encrypted private key (FS-SSH-019a).
 *
 * Mirrors Rust PassphrasePromptEvent.
 */
export interface PassphrasePromptEvent {
  paneId: PaneId;
  /** Filename only — never the full path. */
  keyPathLabel: string;
  /** `true` when a previous attempt failed — frontend shows an error indicator. */
  failed: boolean;
  /** `true` when the OS keychain is available — frontend shows "Save in keychain" checkbox. */
  isKeychainAvailable: boolean;
}

/**
 * Emitted on first connection or when the host key has changed.
 *
 * Mirrors Rust HostKeyPromptEvent.
 */
export interface HostKeyPromptEvent {
  paneId: PaneId;
  /** Connection config ID — used to reopen the connection after TOFU acceptance. */
  connectionId: string;
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
 * Emitted when preferences change externally (another TauTerm instance).
 * The frontend replaces its in-memory preferences with the payload.
 * Mirrors Rust PreferencesChangedEvent (events/types.rs).
 * Event name: "preferences-changed"
 */
export interface PreferencesChangedEvent {
  preferences: Preferences;
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
  /**
   * Whether to launch a login shell (FS-PTY-013).
   * Pass `true` for the first tab so ~/.bash_profile / ~/.zprofile are sourced.
   */
  login?: boolean;
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
 * Fullscreen chrome behavior.
 * - 'autoHide': tab bar and status bar float as fixed overlays, auto-hide after 1.5s.
 * - 'alwaysVisible': bars stay in the flex flow — no auto-hide, no hover zones.
 * Mirrors Rust FullscreenChromeBehavior: #[serde(rename_all = "camelCase")]
 */
export type FullscreenChromeBehavior = 'autoHide' | 'alwaysVisible';

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
  /** Whether the window starts in fullscreen mode. Default: false. */
  fullscreen: boolean;
  /** Hide the mouse cursor when the user types in the terminal. Default: true. */
  hideCursorWhileTyping: boolean;
  /** Display a slim title bar at the top of each pane in multi-pane layouts. Default: true. */
  showPaneTitleBar: boolean;
  /**
   * Controls tab bar / status bar behavior in fullscreen mode.
   * 'autoHide': bars are positioned as fixed overlays, hidden after 1.5s, recalled on hover.
   * 'alwaysVisible': bars stay in the flex flow — no overlay, no auto-hide.
   * Default: 'autoHide'.
   */
  fullscreenChromeBehavior: FullscreenChromeBehavior;
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

/**
 * Partial appearance preferences for patch operations.
 * All fields optional — only provided fields are updated.
 * Mirrors Rust AppearancePatch: #[serde(rename_all = "camelCase")]
 */
export interface AppearancePatch {
  fontFamily?: string;
  fontSize?: number;
  cursorStyle?: CursorStyle;
  cursorBlinkMs?: number;
  themeName?: string;
  opacity?: number;
  language?: Language;
  contextMenuHintShown?: boolean;
  fullscreen?: boolean;
  hideCursorWhileTyping?: boolean;
  showPaneTitleBar?: boolean;
  fullscreenChromeBehavior?: FullscreenChromeBehavior;
}

/**
 * Partial terminal preferences for patch operations.
 * All fields optional — only provided fields are updated.
 * Mirrors Rust TerminalPatch: #[serde(rename_all = "camelCase")]
 */
export interface TerminalPatch {
  scrollbackLines?: number;
  allowOsc52Write?: boolean;
  wordDelimiters?: string;
  bellType?: BellType;
  confirmMultilinePaste?: boolean;
}

/**
 * Partial keyboard preferences for patch operations.
 * All fields optional — only provided fields are updated.
 * Mirrors Rust KeyboardPatch: #[serde(rename_all = "camelCase")]
 */
export interface KeyboardPatch {
  bindings?: Record<string, string>;
}

/** Mirrors Rust PreferencesPatch — all fields optional. */
export interface PreferencesPatch {
  appearance?: AppearancePatch;
  terminal?: TerminalPatch;
  keyboard?: KeyboardPatch;
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
  /** When `true`, the backend stores the accepted password in the OS keychain (FS-CRED-007). */
  saveInKeychain?: boolean;
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
 * Emitted when a deprecated SSH algorithm is detected during the connection handshake.
 *
 * Mirrors Rust SshWarningEvent (events/types.rs).
 * Event name: "ssh-warning"
 */
export interface SshWarningEvent {
  paneId: PaneId;
  /** The deprecated algorithm name, e.g. "ssh-rsa" or "ssh-dss". */
  algorithm: string;
  /** Human-readable explanation of why this algorithm is deprecated. */
  reason: string;
}

/**
 * Emitted immediately after a successful SSH reconnect.
 *
 * Mirrors Rust SshReconnectedEvent (events/types.rs).
 * Event name: "ssh-reconnected"
 */
export interface SshReconnectedEvent {
  paneId: PaneId;
  /** Unix timestamp in milliseconds at the moment of reconnection. */
  timestampMs: number;
}

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

// ---------------------------------------------------------------------------
// Fullscreen types (ARCHITECTURE §4.x)
// ---------------------------------------------------------------------------

/**
 * Returned by `toggle_fullscreen` command.
 * Mirrors Rust FullscreenState.
 */
export interface FullscreenState {
  isFullscreen: boolean;
}

/**
 * Emitted by the backend when the fullscreen state changes (e.g. via WM shortcut).
 * Event name: "fullscreen-state-changed"
 * Mirrors Rust FullscreenStateChangedEvent.
 */
export interface FullscreenStateChangedEvent {
  isFullscreen: boolean;
}
