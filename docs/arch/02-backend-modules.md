<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Rust Module Decomposition and Error Handling

> Part of the [Architecture](README.md).

---

## 3. Rust Module Decomposition

### 3.1 File Layout Convention

The codebase follows the Rust 2018+ module convention: a module `foo` with submodules is written as `src/foo.rs` (re-exports and `mod` declarations) + `src/foo/bar.rs` (submodule implementations). The `src/foo/mod.rs` form is **never used**.

### 3.2 Module Map

```
src-tauri/src/
  lib.rs              — Tauri setup: plugin registration, command registration, State injection
  main.rs             — thin entrypoint: lib::run()

  vt.rs               — re-exports: VtProcessor, ScreenBuffer, ScreenSnapshot, Cell,
                        CellAttrs, SearchQuery, SearchMatch, DirtyRegion
  vt/
    processor.rs      — VtProcessor: struct, public API (new/process/resize/get_snapshot/
                        get_scrollback_line/search), state queries (take_*/mode_state/
                        register_bell); declares sub-modules
    processor/
      write.rs        — write_char(), apply_wrap_pending(), write_char_at_width():
                        core character placement logic (pub(super) helpers)
      emoji.rs        — is_emoji_vs_eligible() (Unicode VS-16 table), flush_pending_emoji()
      regional_indicator.rs — handle_regional_indicator(), RI pairing and narrow flush
      screen.rs       — enter_alternate(), leave_alternate(), cursor/buffer helpers
      dispatch.rs     — impl vte::Perform for VtPerformBridge: CSI/OSC/ESC/execute dispatch;
                        delegates to sub-modules by sequence family
      dispatch/
        helpers.rs    — common param extraction utilities
        print.rs      — print(), DEC special graphics
        execute.rs    — C0 controls: BEL, BS, HT, LF, CR, SI/SO
        osc.rs        — osc_dispatch(): title stack, hyperlinks, clipboard
        csi_cursor.rs — CUU/CUD/CUF/CUB/CUP/HVP/CHA/HPA/VPA cursor movement
        csi_erase.rs  — ED/EL/ECH/DECSR erase operations
        csi_scroll.rs — scroll region, IL/DL/S/T
        csi_modes.rs  — DECSET/DECRST mode handlers
        csi_misc.rs   — DECSCUSR/DSR/DA cursor shape; DECSC/DECRC save/restore
        esc.rs        — ESC sequences: RI, charset, keypad modes
      tests.rs        — unit + security test entry point (cfg(test)); declares test sub-modules
      tests/
        helpers.rs    — make_vt(), grapheme_at(), attrs_at() (shared test utilities)
        security.rs   — SEC-PTY-001..007 security tests
        basic.rs      — TEST-VT-002..007: split CSI, UTF-8, wide chars, SGR
        modes.rs      — TEST-VT-008..023: cursor, alt screen, scroll region, charset
        editing.rs    — ICH/DCH/IL/DL/RI/DECAWM/CSI scroll/cursor shape/BEL; declares sub-modules
        editing/
          cursor.rs              — resize/cursor-position tests
          text_composition.rs    — combining chars, phantom cells
          scrolling.rs           — CSI S/T scroll tests
          cursor_shape.rs        — DECSCUSR tests
          bell.rs                — BEL rate limiting tests
          char_insert_delete.rs  — ICH/DCH tests
          line_insert_delete.rs  — IL/DL tests
          reverse_index.rs       — RI tests
          wrap_mode.rs           — DECAWM tests
          osc_title.rs           — OSC title sanitization tests
        features.rs   — TEST-SB-002, TEST-OSC8 (hyperlinks), TEST-OSC52 (clipboard)
        cursor_dirty.rs — DirtyRegion.cursor_moved tracking
        resize_full_redraw.rs — full_redraw flag on resize
    screen_buffer.rs  — ScreenBuffer: cell grid (normal + alternate), scrollback ring,
                        dirty tracking, resize, snapshot. Scrollback policy: only lines
                        scrolled off the top of a full-screen scroll region enter the ring.
                        Lines evicted by a partial DECSTBM region (margins not spanning the
                        full screen) are discarded — they do not enter scrollback (FS-VT-053,
                        FS-SB-004). Declares sub-modules.
    screen_buffer/
      dirty_rows.rs   — DirtyRows: bitfield dirty-row tracking (up to 256 rows)
      scrollback.rs   — ScrollbackLine, ScrollbackLineRef structs
      snapshot.rs     — ScreenSnapshot, SnapshotCell: serializable frontend rendering contract
      dirty_region.rs — DirtyRegion: cursor + cell dirty tracking, merge logic
      buffer.rs       — ScreenBuffer struct: grid access (new/get/get_mut/get_row)
      operations.rs   — erase/insert/delete/scroll/resize/take_dirty/snapshot methods
    cell.rs           — Cell, CellAttrs (SGR attributes), Color (Ansi16/Ansi256/Rgb),
                        Hyperlink; all Copy/Clone/PartialEq
    modes.rs          — ModeState: all DECSET/DECRST boolean and enum modes;
                        save/restore on alternate screen switch
    sgr.rs            — apply_sgr()/parse_extended_color(): SGR attribute parsing → CellAttrs delta;
                        colon sub-params for ITU T.416 and extended underline.
                        Test module split into sgr/tests/ sub-modules by attribute family
                        (reset, attributes, ansi_colors, color_256, truecolor, underline, edge_cases).
    osc.rs            — OSC dispatch: title (0/1/2), title stack (22/23),
                        hyperlink (8), clipboard (52) with per-connection policy
    mouse.rs          — Mouse event encoding: X10, SGR (1006), URXVT (1015);
                        mode arbitration: if SGR (1006) active → encode as SGR regardless
                        of other modes; else if URXVT (1015) active → encode as URXVT;
                        else encode as X10 (limited to col/row ≤ 223). Matches xterm
                        reference behavior (FS-VT-081).
    search.rs         — SearchQuery, SearchMatch types; re-exports search_scrollback; declares sub-modules
    search/
      matcher.rs           — Matcher enum, build_matcher(): regex/literal dispatch
      text_conversion.rs   — cells_to_text(), logical_line_to_text(), phantom cell handling
      logical_lines.rs     — LogicalLine, build_logical_lines(): soft-wrap boundary joining
      literal.rs           — find_literal_logical(): literal match finding
      api.rs               — search_scrollback() public entry point
    charset.rs        — DEC Special Graphics mapping; SI/SO charset switching;
                        G0/G1 designator state

  session.rs          — re-exports: SessionRegistry, TabSession, PaneSession,
                        SessionState, PaneState, TabState, TabId, PaneId,
                        SplitDirection, CreateTabConfig
  session/
    registry.rs       — SessionRegistry: struct, new(), get_state_snapshot(),
                        CreateTabConfig, ScrollPositionState, constants; declares sub-modules
    registry/
      tab_ops.rs      — create_tab, close_tab, rename_tab, reorder_tab, set_active_tab,
                        update_pane_title, get_state_snapshot (tab-level state reads)
      pane_ops.rs     — split_pane, close_pane, send_input, scroll_pane, resize_pane,
                        set_active_pane
      pane_state.rs   — accessors: get_pane_vt/dims/snapshot/termination_info;
                        lifecycle: mark_pane_terminated, has_foreground_process;
                        queries: is_active_pane, get_tab_state_for_pane, is_local_pane
      shell.rs        — resolve_shell_path(): login shell detection, $SHELL fallback
      layout.rs       — PaneNode tree helpers: replace_leaf_with_split,
                        update_pane_title_in_tree, remove_pane_from_tree
      pty_helpers.rs  — get_reader_handle(), get_writer_handle(): PTY I/O access
      tests.rs        — unit tests (login shell detection, etc.)
    tab.rs            — TabSession: Vec<PaneId>, metadata, notification state
    pane.rs           — PaneSession: owns PtyTaskHandle or SshChannelHandle;
                        Arc<RwLock<VtProcessor>>; PaneLifecycleState
    lifecycle.rs      — PaneLifecycleState enum; transitions; restart logic
    output.rs         — Source-agnostic emit pipeline: ProcessOutput, Coalescer,
                        CoalescerConfig, emitter, event builders (ADR-0028)
    output/
      process_output.rs — ProcessOutput type and merge logic
      coalescer.rs      — Coalescer, CoalescerConfig, CoalescerContext, async fn run()
      emitter.rs        — EmitOutcome, output_emits_screen_update, emit_all_pending()
      event_builders.rs — build_mode_state_event(), build_screen_update_event(), cell_color_to_dto()
      tests.rs          — TEST-ACK-*, TEST-ADPT-*, TEST-PIPC2-UNIT-*, COAL-MERGE-*
    pty_task.rs       — PtyTaskHandle; re-exports spawn_pty_read_task
    pty_task/
      reader.rs        — Task 1: PTY blocking read loop + termination
    ssh_task.rs       — 2-task SSH pipeline: async channel reader + shared coalescer;
                        extract_process_output helper; DSR/CPR response coalescing (ADR-0028)
    ssh_injectable.rs — (e2e-testing) SshInjectableRegistry for E2E test injection
    resize.rs         — debounce resize (16–33ms Tokio timer); TIOCSWINSZ;
                        SSH window-change; SIGWINCH
    ids.rs            — TabId, PaneId, ConnectionId newtypes; UUID generation

  ssh.rs              — re-exports: SshManager, SshConnectionConfig,
                        SshLifecycleState, Credentials, HostKeyInfo
  ssh/
    manager.rs        — SshManager struct def; re-exports all public items; declares sub-modules.
                        Manages live sessions only. Saved SshConnectionConfig are owned by
                        PreferencesStore (sub-key `connections`) — SshManager reads/writes
                        them via State<PreferencesStore>; it holds no connection store of its own.
    manager/
      pending.rs     — PendingCredentials, PendingHostKey structs
      credentials.rs — Credentials struct + manual Debug impl (SEC-CRED-003: no token leak)
      lifecycle.rs   — new(), open_connection(), close_connection(), reconnect()
      auth.rs        — connect_task(), try_authenticate()
      io_ops.rs      — send_input(), resize_pane(), provide_credentials(), get_state(), connection_count()
    connection.rs     — SshConnection: state machine; russh client handle;
                        routes PTY output → VtProcessor; resize; emits ssh-state-changed
    auth.rs           — auth sequence: publickey → keyboard-interactive → password;
                        credential prompt round-trip
    known_hosts.rs    — KnownHostEntry, KnownHostLookup types; re-exports KnownHostsStore;
                        OpenSSH-compatible format; import from ~/.ssh/known_hosts on explicit
                        user action only (see [§8.3](06-appendix.md#83-ssh-security)); declares sub-modules
    known_hosts/
      store.rs        — KnownHostsStore: new/load/lookup/lookup_with_system_fallback/
                        add_entry/remove_entries_for_host/import_from
    keepalive.rs      — Tokio keepalive task: interval, miss counter, disconnect trigger
    algorithms.rs     — deprecated algorithm detection; emits in-pane banner event

  preferences.rs      — re-exports: PreferencesStore, Preferences, UserTheme,
                        PreferencesPatch
  preferences/
    store.rs          — PreferencesStore: struct, public API (load/apply_patch/save_theme/
                        save_connection/etc.); declares sub-modules
    store/
      io.rs           — load_from_disk(), clamp_connections(), parse_toml_prefs():
                        disk I/O, TOML parsing, JSON migration fallback
      schema_convert.rs — rename_toml_keys(), camel_to_snake(), snake_to_camel():
                          key conversion bridge (IPC camelCase ↔ TOML snake_case)
      path.rs         — preferences_path(), dirs_or_home(): XDG config path resolution
      tests.rs        — unit tests: apply_patch, key conversion, connection limits
    schema.rs         — Preferences top-level struct; re-exports all nested types; declares sub-modules
    schema/
      appearance.rs  — AppearancePrefs, AppearancePatch, CursorStyle
      terminal.rs    — TerminalPrefs, BellType
      keyboard.rs    — KeyboardPrefs
      language.rs    — Language enum (FS-I18N-006, SEC-IPC-005 — never a free String)
      theme.rs       — UserTheme struct
      patch.rs       — PreferencesPatch struct

  credentials.rs      — public API: CredentialManager (wraps PAL CredentialStore)

  security_load.rs    — Load tests: SPL-RM-001 (fd leak check after pane open/close),
                        SPL-SZ-004 (rapid input validation under load); run in test suite
  security_static_checks.rs — Static security tests: SEC-CSP-002 (unsafe-eval absence),
                        SEC-CSP-003 (no @html with message accessors),
                        SEC-IPC-003 (Credentials Debug redaction); run at test time

  events.rs           — typed event definitions and emit helpers
  events/
    types.rs          — SessionStateChanged, SshStateChangedEvent, ScreenUpdateEvent,
                        ScrollPositionChangedEvent, etc. (mirrors UXD §15 types as Rust structs)

  commands.rs         — re-exports all command handler functions for generate_handler![]
  commands/
    session_cmds.rs   — create_tab, close_tab, rename_tab, reorder_tab,
                        split_pane, close_pane, set_active_tab, set_active_pane
    input_cmds.rs     — send_input, scroll_pane, scroll_to_bottom, search_pane,
                        get_pane_screen_snapshot, resize_pane
    ssh_cmds.rs       — open_ssh_connection, close_ssh_connection, reconnect_ssh
    ssh_prompt_cmds.rs — provide_credentials, accept_host_key, reject_host_key,
                        dismiss_ssh_algorithm_warning
    connection_cmds.rs — get_connections, save_connection, delete_connection,
                        duplicate_connection
    preferences_cmds.rs — get_preferences, update_preferences, get_themes,
                        save_theme, delete_theme
    system_cmds.rs    — re-exports all command functions; declares sub-modules
    system_cmds/
      clipboard.rs   — copy_to_clipboard, get_clipboard, MAX_CLIPBOARD_LEN
      url.rs         — open_url, validate_url_scheme (SEC-PATH-003/004)
      window.rs      — toggle_fullscreen
      preferences.rs — mark_context_menu_used
      session.rs     — get_session_state
    testing.rs        — E2E testing commands (--features e2e-testing only):
                        inject_pty_output, inject_ssh_failure, SshFailureRegistry

  platform.rs         — trait definitions: PtyBackend, PtySession, CredentialStore,
                        ClipboardBackend, NotificationBackend; factory fns: create_pty_backend(),
                        create_credential_store(), create_clipboard_backend(),
                        create_notification_backend();
                        #[cfg(target_os = ...)] dispatch lives here, not in sub-files
  platform/
    pty_linux.rs          — re-exports LinuxPtyBackend, LinuxPtySession; declares sub-modules
    pty_linux/
      backend.rs    — LinuxPtyBackend + PtyBackend impl (open_session)
      session.rs    — LinuxPtySession + PtySession impl (read/write/resize/close/process query)
    credentials_linux.rs  — SecretService D-Bus adapter; SecVec<u8> zeroizing; fallback
    clipboard_linux.rs    — arboard adapter; X11 PRIMARY; Wayland fallback
    notifications_linux.rs — D-Bus org.freedesktop.Notifications adapter; no-op fallback
                             if D-Bus unavailable; triggered by VtProcessor on BEL in
                             non-active pane (FS-VT-090)
    pty_macos.rs          — stub (unimplemented!())
    credentials_macos.rs  — stub
    clipboard_macos.rs    — stub
    notifications_macos.rs — stub
    pty_windows.rs        — stub
    credentials_windows.rs — stub
    clipboard_windows.rs  — stub
    notifications_windows.rs — stub
    pty_injectable.rs     — Injectable PTY backend (--features e2e-testing only):
                            InjectableRegistry, InjectablePtyBackend, InjectablePtySession;
                            replaces real PTY with in-process mpsc channel for E2E test byte injection
    validation.rs         — Path validation utilities: validate_ssh_identity_path(),
                            validate_shell_executable_path(), permission checks (Unix mode 0700)
```

### 3.3 Public Interfaces per Module

#### `session` module

```rust
// Injected into Tauri State<T>
pub struct SessionRegistry {
    // private fields
}

impl SessionRegistry {
    pub fn create_tab(&self, config: CreateTabConfig) -> Result<TabState>;
    pub fn close_tab(&self, id: TabId) -> Result<()>;
    pub fn rename_tab(&self, id: TabId, label: Option<String>) -> Result<TabState>;
    pub fn reorder_tab(&self, id: TabId, new_order: u32) -> Result<()>;
    // Returns the updated TabState (including full PaneNode tree) after the split.
    pub fn split_pane(&self, pane_id: PaneId, direction: SplitDirection) -> Result<TabState>;
    // Returns Some(TabState) if the tab still has panes after closing, None if the tab was removed.
    pub fn close_pane(&self, pane_id: PaneId) -> Result<Option<TabState>>;
    pub fn send_input(&self, pane_id: PaneId, data: Vec<u8>) -> Result<()>;
    pub fn scroll_pane(&self, pane_id: PaneId, offset: i64) -> Result<ScrollPositionState>;
    pub fn get_state_snapshot(&self) -> SessionState;
}
```

#### `vt` module

```rust
pub struct VtProcessor {
    // private: vte::Parser, ScreenBuffer, ModeState
}

impl VtProcessor {
    pub fn new(cols: u16, rows: u16) -> Self;
    pub fn process(&mut self, bytes: &[u8]) -> DirtyRegion;
    pub fn resize(&mut self, cols: u16, rows: u16);
    pub fn get_snapshot(&self) -> ScreenSnapshot;
    pub fn get_scrollback_line(&self, index: usize) -> Option<Vec<Cell>>;
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchMatch>;
}
```

#### `ssh` module

`SshManager` manages live SSH sessions only. It does not own a connection config store. Saved `SshConnectionConfig` are read from and written to `PreferencesStore` (sub-key `connections`) via `State<PreferencesStore>`. Command handlers in `ssh_cmds.rs` retrieve the config from `PreferencesStore` before passing it to `SshManager::open_connection`.

```rust
pub struct SshManager {
    // private: DashMap<PaneId, SshConnection>
    // No connection config store — configs come from PreferencesStore.
}

impl SshManager {
    pub async fn open_connection(
        &self,
        pane_id: PaneId,
        config: &SshConnectionConfig,
        credentials: Option<Credentials>,
    ) -> Result<()>;
    pub async fn close_connection(&self, pane_id: PaneId) -> Result<()>;
    pub async fn reconnect(&self, pane_id: PaneId) -> Result<()>;
}
```

#### `preferences` module

```rust
pub struct PreferencesStore {
    // private
}

impl PreferencesStore {
    pub fn load() -> Result<Self>;
    pub fn get(&self) -> &Preferences;
    pub fn apply_patch(&self, patch: PreferencesPatch) -> Result<Preferences>;
    pub fn get_themes(&self) -> Vec<UserTheme>;
    pub fn save_theme(&self, theme: UserTheme) -> Result<()>;
    pub fn delete_theme(&self, name: &str) -> Result<()>;
}
```

### 3.4 Newtype IDs

All entity IDs are newtypes over `String` (UUID v4), defined in `session/ids.rs` and re-exported from `session.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(String);
```

These prevent silent mixing of IDs across entity types. All command handlers accept newtype IDs, not raw strings.

---

## 9. Error Handling Strategy

### 9.1 Rust Backend

- `?` operator for propagation throughout internal code.
- `thiserror` for defining module-specific error types with descriptive variants.
- `anyhow` is permitted in command handlers (where context enrichment is needed) but not in library code (where callers need to match on specific error variants).
- No `unwrap()` or `expect()` on any data that originates from user input, the filesystem, OS calls, or the network. `unwrap()` is permitted only in initialization code where failure is a programming error (e.g., building a regex from a literal pattern). `PreferencesStore::load_or_default()` is the canonical example of this policy: preference file corruption is an expected filesystem condition, not a programming error, and is handled with a logged fallback to defaults (see [§7.6](04-runtime-platform.md#76-preferencesstore-load-strategy)).
- Errors from the PTY read task (unexpected fd close, OS errors) transition the pane to `Terminated` state with an error description; they do not crash the application.
- Errors from the SSH library (network errors, protocol errors) transition the SSH session to `Disconnected` state; they are communicated to the frontend via `SshStateChangedEvent`.

### 9.2 Mapping to Frontend

Every backend error that reaches a command handler must be converted to `TauTermError` with:
- A stable `code` string (snake_case, upper-case, module-prefixed: e.g., `SSH_KEEPALIVE_TIMEOUT`, `PTY_SPAWN_FAILED`, `PREF_INVALID_VALUE`)
- A human-readable `message` that a non-technical user can understand (FS-UX-001)
- An optional `detail` with the raw system error for technical users

### 9.3 Frontend

- All `invoke()` calls are wrapped in `try/catch`. Unhandled `invoke` errors are reported via the application's error display mechanism (FS-UX-001 pattern: plain message + collapsible detail).
- Svelte component-level errors (rendering failures, unexpected nulls) do not propagate to the terminal session; each pane is isolated.
