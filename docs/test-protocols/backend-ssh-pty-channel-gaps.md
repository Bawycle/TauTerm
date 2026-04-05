# Test Protocol — Backend SSH PTY Channel & Implementation Gaps

> **Document status:** Initial revision — 2026-04-05
> **Author:** test-engineer
> **Based on:** FS.md (FS-SSH-011 through FS-SSH-013, FS-SSH-040), ARCHITECTURE.md §5 (SSH lifecycle), CLAUDE.md bootstrap state (session h)
> **FS requirements covered:** FS-SSH-011, FS-SSH-012, FS-SSH-013, FS-SSH-040, FS-CONN-*, FS-SB-006 (scroll offset), FS-VT-090 (notifications)

---

## 1. Purpose & Scope

This protocol covers the backend gaps identified after session h. It supplements — and does not supersede — the main test protocol at `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`.

### Gaps addressed

| Domain | Gap | Current state |
|---|---|---|
| **SSH PTY channel** | `channel_open_session` + `request_pty` + `shell` + read loop + `screen-update` events (FS-SSH-013) | `connect_task` drops the russh Handle immediately after `Connected`; channel never opened |
| **SSH TOFU flow** | `accept_host_key` / `reject_host_key` commands persist to / abort against `KnownHostsStore`; 120 s timeout | Commands are stubs (`TODO`) |
| **SSH credential prompt** | `provide_credentials` oneshot-sender pattern; timeout; cleanup on `close_connection` | Command is a stub |
| **SSH algorithm warning** | `dismiss_ssh_algorithm_warning` clears per-pane alert state | Command is a stub |
| **SSH reconnect** | `reconnect_ssh` re-runs the full connect flow using stored credentials (FS-SSH-040) | `SshManager::reconnect` is a no-op stub |
| **Connection persistence** | `save_connection` / `delete_connection` write through to `PreferencesStore::save_to_disk()` | Disk write is a `TODO` comment in `connection_cmds.rs` |
| **Scroll offset state** | `scroll_pane` maintains `scroll_offset` in `PaneSession`, clamps to `[0, scrollback_lines]`, persists across tab switches | Registry returns the raw `offset` parameter without storing it |
| **`open_url`** | Opens URL via `tauri-plugin-opener`; scheme already validated | Validated but opener call is a `TODO` |
| **`mark_context_menu_used`** | Persists the "hint shown" flag into preferences | Complete stub |

### Out of scope for this protocol

- VT parser correctness (covered in main protocol §4.2)
- Local PTY lifecycle (covered in main protocol §4.1)
- Preferences schema validation and i18n (covered in main protocol §4.6/§4.7)
- Security threat scenarios for SSH (covered in `security-pty-ipc-ssh-credentials-csp-osc52.md`)

### ID series

| Prefix | Domain |
|---|---|
| `BSSH-*` | SSH PTY channel and flow gaps |
| `BCONN-*` | Connection config persistence |
| `BSCR-*` | Scroll offset state |
| `BSYS-*` | System commands (`open_url`, `mark_context_menu_used`) |

### Test layer notation

- **Unit (Rust)** — `#[cfg(test)]` inline module, run by `cargo nextest run`. No network, no PTY, no display server.
- **Integration (Rust)** — `src-tauri/tests/` crate-level, run by `cargo nextest run`. May use a mock SSH server or temp files.
- **E2E** — WebdriverIO + tauri-driver (`pnpm wdio`). Requires a production build. Marked `[E2E-DEFERRED: build required]` until the build gate is green.

---

## 2. SSH PTY Channel (FS-SSH-013)

The tests in this section unblock once `connect_task` in `ssh/manager.rs` opens a PTY channel after the `Connected` transition instead of dropping the Handle.

---

### BSSH-001
**FS requirements:** FS-SSH-013
**Layer:** Unit (Rust) — mock russh channel
**Priority:** Must
**Location:** `src-tauri/src/ssh/manager.rs` or a new `src-tauri/src/ssh/channel.rs`

**Preconditions:** A mock `russh::client::Handle` that records method calls. An `SshManager` with one entry in `Connected` state.

**Steps:**
1. Call the post-auth PTY channel opening routine with a `PaneId` and initial `cols=80, rows=24`.
2. Assert `channel_open_session()` was called exactly once.
3. Assert `request_pty("xterm-256color", 80, 24, ...)` was called with the correct term-env string (FS-PTY-011) and matching dimensions.
4. Assert `shell()` was called after the PTY request was acknowledged.
5. Assert no `exec()` call was made (FS-SSH-013 uses `shell`, not `exec`).

**Expected result:** Channel open → PTY request → shell launch is called in that exact order with correct parameters.

**Status:** [BLOCKED: channel not yet opened in connect_task]

---

### BSSH-002
**FS requirements:** FS-SSH-013
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/ssh/channel.rs` (future)

**Preconditions:** A mock PTY channel that yields pre-encoded output bytes when read.

**Steps:**
1. Feed 48 bytes of ANSI output through the mock channel read loop (simulating the SSH data-channel stream).
2. Assert that `VtProcessor::process()` was called once or more with the full byte sequence.
3. Assert that a `screen-update` event was emitted (check via a mock `AppHandle` event collector).
4. Assert the emitted event payload is a valid `ScreenUpdateEvent` with `pane_id` matching the channel's pane.

**Expected result:** Data arriving on the SSH channel is processed through the VT pipeline and triggers a `screen-update` event identical to the local PTY path.

**Status:** [BLOCKED: channel read loop not implemented]

---

### BSSH-003
**FS requirements:** FS-SSH-013
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/ssh/channel.rs` (future)

**Preconditions:** A mock channel where `read` returns `None` (EOF) on the first call.

**Steps:**
1. Start the SSH channel read loop.
2. Assert the loop exits cleanly (no panic, no infinite wait).
3. Assert a `session-state-changed` event is emitted with a `pane-metadata-changed` changeType reflecting the terminal process termination (pane state = `Terminated`).

**Expected result:** Channel EOF is treated as remote shell exit. The pane transitions to `Terminated` state, triggering the same visual overlay as a local process exit.

**Status:** [BLOCKED: channel read loop not implemented]

---

### BSSH-004
**FS requirements:** FS-SSH-013
**Layer:** Integration (Rust) — mock SSH server
**Priority:** Must
**Location:** `src-tauri/tests/ssh_pty_channel.rs` (new file)

**Preconditions:** A mock SSH server (in-process, based on `russh` server API) that accepts a known test key, opens a PTY channel, and echoes any input back. The server listens on `127.0.0.1:0` (OS-assigned port).

**Steps:**
1. Start the mock server and obtain the bound port.
2. Call `SshManager::open_connection` with a config pointing at the mock server.
3. Wait for the `ssh-state-changed` event with `state = Connected` (100 ms timeout).
4. Send input via `send_input` to the SSH pane.
5. Wait for a `screen-update` event (200 ms timeout).
6. Assert the `screen-update` payload includes the echoed bytes.

**Expected result:** Full round-trip from `send_input` → SSH channel write → echo back → VT processing → `screen-update` event completes without error.

**Status:** [BLOCKED: channel not yet opened]

---

### BSSH-005
**FS requirements:** FS-SSH-013
**Layer:** E2E
**Priority:** Must
**Location:** `tests/e2e/ssh-pty-roundtrip.spec.ts` (new or extend existing)

**Preconditions:** TauTerm built and running. Mock SSH server started as a test fixture. A saved connection pointing to that mock server.

**Steps:**
1. Open the saved SSH connection from the connection manager.
2. Wait for the SSH badge to show "Connected" state in the tab.
3. Type `echo hello` and press Enter.
4. Wait for "hello" to appear in the terminal grid.

**Expected result:** The string "hello" is visible in the terminal. The tab title reflects the remote process title (or the connection label). The status bar shows the SSH Connected indicator.

**Status:** [E2E-DEFERRED: build required] [BLOCKED: channel not yet opened]

---

## 3. SSH TOFU Flow (FS-SSH-011, FS-SSH-012)

The commands `accept_host_key`, `reject_host_key`, and `provide_credentials` are currently stubs in `src-tauri/src/commands/ssh_prompt_cmds.rs`. The tests below specify their required behavior and serve as the acceptance specification for the implementation.

---

### BSSH-006
**FS requirements:** FS-SSH-011
**Layer:** Unit (Rust)
**Priority:** Must (security)
**Location:** `src-tauri/src/commands/ssh_prompt_cmds.rs`

**Preconditions:** An `SshManager` with one pane in `Connecting` state. A `KnownHostsStore` backed by a temp file. The connection flow is paused awaiting host-key resolution (the handler returned `AwaitingTofuConfirmation`).

**Steps:**
1. Call `accept_host_key(pane_id)`.
2. Assert `KnownHostsStore::add_entry()` was called with the host and the key presented during handshake.
3. Assert the connection flow resumes (the connection transitions to `Authenticating` within 50 ms).
4. Assert the temp known_hosts file now contains the new entry.
5. Assert the entry's permissions are `0o600`.

**Expected result:** `accept_host_key` persists the key to disk and resumes the connection. File permissions are correct (FS-SSH-011 + SEC-SSH-003).

**Status:** [BLOCKED: stub]

---

### BSSH-007
**FS requirements:** FS-SSH-011
**Layer:** Unit (Rust)
**Priority:** Must (security)
**Location:** `src-tauri/src/commands/ssh_prompt_cmds.rs`

**Preconditions:** Same as BSSH-006 — connection paused awaiting TOFU confirmation.

**Steps:**
1. Call `reject_host_key(pane_id)`.
2. Assert the connection is removed from `SshManager` (entry no longer present after 50 ms).
3. Assert a `ssh-state-changed` event is emitted with `state = Disconnected` and a non-null `reason` string.
4. Assert `KnownHostsStore::add_entry()` was NOT called.

**Expected result:** `reject_host_key` aborts the connection without adding any entry to known_hosts. The pane transitions to `Disconnected`.

**Status:** [BLOCKED: stub]

---

### BSSH-008
**FS requirements:** FS-SSH-011
**Layer:** Unit (Rust)
**Priority:** Must (security)
**Location:** `src-tauri/src/commands/ssh_prompt_cmds.rs`

**Preconditions:** A connection in `Connecting` state. A TOFU prompt has been emitted to the frontend. No user action is taken.

**Steps:**
1. Wait 121 seconds (using `tokio::time::advance` — do not actually sleep in CI).
2. Assert the connection is removed from `SshManager`.
3. Assert a `ssh-state-changed` event is emitted with `state = Disconnected` and a `reason` indicating timeout.

**Expected result:** The TOFU confirmation window expires after 120 seconds. The connection is aborted automatically. No resource leak occurs.

**Status:** [BLOCKED: stub]

---

### BSSH-009
**FS requirements:** FS-SSH-012
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/commands/ssh_prompt_cmds.rs`

**Preconditions:** An SSH connection paused awaiting credentials (pubkey auth failed or no identity file configured). A oneshot channel is set up for the pending credential prompt. `SshManager` holds a reference to the sender.

**Steps:**
1. Call `provide_credentials(pane_id, credentials)` with a valid username and password.
2. Assert the oneshot receiver fires within 50 ms with the provided credentials.
3. Assert the connection resumes (transitions to `Authenticating`).
4. Call `provide_credentials(pane_id, credentials)` a second time for the same pane while still in `Connecting` state.
5. Assert the second call returns `Err` (no pending prompt).

**Expected result:** `provide_credentials` delivers credentials exactly once to the waiting auth task via the oneshot channel. A duplicate call is rejected.

**Status:** [BLOCKED: stub]

---

### BSSH-010
**FS requirements:** FS-SSH-012
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/commands/ssh_prompt_cmds.rs`

**Preconditions:** An SSH connection awaiting credentials. No user response within the timeout window.

**Steps:**
1. Set the credential prompt timeout to 5 seconds (overridden via env var or test constructor injection).
2. Advance Tokio time by 6 seconds.
3. Assert the oneshot sender is dropped (receiver receives `Err(RecvError)`).
4. Assert a `ssh-state-changed` event is emitted with `state = Disconnected`, `reason` = credential prompt timeout.

**Expected result:** Unanswered credential prompts time out. The connection is cleaned up without hanging.

**Status:** [BLOCKED: stub]

---

### BSSH-011
**FS requirements:** FS-SSH-012
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/commands/ssh_prompt_cmds.rs`

**Preconditions:** An SSH connection awaiting credentials. The connection is then closed by calling `close_connection(pane_id)` before the prompt is answered.

**Steps:**
1. Call `close_connection(pane_id)`.
2. Assert the oneshot sender is dropped (no orphaned sender in the map).
3. Assert calling `provide_credentials(pane_id, ...)` after close returns `Err(PaneNotFound)`.

**Expected result:** `close_connection` cleans up any pending credential oneshot state. No dangling senders remain after a pane is closed.

**Status:** [BLOCKED: stub]

---

### BSSH-012
**FS requirements:** FS-SSH-014
**Layer:** Unit (Rust)
**Priority:** Should
**Location:** `src-tauri/src/commands/ssh_prompt_cmds.rs`

**Preconditions:** A pane with an active `AlgorithmWarning` state set in `SshManager` (or a parallel per-pane state structure). The warning was set during connection negotiation when a deprecated algorithm was detected.

**Steps:**
1. Call `dismiss_ssh_algorithm_warning(pane_id)`.
2. Assert the pane's algorithm warning flag is cleared.
3. Assert a `session-state-changed` event is emitted (or equivalent) so the frontend can hide the warning banner.
4. Call `dismiss_ssh_algorithm_warning(pane_id)` again (idempotent call).
5. Assert the second call returns `Ok(())` without error.

**Expected result:** `dismiss_ssh_algorithm_warning` clears the warning state once and is idempotent thereafter.

**Status:** [BLOCKED: stub]

---

### BSSH-013
**FS requirements:** FS-SSH-011
**Layer:** E2E
**Priority:** Must (security)
**Location:** `tests/e2e/ssh-tofu.spec.ts` (new file)

**Preconditions:** TauTerm built and running. Mock SSH server whose host key is not in the app's known_hosts file.

**Steps:**
1. Open a new SSH connection to the mock server.
2. Wait for the TOFU dialog to appear.
3. Verify the dialog displays: SHA-256 fingerprint, key type (e.g., `ED25519`), plain-language explanation.
4. Click "Accept".
5. Wait for the "Connected" state indicator.
6. Quit TauTerm. Relaunch TauTerm.
7. Open the same connection again.
8. Assert no TOFU dialog appears (key is already trusted).

**Expected result:** TOFU dialog appears on first connection, key is persisted, second connection proceeds without a prompt.

**Status:** [E2E-DEFERRED: build required] [BLOCKED: stub]

---

### BSSH-014
**FS requirements:** FS-SSH-011
**Layer:** E2E
**Priority:** Must (security)
**Location:** `tests/e2e/ssh-tofu.spec.ts`

**Preconditions:** TauTerm built. Mock SSH server whose host key is already trusted in known_hosts. Server's key is then rotated to a different key.

**Steps:**
1. Open the SSH connection.
2. Wait for the key-mismatch warning dialog.
3. Verify the dialog shows both the stored and presented fingerprints, a MITM warning, and that the default action button is "Reject" (not Accept).
4. Click "Reject".
5. Assert the connection is closed and the pane shows `Disconnected` state.

**Expected result:** Key mismatch triggers a blocking dialog. The safe default is rejection. Connection is not established on rejection.

**Status:** [E2E-DEFERRED: build required] [BLOCKED: stub]

---

## 4. SSH Reconnect (FS-SSH-040)

`SshManager::reconnect` is currently a no-op. The tests below define the required behavior for the full reconnect flow.

---

### BSSH-015
**FS requirements:** FS-SSH-040
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/ssh/manager.rs`

**Preconditions:** An `SshManager` with one entry in `Disconnected` state. The original `SshConnectionConfig` and last-used `Credentials` are available in the manager (stored during initial connect).

**Steps:**
1. Call `SshManager::reconnect(pane_id)`.
2. Assert the state transitions immediately to `Connecting`.
3. Assert a new `connect_task` is spawned (observable via a mock that records task spawns, or by asserting the state advances to `Authenticating` within a reasonable timeout against a mock server).

**Expected result:** `reconnect` re-runs the full connect flow (`Connecting → Authenticating → Connected`) using the stored config and credentials. The existing `PaneId` is preserved.

**Status:** [BLOCKED: stub]

---

### BSSH-016
**FS requirements:** FS-SSH-040
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/ssh/manager.rs`

**Preconditions:** An `SshManager` with one entry in `Connected` state (not `Disconnected`).

**Steps:**
1. Call `SshManager::reconnect(pane_id)`.
2. Assert the call returns `Err` (reconnect on a live connection is a no-op or guard error).

**Expected result:** Reconnect is only valid from `Disconnected` state. Calling it on a `Connected` pane is rejected to prevent accidental disconnection.

**Status:** [BLOCKED: stub]

---

### BSSH-017
**FS requirements:** FS-SSH-040, FS-SSH-012
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/ssh/manager.rs`

**Preconditions:** A pane that was originally connected with password credentials. The pane is now `Disconnected`. The credential store has the password available for that pane.

**Steps:**
1. Call `SshManager::reconnect(pane_id)`.
2. Assert the reconnect task retrieves credentials from the credential store (not from IPC input).
3. Assert no `credential-prompt` event is emitted to the frontend (stored credentials are used silently).

**Expected result:** Reconnect with stored credentials requires no user interaction. The user is only prompted if credentials have been deleted since the initial connection.

**Status:** [BLOCKED: stub]

---

### BSSH-018
**FS requirements:** FS-SSH-040
**Layer:** E2E
**Priority:** Must
**Location:** `tests/e2e/ssh-reconnect.spec.ts` (new file)

**Preconditions:** TauTerm running. An active SSH connection to the mock server. The mock server is then stopped to simulate a network interruption.

**Steps:**
1. Stop the mock SSH server.
2. Wait for the pane to display `Disconnected` state (keepalive timeout or connection error within 10 s).
3. Observe the reconnect action in the status bar or terminated pane overlay.
4. Restart the mock SSH server.
5. Click the reconnect action.
6. Wait for the `Connected` indicator to reappear (10 s timeout).

**Expected result:** After network interruption, TauTerm detects disconnection and shows a reconnect affordance. Clicking reconnect re-establishes the session without requiring the user to re-enter credentials.

**Status:** [E2E-DEFERRED: build required] [BLOCKED: stub]

---

## 5. Connection Config Persistence (FS-SSH-030, FS-SSH-031)

`save_connection` and `delete_connection` in `src-tauri/src/commands/connection_cmds.rs` modify an in-memory snapshot of preferences but do not call `save_to_disk()`. The following tests verify the disk-write behavior that must be added.

---

### BCONN-001
**FS requirements:** FS-SSH-030
**Layer:** Integration (Rust)
**Priority:** Must
**Location:** `src-tauri/tests/connection_persistence.rs` (new file)

**Preconditions:** A `PreferencesStore` backed by a temp file. No connections in preferences.

**Steps:**
1. Call the `save_connection` command handler with a valid `SshConnectionConfig` (no identity file).
2. Assert the returned `ConnectionId` matches the config's `id`.
3. Read the temp preferences file from disk.
4. Assert the file contains the connection with correct host, port, username fields.
5. Assert no `password` field is present in the JSON.

**Expected result:** `save_connection` persists the config to the preferences file. Password is never serialized.

**Status:** [BLOCKED: save_to_disk call missing in connection_cmds.rs]

---

### BCONN-002
**FS requirements:** FS-SSH-030
**Layer:** Integration (Rust)
**Priority:** Must
**Location:** `src-tauri/tests/connection_persistence.rs`

**Preconditions:** A `PreferencesStore` backed by a temp file. Two existing connections.

**Steps:**
1. Call `save_connection` with a config whose `id` matches an existing connection but with a changed `host`.
2. Read the preferences file from disk.
3. Assert there is still exactly two connections in the file (no duplicate created).
4. Assert the updated connection has the new `host` value.

**Expected result:** `save_connection` on an existing `id` updates in place rather than appending.

**Status:** [BLOCKED: save_to_disk call missing]

---

### BCONN-003
**FS requirements:** FS-SSH-031
**Layer:** Integration (Rust)
**Priority:** Must
**Location:** `src-tauri/tests/connection_persistence.rs`

**Preconditions:** A `PreferencesStore` backed by a temp file. One existing connection.

**Steps:**
1. Call `delete_connection(connection_id)` with the existing connection's ID.
2. Read the preferences file from disk.
3. Assert the file contains zero connections.
4. Call `delete_connection(connection_id)` again.
5. Assert the second call returns `Ok(())` (idempotent — deleting a non-existent entry is not an error).

**Expected result:** `delete_connection` removes the entry from disk. The operation is idempotent.

**Status:** [BLOCKED: save_to_disk call missing]

---

### BCONN-004
**FS requirements:** FS-SSH-030
**Layer:** Unit (Rust)
**Priority:** Must (security)
**Location:** `src-tauri/src/commands/connection_cmds.rs`

**Preconditions:** None.

**Steps:**
1. Call `save_connection` with a config where `identity_file = Some("../../../etc/passwd")`.
2. Assert the command returns `Err` with code `INVALID_PATH`.
3. Call `save_connection` with `identity_file = Some("relative/path/key")`.
4. Assert the command returns `Err` with code `INVALID_PATH`.
5. Call `save_connection` with `identity_file = Some("/home/user/.ssh/id_ed25519\0injected")`.
6. Assert the command returns `Err` with code `INVALID_PATH`.

**Expected result:** Path traversal, relative paths, and null-byte injection are all rejected before the config reaches `PreferencesStore`. (Tests for existing validation — ensures no regression when disk-write is wired.)

**Status:** Validation is already implemented; this test guards regression during the save_to_disk wiring.

---

### BCONN-005
**FS requirements:** FS-SSH-030, FS-SSH-031
**Layer:** E2E
**Priority:** Must
**Location:** `tests/e2e/connection-persistence.spec.ts` (new file)

**Preconditions:** TauTerm built and running.

**Steps:**
1. Open the connection manager.
2. Add a new SSH connection (host: `test-server`, port: 2222, username: `alice`).
3. Save the connection.
4. Quit TauTerm and relaunch it.
5. Open the connection manager.
6. Assert the saved connection appears with the correct host, port, and username.
7. Delete the connection.
8. Quit and relaunch TauTerm.
9. Open the connection manager.
10. Assert the connection is no longer present.

**Expected result:** Connections persist across application restarts. Deletions also persist.

**Status:** [E2E-DEFERRED: build required] [BLOCKED: save_to_disk call missing]

---

## 6. Scroll Offset State per Pane

`SessionRegistry::scroll_pane` returns the raw `offset` parameter without storing it. The frontend therefore cannot restore scroll position when switching tabs.

---

### BSCR-001
**FS requirements:** FS-SB-006
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/session/registry.rs`

**Preconditions:** A `SessionRegistry` with one pane. The pane's VT processor has 100 lines of scrollback.

**Steps:**
1. Call `scroll_pane(pane_id, -10)` (scroll up 10 lines).
2. Assert the returned `ScrollPositionState.offset` is 10 (stored, not echoed).
3. Call `scroll_pane(pane_id, -5)` (scroll up 5 more lines).
4. Assert the returned offset is 15 (cumulative).
5. Call `scroll_pane(pane_id, 100)` (scroll far past the end).
6. Assert the returned offset is clamped to `scrollback_lines` (100 in this fixture).

**Expected result:** Scroll offset is maintained as a running total. It is clamped to `[0, scrollback_lines]`.

**Status:** [BLOCKED: offset state not stored in PaneSession]

---

### BSCR-002
**FS requirements:** FS-SB-006
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/session/registry.rs`

**Preconditions:** A `SessionRegistry` with two panes in separate tabs. Pane A has been scrolled to offset 20; pane B has not been scrolled (offset 0).

**Steps:**
1. Call `get_pane_screen_snapshot(pane_id_A)`.
2. Assert the snapshot includes `scroll_offset: 20`.
3. Call `get_pane_screen_snapshot(pane_id_B)`.
4. Assert the snapshot includes `scroll_offset: 0`.
5. Switch the active tab (call `set_active_pane(pane_id_B)`) and switch back to pane A.
6. Call `get_pane_screen_snapshot(pane_id_A)` again.
7. Assert `scroll_offset` is still 20 (scroll position survives tab switch).

**Expected result:** Each pane maintains its own scroll offset independently. Tab switches do not reset scroll state.

**Status:** [BLOCKED: offset state not stored in PaneSession]

---

### BSCR-003
**FS requirements:** FS-SB-006
**Layer:** Unit (Rust)
**Priority:** Should
**Location:** `src-tauri/src/session/registry.rs`

**Preconditions:** A pane scrolled to offset 50 (50 lines above the bottom of scrollback).

**Steps:**
1. Feed new output to the pane via `VtProcessor::process()` that advances the screen by 5 lines.
2. Assert the scroll offset is adjusted so the view remains at the same relative content position (or is reset to 0 if the implementation follows the "new output scrolls to bottom" model per FS-SB-007).
3. Verify the behavior is consistent with the design decision documented in FS-SB-007 (new output while not at bottom: implementation must choose one of: stay, follow, or reset — document the chosen behavior).

**Expected result:** Scroll offset behavior on new output is deterministic and matches the chosen policy. This test should codify the policy decision.

**Note:** This scenario requires a design decision before implementation. The test is provided as a specification skeleton. The rust-dev must fill in the expected result once the policy is confirmed with the architect.

**Status:** [BLOCKED: design decision pending — coordinate with architect before implementing]

---

### BSCR-004
**FS requirements:** FS-SB-006
**Layer:** E2E
**Priority:** Should
**Location:** `tests/e2e/scroll-state.spec.ts` (new file)

**Preconditions:** TauTerm running. Two terminal tabs open.

**Steps:**
1. In tab 1, generate 200 lines of output (e.g., `seq 1 200`).
2. Scroll up 50 lines in tab 1 (keyboard shortcut or mouse wheel).
3. Switch to tab 2.
4. Switch back to tab 1.
5. Assert the scroll position is still 50 lines from the bottom.

**Expected result:** Scroll offset is preserved per pane across tab switches.

**Status:** [E2E-DEFERRED: build required] [BLOCKED: offset state not stored]

---

## 7. System Commands

### BSYS-001
**FS requirements:** FS-VT-071 (hyperlink activation), FS-SEC-004 (URL scheme whitelist)
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/commands/system_cmds.rs`

**Preconditions:** `tauri-plugin-opener` is registered in the Tauri app builder.

**Steps:**
1. Call `open_url("https://example.com")`.
2. Assert `tauri_plugin_opener::open_url` was called with the same URL (mock or inspect via test double).
3. Call `open_url("http://example.com")` — assert delegation.
4. Call `open_url("mailto:user@example.com")` — assert delegation.
5. Call `open_url("ssh://server.example.com")` — assert delegation.

**Expected result:** All whitelisted schemes are forwarded to the opener plugin.

**Status:** [BLOCKED: opener call is a TODO]

---

### BSYS-002
**FS requirements:** FS-SEC-004
**Layer:** Unit (Rust)
**Priority:** Must (security)
**Location:** `src-tauri/src/commands/system_cmds.rs`

**Preconditions:** None.

**Steps:**
1. Call `open_url("file:///etc/passwd")` — assert `Err(INVALID_URL)`.
2. Call `open_url("javascript:alert(1)")` — assert `Err(INVALID_URL)`.
3. Call `open_url("data:text/html,<script>alert(1)</script>")` — assert `Err(INVALID_URL)`.
4. Call `open_url(url_of_length_2049)` — assert `Err(INVALID_URL)`.
5. Call `open_url("https://\x01evil.com")` — assert `Err(INVALID_URL)` (C0 control character).

**Expected result:** All non-whitelisted schemes, oversized URLs, and control-character injections are rejected before reaching the opener plugin. (Tests for existing validation — guards regression when opener is wired.)

**Status:** Validation is already implemented. Test guards regression during opener wiring.

---

### BSYS-003
**FS requirements:** FS-UX-001 (context menu hint is a one-shot UX affordance)
**Layer:** Unit (Rust)
**Priority:** Should
**Location:** `src-tauri/src/commands/system_cmds.rs`

**Preconditions:** A `PreferencesStore` backed by a temp file. The `context_menu_hint_shown` flag is `false` initially.

**Steps:**
1. Call `mark_context_menu_used()`.
2. Read the preferences file from disk.
3. Assert the file contains `"contextMenuHintShown": true`.
4. Call `mark_context_menu_used()` again.
5. Assert the second call returns `Ok(())` (idempotent).
6. Assert the file still contains `"contextMenuHintShown": true` (no data corruption on second write).

**Expected result:** `mark_context_menu_used` persists the flag to disk once. Subsequent calls are idempotent and do not corrupt preferences.

**Status:** [BLOCKED: stub]

---

### BSYS-004
**FS requirements:** FS-UX-001
**Layer:** Integration (Rust)
**Priority:** Should
**Location:** `src-tauri/tests/system_cmds_integration.rs` (new file)

**Preconditions:** A full `PreferencesStore` round-trip setup (temp file, default preferences).

**Steps:**
1. Load preferences. Assert `context_menu_hint_shown == false`.
2. Call `mark_context_menu_used()`.
3. Drop the `PreferencesStore` and reload from the temp file.
4. Assert `context_menu_hint_shown == true` after reload.

**Expected result:** The persisted flag survives a store reload — confirming actual disk persistence, not just in-memory mutation.

**Status:** [BLOCKED: stub]

---

### BSYS-005
**FS requirements:** FS-VT-090, FS-NOTIF-001
**Layer:** Unit (Rust)
**Priority:** Should
**Location:** `src-tauri/src/platform/notifications_linux.rs`

**Preconditions:** A `LinuxNotifications` instance. D-Bus is unavailable in the test environment (no session daemon).

**Steps:**
1. Call `LinuxNotifications::notify("TauTerm", "Activity in tab 2")`.
2. Assert the call returns without panic and without error (graceful no-op when D-Bus is unavailable).

**Expected result:** The notification backend degrades gracefully when D-Bus is absent. No panic, no error propagation — consistent with the documented fallback policy (§7.4 ARCHITECTURE.md).

**Note:** Testing the happy path (D-Bus available + notification delivered) requires a live D-Bus session and is deferred to integration CI with a display server.

**Status:** The no-op path can be tested now. The live D-Bus path is [BLOCKED: requires D-Bus session in CI].

---

## 8. Blocked Test Tracker

| Test ID | Blocked by | Unblocked when |
|---|---|---|
| BSSH-001 | `connect_task` drops russh Handle | PTY channel opens after `Connected` |
| BSSH-002 | SSH channel read loop not implemented | Channel read loop + `VtProcessor` wiring |
| BSSH-003 | SSH channel read loop not implemented | Channel read loop + EOF handling |
| BSSH-004 | Channel not yet opened | BSSH-001 + BSSH-002 |
| BSSH-005 | Channel not opened + build required | BSSH-004 + production build |
| BSSH-006 | `accept_host_key` is a stub | TOFU flow implemented in `ssh_prompt_cmds.rs` |
| BSSH-007 | `reject_host_key` is a stub | TOFU flow implemented |
| BSSH-008 | TOFU timeout not implemented | Timeout mechanism added to connection flow |
| BSSH-009 | `provide_credentials` is a stub + oneshot not wired | Credential oneshot pattern implemented |
| BSSH-010 | Credential prompt timeout not implemented | Timeout added to credential wait |
| BSSH-011 | `close_connection` does not clean up pending oneshot | Oneshot cleanup added to `close_connection` |
| BSSH-012 | `dismiss_ssh_algorithm_warning` is a stub | Per-pane warning state implemented |
| BSSH-013 | Stub + build required | BSSH-006 + production build |
| BSSH-014 | Stub + build required | BSSH-006/007 + production build |
| BSSH-015 | `SshManager::reconnect` is a no-op | Reconnect re-runs `connect_task` with stored credentials |
| BSSH-016 | Reconnect guard not implemented | Guard added in `reconnect` |
| BSSH-017 | Credential store not injected into reconnect path | Credential store access wired to `SshManager` |
| BSSH-018 | Stub + build required | BSSH-015 + production build |
| BCONN-001 | `save_to_disk` call missing in `save_connection` | `PreferencesStore::save_to_disk()` called after mutation |
| BCONN-002 | `save_to_disk` call missing | Same as BCONN-001 |
| BCONN-003 | `save_to_disk` call missing in `delete_connection` | `PreferencesStore::save_to_disk()` called after mutation |
| BCONN-004 | None — validation already present | Can be written now |
| BCONN-005 | `save_to_disk` missing + build required | BCONN-001 + production build |
| BSCR-001 | Scroll offset not stored in `PaneSession` | `scroll_offset: i64` field added to `PaneSession`, maintained by `scroll_pane` |
| BSCR-002 | Scroll offset not stored | Same as BSCR-001 |
| BSCR-003 | Design decision pending + offset not stored | Architect decision on FS-SB-007 policy + BSCR-001 |
| BSCR-004 | Offset not stored + build required | BSCR-001 + production build |
| BSYS-001 | `tauri-plugin-opener` call missing | Opener plugin call wired in `open_url` |
| BSYS-002 | None — validation already present | Can be written now |
| BSYS-003 | `mark_context_menu_used` is a stub | Flag persisted via `PreferencesStore` |
| BSYS-004 | Stub | Same as BSYS-003 |
| BSYS-005 | No-op path: can be written now. D-Bus path: requires CI session | No-op: immediate. Live D-Bus: integration CI |

---

## 9. Coverage Summary

| Domain | Unit (Rust) | Integration (Rust) | E2E | FS Requirements |
|---|---|---|---|---|
| SSH PTY channel (FS-SSH-013) | BSSH-001, BSSH-002, BSSH-003 | BSSH-004 | BSSH-005 | FS-SSH-013 |
| SSH TOFU flow (FS-SSH-011) | BSSH-006, BSSH-007, BSSH-008 | — | BSSH-013, BSSH-014 | FS-SSH-011 |
| SSH credential prompt (FS-SSH-012) | BSSH-009, BSSH-010, BSSH-011 | — | — | FS-SSH-012 |
| SSH algorithm warning (FS-SSH-014) | BSSH-012 | — | — | FS-SSH-014 |
| SSH reconnect (FS-SSH-040) | BSSH-015, BSSH-016, BSSH-017 | — | BSSH-018 | FS-SSH-040 |
| Connection persistence (FS-SSH-030/031) | BCONN-004 | BCONN-001, BCONN-002, BCONN-003 | BCONN-005 | FS-SSH-030, FS-SSH-031 |
| Scroll offset (FS-SB-006) | BSCR-001, BSCR-002, BSCR-003 | — | BSCR-004 | FS-SB-006, FS-SB-007 |
| `open_url` (FS-VT-071, FS-SEC-004) | BSYS-001, BSYS-002 | — | — | FS-VT-071, FS-SEC-004 |
| `mark_context_menu_used` (FS-UX-001) | BSYS-003 | BSYS-004 | — | FS-UX-001 |
| Notifications (FS-VT-090, FS-NOTIF-001) | BSYS-005 | — | — | FS-VT-090, FS-NOTIF-001 |

**Total scenarios:** 31 (18 unit, 8 integration, 5 E2E)

**Immediately executable (no stubs to implement first):** BCONN-004, BSYS-002, BSYS-005 (no-op path)

**Design decision required before implementation:** BSCR-003 (scroll-on-new-output policy — coordinate with architect)
