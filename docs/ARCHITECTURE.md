<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Architecture Document

> **Version:** 1.5.0
> **Date:** 2026-04-04
> **Status:** Living document — update when architectural decisions change
> **Author:** Software Architect — TauTerm team

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architectural Principles](#2-architectural-principles)
3. [Rust Module Decomposition](#3-rust-module-decomposition)
4. [IPC Contract](#4-ipc-contract)
5. [State Machines](#5-state-machines)
6. [Concurrency Model](#6-concurrency-model)
7. [Platform Abstraction Layer](#7-platform-abstraction-layer)
8. [Security Architecture](#8-security-architecture)
9. [Error Handling Strategy](#9-error-handling-strategy)
10. [Build Architecture](#10-build-architecture)
11. [Frontend Architecture](#11-frontend-architecture)
12. [Future Extensibility](#12-future-extensibility)
13. [ADR Index](#13-adr-index)
14. [Testing Strategy](#14-testing-strategy)

---

## 1. Overview

### 1.1 System Layers

TauTerm is structured as a two-process application separated by Tauri's IPC boundary:

```
┌─────────────────────────────────────────────────────────────────┐
│  Frontend (WebView — WebKitGTK on Linux)                        │
│                                                                 │
│  Svelte 5 + Tailwind 4 + Bits UI + Lucide                       │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │ Tab Bar  │ │ Terminal │ │ SSH/Conn.│ │ Preferences Panel │  │
│  │          │ │ Renderer │ │ Manager  │ │                   │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────────┘  │
└───────────────────────────┬─────────────────────────────────────┘
                            │ invoke() / listen()  [IPC boundary]
┌───────────────────────────┴─────────────────────────────────────┐
│  Backend (Rust process — Tokio async runtime)                   │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ session      │  │ vt / parser  │  │ ssh                  │  │
│  │ (tab, pane,  │  │ (vte crate + │  │ (russh / ssh2-rs)    │  │
│  │  lifecycle)  │  │  ScreenBuf)  │  │                      │  │
│  └──────┬───────┘  └──────┬───────┘  └─────────┬────────────┘  │
│         │                 │                     │               │
│  ┌──────┴─────────────────┴─────────────────────┴────────────┐  │
│  │ platform/ (PAL)                                            │  │
│  │  PtyBackend │ CredentialStore │ ClipboardBackend           │  │
│  └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
         │ PTY master fd      │ Secret Service D-Bus
         ↓                    ↓
    [Shell / SSH]          [Keychain]
```

**Data flow for terminal output:**

```
PTY read (async) → vt/parser (vte crate) → ScreenBuffer mutations
   → dirty cell tracking → screen-update event → IPC → Frontend renderer
```

**Data flow for keyboard input:**

```
Frontend keydown → invoke('send_input', {pane_id, data})
   → session module → PtySession::write() → PTY master fd
```

### 1.2 Technology Stack

| Layer | Technology | Decision |
|-------|-----------|---------|
| Application shell | Tauri 2 | ADR-0001 |
| Backend language | Rust (edition 2024) | — |
| Async runtime | Tokio | — |
| VT parser | `vte` crate | ADR-0003 |
| PTY management | `portable-pty` via PAL | ADR-0002, ADR-0005 |
| SSH client | `russh` (preferred) or `ssh2-rs` | ADR-0007 |
| Frontend framework | Svelte 5 with runes | ADR-0004 |
| Frontend state | Svelte 5 runes (`$state`, `$derived`, `$effect`) | ADR-0004 |
| CSS framework | Tailwind 4 (`@theme` design tokens) | — |
| Component primitives | Bits UI | — |
| Icons | Lucide-svelte | — |
| Terminal renderer | DOM + row virtualization + attribute-run merging | ADR-0008 |
| Build tool (frontend) | Vite via SvelteKit | — |

### 1.3 Platform Targets

| Platform | v1 Status | v2+ Path |
|----------|-----------|---------|
| Linux x86\_64 | Supported | — |
| Linux x86, ARM32, ARM64, RISC-V | Supported | — |
| macOS | Not supported | PAL stubs ready; russh cross-platform; Keychain via PAL |
| Windows | Not supported | PAL stubs ready; russh/ssh2-rs cross-platform; ConPTY via portable-pty |

---

## 2. Architectural Principles

These principles govern every module boundary and interface decision in the codebase.

### 2.1 Single Source of Truth

Each piece of state has exactly one authoritative owner:
- **Session topology** (tabs, panes, active pane): the `session` module in the Rust backend. The frontend receives it via events and does not maintain its own authoritative copy.
- **Screen buffer content**: the `ScreenBuffer` in the `vt` module, one instance per pane.
- **PTY state**: the `PtySession` trait object, managed by the `session` module.
- **User preferences**: the `PreferencesStore` in the `preferences` module.
- **SSH lifecycle state**: the `SshSession` state machine in the `ssh` module.

The frontend holds a **replica** of backend state for rendering. It updates this replica in response to events; it never speculatively modifies it.

### 2.2 Unidirectional Data Flow

```
User action → Frontend invoke() → Backend command handler → State change
   → Backend emit() event → Frontend replica update → UI re-render
```

The frontend never mutates shared state directly. The backend never pushes imperative UI instructions. Events carry state, not commands.

### 2.3 Module Isolation

Each Rust module exposes a public API through a small number of types and functions. Implementation details (internal state types, helper functions) are private to the module. Cross-module communication happens through the `session` module as coordinator, or through Tauri's `State<T>` injection — never through direct coupling between sibling modules.

### 2.4 No Global Mutable State

No `static mut`, no `lazy_static!` with interior mutability, no `Arc<Mutex<GlobalSomething>>` accessible from multiple unrelated modules. State is owned by the `session` module and passed to submodules through function parameters or trait method calls.

### 2.5 Parse Don't Validate at the IPC Boundary

Every `#[tauri::command]` function receives strongly-typed inputs. Newtype wrappers (`PaneId`, `TabId`, `ConnectionId`) prevent confusion between IDs of different entity kinds. All validation (path traversal checks, URI scheme validation, sequence length limits) happens at the entry point — not scattered through internal logic.

### 2.6 YAGNI

The architecture is designed to accommodate future features (session persistence, plugin system, cloud sync) without requiring redesign, but it does not implement them. Extension points are trait boundaries and module interfaces, not pre-built infrastructure.

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
                        get_scrollback_line/search), private helpers; declares sub-modules
    processor/
      dispatch.rs     — impl vte::Perform for VtPerformBridge: CSI/OSC/ESC/execute dispatch
      tests.rs        — unit + security tests (cfg(test))
    screen_buffer.rs  — ScreenBuffer: cell grid (normal + alternate), scrollback ring,
                        dirty tracking, resize, snapshot. Scrollback policy: only lines
                        scrolled off the top of a full-screen scroll region enter the ring.
                        Lines evicted by a partial DECSTBM region (margins not spanning the
                        full screen) are discarded — they do not enter scrollback (FS-VT-053,
                        FS-SB-004).
    cell.rs           — Cell, CellAttrs (SGR attributes), Color (Ansi16/Ansi256/Rgb),
                        Hyperlink; all Copy/Clone/PartialEq
    modes.rs          — ModeState: all DECSET/DECRST boolean and enum modes;
                        save/restore on alternate screen switch
    sgr.rs            — SGR attribute parsing: parse_sgr_params() → CellAttrs delta;
                        colon sub-params for ITU T.416 and extended underline
    osc.rs            — OSC dispatch: title (0/1/2), title stack (22/23),
                        hyperlink (8), clipboard (52) with per-connection policy
    mouse.rs          — Mouse event encoding: X10, SGR (1006), URXVT (1015);
                        mode arbitration: if SGR (1006) active → encode as SGR regardless
                        of other modes; else if URXVT (1015) active → encode as URXVT;
                        else encode as X10 (limited to col/row ≤ 223). Matches xterm
                        reference behavior (FS-VT-081).
    search.rs         — search(): iterate scrollback, skip soft-wrap boundaries,
                        return SearchMatch positions
    charset.rs        — DEC Special Graphics mapping; SI/SO charset switching;
                        G0/G1 designator state

  session.rs          — re-exports: SessionRegistry, TabSession, PaneSession,
                        SessionState, PaneState, TabState, TabId, PaneId,
                        SplitDirection, CreateTabConfig
  session/
    registry.rs       — SessionRegistry: HashMap<TabId, TabSession>; public API;
                        emits session-state-changed; State<Arc<SessionRegistry>>
    tab.rs            — TabSession: Vec<PaneId>, metadata, notification state
    pane.rs           — PaneSession: owns PtyTaskHandle or SshChannelHandle;
                        Arc<RwLock<VtProcessor>>; PaneLifecycleState
    lifecycle.rs      — PaneLifecycleState enum; transitions; restart logic
    pty_task.rs       — PtyReadTask: Tokio task per pane; reads AsyncFd,
                        calls VtProcessor::process, coalesces dirty regions,
                        emits screen-update; back-pressure
    resize.rs         — debounce resize (16–33ms Tokio timer); TIOCSWINSZ;
                        SSH window-change; SIGWINCH
    ids.rs            — TabId, PaneId, ConnectionId newtypes; UUID generation

  ssh.rs              — re-exports: SshManager, SshConnectionConfig,
                        SshLifecycleState, Credentials, HostKeyInfo
  ssh/
    manager.rs        — SshManager: DashMap<PaneId, SshConnection>; open/close/reconnect.
                        Manages live sessions only. Saved SshConnectionConfig are owned by
                        PreferencesStore (sub-key `connections`) — SshManager reads/writes
                        them via State<PreferencesStore>; it holds no connection store of its own.
    connection.rs     — SshConnection: state machine; russh client handle;
                        routes PTY output → VtProcessor; resize; emits ssh-state-changed
    auth.rs           — auth sequence: publickey → keyboard-interactive → password;
                        credential prompt round-trip
    known_hosts.rs    — TauTerm known-hosts file: parse, lookup, add, update;
                        OpenSSH-compatible format; import action from ~/.ssh/known_hosts
                        (explicit user action only — not read automatically at startup; see §8.3)
    keepalive.rs      — Tokio keepalive task: interval, miss counter, disconnect trigger
    algorithms.rs     — deprecated algorithm detection; emits in-pane banner event

  preferences.rs      — re-exports: PreferencesStore, Preferences, UserTheme,
                        PreferencesPatch
  preferences/
    store.rs          — PreferencesStore: load/save from disk with schema validation
    schema.rs         — Preferences struct and all nested types (serde + validation)

  credentials.rs      — public API: CredentialManager (wraps PAL CredentialStore)

  commands.rs         — re-exports all command handler functions for generate_handler![]
  commands/
    session_cmds.rs   — create_tab, close_tab, rename_tab, reorder_tab,
                        split_pane, close_pane, set_active_pane
    input_cmds.rs     — send_input, scroll_pane, scroll_to_bottom, search_pane,
                        get_pane_screen_snapshot
    ssh_cmds.rs       — open_ssh_connection, close_ssh_connection, reconnect_ssh
    ssh_prompt_cmds.rs — provide_credentials, accept_host_key, reject_host_key,
                        dismiss_ssh_algorithm_warning
    connection_cmds.rs — get_connections, save_connection, update_connection,
                        delete_connection
    preferences_cmds.rs — get_preferences, update_preferences, get_themes,
                        save_theme, delete_theme
    system_cmds.rs    — copy_to_clipboard, get_clipboard, open_url,
                        mark_context_menu_used, get_session_state

  platform.rs         — trait definitions: PtyBackend, PtySession, CredentialStore,
                        ClipboardBackend, NotificationBackend; factory fns: create_pty_backend(),
                        create_credential_store(), create_clipboard_backend(),
                        create_notification_backend();
                        #[cfg(target_os = ...)] dispatch lives here, not in sub-files
  platform/
    pty_linux.rs          — UnixPtySystem wrapper; AsyncFd extraction; O_CLOEXEC
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

  events.rs           — typed event definitions and emit helpers
  events/
    types.rs          — SessionStateChanged, SshStateChangedEvent, ScreenUpdateEvent,
                        ScrollPositionChangedEvent (mirrors UXD §15 types as Rust structs)
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

## 4. IPC Contract

### 4.1 Guiding Policy

- One `#[tauri::command]` per user action (ADR-0006)
- All input and output types are `serde`-serializable — no OS handles, no raw pointers
- Error responses use a uniform error envelope (`TauTermError` with a code and human-readable message)
- Commands return `Result<T, TauTermError>` — the frontend receives either data or an error, never panics or opaque failures
- Events are emitted by the backend only; the frontend does not emit events to the backend

### 4.2 Commands (invoke — frontend → backend)

| Command | Input | Output | Description |
|---------|-------|--------|-------------|
| `get_session_state` | — | `SessionState` | Full snapshot on frontend mount |
| `create_tab` | `CreateTabConfig` | `TabState` | Open new tab with local PTY |
| `close_tab` | `{ tab_id: TabId }` | `()` | Close tab (confirmation handled by frontend) |
| `rename_tab` | `{ tab_id: TabId, label: Option<String> }` | `TabState` | Set/clear user label |
| `reorder_tab` | `{ tab_id: TabId, new_order: u32 }` | `()` | Move tab to position |
| `split_pane` | `{ pane_id: PaneId, direction: SplitDirection }` | `TabState` | Split active pane; returns updated tab layout tree |
| `close_pane` | `{ pane_id: PaneId }` | `TabState \| null` | Close pane; returns updated `TabState` if the tab still exists, `null` if the last pane was closed (tab removed) |
| `set_active_pane` | `{ pane_id: PaneId }` | `()` | Change focus to pane |
| `send_input` | `{ pane_id: PaneId, data: Vec<u8> }` | `()` | Write bytes to PTY |
| `scroll_pane` | `{ pane_id: PaneId, offset: i64 }` | `ScrollPositionState` | Scroll scrollback |
| `scroll_to_bottom` | `{ pane_id: PaneId }` | `()` | Jump to bottom of scrollback |
| `search_pane` | `{ pane_id: PaneId, query: SearchQuery }` | `Vec<SearchMatch>` | Search scrollback |
| `open_ssh_connection` | `{ pane_id: PaneId, connection_id: ConnectionId }` | `()` | Begin SSH connect flow |
| `close_ssh_connection` | `{ pane_id: PaneId }` | `()` | Close SSH session |
| `reconnect_ssh` | `{ pane_id: PaneId }` | `()` | Reconnect after Disconnected |
| `get_connections` | — | `Vec<SshConnectionConfig>` | List saved SSH connections |
| `save_connection` | `SshConnectionConfig` | `ConnectionId` | Create or update a saved connection |
| `delete_connection` | `{ connection_id: ConnectionId }` | `()` | Delete saved connection |
| `get_preferences` | — | `Preferences` | Read current preferences |
| `update_preferences` | `PreferencesPatch` | `Preferences` | Write preferences (immediate apply) |
| `get_themes` | — | `Vec<UserTheme>` | List all user themes |
| `save_theme` | `UserTheme` | `()` | Create or update a theme |
| `delete_theme` | `{ name: String }` | `()` | Delete a user theme |
| `provide_credentials` | `{ pane_id: PaneId, credentials: Credentials }` | `()` | Respond to SSH credential prompt |
| `accept_host_key` | `{ pane_id: PaneId }` | `()` | Accept new/changed host key |
| `reject_host_key` | `{ pane_id: PaneId }` | `()` | Reject host key (abort connection) |
| `dismiss_ssh_algorithm_warning` | `{ pane_id: PaneId }` | `()` | Dismiss deprecated-algorithm banner |
| `copy_to_clipboard` | `{ text: String }` | `()` | Copy to CLIPBOARD selection |
| `get_clipboard` | — | `String` | Read CLIPBOARD content |
| `open_url` | `{ url: String }` | `()` | Open validated URL in system browser |
| `mark_context_menu_used` | — | `()` | Clear first-launch context menu hint |
| `resize_pane` | `{ pane_id: PaneId, cols: u16, rows: u16, pixel_width: u16, pixel_height: u16 }` | `()` | Notify backend of pane resize; triggers `TIOCSWINSZ` + `SIGWINCH`. `pixel_width`/`pixel_height` are required for complete `TIOCSWINSZ` (image protocols, multiplexers). Resize events are debounced — see §6.5. |
| `get_pane_screen_snapshot` | `{ pane_id: PaneId }` | `ScreenSnapshot` | Full screen state for initial render |

### 4.3 Events (emit — backend → frontend)

| Event name | Payload type | Trigger |
|------------|-------------|---------|
| `session-state-changed` | `SessionStateChanged` | Topology changes that originate asynchronously or outside a direct command: process title changed (OSC), pane process exited (SIGCHLD), active-tab/active-pane changed by user action via `set_active_pane`. **Not** emitted for `split_pane` or `close_pane` — those commands return the updated `TabState` directly. |
| `ssh-state-changed` | `SshStateChangedEvent` | SSH lifecycle state transition |
| `screen-update` | `ScreenUpdateEvent` | Terminal output processed (cell diffs or full snapshot) |
| `mode-state-changed` | `ModeStateChangedEvent` | A terminal mode relevant to frontend input encoding changed. Payload: `{ paneId: PaneId, decckm: bool, deckpam: bool }`. Without these flags, `keyboard.ts` cannot distinguish normal mode (ESC [ A/B/C/D) from application cursor mode (ESC O A/B/C/D), causing arrow key encoding errors in vim and readline (FS-KBD-007, FS-KBD-010). Emitted on DECSET/DECRST of modes 1 (DECCKM) and DECKPAM/DECKPNM (ESC =/ESC >). |
| `scroll-position-changed` | `ScrollPositionChangedEvent` | Scrollback position changed |
| `credential-prompt` | `CredentialPromptEvent` | Backend needs credentials from user |
| `host-key-prompt` | `HostKeyPromptEvent` | First connection or key change requiring user verification |
| `notification-changed` | `NotificationChangedEvent` | Tab/pane activity notification added or cleared |

### 4.4 Error Envelope

All commands return `Result<T, TauTermError>`. `TauTermError` is serialized as:

```typescript
interface TauTermError {
  code: string;       // machine-readable code, e.g., "PTY_SPAWN_FAILED", "INVALID_PANE_ID"
  message: string;    // human-readable summary (FS-UX-001: plain language)
  detail?: string;    // optional technical detail (errno, exit code, system message)
}
```

The frontend maps `code` to localized user-facing messaging; `detail` is displayed as collapsible technical information per FS-UX-001.

### 4.5 Pane Layout Topology and Delta Event Granularity

#### 4.5.1 Tree-structured layout

**Decision:** the IPC contract uses an **arborescent (tree-structured) pane layout**, not a flat array with `splitDirection`/`splitRatio` fields per pane.

UXD §15.1 specified `splitDirection` and `splitRatio` as flat fields on `PaneState`. This is insufficient for correctly representing a split pane tree of arbitrary depth: a flat representation cannot distinguish the structure of `(A | (B / C))` from `((A | B) / C)` without reconstructing the tree from parent references — which is fragile and order-dependent. The `split-tree.ts` frontend module requires a tree structure. The flat model in UXD §15.1 is superseded by this decision.

**`TabState.layout` replaces `TabState.panes`:**

```typescript
type PaneNode =
  | { type: 'leaf'; paneId: PaneId; state: PaneState }
  | { type: 'split'; direction: 'horizontal' | 'vertical'; ratio: number;
      first: PaneNode; second: PaneNode };

interface TabState {
  id: TabId;
  label: string | null;
  activePaneId: PaneId;
  order: number;
  layout: PaneNode;   // replaces panes: PaneState[]
}
```

The Rust equivalent lives in `events/types.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PaneNode {
    Leaf { pane_id: PaneId, state: PaneState },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}
```

#### 4.5.2 Delta event granularity — `SessionStateChanged`

**Decision:** `SessionStateChanged` carries the **complete `TabState`** of the affected tab, not a free-form `Partial<SessionState>`.

Rationale: a `Partial<SessionState>` approach requires the frontend to implement a deep partial merge over a recursive union type (`PaneNode`), which is non-trivial and error-prone (ambiguous semantics when a subtree is absent vs. unchanged). Sending the complete `TabState` of the changed tab is semantically unambiguous: the frontend atomically replaces its replica of that tab. The serialization cost is negligible — a tab with 1–8 panes produces a JSON payload of ≤ 2 KB.

A full `SessionState` snapshot on every change (alternative C) is rejected: it would couple the frequency of topology events to the total session size, which grows unboundedly.

```typescript
interface SessionStateChanged {
  changeType: 'tab-created' | 'tab-closed' | 'tab-reordered'
    | 'active-tab-changed' | 'active-pane-changed'
    | 'pane-metadata-changed';
  // Present for all changeTypes except 'tab-closed' (tab no longer exists).
  // Contains the complete, updated TabState of the affected tab.
  tab?: TabState;
  // Present when changeType is 'active-tab-changed' or 'tab-closed'
  // to allow the frontend to update its activeTabId replica.
  activeTabId?: string;
}
```

**`session-state-changed` is not emitted for `split_pane` or `close_pane`:** those commands return the updated `TabState` (or `null`) directly in their response. The command response is the authoritative new state; no event is emitted. This avoids a race between the command response and a redundant event.

`session-state-changed` is emitted only for changes that originate asynchronously or from outside a direct user command: OSC-driven process title change, pane process exit (SIGCHLD leading to `Terminated` state), and `set_active_pane` confirmation.

#### 4.5.3 `close_pane` return value and last-pane behavior

**Decision:** `close_pane` returns `TabState | null` (Rust: `Result<Option<TabState>, TauTermError>`), not `()`.

- `TabState` — the tab still exists; the returned value is its updated layout tree (sibling pane expanded to fill the closed pane's space).
- `null` — the closed pane was the last pane in the tab. The backend removes the tab atomically as part of the same operation and emits a single `session-state-changed` event with `changeType: 'tab-closed'`. The frontend does **not** invoke `close_tab` separately — the tab removal is implicit and complete upon receiving the `null` response.

**Last-tab behavior:** if the closed pane was the only pane in the last remaining tab, `close_pane` returns `null`. Per FS-TAB-008, closing the last tab closes the application window. The frontend is responsible for detecting this case (active tab list becomes empty after processing the `tab-closed` event) and calling Tauri's window close API.

This is consistent with `split_pane → TabState`. Both commands mutate the same layout tree; both return the post-mutation state synchronously. The frontend does not need to wait for a `session-state-changed` event to discover the new layout, and there is no inconsistency window between command completion and event delivery.

#### 4.5.4 `NotificationChangedEvent`

The `notification-changed` event carries per-pane notification state updates that originate asynchronously (bell received, output in background pane, process exit). It does not carry a full `TabState` — notifications are a narrow slice of `PaneState` and this event is higher-frequency than topology events.

```typescript
interface NotificationChangedEvent {
  tabId: string;
  paneId: string;
  notification: PaneNotification | null; // null = notification cleared
}
```

The frontend merges this into the `PaneState` within its `TabState` replica by locating the leaf node for `paneId` in the layout tree and updating its `notification` field.

#### 4.5.5 `session/registry.rs` responsibilities

`SessionRegistry` is responsible for building `TabState` (including the full `PaneNode` tree) on demand. It maintains a per-tab layout tree representation internally (not reconstructed from a flat map on every call). This tree is updated atomically on `split_pane`, `close_pane`, `rename_tab`, `reorder_tab`, and OSC title changes. The `get_state_snapshot()` method assembles `SessionState` by collecting all tab layouts.

### 4.6 Type Definitions (Rust ↔ TypeScript)

The canonical type definitions are in `src-tauri/src/events/types.rs` (Rust, authoritative) and mirrored in `src/lib/ipc/types.ts` (TypeScript, manually kept in sync). These must be kept coherent — any change to the Rust types requires a corresponding update to the TypeScript types. A future improvement is to generate TypeScript types from Rust using `specta` or `ts-rs`; this is not required for v1.

The types from UXD §15 (`SshLifecycleState`, `ScreenUpdateEvent`, `CellUpdate`, etc.) remain normative for the event payloads. The following supersessions apply (UXD §15 is to be read with these overrides):

| UXD §15 definition | Superseded by |
|--------------------|---------------|
| `TabState.panes: PaneState[]` | `TabState.layout: PaneNode` (§4.5.1) |
| `SessionStateChanged.state: Partial<SessionState>` | `SessionStateChanged.tab?: TabState` + `activeTabId?` (§4.5.2) |
| `close_pane → ()` | `close_pane → TabState \| null` (§4.5.3) |
| `notification-changed` payload (undefined) | `NotificationChangedEvent` (§4.5.4) |

---

## 5. State Machines

### 5.1 PTY Session Lifecycle

```
                    create_tab() / split_pane()
                            │
                            ▼
                    ┌───────────────┐
                    │   Spawning    │  — PTY allocated, child process forking
                    └───────┬───────┘
                            │ fork+exec success
                            ▼
                    ┌───────────────┐
                    │    Running    │  — PTY I/O active, input/output flowing
                    └───┬───────┬──┘
                        │       │
              SIGCHLD   │       │  close_pane() / close_tab()
              (exit)    │       │
                        ▼       ▼
              ┌──────────────┐  ┌──────────────┐
              │  Terminated  │  │   Closing    │  — SIGHUP sent to process group
              │  (exit code) │  └──────┬───────┘
              └──────┬───────┘         │ process exited
                     │                 ▼
              user: restart     ┌──────────────┐
              or close          │    Closed    │
                                └──────────────┘
```

**Terminated state:** The pane remains visible with the exit code displayed. The user can choose to restart (→ Spawning) or close (→ Closed). Closing the pane from Terminated does not require confirmation (FS-PTY-005, FS-PTY-006).

**Running → Closing:** Triggered by user close action. If a foreground process is running, the frontend shows a confirmation dialog before invoking `close_pane`. The backend receives `close_pane` only after user confirmation; no process check occurs inside the backend command handler.

**Last-pane close:** When the last pane in a tab is closed via `close_pane`, the tab is removed atomically by the backend. A single `session-state-changed` event with `changeType: 'tab-closed'` is emitted. If this was the last tab, the frontend detects the empty tab list and closes the window (FS-TAB-008). See §4.5.3 for the full return-value contract.

### 5.2 SSH Session Lifecycle

```
              open_ssh_connection()
                      │
                      ▼
              ┌───────────────┐
              │  Connecting   │  — TCP connection in progress
              └───────┬───────┘
                      │ TCP connected
                      ▼
              ┌───────────────────┐
              │  Authenticating   │  — SSH handshake + credential exchange
              └───────┬───────────┘
                      │ auth success
                      ▼
              ┌───────────────┐◄────────────────────┐
              │   Connected   │                     │ reconnect_ssh()
              └───┬───────┬───┘                     │
                  │       │                         │
    network drop  │       │ close_ssh_connection()  │
    / keepalive   │       │ / remote exit (code 0)  │
    timeout       │       │                         │
                  ▼       ▼                         │
        ┌──────────────┐  ┌──────────┐              │
        │ Disconnected │  │  Closed  │              │
        └──────┬───────┘  └──────────┘              │
               │                                    │
               └────────────────────────────────────┘
```

**Disconnected:** Network drop, keepalive timeout (FS-SSH-020: 3 missed keepalives), write failure, or remote process exit with non-zero code. Reconnection is available from this state.

**Closed:** User-initiated close, or remote process exit with code 0. No reconnection available from this state; a new `open_ssh_connection` is required.

The `SshLifecycleState` enum is the single source of truth for SSH state. It is emitted to the frontend via `ssh-state-changed` events and embedded in `PaneState` for snapshot queries.

### 5.3 VT Terminal Mode State

The `VtProcessor` maintains a set of terminal mode flags that control how incoming bytes are interpreted. These are not a linear state machine but a collection of orthogonal boolean and enumerated modes:

| Mode | Type | Reset value | Controls |
|------|------|-------------|---------|
| Screen buffer | `Normal | Alternate` | `Normal` | Which screen buffer is active (modes 1049, 47, 1047) |
| Cursor mode | `Normal | Application` | `Normal` | DECCKM: arrow key encoding |
| Mouse reporting | `None | X10 | Normal | ButtonEvent | AnyEvent` | `None` | Which mouse events go to PTY |
| Mouse encoding | `X10 | SGR | URXVT` | `X10` | Mouse event encoding format |
| Bracketed paste | `bool` | `false` | Whether paste is wrapped |
| Focus events | `bool` | `false` | Mode 1004 |
| Keypad mode | `Normal | Application` | `Normal` | DECKPAM/DECKPNM |
| Cursor visible | `bool` | `true` | DECTCEM |
| Scroll region | `(top: u16, bottom: u16)` | Full screen | DECSTBM |

On transition to alternate screen (mode 1049): save the entire mode state for the normal screen, reset to defaults for the alternate screen. On return: restore the saved normal screen state. This is implemented as a `ScreenState` struct that is saved/restored with the screen buffer.

On PTY/SSH session close: all modes are reset to defaults before emitting the final `screen-update` event.

**SGR 6 (rapid blink):** SGR 6 is treated identically to SGR 5 (slow blink) — same CSS rendering, same blink class. This matches xterm reference behavior. There is no distinction between slow and rapid blink in v1.

**Keyboard encoding constraints (v1):**
- `modifyOtherKeys` (CSI 27;mod;codepoint~) is **not** implemented in v1. Applications relying on `<C-Enter>`, `<S-Enter>`, or other keys only distinguishable via `modifyOtherKeys` may not receive the expected sequences. This is a documented v1 limitation.
- The Kitty keyboard protocol is out of scope for v1 (see §12.4).
- Shift+Enter is indistinguishable from Enter (both = 0x0D) in standard xterm encoding (FS-KBD-013).

---

## 6. Concurrency Model

### 6.1 VT Processing Pipeline

#### C1 control codes (0x80–0x9F)

TauTerm operates in a UTF-8 environment. Bytes in the range 0x80–0x9F are **not** interpreted as 8-bit C1 control codes (i.e., CSI/OSC/DCS 8-bit equivalents). They are treated as the leading bytes of UTF-8 multi-byte sequences. If the `vte` crate exposes an option to disable 8-bit C1 processing, it must be disabled. This avoids conflicts between C1 8-bit code processing and valid UTF-8 multi-byte sequences, which share the same byte range.

#### DCS dispatch

For v1, DCS sequences are handled as follows:
- The `vte::Perform` callbacks `dcs_hook`, `dcs_put`, and `dcs_unhook` are no-ops for all unrecognized DCS sequences — they are silently ignored.
- DECRQSS (Device Control Request Status String) receives an error response: the reply `P0$r<params>` with `P0` indicating "invalid request". This is the correct response for unsupported parameters and prevents applications from hanging waiting for a response.
- No other DCS sequences are recognized in v1.

#### Tokio Runtime

The backend uses a single Tokio multi-threaded runtime (standard `#[tauri::main]` setup). All async operations (PTY reads, SSH I/O, keepalive tasks) run on Tokio's thread pool.

### 6.2 PTY I/O Task

`PaneSession` holds an `Arc<RwLock<VtProcessor>>`. The `PtyReadTask` receives a clone of this `Arc` at creation time. Each `PaneSession` has a dedicated Tokio task (`tokio::spawn`) that runs the PTY read loop:

```
PtyReadTask (per pane):
  loop {
    let n = pty_master.read(&mut buf).await;       // non-blocking async read
    {
      let mut proc = vt_processor.write();          // write lock — held briefly
      proc.process(&buf[..n]);                     // VtProcessor: parse + update ScreenBuffer
      let dirty = proc.take_dirty_cells();         // collect cell diffs
      drop(proc);                                  // release write lock
      if !dirty.is_empty() {
        app_handle.emit("screen-update", ScreenUpdateEvent { ... });
      }
    }
  }
```

The write lock is held only for the duration of `process()` + `take_dirty_cells()` — a short, CPU-bound window. Command handlers (e.g., `get_pane_screen_snapshot`) acquire the read lock for snapshots or search; there is no structural conflict between the read task and command handlers. The `ScreenBuffer` is never accessed outside the `RwLock`.

### 6.3 Write Path (Input)

`send_input` command handler runs on Tokio's thread pool (Tauri spawns command handlers on the runtime). It acquires a write handle to the `PtySession` and writes synchronously (the PTY write is fast and non-blocking for small payloads). No lock contention with the read task.

### 6.4 SSH I/O

SSH sessions have their own async task structure managed by the SSH library (`russh` / `ssh2-rs`). The SSH channel output is piped to a `VtProcessor` in the same way as local PTY output. The `SshConnection` state machine runs in a separate Tokio task per connection.

### 6.5 Back-pressure

The backend emits `screen-update` events at the rate that PTY output arrives. At high terminal output rates (e.g., `cat /dev/urandom | head -c 10M`), this can produce many events per second. The frontend renderer must be able to process these without queuing unbounded events. Mitigation strategies:

1. **Coalescing:** The backend PTY read task coalesces multiple reads into a single event if reads complete faster than the event loop can process them. This is implemented by processing all available bytes before emitting a single event.
2. **Rate limiting:** If event frequency exceeds a configured threshold (e.g., 60 events/s per pane), the backend coalesces further before emitting.
3. **Frontend rendering:** The frontend does not re-render on every individual event; it uses `requestAnimationFrame` batching to render at most once per frame.

Back-pressure between the PTY read and the Tauri event system is a known performance risk (noted in ADR-0001). It requires profiling during development.

**Resize debounce:** `resize_pane` IPC calls from `TerminalPane`'s `ResizeObserver` are debounced by the backend (`session/resize.rs`): a 16–33ms Tokio timer is reset on each incoming call; `TIOCSWINSZ` + SIGWINCH are only issued after the timer fires (FS-PTY-010). The final size is always applied.

**Scroll follow semantics (`scroll.svelte.ts`):** when new output arrives on the normal screen and the user has scrolled back (viewport is not at bottom), the viewport stays at its current scroll position — it does not follow new output. A visual indicator ("new output below") is shown. The viewport follows output automatically only when already at the bottom. The user returns to the bottom via scroll action or keyboard shortcut. When on the alternate screen buffer, scroll navigation is disabled entirely (FS-SB-005).

**Combining characters and run-merge boundaries (`TerminalRow.svelte`):** combining characters (Unicode codepoints of width 0, categories Mn/Mc/Me) are stored in the `Cell` of the preceding base character, not in an independent cell. During attribute-run merging, a width-0 cell never starts a new `<span>` run — it is always folded into the preceding cell's run. This guarantees that the browser text shaping engine receives grapheme-complete sequences in each `<span>`, ensuring correct glyph rendering for accented characters and diacritics (FS-VT-012).

### 6.6 State Access Patterns

| State | Owner | Access pattern |
|-------|-------|---------------|
| `SessionRegistry` | `State<Arc<SessionRegistry>>` | `Arc` for multi-command access, internal `RwLock` per tab |
| `VtProcessor` (per pane) | `Arc<RwLock<VtProcessor>>` in `PaneSession` | Write lock: `PtyReadTask` (process + take_dirty_cells, brief). Read lock: command handlers (snapshot, search). |
| `ScreenBuffer` (per pane) | `VtProcessor` (internal) | Accessed only via the `RwLock` on `VtProcessor`. Never accessed directly from outside. |
| `PreferencesStore` | `State<Arc<RwLock<PreferencesStore>>>` | Read: many readers. Write: preferences command handler |
| `SshManager` | `State<Arc<SshManager>>` | `Arc` + internal `DashMap` for per-connection state |

---

## 7. Platform Abstraction Layer

See ADR-0005 for the full rationale. This section documents the four PAL traits and their Linux v1 implementations. ADR-0005 lists four OS primitives; the fourth (notifications) is documented in §7.4 below.

### 7.1 PtyBackend / PtySession

**Linux v1 implementation:** `portable-pty` crate (`UnixPtySystem`).

The `PtySession` trait wraps the `portable-pty` `MasterPty` and `Child` handles. Resize is delegated to `MasterPty::resize()`. The master file descriptor is exposed as a `tokio::io::unix::AsyncFd` for the PTY read task.

**SIGHUP delivery on close:** closing a `PtySession` (via `Drop` or explicit close) must close the master file descriptor. This is the kernel mechanism that delivers SIGHUP to the foreground process group (FS-PTY-007). The implementation must verify that `portable-pty`'s ownership model closes the underlying fd on `Drop` of the `MasterPty` handle — not merely dropping a Rust wrapper that leaves the fd open. If `portable-pty` does not guarantee this, an explicit `close(fd)` call must be issued in the `PtySession::Drop` implementation before the `portable-pty` handle is dropped.

**Login shell:** the first tab launches a login shell (FS-PTY-013). Since `portable-pty`'s `CommandBuilder` does not natively support the POSIX argv[0] prefix convention (prepending `-` to the shell name), the v1 mechanism is to pass `--login` as an explicit argument: e.g., `CommandBuilder::new("/bin/bash").args(["--login"])`. Subsequent tabs and panes launch interactive non-login shells (no `--login` flag). This behavior is implemented in `session/spawn.rs`.

**Future (Windows):** `portable-pty`'s `ConPtySystem` provides Windows ConPTY support behind the same API. The `PtySession` implementation switches; no other code changes.

### 7.2 CredentialStore

**Linux v1 implementation:** `secret-service` crate (D-Bus Secret Service API, compatible with GNOME Keyring and KWallet).

If the Secret Service is unavailable (`is_available()` returns `false`), TauTerm prompts for credentials on each connection attempt per FS-CRED-005. Credentials are stored in a `SecVec<u8>` (zeroed on drop) during authentication and cleared immediately after the handshake completes.

**Future (macOS):** `keychain-services` or `security-framework` crate. **Future (Windows):** `windows-credentials` crate.

### 7.3 ClipboardBackend

**Linux v1 implementation:** `arboard` crate handles both X11 and Wayland. For X11 PRIMARY selection (FS-CLIP-004), `arboard`'s `SetExtX11` API or a direct `x11-clipboard` crate integration is used; API availability must be verified at implementation time.

**Future (macOS/Windows):** `arboard` supports both natively.

### 7.4 NotificationBackend

**Trait:**

```rust
pub trait NotificationBackend: Send + Sync {
    fn notify(&self, title: &str, body: &str) -> Result<()>;
}
```

**Linux v1 implementation:** D-Bus `org.freedesktop.Notifications` interface (`notify-rust` crate or direct D-Bus call). If D-Bus is unavailable at startup, `create_notification_backend()` returns a no-op implementation that silently discards notifications (no error returned to callers).

**Usage:** `VtProcessor` triggers `NotificationBackend::notify()` when it receives BEL (0x07) from a pane that is not currently active (FS-VT-090, FS-VT-093). The pane-active check is performed by the `PtyReadTask` before invoking the backend — the `NotificationBackend` itself is stateless.

**Future (macOS):** `NSUserNotification` / `UNUserNotificationCenter`. **Future (Windows):** Win32 toast notifications.

### 7.5 PAL Injection

All four traits are registered in Tauri's managed state at startup in `lib.rs`:

```rust
tauri::Builder::default()
    .manage(platform::create_pty_backend())           // Arc<dyn PtyBackend>
    .manage(platform::create_credential_store())      // Arc<dyn CredentialStore>
    .manage(platform::create_clipboard_backend())     // Arc<dyn ClipboardBackend>
    .manage(platform::create_notification_backend())  // Arc<dyn NotificationBackend>
    .manage(SessionRegistry::new())
    .manage(PreferencesStore::load_or_default())
    .manage(SshManager::new())
    .invoke_handler(tauri::generate_handler![...])
    .run(ctx)
```

**`PreferencesStore::load_or_default()`:** if the preferences file does not exist, loading returns a default instance. If the file exists but is invalid (corrupted JSON, I/O error), the error is logged and a default `PreferencesStore` is returned — TauTerm does not crash on preference corruption. See §7.6.

### 7.6 PreferencesStore Load Strategy

`PreferencesStore::load_or_default()` replaces the original `load().expect(...)` call. The strategy:

1. Attempt to read `~/.config/tauterm/preferences.json` (XDG_CONFIG_HOME).
2. If the file does not exist: return `PreferencesStore::default()`.
3. If the file exists but cannot be read (I/O error) or cannot be parsed (JSON error): log the error at `WARN` level with the filesystem path and error description, then return `PreferencesStore::default()`. The corrupted file is not deleted automatically; the user retains it for inspection.
4. On successful load: validate values against schema ranges (see §8.1). Out-of-range values are replaced with defaults, and a `WARN` log entry is emitted per replaced field.

This strategy satisfies §9.1 (no `unwrap()` on filesystem data) and prevents application startup failure due to preference corruption (FS-SEC-003).

---

## 8. Security Architecture

### 8.1 IPC Boundary Validation

Every `#[tauri::command]` that accepts user-provided data applies validation at entry:

- **Path inputs** (identity file path in `SshConnectionConfig`): resolved to absolute path, checked for path traversal components (`..`), verified to point to a regular file (FS-CRED-006, FS-SEC-003).
- **URI inputs** (hyperlink URIs): scheme whitelisted to `http`, `https`, `mailto`, `ssh`; `file` scheme only for local sessions; length ≤ 2048 bytes; no C0/C1 characters (FS-VT-073).
- **Tab titles** (from OSC sequences via the VtProcessor): C0/C1 stripped, truncated to 256 characters (FS-VT-062).
- **IPC sequence length**: OSC and DCS sequences are limited to 4096 bytes in the VtProcessor (FS-SEC-005).
- **Preferences on load**: validated against a schema; out-of-range values replaced with defaults (FS-SEC-003). See §7.6 for the load strategy.

**`PreferencesStore` structure:** The `Preferences` struct (defined in `preferences/schema.rs`) owns the following top-level keys in `preferences.json`:

| Sub-key | Type | Description |
|---------|------|-------------|
| `appearance` | `AppearancePrefs` | Font, font size, cursor style, theme name, opacity, language |
| `terminal` | `TerminalPrefs` | Scrollback size, `allow_osc52_write`, word delimiters, bell type |
| `keyboard` | `KeyboardPrefs` | Shortcut bindings |
| `connections` | `Vec<SshConnectionConfig>` | Saved SSH connections. **Authoritative source for connection configs** — `SshManager` reads and writes this list via `State<PreferencesStore>`; it holds no independent connection store. |
| `themes` | `Vec<UserTheme>` | User-defined themes |

### 8.2 PTY Isolation

- Master file descriptors opened with `O_CLOEXEC` (FS-SEC-002). The `portable-pty` crate applies this by default; verify at implementation.
- Child processes have no access to other panes' PTY fds, the application's D-Bus connection, or credential memory.
- OSC 52 clipboard read is permanently rejected in the VtProcessor (FS-VT-076). OSC 52 clipboard write policy (FS-VT-075): for local PTY sessions (no saved connection), write is controlled by the global preference `allow_osc52_write: bool` (default: `false`). For saved connections (local or SSH), a per-connection `allow_osc52_write` flag overrides the global default. This prevents a global "enabled" setting from inadvertently enabling OSC 52 write in SSH sessions where it was not explicitly authorized.

### 8.3 SSH Security

- Host key verification is TOFU, stored in `~/.config/tauterm/known_hosts` (OpenSSH-compatible format). TauTerm does **not** read `~/.ssh/known_hosts` automatically at startup or during connection (FS-SSH-011). The Preferences UI offers an explicit "Import from OpenSSH" action that copies entries from `~/.ssh/known_hosts` into TauTerm's own known-hosts file on user request; this is the sole interaction with the OpenSSH file (`ssh/known_hosts.rs`). Once imported, entries are managed independently.
- Deprecated algorithm detection (FS-SSH-014): `ssh-rsa` with SHA-1 and `ssh-dss` trigger a non-blocking in-pane warning.
- SSH agent forwarding is permanently disabled (FS-SEC-004).
- Credentials are never logged, never embedded in IPC payloads, and never cached beyond the authentication handshake (FS-CRED-003, FS-CRED-004).

### 8.4 Content Security Policy

The WebView CSP is configured in `tauri.conf.json` and tightened incrementally:

**v1 minimum (per FS-SEC-001):**
```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
connect-src ipc: http://ipc.localhost;
img-src 'self' asset: http://asset.localhost;
```

`unsafe-inline` for styles is required by Tailwind 4's runtime token injection. `unsafe-eval` and inline scripts are permanently forbidden.

**Future tightening:** As features stabilize, `style-src 'unsafe-inline'` should be replaced with a nonce-based policy if WebKit's support allows. Each capability grant in `capabilities/default.json` is audited when new commands are added.

### 8.5 Terminal Injection Prevention

- Property read-back sequences (CSI 21t, OSC queries that echo into PTY input, DECRQSS responses) are permanently silently discarded in the VtProcessor (FS-VT-063). These are a known injection vector.
- Tab titles set via OSC are sanitized before display (FS-VT-062).
- Multi-line paste confirmation (FS-CLIP-009) prevents accidental command execution from untrusted paste content.

---

## 9. Error Handling Strategy

### 9.1 Rust Backend

- `?` operator for propagation throughout internal code.
- `thiserror` for defining module-specific error types with descriptive variants.
- `anyhow` is permitted in command handlers (where context enrichment is needed) but not in library code (where callers need to match on specific error variants).
- No `unwrap()` or `expect()` on any data that originates from user input, the filesystem, OS calls, or the network. `unwrap()` is permitted only in initialization code where failure is a programming error (e.g., building a regex from a literal pattern). `PreferencesStore::load_or_default()` is the canonical example of this policy: preference file corruption is an expected filesystem condition, not a programming error, and is handled with a logged fallback to defaults (see §7.6).
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

---

## 10. Build Architecture

### 10.1 Pipeline

```
pnpm tauri build
  │
  ├─ Vite build (frontend)
  │    SvelteKit static adapter → build/
  │    Tailwind 4 CSS processing
  │    TypeScript compilation
  │    Tree-shaking, minification (production)
  │
  └─ Cargo build (src-tauri/)
       Rust edition 2024
       Release profile: opt-level = 3, LTO = thin
       Output: src-tauri/target/release/tau-term
       Tauri bundles: AppImage, .deb, .rpm (Linux)
       AppImage: requires "appimage" in bundle.targets (tauri.conf.json) — see §10.6 and ADR-0014
```

### 10.2 Development Mode

```
pnpm tauri dev
  │
  ├─ Vite dev server (localhost:1420) — HMR for frontend changes
  └─ Cargo incremental build — Rust recompile on change, process restart
```

Frontend-only iteration: `pnpm dev` — Vite dev server only, no Tauri backend. IPC calls fail gracefully (mock stubs required for frontend-only development).

### 10.3 Profiles

| Profile | Rust opt-level | LTO | Debug info | Use |
|---------|---------------|-----|-----------|-----|
| debug | 0 | off | full | Development, fast iteration |
| release | 3 | thin | none | Distribution builds |

### 10.4 Testing

See [§14 — Testing Strategy](#14-testing-strategy) for the complete test organization, pyramid rationale, per-layer coverage targets, CI gate definition, and no-regression policy.

### 10.5 Internationalisation (i18n)

**Library:** Paraglide JS (`@inlang/paraglide-sveltekit`) — the idiomatic i18n solution for SvelteKit. It performs compile-time message extraction and generates fully tree-shakeable, zero-runtime-cost string accessor functions. See ADR-0013.

**Locale files:** `src/lib/i18n/messages/en.json` (source, fallback) and `src/lib/i18n/messages/fr.json`. Both are JSON objects mapping message keys to string values. Keys use snake_case and are namespaced by UI area (e.g., `"prefs.language.label"`, `"tab.new"`, `"ssh.state.connecting"`).

**Loading strategy:** Paraglide generates typed accessor functions at build time from the JSON catalogues. The compile step (`pnpm exec paraglide-js compile`) is run automatically via the Vite plugin integration during `pnpm dev` and `pnpm tauri build`. The generated output lives in `src/lib/paraglide/` and must not be hand-edited. At runtime, the active locale is a Svelte 5 reactive value stored in `src/lib/state/locale.svelte.ts`. The locale value is initialised on mount from `preferences.appearance.language` (IPC: `get_preferences`) and defaults to `"en"` if missing or unknown (FS-I18N-006). Locale switching (FS-I18N-004) updates the reactive locale value; all components that consume message accessors re-render automatically via Svelte 5's fine-grained reactivity.

**Frontend string resolution:** Components import message accessor functions from the Paraglide-generated module (e.g., `import * as m from '$lib/paraglide/messages'`) and call them as plain functions (`m.prefs_language_label()`). There is no runtime lookup table and no string interpolation at the framework level — strings are resolved to their target-locale value at the call site.

**Tauri integration:** Locale files are static frontend assets bundled by Vite. No Rust-side i18n is required: all user-visible strings live in the frontend. The backend emits string keys (error codes, status codes) which the frontend maps to locale strings via its own message catalogue. This keeps the IPC contract locale-agnostic. The backend never reads or modifies PTY environment variables (`LANG`, `LC_*`) based on the UI language selection (FS-I18N-007).

**Persistence:** The active locale is saved to `preferences.json` under `appearance.language` via the standard `update_preferences` command. On next launch, `get_preferences` returns the saved locale; the frontend restores it before first render.

**IPC safety — `language` field:** The `language` field on `AppearancePrefs` MUST NOT be a free `String` across the IPC boundary. It MUST be deserialised on the Rust side to an enum validated against the known allowlist:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Fr,
}
```

With `#[serde(default)]`, any unknown locale code in `preferences.json` (e.g., `"de"`) deserialises to `Language::En` instead of propagating an arbitrary string through the IPC layer and into the frontend (FS-I18N-006). The serialised form remains the lowercase string (`"en"` / `"fr"`) for JSON compatibility.

**Module map additions:**

```
src/
  lib/
    i18n/
      messages/
        en.json         — English message catalogue (source, fallback) (FS-I18N-001, FS-I18N-002)
        fr.json         — French message catalogue (FS-I18N-002)
    paraglide/          — Paraglide-generated code (build artefact; not hand-edited)
    state/
      locale.svelte.ts  — reactive locale state; setLocale(lang) writes to preferences;
                          getLocale() returns current locale (FS-I18N-003, FS-I18N-004, FS-I18N-005)
```

### 10.6 Distribution: AppImage

**Artefact:** One AppImage binary per target architecture (FS-DIST-003). Naming convention: `TauTerm-{version}-{arch}.AppImage`.

**Bundler:** Tauri's native AppImage bundler. Configured via `bundle.targets: ["appimage"]` in `tauri.conf.json`. No external toolchain (`appimagetool`, `linuxdeploy`) is required. See ADR-0014.

**Runtime dependency:** WebKitGTK (`libwebkit2gtk-4.1` on Ubuntu 22.04+, `libwebkit2gtk-4.0` on older distributions). This is the only dependency not bundled in the AppImage — it is a standard component of any GNOME-compatible Linux desktop environment. All other dependencies (Rust binary, frontend assets, locale JSON files, application icon, `.desktop` entry) are bundled (FS-DIST-002, FS-DIST-005).

**Multi-architecture build strategy:** Cross-compilation is avoided. Tauri's AppImage bundler requires WebKitGTK headers and libraries matching the target architecture at build time, making cross-compilation impractical without a full matching sysroot. The strategy is **separate CI jobs per architecture**, each running on a native or QEMU-emulated runner (FS-DIST-003):

| Architecture | Rust target triple | CI runner strategy |
|---|---|---|
| x86_64 | `x86_64-unknown-linux-gnu` | Native x86_64 runner |
| x86 (i686) | `i686-unknown-linux-gnu` | Native x86_64 runner with `--target i686` + multilib |
| ARM64 (aarch64) | `aarch64-unknown-linux-gnu` | Native ARM64 runner or QEMU |
| ARM32 (armhf) | `armv7-unknown-linux-gnueabihf` | QEMU or cross-compiler toolchain |
| RISC-V (riscv64) | `riscv64gc-unknown-linux-gnu` | QEMU or cross-compiler toolchain |

Each CI job produces a single AppImage artefact. Release artefacts are published as a set of five files.

**Minimum supported WebKitGTK version:** `libwebkit2gtk-4.1 >= 2.38` (Ubuntu 22.04+) or `libwebkit2gtk-4.0 >= 2.38` (older distributions). Version 2.38 is the threshold that introduced post-2022 WebKit security patches addressing multiple CVE-class vulnerabilities. Distributions shipping an older WebKitGTK release are not officially supported; TauTerm may run but security properties are not guaranteed. The CI build environment enforces this minimum by targeting Ubuntu 22.04 as the baseline.

---

## 11. Frontend Architecture

### 11.1 Module Map

```
src/
  routes/
    +page.svelte          — main view: mounts PaneTree for the active tab
    +layout.svelte        — global keydown handler (gated on !isRecordingShortcut);
                            IPC event subscriptions lifecycle; SSR disabled
    +layout.ts            — export const ssr = false

  lib/
    ipc/
      types.ts            — TypeScript mirrors of all Rust IPC types: SessionState,
                            TabState, PaneNode, PaneState, SshLifecycleState,
                            ScreenUpdateEvent, CellUpdate, CellAttrs, Preferences,
                            UserTheme, SshConnectionConfig, TauTermError, etc.
      commands.ts         — typed invoke() wrappers for all 28 IPC commands
      events.ts           — typed listen() wrappers for all 7 events; return unsubscribe fn
      errors.ts           — TauTermError type + user-facing message helper +
                            error code → display string mapping

    state/
      session.svelte.ts   — SessionState replica; delta merge; getPane(id) helper
      ssh.svelte.ts       — SSH state keyed by PaneId
      notifications.svelte.ts — notification badges per pane/tab; cleared on activation
      preferences.svelte.ts   — Preferences replica; optimistic update
      scroll.svelte.ts    — scroll position per PaneId
      locale.svelte.ts    — reactive locale state; setLocale(lang) writes to preferences;
                            getLocale() returns current locale (FS-I18N-003, FS-I18N-004, FS-I18N-005)

    terminal/
      grid.ts             — ScreenGrid: applyDiff(), applySnapshot(), getAttributeRuns()
      mouse.ts            — mouse event routing: PTY vs TauTerm; xterm encoding
      selection.ts        — selection state machine: drag, word-select, line-select,
                            cell-boundary snapping, word delimiters (FS-CLIP-002, FS-CLIP-003)
      keyboard.ts         — keydown → PTY encoding: C0, Alt prefix, function keys,
                            modified keys (FS-KBD-004 through FS-KBD-012)
      hyperlinks.ts       — OSC 8: cell range → URI tracking, hover detection,
                            URI scheme validation before open_url invoke
      virtualization.ts   — row virtualization: visible viewport computation, DOM recycling
      ansi-palette.ts     — ANSI indices 0-15 → theme CSS tokens; 256-color cube/ramp;
                            truecolor passthrough

    layout/
      split-tree.ts       — SplitNode type; buildFromPaneNode(); updateRatio(); findLeaf()
      resize.ts           — drag resize math; minimum pane constraint; debounce SIGWINCH

    theming/
      apply.ts            — applyTheme(theme): void; setProperty() on :root;
                            cross-fade transition (FS-THEME-006)
      tokens.ts           — UMBRA_DEFAULT_TOKENS: fallback reference + reset to default
      validate.ts         — client-side validation before save: required tokens,
                            CSS.supports(), contrast ratio checks

    preferences/
      contrast.ts         — WCAG relativeLuminance(), contrastRatio(); pure math
      memory-estimate.ts  — lines → bytes → MB string; pure (FS-SB-002)
      shortcuts.ts        — conflict detection; key combo normalization;
                            isRecordingShortcut: boolean (reactive export — see §11.3)

  components/
    terminal/
      TerminalPane.svelte         — primary container; composes all sub-components;
                                   ResizeObserver → resize_pane; keydown capture
      TerminalPane.svelte.ts      — composable: IPC subscriptions, ScreenGrid instance,
                                   selection state, cursor state (see §11.2)
      TerminalViewport.svelte     — scrollable viewport; virtualized rows
      TerminalRow.svelte          — one line: attribute runs → <span> elements
      TerminalCursor.svelte       — 6 DECSCUSR shapes; blink; focused/unfocused outline
      TerminalSelection.svelte    — selection overlay; copy flash; PRIMARY clipboard
      TerminalScrollbar.svelte    — overlay scrollbar with auto-hide (FS-SB-007)
      ScrollToBottom.svelte       — scroll-to-bottom indicator (UXD §8.3)
      SearchOverlay.svelte        — search input, match count, prev/next (FS-SEARCH-007)
      DisconnectBanner.svelte     — SSH Disconnected / Reconnecting states (UXD §7.5.2)
      TerminatedBanner.svelte     — process exited; restart/close actions (FS-PTY-006)
      DeprecatedAlgoBanner.svelte — deprecated SSH algorithm warning (FS-SSH-014)
      ReconnectSeparator.svelte   — reconnection timestamp separator (FS-SSH-042)
      FirstLaunchHint.svelte      — right-click hint, first launch only (FS-UX-002)

    tabs/
      TabBar.svelte
      TabItem.svelte
      TabInlineRename.svelte
      TabActivityIndicator.svelte
      SshBadge.svelte
      TabContextMenu.svelte
      TabScrollArrow.svelte

    layout/
      PaneTree.svelte     — recursive component: leaf → TerminalPane,
                            split → PaneTree + PaneDivider + PaneTree
      PaneDivider.svelte  — 1px visual line, 8px hit area; drag-to-resize;
                            double-click → equal split (UXD §7.2)

    preferences/
      PreferencesPanel.svelte
      sections/
        KeyboardSection.svelte
        AppearanceSection.svelte
        TerminalBehaviorSection.svelte
        ConnectionsSection.svelte
        ThemesSection.svelte
      shared/
        ShortcutRow.svelte
        ShortcutRecorder.svelte     — activates isRecordingShortcut flag (see §11.3)
        ThemeEditor.svelte
        ColorPicker.svelte
        ContrastAdvisory.svelte
        MemoryEstimate.svelte

    connections/
      ConnectionManager.svelte      — slide-in panel; composes List + Form
      ConnectionList.svelte         — grouped list; reusable in Preferences
      ConnectionListItem.svelte     — item: icon, label, hover actions
      ConnectionEditForm.svelte     — create/edit form; client-side path validation

    overlays/
      ConfirmDialog.svelte          — reusable dialog: heading, body, action variant
      HostKeyDialog.svelte          — host key verification (first-connect + key-change)
      ContextMenu.svelte            — base context menu (Bits UI Menu)
```

### 11.2 TerminalPane Component Split

`TerminalPane.svelte` is the most complex component in the application. To prevent it from becoming an unmaintainable monolith, the following governance rule applies:

**If `TerminalPane.svelte` exceeds 250 lines, extract reactive logic to `TerminalPane.svelte.ts`.**

`TerminalPane.svelte.ts` exports composables (functions returning reactive state):
- IPC event subscription management (`$effect` with cleanup)
- `ScreenGrid` instance and diff application
- Selection state machine integration
- Cursor blink timer state

`TerminalPane.svelte` retains only: the component template markup, event handler binding (`on:keydown`, `on:mousedown`, etc.), and calls to the composable functions. This separation keeps the template readable and the logic testable in isolation.

This pattern applies to any component that grows beyond 250 lines. The rule is: **logic goes in the `.svelte.ts` composable file; template and DOM bindings stay in the `.svelte` file.**

### 11.3 Keyboard Shortcut Interception and Recording

The global `keydown` handler in `+layout.svelte` intercepts application shortcuts (FS-KBD-001). `ShortcutRecorder.svelte` (inside the Preferences panel) also needs to capture keyboard input to record new shortcuts — if the global handler fires first, the shortcut recorder cannot receive the keys it needs to record.

**Decision:** `lib/preferences/shortcuts.ts` exports a reactive flag:

```typescript
// shortcuts.ts
export let isRecordingShortcut = $state(false);
```

The global handler in `+layout.svelte` gates all shortcut interception on `!isRecordingShortcut`:

```typescript
// +layout.svelte (keydown handler)
if (isRecordingShortcut) return; // pass all keys through to ShortcutRecorder
// ... normal shortcut dispatch
```

`ShortcutRecorder.svelte` sets `isRecordingShortcut = true` when it enters recording mode (focus or explicit activation) and resets it to `false` on Enter, Escape, or blur. The flag is the single coordination point between the global interceptor and any component that legitimately needs to capture all keyboard input.

**Generalization:** any future component that needs to capture keyboard input unconditionally (e.g., a search input inside the terminal, a modal text field that must receive Escape) follows the same pattern: import and set `isRecordingShortcut` for the duration of its capture window. The name reflects the original use case but the mechanism is general.

---

## 12. Future Extensibility

This section documents the planned extension points for features that are out of scope for v1 but must not require architectural rework.

### 12.1 Session Persistence (Post-v1)

Session persistence requires serializing the `SessionRegistry` state to disk (tab topology, pane types, working directories) and restoring it on startup. The extension point is the `SessionRegistry`: adding `fn serialize_to_disk() -> Result<()>` and `fn restore_from_disk() -> Result<()>`. The `VtProcessor` screen buffer state (current screen content) cannot be fully restored without replaying PTY output, which is not feasible. Restoration will recreate the session structure but not the terminal content.

Architecture readiness: `SessionRegistry`, `TabState`, `PaneNode`, `PaneState`, `SshConnectionConfig` are all `Serialize`/`Deserialize`. No structural change is required.

### 12.2 Plugin / Extension System (Post-v1)

A plugin system would allow third parties to add new session types (e.g., serial port connections), custom tab title formatters, or additional IPC commands. The extension point is the `PtyBackend` trait (ADR-0005): any new session type that can satisfy `PtySession` can be integrated into the `SessionRegistry` without changing the core. Command registration in `lib.rs` would need to support dynamic command registration, which Tauri currently does not support natively; this may require a different plugin approach (e.g., IPC via a local socket to a plugin process). This is noted as an open design problem for the plugin system version.

### 12.3 Cloud Sync (Post-v1 — explicitly out of scope)

Preferences and saved connections are stored in `~/.config/tauterm/` as JSON files (validated on load). A cloud sync feature would add a sync layer above `PreferencesStore`. The `PreferencesStore` interface (`get`, `apply_patch`, `get_themes`, `save_theme`, `delete_theme`) is the abstraction boundary. No structural change is required; a `SyncedPreferencesStore` could wrap the base store.

### 12.4 Kitty Keyboard Protocol (Post-v1)

The Kitty protocol requires changes to the VT parser (new mode flags) and the key encoding logic in the frontend. The `VtProcessor`'s `Perform` implementation is the extension point; new mode flags would be added to the terminal mode state (§5.3). No structural change is required.

### 12.5 Windows / macOS Port (Post-v1)

See ADR-0005. The PAL stubs are in `platform/pty_macos.rs`, `platform/credentials_macos.rs`, `platform/clipboard_macos.rs` and their Windows equivalents. The Tauri framework handles the WebView layer. The SSH library (`russh` or `ssh2-rs`) is already cross-platform. The `portable-pty` crate provides ConPTY on Windows. The primary porting work is implementing the PAL trait implementations for each platform; all other code is platform-agnostic.

---

## 13. ADR Index

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-0001](adr/ADR-0001-tauri-2-as-application-framework.md) | Tauri 2 as application framework | Accepted |
| [ADR-0002](adr/ADR-0002-pty-native-rust.md) | PTY management in native Rust | Accepted |
| [ADR-0003](adr/ADR-0003-vt-parser-library.md) | VT parser: use the `vte` crate | Accepted |
| [ADR-0004](adr/ADR-0004-svelte-5-runes-frontend-state.md) | Svelte 5 runes as frontend state management | Accepted |
| [ADR-0005](adr/ADR-0005-platform-abstraction-layer.md) | Platform Abstraction Layer for OS primitives | Accepted |
| [ADR-0006](adr/ADR-0006-ipc-coarse-grained.md) | Coarse-grained IPC: one command per user action | Accepted |
| [ADR-0007](adr/ADR-0007-ssh-via-rust-ssh-library.md) | SSH implementation via pure-Rust SSH library | Accepted |
| [ADR-0008](adr/ADR-0008-terminal-rendering-strategy.md) | Terminal rendering strategy: DOM-based with row virtualization | Accepted |
| [ADR-0009](adr/ADR-0009-pane-structure-flat-list.md) | Pane layout structure: flat list with split metadata vs. recursive tree | Accepted |
| [ADR-0010](adr/ADR-0010-session-state-delta-events.md) | `session-state-changed` event: complete TabState vs. partial diff | Accepted |
| [ADR-0011](adr/ADR-0011-scrollback-rust-ring-buffer.md) | Scrollback storage: Rust ring buffer in backend | Accepted |
| [ADR-0012](adr/ADR-0012-preferences-json-file.md) | Preferences persistence: JSON file in XDG_CONFIG_HOME | Accepted |
| [ADR-0013](adr/ADR-0013-i18n-paraglide-js.md) | i18n library: Paraglide JS (Inlang) | Accepted |
| [ADR-0014](adr/ADR-0014-appimage-tauri-bundler.md) | AppImage distribution via Tauri bundler | Accepted |

---

## 14. Testing Strategy

### 14.1 Test Pyramid and Layer Rationale

TauTerm's test pyramid is deliberately bottom-heavy. The dominant correctness risk is the VT parser and screen buffer, both governed by externally defined standards (ECMA-48, xterm extensions, VT220 subset). This behavior is fully deterministic and exercisable without a PTY, a window, or any UI.

```
            ┌──────────┐
            │   E2E    │  ~5%   WebdriverIO + tauri-driver
            ├──────────┤
            │ Integrat.│  ~20%  cargo nextest (tests/)
            ├──────────┤
            │  Unit +  │  ~75%  cargo nextest (inline) + Vitest
            │ VT conf. │        + VT corpus (nextest, in-process)
            └──────────┘
```

E2E tests are restricted to scenarios requiring end-to-end system behavior: visual state, real keyboard input, OS clipboard interaction, SSH connection flows. The `vte` crate's `Perform` trait is tested through its implementation (`VtProcessor`) — no mocking.

**Quick reference — runners and commands:**

| Scope | Tool | Command |
|-------|------|---------|
| Rust unit + integration | `cargo nextest` | `cargo nextest run` (from `src-tauri/`) |
| Rust VT conformance | `cargo nextest` | included in `cargo nextest run` |
| Rust SecretService (keyring) | Podman + nextest | `./scripts/run-keyring-tests.sh` |
| Rust formatting | `rustfmt` | `cargo fmt -- --check` |
| Rust linting | `clippy` | `cargo clippy -- -D warnings` |
| Frontend unit | Vitest | `pnpm vitest run` |
| Frontend types | SvelteKit | `pnpm check` |
| Frontend formatting | Prettier | `pnpm prettier --check src/` |
| E2E | WebdriverIO + tauri-driver | `pnpm wdio` (after `pnpm tauri build`) |

---

### 14.2 Rust Unit Tests

#### Module suitability

| Module | Unit testable? | Notes |
|--------|---------------|-------|
| `vt/processor.rs` + `vt/processor/dispatch.rs` | Yes | Pure state transformation; tests in `vt/processor/tests.rs` |
| `vt/screen_buffer.rs` | Yes | Pure grid/scrollback data structure |
| `vt/cell.rs` | Yes | Value types |
| `vt/sgr.rs` | Yes | Pure parsing function |
| `vt/osc.rs` | Yes | Pure dispatch logic |
| `vt/modes.rs` | Yes | Flag state |
| `vt/mouse.rs` | Yes | Encoding logic is pure |
| `vt/search.rs` | Yes | Operates on ScreenBuffer snapshot |
| `vt/charset.rs` | Yes | DEC mapping tables |
| `session/lifecycle.rs` | Yes | State machine transitions |
| `session/ids.rs` | Yes | Newtype construction |
| `ssh/known_hosts.rs` | Yes | File parsing — operates on `&str` |
| `ssh/algorithms.rs` | Yes | String classification — pure |
| `preferences/schema.rs` | Yes | Serde round-trip |
| `preferences/store.rs` | Partial | Requires temp dir fixture |
| `session/registry.rs` | No — integration | Requires `SessionRegistry` + `PaneSession` |
| `session/pty_task.rs` | No — integration | Requires PTY pair or pipe |
| `platform/` impls | No — integration | OS resources |
| `ssh/manager.rs`, `ssh/connection.rs` | No — integration | Network or mock SSH |

#### `vt/processor.rs` and `vt/screen_buffer.rs`

Feed escape sequences to `VtProcessor::process()`; assert on `ScreenBuffer` state. Key areas:
- Cursor position after movement sequences (CUP, CUF, CUB, CUU, CUD, CR, LF, HT)
- Cell content and attributes after SGR (fg/bg/bold/italic/underline/blink/inverse/hidden/strikethrough)
- Screen buffer structure: row/col count, cursor, scroll region bounds, dirty cell tracking
- Alternate screen switch (DECSET 1049): cursor preserved, alternate clear, restored on DECRST 1049
- Scrollback ring: line eviction at capacity, soft-wrap metadata (FS-SB-008)
- Resize: reflow/truncation behavior

#### `vt/sgr.rs`, `vt/osc.rs`, `vt/modes.rs`

- **`sgr.rs`**: every SGR code → expected `CellAttrs` delta. Colon sub-params (ITU T.416, extended underline 4:1–4:5, underline color 58:2:R:G:B). Invalid/unknown codes → no change.
- **`osc.rs`**: OSC 0/1/2 title; OSC 22/23 title stack; OSC 8 hyperlink start/end with URI validation; OSC 52 disabled/enabled per policy; OSC 52 read always rejected; C0 stripped; title truncated at 256 chars.
- **`modes.rs`**: each mode via DECSET/DECRST; save/restore on alternate screen switch.

#### `session/lifecycle.rs`

All valid transitions: Spawning→Running, Running→Terminated, Running→Closing, Closing→Closed, Terminated→Spawning, Terminated→Closed. Invalid transitions return `Err`. No PTY allocated — pure enum transition function.

#### `ssh/known_hosts.rs`

Parser corpus: single-line entries, hashed hostnames, comment/blank lines, `@revoked`/`@cert-authority` (parse without crash, not trusted), malformed lines (no panic). Lookup: match by hostname (exact/hashed). Add/update: round-trip. Import from `~/.ssh/known_hosts`: fixture with mixed entry types.

#### `ssh/algorithms.rs`

Deprecated: `ssh-rsa` (SHA-1), `ssh-dss`. Not deprecated: `ssh-ed25519`, `ecdsa-sha2-nistp256`, `rsa-sha2-256`, `rsa-sha2-512`.

#### Inline vs. separate file rule

- **Inline `#[cfg(test)]` modules**: default for all unit tests.
- **Separate `tests.rs` file**: when the test module exceeds ~150 lines OR the source file exceeds ~400 lines. Declared as `#[cfg(test)] mod tests;` in the source; file at `<module>/tests.rs`. No `mod.rs` (§3.1 convention).
- `vt/processor.rs` and `vt/screen_buffer.rs`: separate test files from the start, given the surface area.

---

### 14.3 Rust Integration Tests

Location: `src-tauri/tests/`. One file per domain.

#### `vt` + `session` pipeline (PTY pipe)

A pipe pair replaces a real PTY for most integration tests. Scenarios:
- Write ANSI escape bytes to write end → assert screen buffer state
- Large block (> 4096 bytes) → no truncation, no partial-sequence artifacts
- Resize mid-stream → grid dimensions update
- Dirty region coalescing: two sequences in rapid succession → single `DirtyRegion`

One real PTY test: spawn `/bin/sh`, write `echo hello\n`, verify `hello` in screen buffer. Tagged `#[cfg(target_os = "linux")]` and `slow` in nextest config.

#### IPC cycle integration

Uses `tauri::test::mock_app`. Scenarios:
- `create_tab` → `TabState` with leaf `PaneNode`, valid `PaneId`
- `split_pane` → `TabState.layout` is `Split` node with two `Leaf` children, distinct `PaneId`s
- `close_pane` (non-last pane) → sibling pane expanded in returned `TabState`
- `close_pane` (last pane) → `null` returned
- `rename_tab` → label updated; subsequent `get_session_state` reflects rename
- `update_preferences` with invalid value → `TauTermError { code: "PREF_INVALID_VALUE" }`

#### SecretService integration tests (Podman container)

`src-tauri/tests/credentials_integration.rs` — exercises `LinuxCredentialStore` against a real GNOME Keyring daemon (SEC-CRED-INT-001 to 005). These tests cannot run in a standard CI environment because they require a live D-Bus Secret Service daemon with an unlocked default collection. They are therefore isolated in a dedicated Podman image.

**Why a custom image:** The standard Rust CI base image (slim-bookworm) has no D-Bus session bus, no GNOME Keyring, and no display server. Creating a real Secret Service session requires:
1. A D-Bus session bus (`dbus-run-session`)
2. GNOME Keyring daemon (`gnome-keyring-daemon --unlock --components=secrets`)
3. A virtual framebuffer (`Xvfb :99`) — gnome-keyring 42 activates `gcr-prompter` via D-Bus to create the initial "login" collection; `gcr-prompter` is a GTK application that requires a display even when only the virtual display is in use
4. `xdotool` to auto-dismiss the password dialog (empty password = no encryption, acceptable for ephemeral CI keyrings)

**Critical ordering constraint:** `Xvfb` and `DISPLAY` must be set *before* `dbus-run-session` is invoked. D-Bus-activated services (`gcr-prompter`) inherit the environment of `dbus-daemon`, not the calling shell. Setting `DISPLAY` after `dbus-run-session` has started means `gcr-prompter` never sees it and crashes with `cannot open display`.

**Image:** `Containerfile.keyring-test` (project root) — single-stage `rust:1.86-slim-bookworm`. Full Tauri Linux build dependencies are required because `tau_term_lib` (which the test binary links against) depends on `gtk`, `gio`, `webkit2gtk`, etc. at compile time. The test binary is pre-compiled during `docker build` to keep `docker run` fast.

**nextest profile:** `keyring` (defined in `src-tauri/.config/nextest.toml`) — `test-threads = 1` (tests share a single daemon; parallelism causes race conditions), `slow-timeout = 60s`, `fail-fast = false`.

**Running:**
```bash
./scripts/run-keyring-tests.sh             # build image + run
./scripts/run-keyring-tests.sh --no-build  # reuse existing image
./scripts/run-keyring-tests.sh --dry-run   # print commands only
```

These tests are **not** part of the default `cargo nextest run` gate. They are an optional step, run on-demand or in a dedicated CI job.

#### Isolation rules

- Temp directories via `tempfile::TempDir` for all filesystem-touching tests
- `SessionRegistry::new()` and `PreferencesStore::load(path)` receive injected paths — no hardcoded `~/.config/tauterm/`
- No port binding in integration tests
- nextest process isolation by default; shared mutable state within a binary → test-scoped `Mutex`
- SecretService integration tests run single-threaded in the `keyring` nextest profile; each test uses a unique attribute key (`tauterm:integration-test:<name>`) and a RAII `Cleanup` guard that deletes the key in `Drop`, preventing keyring pollution across test runs

---

### 14.4 VT Conformance Tests

Location: `src-tauri/tests/vt_conformance.rs`. Data-driven test runner over a `VtTestCase` array.

```rust
struct VtTestCase {
    name: &'static str,           // e.g. "FS-VT-022-truecolor-colon"
    input: &'static [u8],
    setup: Option<&'static [u8]>, // preamble bytes applied before input
    cols: u16,
    rows: u16,
    expected: ExpectedState,
}

struct ExpectedState {
    cells: &'static [(u16, u16, ExpectedCell)], // sparse cell assertions
    cursor: Option<(u16, u16)>,
    modes: &'static [ExpectedMode],
    scrollback_lines: Option<usize>,
}
```

Sequences are inline Rust byte literals — no external fixture files for short sequences. Binary file pairing (`name.bin` + `name.snap`, matched by name convention) is reserved for large binary captures that would be unreadable inline.

**Required FS-VT coverage:**

| FS | Test |
|----|------|
| FS-VT-010 | UTF-8 split across `process()` calls |
| FS-VT-011 | CJK wide char at last column wraps |
| FS-VT-012 | Combining character — no cursor advance |
| FS-VT-013 | ZWJ sequence — 2 cells |
| FS-VT-016 | Overlong UTF-8 → U+FFFD |
| FS-VT-020 | All 16 standard colors |
| FS-VT-021 | 256-color spot-checks |
| FS-VT-022 | Truecolor semicolon and colon variants |
| FS-VT-024 | All 9 SGR attributes; SGR 0 resets all |
| FS-VT-025 | Extended underline 4:1–4:5; underline color 58:2 |
| FS-VT-030 | DECSCUSR 0–6 |
| FS-VT-031 | DECTCEM show/hide |
| FS-VT-033 | DECSC/DECRC per screen buffer |
| FS-VT-040–044 | Alternate screen (1049, 47, 1047) |
| FS-VT-050–053 | Scroll region (DECSTBM), partial scroll |
| FS-VT-060–063 | OSC title, truncation, no read-back injection |
| FS-VT-073 | URI scheme rejection, length limit |
| FS-VT-075–076 | OSC 52 policy matrix; read permanently rejected |
| FS-VT-080–086 | Mouse encoding, mode reset on session close |
| FS-VT-090–092 | BEL notification, rate limit 100 ms |

**External vttest/esctest:** excluded from v1 merge gate. Recommended as a nightly CI job post-v1.

---

### 14.5 Frontend Unit Tests (Vitest)

Pure TypeScript, no Svelte components, no DOM.

| Module | What to test |
|--------|-------------|
| `lib/terminal/grid.ts` | `applyDiff()`, `applySnapshot()`, `getAttributeRuns()` |
| `lib/terminal/selection.ts` | Selection state machine transitions |
| `lib/terminal/keyboard.ts` | Keydown → byte encoding |
| `lib/terminal/mouse.ts` | Mouse event routing decision (PTY vs TauTerm UI); X10/SGR-1006/URXVT-1015 encoding given button, modifiers, row, col; mode-to-encoding arbitration |
| `lib/terminal/hyperlinks.ts` | URI scheme validation |
| `lib/terminal/ansi-palette.ts` | Color index → CSS token mapping |
| `lib/theming/validate.ts` | Token presence, contrast ratio enforcement |
| `lib/theming/tokens.ts` | Default token set completeness |
| `lib/preferences/contrast.ts` | `relativeLuminance()`, `contrastRatio()` |
| `lib/preferences/memory-estimate.ts` | Lines → MB formula |
| `lib/preferences/shortcuts.ts` | Conflict detection, key combo normalization |
| `lib/layout/split-tree.ts` | `buildFromPaneNode()`, `updateRatio()`, `findLeaf()` |
| `lib/state/session.svelte.ts` | Delta merge, `getPane()` traversal |
| `lib/state/locale.svelte.ts` | `setLocale()` writes to preferences via IPC; `getLocale()` returns current locale; unknown locale code from backend defaults to `"en"` (FS-I18N-006) |
| `lib/ipc/commands.ts` | Correct command name and parameter shape passed to `invoke()`; `TauTermError` propagated as thrown value; each wrapper calls the right command string |

#### `lib/terminal/grid.ts` detail

- `applySnapshot()`: every cell char and attribute matches input
- `applyDiff()`: only specified cells changed; unchanged cells retain prior values
- `getAttributeRuns()`: alternating attribute groups → correct run count with no unnecessary splits; adjacent identical attributes merged
- Edge cases: wide chars (width 2 + placeholder), combining chars (no cursor advance), empty rows (non-null run list)

#### `lib/terminal/mouse.ts` detail

- **Routing:** mouse event over terminal content with PTY mouse mode active → routes to PTY encoder; event over terminal content with PTY mode off → routes to TauTerm selection handler; event over chrome (tab bar, scrollbar) → always routes to TauTerm UI regardless of PTY mode
- **X10 encoding:** button 0/1/2, no modifier → correct `\033[M<b><x><y>` bytes; coordinates clamped to 0–223; button > 2 (wheel) → not encoded in X10 mode
- **SGR 1006 encoding:** button, modifiers (Shift/Alt/Ctrl), press vs release → `\033[<b;x;yM` vs `\033[<b;x;ym`; coordinates unbounded; wheel buttons (button 64/65) encoded correctly
- **URXVT 1015 encoding:** button + modifier bitmask → `\033[<b;x;yM`; release always button 3
- **Mode arbitration:** SGR preferred over URXVT when both modes set; X10 takes precedence over nothing

#### `lib/ipc/commands.ts` detail

Mock setup: `vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))`. For each wrapper:
- The `invoke` spy is called with the exact expected command string (e.g., `'create_tab'`, `'close_pane'`)
- Input parameters are forwarded verbatim to `invoke` — no silent dropping or renaming
- When `invoke` rejects with a `TauTermError`-shaped object, the wrapper re-throws it (not swallows it)
- Return value: the wrapper resolves to the value that `invoke` resolves to — no transformation
- Wrappers that return `TabState | null` (`close_pane`) correctly type `null` when `invoke` resolves to `null`

#### `lib/state/session.svelte.ts` detail

- Delta merge: one tab updated → other tabs unchanged in replica
- `getPane()` traversal: tree depth 3, each leaf reachable; non-existent ID → `undefined`
- Tab-closed change type: tab removed from replica; `activeTabId` updated if closed tab was active

#### IPC mock

```typescript
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
```

`listen()` mocked similarly — tests call the registered listener callback directly.

---

### 14.6 Svelte Component Tests (Vitest + @testing-library/svelte)

#### Selection policy

A component warrants a dedicated test only when **all three** conditions hold:
1. Non-trivial internal state logic that cannot be extracted to a testable `lib/` module
2. Rendering output is correctness-critical
3. Component is reused across multiple contexts

Pure rendering components (`MemoryEstimate`, `ContrastAdvisory`, `SshBadge`): no component test — logic covered in `lib/` unit tests.

**`TerminalCursor.svelte` — evaluation against the three criteria:**

1. **Non-trivial internal state impossible to extract to `lib/`?** Partially. The blink timer (`setInterval` managed in `onMount`/`onDestroy`) is lifecycle-bound and cannot be a pure function. However, the remaining logic — mapping `cursorStyle` prop to a CSS class, toggling visibility from `cursorVisible` prop, toggling a focused style from the `isFocused` prop — is prop-driven rendering with no internal state worth testing.
2. **Rendering output correctness-critical?** Yes. An incorrect cursor shape or missing blink would be immediately visible. However, the correctness of which shape to render is determined by the backend VT state machine (tested in §14.4, FS-VT-030/031) and propagated as a prop. The component only maps a string prop to a CSS class.
3. **Reused across multiple contexts?** No. Instantiated once per pane, inside `TerminalPane.svelte` only.

**Verdict: criterion 3 not met → no component test file.** The one non-extractable behavior (blink timer lifecycle) is too thin to warrant a full component test setup. Backend correctness for DECSCUSR shapes is covered by VT conformance tests (§14.4). `TerminalCursor.svelte.test.ts` is not created.

#### `ShortcutRecorder.svelte`

- Focus → `isRecordingShortcut = true`
- Keydown while recording → displayed combination updates
- Enter → `on:record` fires with correct combo; `isRecordingShortcut = false`
- Escape → recording cancelled; `isRecordingShortcut = false`
- Blur → `isRecordingShortcut = false`
- Single modifier key alone → not accepted; recorder stays in recording state

#### `ThemeEditor.svelte`

- Render with default tokens → each input shows correct initial value
- Valid color change → `on:change` fires with updated `UserTheme`
- Invalid color (`#gg0000`) → inline error; `on:change` not fired
- Contrast below WCAG AA → `ContrastAdvisory` appears; above threshold → disappears

#### `TerminalRow.svelte`

- 3 attribute runs → exactly 3 `<span>` elements with correct content
- Wide character (2-cell) → not split across spans
- Empty row → single non-breaking space element (row height preserved)
- Row with search match → matched range has highlight token applied

---

### 14.7 E2E Tests (WebdriverIO + tauri-driver)

Require full release build (`pnpm tauri build`). Run on merge to `dev`; gate for promotion `dev` → `main`.

#### Mandatory scenarios (v1)

| Scenario | FS |
|----------|----|
| Initial tab with running shell | FS-PTY-001, FS-TAB-002 |
| New tab (keyboard + button); two independent sessions | FS-TAB-001, FS-TAB-003 |
| Close tab (no process) — no dialog; close last tab (window closes) | FS-TAB-004, FS-TAB-008 |
| Close tab with running process → confirmation dialog | FS-PTY-008 |
| Split pane horizontal; type in each pane; verify independence | FS-PANE-001, FS-PANE-002 |
| Navigate panes with keyboard | FS-PANE-005 |
| Resize pane by drag; verify SIGWINCH (`stty size`) | FS-PANE-003, FS-PTY-009 |
| Select text; paste Ctrl+Shift+V (clipboard round-trip) | FS-CLIP-004, FS-CLIP-005 |
| OSC title via printf → tab bar displays title | FS-VT-060 |
| Double-click tab → inline rename → Enter → label updated | FS-TAB-006 |
| Ctrl+Shift+F → search overlay → match highlighted | FS-SEARCH-006, FS-SEARCH-007 |
| Open preferences; change font size; terminal redraws | FS-PREF-003 |
| Switch theme → tokens applied immediately | FS-THEME-006 |
| Shell exits → terminated banner with exit code; restart | FS-PTY-005, FS-PTY-006 |

**SSH E2E** (local mock SSH server in `BeforeAll` hook):

| Scenario | FS |
|----------|----|
| First connect → host key prompt with SHA-256 fingerprint | FS-SSH-011 |
| Accept key → Connected; type command; verify output | FS-SSH-010, FS-SSH-012 |
| Changed host key → key-change warning; default action = Reject | FS-SSH-011 |
| Network drop → Disconnected; click Reconnect | FS-SSH-022, FS-SSH-040 |

#### Explicit limitations

Not testable in E2E, excluded from the suite:
- Pixel-level rendering accuracy (visual regression tooling — out of scope v1)
- IME composition (OS-level events not reproducible)
- X11 PRIMARY selection / middle-click paste (manual testing on X11)
- System audio bell
- OS keychain integration in CI (mock `CredentialStore` in integration tests; manual acceptance testing with real SecretService)

---

### 14.8 Security Testing

#### IPC boundary validation

Each `#[tauri::command]` handler has a dedicated test module exercising the validation layer without a running Tauri instance.

| Input class | Vectors |
|-------------|---------|
| String fields (tab label, theme name) | Empty, 10 000 chars, C0/C1 chars, embedded NUL |
| Numeric fields | `i64::MIN`, `i64::MAX`, 0, −1 on unsigned-expected, out-of-range |
| `PaneId`/`TabId`/`ConnectionId` | Valid UUID with no live session → `INVALID_PANE_ID`, no panic |
| Identity file path | `../../etc/passwd`, path with `..`, directory, symlink chain, > `PATH_MAX`, embedded NUL |
| URL/URI fields (`open_url`) | Non-whitelisted scheme (`file://`, `javascript:`, `data:`), 4096-byte URI, URI with C0 |

Acceptance: no panic, no crash, well-formed `TauTermError` code. HTML injection via `rename_tab` and OSC 0/1/2 title: stored as raw string, rendered as text content, never interpreted as markup.

#### PTY isolation

Unit tests on `VtProcessor` for each hostile sequence:

| Sequence | Expected |
|----------|---------|
| OSC 52 read (`\033]52;c;?\007`) | Discarded; no PTY write |
| OSC 52 write, policy=Disabled | Discarded; clipboard not written |
| OSC 52 write, policy=Allow | Clipboard backend `write()` called |
| OSC 8 non-whitelisted scheme | URI not stored; no hyperlink |
| OSC 8 URI > 2048 bytes | Discarded |
| DSR/CPR read-back (`\033[5n`, `\033[6n`) | Discarded; no response |
| DECRQSS | Discarded |
| OSC payload > 4096 bytes | Discarded; subsequent sequences processed normally |

BEL rate-limit saturation: 1000 BEL chars → ≤ N notifications per second; no blocking or panic.

`O_CLOEXEC` hygiene: enumerate `/proc/self/fd` and `/proc/<child>/fd`; assert no other pane's PTY master fd appears in the child's fd table. `#[cfg(target_os = "linux")]`.

#### SSH security

Host key TOFU integration tests:
- First connect → `host-key-prompt` event before channel open; fingerprint correct; `reject_host_key` → no known-hosts entry written
- Known-good host → second connection, no prompt, connects directly
- Changed key → event with `is_changed: true`, old + new fingerprints; `accept_host_key` updates file; `reject_host_key` leaves file unchanged

Credentials in memory and logs:
- `credential-prompt` payload contains no `password`/`passphrase` field
- `RUST_LOG=trace` during auth → no log line contains the test credential string (custom tracing subscriber)
- `SecVec<u8>` zeroize: fill with known pattern, drop, assert memory cleared
- Preferences file: no plaintext credential, only keychain lookup key

Deprecated algorithm (FS-SSH-014): mock server negotiates `ssh-rsa` → `ssh-algorithm-warning` event; connection functional; `dismiss_ssh_algorithm_warning` suppresses event for session.

Agent forwarding disabled (FS-SEC-004): no `auth-agent-req@openssh.com` channel request, regardless of `SSH_AUTH_SOCK`.

#### OSC 52 policy matrix

| Policy | Write | Read | Expected |
|--------|-------|------|---------|
| Disabled (default) | Any payload | — | Discarded; clipboard not written |
| Allow | Valid base64 | — | Clipboard `write()` called |
| Allow | Malformed base64 | — | Discarded; no panic |
| Disabled or Allow | — | Read request | Always discarded |

Cross-connection isolation: two panes with different policies → policy state is per-`VtProcessor`, not global.

#### CSP (E2E)

- CSP header/meta present with required directives (`default-src 'self'`, `script-src 'self'`, no `unsafe-eval`, no `unsafe-inline` for scripts)
- Injected `<script>` element does not execute
- `eval("1+1")` throws a CSP error
- `fetch()` to a non-whitelisted origin is blocked

#### Fuzzing

Two `cargo-fuzz` targets in `src-tauri/fuzz/fuzz_targets/` (cargo-fuzz convention: adjacent to the target crate):

- `fuzz_vt_processor.rs`: 80×24 processor, arbitrary bytes → no panic, no unbounded allocation
- `fuzz_osc_dispatch.rs`: OSC sequence parsing in isolation with mock backends
- `fuzz_ipc_commands.rs`: arbitrary JSON → serde never panics; downstream validation produces no secondary panic

Fuzzing is not in the `nextest run` gate. Runs:
- Manually before declaring VT feature complete (`-max_total_time=300`)
- Weekly CI scheduled job (10 min/target)
- Any crash → minimized reproducer → deterministic nextest regression test before fix

Seed corpora committed to `src-tauri/fuzz/corpus/`.

#### Security regression rule

When a vulnerability is identified: write a reproducer test first, fix, verify test passes. Tag: `// Security regression: <issue-id> — <description>`. Lives in the same module as the vulnerable code. Mandatory — unfixed without a regression test is not mergeable.

#### Out of scope for automated testing

- Every `unsafe` block: requires a review comment documenting safety invariants; security-expert sign-off
- `platform/credentials_linux.rs` and `ssh/auth.rs`: security-expert sign-off on non-trivial changes
- `capabilities/default.json` changes: least-privilege review required

**Dependency auditing:**

| Tool | Frequency |
|------|-----------|
| `cargo audit` | On every dependency change; weekly in CI; before every release |
| `pnpm audit` | On every dependency change; weekly in CI |

HIGH/CRITICAL finding = release blocker. MODERATE = documented decision required.

Pre-v1.0: manual penetration assessment of the PTY injection surface and SSH TOFU workflow.

---

### 14.9 Test File Organization

**Governing rule:** tests live as close as possible to the code they test, except when they cross module boundaries.

```
src-tauri/src/
  vt/
    processor.rs              — declares: #[cfg(test)] mod tests;
    processor/
      tests.rs                — unit tests for VtProcessor (separate: large surface)
    screen_buffer.rs          — declares: #[cfg(test)] mod tests;
    screen_buffer/
      tests.rs                — unit tests for ScreenBuffer (separate: large surface)
    cell.rs                   — inline tests
    sgr.rs                    — inline tests
    osc.rs                    — inline tests
    modes.rs                  — inline tests
    mouse.rs                  — inline tests
    search.rs                 — inline tests
    charset.rs                — inline tests
  session/
    lifecycle.rs              — inline tests
    ids.rs                    — inline tests
    registry.rs               — inline tests (separate file if grows large)
    pane.rs                   — inline tests
    ...

src-tauri/tests/
  common/
    mod.rs                    — re-exported by each integration test via `mod common;`
    pty_harness.rs            — test PtyBackend (in-memory pipe)
    vt_harness.rs             — feed_str(), feed_bytes(), snapshot_as_text()
    fixtures.rs               — fixture path resolution (CARGO_MANIFEST_DIR)
  vt_integration.rs           — VtProcessor + ScreenBuffer full pipeline
  vt_conformance.rs           — VT conformance corpus runner
  session_integration.rs      — SessionRegistry lifecycle
  ssh_integration.rs          — SSH state machine (mocked transport)
  preferences_integration.rs  — load/save/patch round-trips
  ipc_commands.rs             — command handler validation
  fixtures/
    vt/
      sequences/              — name.bin (raw bytes), paired by name with snapshots
      snapshots/              — name.snap (UTF-8 grid dump)
    prefs/
      valid_prefs.json
      invalid_prefs.json

src-tauri/fuzz/               — cargo-fuzz crate (adjacent to src-tauri/)
  fuzz_targets/
    fuzz_vt_processor.rs
    fuzz_osc_dispatch.rs
    fuzz_ipc_commands.rs
  corpus/
    fuzz_vt_processor/
    fuzz_osc_dispatch/
    fuzz_ipc_commands/

src/lib/
  terminal/
    grid.ts
    grid.test.ts
    selection.ts
    selection.test.ts
    keyboard.ts
    keyboard.test.ts
    mouse.ts
    mouse.test.ts
  state/
    session.svelte.ts
    session.svelte.test.ts
    locale.svelte.ts
    locale.svelte.test.ts
  ipc/
    commands.ts
    commands.test.ts

src/components/
  terminal/
    TerminalRow.svelte
    TerminalRow.svelte.test.ts
    TerminalCursor.svelte              — no test file (see §14.6 evaluation)
  tabs/
    TabItem.svelte
    TabItem.svelte.test.ts
  preferences/
    shared/
      ShortcutRecorder.svelte
      ShortcutRecorder.svelte.test.ts
      ThemeEditor.svelte
      ThemeEditor.svelte.test.ts

tests/
  e2e/
    fixtures/
      ssh-server/             — sshd config + test keys
      prefs/
        default.json
    helpers/
      app.ts                  — browser setup/teardown
      pane.ts                 — PaneObject
      tab.ts                  — TabObject
      session.ts              — waitForPrompt(), waitForOutput()
    page-objects/
      TerminalPage.ts
      PreferencesPage.ts
      ConnectionManagerPage.ts
    specs/
      terminal/
        pty-lifecycle.e2e.ts
        split-pane.e2e.ts
        keyboard-input.e2e.ts
        scrollback.e2e.ts
        selection-copy.e2e.ts
      ssh/
        connect-disconnect.e2e.ts
        reconnect.e2e.ts
        host-key-dialog.e2e.ts
      preferences/
        theme-switch.e2e.ts
        shortcut-recording.e2e.ts
      first-launch/
        first-launch-hint.e2e.ts
```

#### File naming conventions

| Test type | Convention |
|-----------|-----------|
| Rust unit inline | `#[cfg(test)] mod tests { }` in source file |
| Rust unit separate | `<module>/tests.rs`, declared with `#[cfg(test)] mod tests;` |
| Rust integration | `src-tauri/tests/<domain>_integration.rs` |
| Rust VT conformance | `src-tauri/tests/vt_conformance.rs` |
| Rust shared helpers | `src-tauri/tests/common/` |
| VT fixtures (binary) | `src-tauri/tests/fixtures/vt/sequences/<name>.bin` |
| VT fixtures (snapshot) | `src-tauri/tests/fixtures/vt/snapshots/<name>.snap` |
| Frontend unit (TS) | `<module>.test.ts` co-located |
| Frontend component | `<Component>.svelte.test.ts` co-located |
| E2E specs | `tests/e2e/specs/<feature>/<scenario>.e2e.ts` |
| E2E helpers/page objects | `tests/e2e/helpers/`, `tests/e2e/page-objects/` |

Project-wide suffix convention: `.test.ts` for TypeScript, `.svelte.test.ts` for Svelte components. No `.spec.ts` — one suffix throughout.

Fixture path resolution in Rust: use `std::env::var("CARGO_MANIFEST_DIR")` — nextest sets this correctly regardless of working directory.

---

### 14.10 Coverage Policy

| Layer | Tool | Target |
|-------|------|--------|
| Rust `vt/` | `cargo llvm-cov` | 90% line, 80% branch |
| Rust `session/` | `cargo llvm-cov` | 80% line |
| Rust `ssh/` | `cargo llvm-cov` | 75% line |
| Rust `preferences/` | `cargo llvm-cov` | 85% line |
| Rust `commands/` | `cargo llvm-cov` | 70% line |
| Frontend `lib/terminal/` | Vitest (v8) | 85% line, 75% branch |
| Frontend `lib/state/` | Vitest (v8) | 80% line |
| Frontend `lib/theming/` | Vitest (v8) | 80% line |
| Frontend `lib/preferences/` | Vitest (v8) | 90% line |
| Svelte components | Vitest (v8) | 60% line (selective policy §14.6) |

**Explicitly excluded from automated coverage:**
- `platform/pty_linux.rs`: covered by the single real-PTY integration test
- `platform/credentials_linux.rs`: requires a running keychain; manual acceptance testing
- `platform/clipboard_linux.rs`: covered by E2E on a real display
- All `*_macos.rs` and `*_windows.rs` stubs (`unimplemented!()`)
- Visual rendering pixel accuracy
- Audio bell

---

### 14.11 No-Regression Policy

#### Bug regression tests

Before any bug fix is committed:
1. Write a test at the lowest applicable level that reproduces the failing behavior (must fail on unfixed code)
2. Fix the bug
3. Verify test passes
4. Tag: `// Regression test for issue #NN: <description>`

No exception. A fix without a regression test is not mergeable.

#### Flaky test policy

A flaky test is a defect. Never skip, ignore, or permanently add retries. Root cause classification:
- **(a) Timing dependency** → replace sleep-based assertions with deterministic signals
- **(b) Shared mutable state** → isolate per §14.3
- **(c) Non-deterministic production code** → fix production code
- **(d) Legitimate environment dependency** → `#[cfg(target_os = "linux")]` with issue reference

A one-retry budget in nextest config is allowed temporarily while investigation is ongoing, tracked with an issue reference and a resolution deadline.

#### CI gates (every PR before merge)

1. `cargo clippy -- -D warnings`
2. `cargo fmt -- --check`
3. `cargo nextest run` (all Rust tests, including VT conformance)
4. `pnpm check` (TypeScript type check)
5. `pnpm prettier --check src/`
6. `pnpm vitest run`

E2E (`pnpm wdio`): runs on merge to `dev`, not on every PR. Gate for promotion `dev` → `main`.

---

*This document is maintained by the TauTerm software architect. Every structural change to the backend module layout, the frontend module layout, the IPC command surface, the pane topology model, or the platform abstraction layer requires updating this document and, where appropriate, adding a new ADR.*
