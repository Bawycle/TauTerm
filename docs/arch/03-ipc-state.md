<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — IPC Contract and State Machines

> Part of the [Architecture](README.md).

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
| `set_active_tab` | `{ tab_id: TabId }` | `()` | Switch to tab |
| `set_active_pane` | `{ pane_id: PaneId }` | `()` | Change focus to pane |
| `has_foreground_process` | `{ pane_id: PaneId }` | `bool` | Returns `true` if the PTY foreground process group differs from the shell's PID — i.e., a child process (e.g. `vim`, `ssh`) is currently running. Used by the frontend to prompt "a process is running" before closing a pane or tab. Implemented via `tcgetpgrp` on the PTY master fd. Returns `false` for SSH panes (shell PID unknown). |
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
| `duplicate_connection` | `{ connection_id: ConnectionId }` | `SshConnectionConfig` | Duplicate a saved SSH connection; returns the full config of the new copy (label suffixed with " (copy)", new `ConnectionId` assigned). Returning the full config avoids a round-trip `get_connections` call. |
| `get_preferences` | — | `Preferences` | Read current preferences |
| `update_preferences` | `PreferencesPatch` | `Preferences` | Write preferences (immediate apply) |
| `get_themes` | — | `Vec<UserTheme>` | List all user themes |
| `save_theme` | `UserTheme` | `()` | Create or update a theme |
| `delete_theme` | `{ name: String }` | `()` | Delete a user theme |
| `provide_credentials` | `{ pane_id: PaneId, credentials: Credentials }` | `()` | Respond to SSH credential prompt |
| `accept_host_key` | `{ pane_id: PaneId }` | `()` | Accept new/changed host key |
| `reject_host_key` | `{ pane_id: PaneId }` | `()` | Reject host key (abort connection) |
| `dismiss_ssh_algorithm_warning` | `{ pane_id: PaneId }` | `()` | Dismiss deprecated-algorithm banner |
| `provide_passphrase` | `{ pane_id: PaneId, passphrase: String, save_in_keychain: bool }` | `()` | Respond to `passphrase-prompt` event — forward passphrase for encrypted SSH private key (FS-SSH-019a). The passphrase is forwarded via a oneshot channel stored in `SshManager::pending_passphrases`. |
| `set_pane_label` | `{ pane_id: PaneId, label: Option<String> }` | `TabState` | Set or clear the user-defined label for a pane. Returns the updated `TabState` for the containing tab. |
| `store_connection_password` | `{ connection_id: ConnectionId, username: String, password: String }` | `()` | Store a password for a saved SSH connection in the OS keychain (Secret Service on Linux). Best-effort: errors are surfaced as typed `TauTermError` but do not abort the connection flow. |
| `toggle_fullscreen` | — | `FullscreenState` | Toggle the application window between windowed and fullscreen modes. Persists the new state to preferences immediately. Emits `fullscreen-state-changed` after a ~200 ms WM stabilisation delay (see §4.6 and ADR-0023). |
| `copy_to_clipboard` | `{ text: String }` | `()` | Copy to CLIPBOARD selection |
| `get_clipboard` | — | `String` | Read CLIPBOARD content |
| `open_url` | `{ url: String }` | `()` | Open validated URL in system browser |
| `mark_context_menu_used` | — | `()` | Clear first-launch context menu hint |
| `resize_pane` | `{ pane_id: PaneId, cols: u16, rows: u16, pixel_width: u16, pixel_height: u16 }` | `()` | Notify backend of pane resize; triggers `TIOCSWINSZ` + `SIGWINCH`. `pixel_width`/`pixel_height` are required for complete `TIOCSWINSZ` (image protocols, multiplexers). Resize events are debounced — see [§6.5](04-runtime-platform.md#65-back-pressure). |
| `frame_ack` | `{ pane_id: PaneId }` | `()` | Acknowledge that the frontend has painted the latest `screen-update` for this pane. Called after each `flushRafQueue()` cycle. Writes a timestamp to the pane's `Arc<AtomicU64>`; Task 2 reads it to drive back-pressure escalation (ADR-0027). Fire-and-forget — errors (e.g., stale pane ID) are silently ignored. Applicable to both PTY and SSH panes (ADR-0028). SSH calls mirror PTY behavior (one per rAF cycle). |
| `get_pane_screen_snapshot` | `{ pane_id: PaneId }` | `ScreenSnapshot` | Full screen state for initial render |
| `inject_pty_output` | `{ pane_id: PaneId, data: Vec<u8> }` | `()` | *E2E testing only — see §4.2.1* |
| `inject_ssh_failure` | `{ count: u32 }` | `()` | *E2E testing only — see §4.2.1* |
| `inject_ssh_delay` | `{ delay_ms: u64 }` | `()` | *E2E testing only — see §4.2.1* |
| `inject_ssh_disconnect` | `{ pane_id: PaneId }` | `()` | *E2E testing only — see §4.2.1* |
| `inject_credential_prompt` | `{ pane_id: PaneId, host: String, username: String }` | `()` | *E2E testing only — see §4.2.1* |

#### 4.2.1 E2E Testing Commands (`--features e2e-testing`)

The following commands are compiled **only** when the `e2e-testing` Cargo feature is enabled (see `CLAUDE.md` — build commands). They are absent from production binaries. They are used by WebdriverIO tests via `tauri-driver` to drive scenarios that require injecting artificial conditions.

| Command | Input | Output | Purpose |
|---------|-------|--------|---------|
| `inject_pty_output` | `{ pane_id: PaneId, data: Vec<u8> }` | `()` | Inject raw bytes into a PTY pane's output channel, bypassing the real PTY. Allows VT round-trip tests without a running shell. |
| `inject_ssh_failure` | `{ count: u32 }` | `()` | Arm a counter that causes the next `count` calls to `open_ssh_connection` to fail immediately with a synthetic error, regardless of pane ID. Each failing call decrements the counter by one. Used to exercise the rollback path in `handleConnectionOpen`. |
| `inject_ssh_delay` | `{ delay_ms: u64 }` | `()` | Set a one-shot delay (milliseconds) that fires at the start of the next SSH `connect_task` run, after the `Connecting` state event has been emitted. Single-shot: the delay is atomically zeroed on first read. Used to hold the connecting overlay visible long enough for assertions. |
| `inject_ssh_disconnect` | `{ pane_id: PaneId }` | `()` | Force-emit a `Disconnected` ssh-state-changed event for a pane. The connect_task may continue running in the background; the frontend immediately sees `Disconnected` and removes the overlay. Used to make the connecting overlay disappear without depending on a real TCP connection timeout. |
| `inject_credential_prompt` | `{ pane_id: PaneId, host: String, username: String }` | `()` | Emit a `credential-prompt` event for the specified pane, simulating an SSH server requesting authentication. Used to test the credential dialog flow without a live SSH server. |

These commands do not follow the `TauTermError` error envelope — they return `Result<(), String>` to keep E2E plumbing minimal. They must never be registered in production builds.

### 4.3 Events (emit — backend → frontend)

| Event name | Payload type | Trigger |
|------------|-------------|---------|
| `session-state-changed` | `SessionStateChanged` | Topology changes that originate asynchronously or outside a direct command: process title changed (OSC 0/2), OSC 7 CWD change that alters the effective tab title (when no OSC 0/2 title is set), pane process exited (SIGCHLD), active-tab/active-pane changed by user action via `set_active_pane`. **Not** emitted for `split_pane` or `close_pane` — those commands return the updated `TabState` directly. |
| `ssh-state-changed` | `SshStateChangedEvent` | SSH lifecycle state transition |
| `screen-update` | `ScreenUpdateEvent` | Terminal output processed (cell diffs or full snapshot) |
| `mode-state-changed` | `ModeStateChangedEvent` | A terminal mode relevant to frontend input encoding changed. Payload: `{ paneId: PaneId, decckm: bool, deckpam: bool }`. Without these flags, `keyboard.ts` cannot distinguish normal mode (ESC [ A/B/C/D) from application cursor mode (ESC O A/B/C/D), causing arrow key encoding errors in vim and readline (FS-KBD-007, FS-KBD-010). Emitted on DECSET/DECRST of modes 1 (DECCKM) and DECKPAM/DECKPNM (ESC =/ESC >). |
| `scroll-position-changed` | `ScrollPositionChangedEvent` | Scrollback position changed |
| `credential-prompt` | `CredentialPromptEvent` | Backend needs credentials from user |
| `host-key-prompt` | `HostKeyPromptEvent` | First connection or key change requiring user verification |
| `notification-changed` | `NotificationChangedEvent` | Tab/pane activity notification added or cleared |
| `cursor-style-changed` | `CursorStyleChangedEvent` | DECSCUSR (CSI Ps SP q) changed the cursor shape for a pane. Payload: `{ paneId, shape: u8 }` where `shape` is the raw DECSCUSR parameter 0–6. See FS-VT-030. Emitted immediately on DECSCUSR — does not wait for the next `screen-update` cycle. Not emitted for DECSET/DECRST ?12 (cursor blink mode), which is propagated via the `cursor.blink` field of `ScreenUpdateEvent`. |
| `bell-triggered` | `BellTriggeredEvent` | Terminal produced a BEL character. Rate-limited to at most one event per 100 ms per pane (FS-VT-090). Payload: `{ paneId }`. |
| `passphrase-prompt` | `PassphrasePromptEvent` | SSH pubkey auth requires a passphrase for an encrypted private key (FS-SSH-019a). Payload: `{ paneId, keyPathLabel: string, failed: bool, isKeychainAvailable: bool }`. The `keyPathLabel` is the filename only — never the full path (security constraint). |
| `fullscreen-state-changed` | `FullscreenStateChangedEvent` | Emitted after the WM has confirmed the fullscreen geometry transition (~200 ms after `toggle_fullscreen`). Payload: `{ isFullscreen: bool }`. Informational — the frontend ResizeObserver triggers `resize_pane` independently. |
| `osc52-write-requested` | `Osc52WriteRequestedEvent` | The terminal requested a clipboard write via OSC 52 and the `allow_osc52_write` policy permits it (FS-VT-075). Payload: `{ paneId, data: string }` where `data` is the decoded UTF-8 clipboard payload. The frontend is responsible for writing to the system clipboard. |
| `ssh-warning` | `SshWarningEvent` | A deprecated SSH algorithm was detected during the connection handshake (`ssh-rsa` SHA-1, `ssh-dss`). Non-blocking — for informational display only (FS-SSH-014). Payload: `{ paneId, algorithm: string, reason: string }`. |
| `ssh-reconnected` | `SshReconnectedEvent` | Emitted immediately after a successful SSH reconnect. The frontend inserts a visual separator in the scrollback to distinguish output from the previous and new sessions (FS-SSH-042). Payload: `{ paneId, timestampMs: number }`. |

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

`session-state-changed` is emitted only for changes that originate asynchronously or from outside a direct user command: OSC 0/2-driven title change, OSC 7 CWD change when the CWD basename becomes the effective title (i.e. no OSC 0/2 title is set — lower-priority fallback in the resolution chain), pane process exit (SIGCHLD leading to a `Terminated` state — `ptyState` field updated), `hasForegroundProcess` transitions (foreground process group change detected), and `set_active_pane` confirmation.

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

### 4.7 Multi-Step Flows

Some IPC interactions span multiple commands and events. The two flows below are documented here to make the full sequence visible in one place.

#### 4.7.1 Fullscreen toggle flow

1. Frontend calls `toggle_fullscreen`.
2. Backend queries the current window state, flips it, and persists the new value to preferences immediately (`prefs.set_fullscreen(target)`).
3. `toggle_fullscreen` returns `FullscreenState { isFullscreen: bool }` synchronously to the frontend.
4. In a separate `tokio::spawn`, the backend waits ~200 ms for the WM to confirm the geometry transition (see ADR-0023 for the rationale).
5. Backend emits `fullscreen-state-changed` with `{ isFullscreen: bool }`.
6. The frontend `onFullscreenStateChanged` handler (in `useTerminalView.core.svelte.ts`) receives the event and restores focus to the active viewport. The ResizeObserver then fires, triggering `resize_pane` → `SIGWINCH` to the active panes.

Note: SIGWINCH is *not* sent from inside `toggle_fullscreen` itself. The frontend ResizeObserver is the authoritative source of resize events; the `fullscreen-state-changed` event is informational and used for focus restoration timing.

#### 4.7.2 Passphrase flow

1. Frontend calls `open_ssh_connection` (or `reconnect_ssh`).
2. During pubkey authentication, the SSH connect task detects that the private key is encrypted.
3. Backend emits `passphrase-prompt` event with `{ paneId, keyPathLabel, failed, isKeychainAvailable }`.
4. `SshPassphraseDialog.svelte` renders in the frontend, displaying the key filename.
5. The user enters the passphrase and optionally checks "Save in keychain". The frontend calls `provide_passphrase({ paneId, passphrase, saveInKeychain })`.
6. The backend SSH connect task receives the passphrase via the oneshot channel stored in `SshManager::pending_passphrases` and resumes the authentication flow.
7. On success, a `ssh-state-changed` event with `state: connected` is emitted. On failure (wrong passphrase), the backend emits another `passphrase-prompt` event with `failed: true`, and the dialog reappears.

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
              ┌────────────────────┐  ┌──────────────┐
              │  Terminated        │  │   Closing    │  — SIGHUP sent to process group
              │                    │  └──────┬───────┘
              │  exit 0 ─────────────────────┤ (auto-close, no user action)
              │  exit ≠ 0 / signal │         │ process exited
              │  (banner shown)    │         ▼
              └──────────┬─────────┘  ┌──────────────┐
                         │            │    Closed    │
              user: restart or close  └──────────────┘
                         │
                         ▼
               restart → Spawning
               close  → Closed
```

**Terminated state — exit code 0:** When the child process exits with code 0 (clean exit), the backend emits `session-state-changed` with `changeType: 'pane-metadata-changed'` and `ptyState: 'terminated-clean'`. The frontend auto-closes the pane immediately — no banner is shown, no user confirmation is required (FS-PTY-005). This is equivalent to a user-initiated `close_pane` and follows the same last-pane/last-tab logic (§4.5.3).

**Terminated state — exit code ≠ 0 or signal:** When the child process exits with a non-zero code or is terminated by a signal, the backend emits `session-state-changed` with `changeType: 'pane-metadata-changed'` and `ptyState: 'terminated-error'` (carrying the exit code or signal number). The pane remains open. The frontend renders a banner with Restart and Close actions (FS-PTY-005, FS-PTY-006). No confirmation is required to close from this state.

**`PtyLifecycleState` in `PaneState`:** `PaneState` carries a `ptyState` field typed as:

```typescript
type PtyLifecycleState =
  | { status: 'running'; hasForegroundProcess: boolean }
  | { status: 'terminated-clean' }
  | { status: 'terminated-error'; exitCode: number | null; signal: string | null }
  | { status: 'closed' };
```

Rust equivalent:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum PtyLifecycleState {
    Running { has_foreground_process: bool },
    TerminatedClean,
    TerminatedError { exit_code: Option<i32>, signal: Option<String> },
    Closed,
}
```

**Foreground process detection:** The backend detects whether the PTY foreground process group differs from the shell's process group using `tcgetpgrp(master_fd)` (Linux: `ioctl(TIOCGPGRP)`) compared against the shell's `pgid` saved at spawn time. This check is performed:
- Eagerly, whenever `session-state-changed` is emitted for `pane-metadata-changed` changes (title OSC, process exit, etc.) — the current value is recomputed and included in the `PaneState` payload. The frontend's replica is always up to date.
- On demand, when the backend builds the `PaneState` snapshot in response to `get_session_state` (initial mount).

The `hasForegroundProcess` flag is the single source of truth for whether a confirmation dialog is required. The backend is responsible for computing it; the frontend only reads it.

**Running → Closing:** Triggered by user close action. Before invoking `close_pane`, the frontend reads `PaneState.ptyState.hasForegroundProcess`. If `true`, it shows a confirmation dialog (FS-PTY-008). The backend receives `close_pane` only after user confirmation (or when `hasForegroundProcess` is `false`); no redundant process check occurs inside the command handler.

**Window close event:** When the user attempts to close the application window, the frontend intercepts Tauri's `CloseRequested` window event before it propagates. It aggregates `hasForegroundProcess` across all panes in all tabs. If any pane has an active foreground process, it shows a single confirmation dialog indicating the count of affected tabs/panes (FS-PTY-008). On confirmation (or when no active processes exist), it calls `close_pane` for each pane sequentially (or invokes the window close API directly if no panes require explicit teardown).

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

**Closed:** User-initiated close (via `close_tab`/`close_pane`), explicit `close_ssh_connection` command, or remote process exit with code 0. No reconnection available from this state; a new `open_ssh_connection` is required.

**SSH teardown on tab/pane close (FS-SSH-043):** The `close_tab` and `close_pane` command handlers call `ssh_manager.close_connection(pane_id)` for each pane being removed, before removing those panes from the `SessionRegistry`. This call is best-effort: a `PaneNotFound` error is silently ignored (the session was already Closed or Disconnected). No `TauTermError` is surfaced to the frontend for a missing SSH entry during a close operation.

**SSH `Closed` state → automatic pane close (FS-SSH-044):** When `ssh_task.rs` emits `ssh-state-changed` with `state: Closed` (remote shell exited with code 0), the frontend handler calls `doClosePane(paneId)` — the same code path used for PTY exit code 0. This invokes `close_pane` on the backend, which applies the same last-pane and last-tab rules (§4.5.3). The `SshLifecycleState` TypeScript union therefore includes the `{ type: 'closed' }` variant, which must be handled in the `ssh-state-changed` event listener.

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
- The Kitty keyboard protocol is out of scope for v1 (see [§12.4](06-appendix.md#124-kitty-keyboard-protocol-post-v1)).
- Shift+Enter is indistinguishable from Enter (both = 0x0D) in standard xterm encoding (FS-KBD-013).
