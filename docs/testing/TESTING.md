<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Testing Strategy

> Extracted from Architecture documentation. See [docs/arch/](../arch/README.md) for the system architecture.

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
| Rust SSH integration         | Podman + nextest | `./scripts/run-ssh-tests.sh` |
| Rust formatting | `rustfmt` | `cargo fmt -- --check` |
| Rust linting | `clippy` | `cargo clippy -- -D warnings` |
| Frontend unit | Vitest | `pnpm vitest run` |
| Frontend types | SvelteKit | `pnpm check` |
| Frontend formatting | Prettier | `pnpm prettier --check src/` |
| E2E | WebdriverIO + tauri-driver | `pnpm wdio` (after `pnpm tauri build --no-bundle -- --features e2e-testing`) |

---

### 14.2 Rust Unit Tests

#### Module suitability

| Module | Unit testable? | Notes |
|--------|---------------|-------|
| `vt/processor.rs` + `vt/processor/dispatch.rs` | Yes | Pure state transformation; tests in `vt/processor/tests.rs` |
| `vt/screen_buffer.rs` | Yes | Pure grid/scrollback data structure; separate `tests.rs` |
| `vt/cell.rs` | Yes | Value types |
| `vt/sgr.rs` | Yes | Pure parsing function; separate `tests.rs` |
| `vt/osc.rs` | Yes | Pure dispatch logic |
| `vt/modes.rs` | Yes | Flag state |
| `vt/mouse.rs` | Yes | Encoding logic is pure |
| `vt/search.rs` | Yes | Operates on ScreenBuffer snapshot; separate `tests.rs` |
| `vt/charset.rs` | Yes | DEC mapping tables |
| `session/lifecycle.rs` | Yes | State machine transitions |
| `session/ids.rs` | Yes | Newtype construction |
| `session/pty_task.rs` | Yes | PTY task state machine |
| `session/pty_task/reader.rs` | Yes | Frame-ack backpressure logic (ACK_STALE, ACK_DROP thresholds, stage transitions) |
| `session/registry.rs` | Yes | Session registry map; separate `tests.rs` |
| `ssh/known_hosts.rs` | Yes | File parsing — operates on `&str`; separate `tests.rs` |
| `ssh/algorithms.rs` | Yes | String classification — pure |
| `ssh/auth.rs` | Yes | Auth method selection |
| `ssh/connection.rs` | Yes | Connection config parsing |
| `ssh/keepalive.rs` | Yes | Keepalive interval logic |
| `ssh/manager.rs` | Yes | SSH session manager; separate `tests.rs` |
| `preferences/schema.rs` | Yes | Serde round-trip; separate `tests.rs` |
| `preferences/types.rs` | Yes | Preferences type serde |
| `preferences/store.rs` | Partial | Requires temp dir fixture; separate `tests.rs` |
| `preferences/store/validation.rs` | Yes | Store schema validation |
| `preferences/store/lock.rs` | Yes | Store file locking |
| `preferences/store/migration.rs` | Yes | Store migration logic |
| `preferences/schema/appearance.rs` | Yes | Appearance schema validation |
| `credentials.rs` | Yes | Credential struct round-trips |
| `security_static_checks.rs` | Yes | Static security assertions |
| `security_load.rs` | Yes | Input size load limits |
| `webview_data_dir.rs` | Yes | WebView data dir paths |
| `events/types.rs` | Yes | IPC event type serde |
| `commands/connection_cmds.rs` | Yes | Connection IPC command handlers |
| `commands/input_cmds.rs` | Yes | Input IPC command handlers |
| `commands/system_cmds.rs` | Yes | System IPC commands; separate `tests.rs` |
| `platform/validation.rs` | Yes | Platform input validation |
| `platform/pty_injectable.rs` | Yes | Injectable PTY stub (E2E) |
| `platform/pty_linux.rs` | Partial | Linux PTY layer; separate `tests.rs`; real PTY needed for some |
| `platform/pty_linux/session.rs` | Yes | Linux PTY session ops |
| `platform/credentials_linux.rs` | Partial | Requires running keychain for full coverage |
| `platform/clipboard_linux.rs` | Partial | Requires display for full coverage |
| `platform/notifications_linux.rs` | Yes | Linux notification dispatch |

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

#### `session/pty_task/reader.rs` — frame-ack backpressure (ADR-0027)

| Test ID | Scenario |
|---------|----------|
| TEST-ACK-001 | Ack age ≤ 200 ms → debounce unchanged (normal adaptive range) |
| TEST-ACK-002 | Ack age > 200 ms (ACK_STALE) → debounce escalated to 250 ms |
| TEST-ACK-003 | Ack age > 1000 ms (ACK_DROP) → dirty cells dropped; non-visual events (mode, cursor shape, bell, OSC 52, title, CWD) preserved |
| TEST-ACK-004 | Transition from drop mode → normal (ack resumes) → full-redraw flag set on next emission |
| TEST-ACK-005 | Backward clock jump (simulated via direct AtomicU64 write of future timestamp) → `saturating_sub` produces age 0, no escalation |
| TEST-ACK-006 | `cursor_moved` dropped in Stage 2, resynced by full-redraw on exit |

Tests construct the coalescer state directly with a mock `AtomicU64` timestamp, feed synthetic `ProcessOutput` values, and assert on the emitted events and escalation state. No real PTY or Tauri runtime required.

#### Inline vs. separate file rule

- **Inline `#[cfg(test)]` modules**: default for all unit tests.
- **Separate `tests.rs` file**: when the test module exceeds ~150 lines OR the source file exceeds ~400 lines. Declared as `#[cfg(test)] mod tests;` in the source; file at `<module>/tests.rs`. No `mod.rs` ([§3.1](../arch/02-backend-modules.md#31-file-layout-convention) convention).
- `vt/processor.rs` and `vt/screen_buffer.rs`: separate test files from the start, given the surface area.

---

### 14.3 Rust Integration Tests

Location: `src-tauri/tests/`. One file per domain or concern.

#### `vt_processor_integration.rs` — VT + session pipeline (PTY pipe)

A pipe pair replaces a real PTY for most integration tests. Scenarios:
- Write ANSI escape bytes to write end → assert screen buffer state
- Large block (> 4096 bytes) → no truncation, no partial-sequence artifacts
- Resize mid-stream → grid dimensions update
- Dirty region coalescing: two sequences in rapid succession → single `DirtyRegion`

One real PTY test: spawn `/bin/sh`, write `echo hello\n`, verify `hello` in screen buffer. Tagged `#[cfg(target_os = "linux")]` and `slow` in nextest config.

#### `ipc_command_handlers.rs` — IPC cycle integration

Uses `tauri::test::mock_app`. Scenarios:
- `create_tab` → `TabState` with leaf `PaneNode`, valid `PaneId`
- `split_pane` → `TabState.layout` is `Split` node with two `Leaf` children, distinct `PaneId`s
- `close_pane` (non-last pane) → sibling pane expanded in returned `TabState`
- `close_pane` (last pane) → `null` returned
- `rename_tab` → label updated; subsequent `get_session_state` reflects rename
- `update_preferences` with invalid value → `TauTermError { code: "PREF_INVALID_VALUE" }`

#### `ipc_type_coherence.rs` — IPC type contracts

Verifies that Rust IPC types and TypeScript types remain in sync (serde shape coherence).

#### `session_registry_topology.rs` — session registry lifecycle

SessionRegistry creation, tab/pane topology mutations, and cleanup on close.

#### `dsr_responses.rs` — Device Status Report

Validates DSR/CPR response handling through the VT pipeline.

#### `async_concurrency.rs` — concurrent session operations

Exercises concurrent tab/pane operations to catch race conditions in SessionRegistry.

#### `pty_teardown.rs` — PTY cleanup on close

Verifies that PTY resources are properly released when sessions are closed.

#### Preferences integration (3 files)

- `preferences_roundtrip.rs` — load/save/patch round-trips
- `preferences_schema_validation.rs` — schema validation against invalid payloads
- `preferences_concurrent.rs` — concurrent preference writes

#### SecretService integration tests (Podman container)

`src-tauri/tests/credentials_integration.rs` — exercises `LinuxCredentialStore` against a real GNOME Keyring daemon (SEC-CRED-INT-001 to 005). These tests cannot run in a standard CI environment because they require a live D-Bus Secret Service daemon with an unlocked default collection. They are therefore isolated in a dedicated Podman image.

**Why a custom image:** The standard Rust CI base image (slim-bookworm) has no D-Bus session bus, no GNOME Keyring, and no display server. Creating a real Secret Service session requires:
1. A D-Bus session bus (`dbus-run-session`)
2. GNOME Keyring daemon (`gnome-keyring-daemon --unlock --components=secrets`)
3. A virtual framebuffer (`Xvfb :99`) — gnome-keyring 42 activates `gcr-prompter` via D-Bus to create the initial "login" collection; `gcr-prompter` is a GTK application that requires a display even when only the virtual display is in use
4. `xdotool` to auto-dismiss the password dialog (empty password = no encryption, acceptable for ephemeral CI keyrings)

**Critical ordering constraint:** `Xvfb` and `DISPLAY` must be set *before* `dbus-run-session` is invoked. D-Bus-activated services (`gcr-prompter`) inherit the environment of `dbus-daemon`, not the calling shell. Setting `DISPLAY` after `dbus-run-session` has started means `gcr-prompter` never sees it and crashes with `cannot open display`.

**Image:** `Containerfile.keyring-test` (project root) — single-stage `rust:1.94.1-slim-bookworm`. Full Tauri Linux build dependencies are required because `tau_term_lib` (which the test binary links against) depends on `gtk`, `gio`, `webkit2gtk`, etc. at compile time. The test binary is pre-compiled during `docker build` to keep `docker run` fast.

**nextest profile:** `keyring` (defined in `src-tauri/.config/nextest.toml`) — `test-threads = 1` (tests share a single daemon; parallelism causes race conditions), `slow-timeout = 60s`, `fail-fast = false`.

**Running:**
```bash
./scripts/run-keyring-tests.sh             # build image + run
./scripts/run-keyring-tests.sh --no-build  # reuse existing image
./scripts/run-keyring-tests.sh --dry-run   # print commands only
```

These tests are **not** part of the default `cargo nextest run` gate. They are an optional step, run on-demand or in a dedicated CI job.

#### Credential regression tests (no daemon required)

`src-tauri/tests/credential_regression.rs` — covers 6 credential security scenarios without requiring a live Secret Service daemon or a real SSH server. Runs under `cargo nextest run --test credential_regression` in any standard CI environment.

| Test ID       | FS reference | Scenario                                                  |
|---------------|--------------|-----------------------------------------------------------|
| SEC-CRED-001  | FS-CRED-001  | `SshConnectionConfig` serializes no password field        |
| SEC-CRED-002  | FS-CRED-003  | `Credentials` implements `ZeroizeOnDrop` (compile-time)   |
| SEC-CRED-003  | FS-CRED-004  | `Credentials` `Debug` impl redacts password value         |
| SEC-CRED-004  | FS-CRED-002  | `SshConnectionConfig` holds identity path only, not key content |
| SEC-CRED-005  | FS-CRED-005  | Unavailable store → `Err(Unavailable)`, no disk fallback  |
| SEC-CRED-006  | FS-SSH-011   | `KnownHostsStore::lookup()` returns `Mismatch` on key change |

### `credential_regression.rs` merge criteria

`src-tauri/tests/credential_regression.rs` MUST pass via `cargo nextest run --test credential_regression` (no Podman, no live Secret Service daemon required) before any merge touching any of:
- `src-tauri/src/credentials.rs`
- `src-tauri/src/ssh/manager.rs`
- `src-tauri/src/commands/ssh_prompt_cmds.rs`

#### SSH integration tests (Podman container)

`src-tauri/tests/ssh_integration.rs` — exercises `russh` auth functions (`authenticate_password`, `authenticate_pubkey`, `authenticate_keyboard_interactive`) and `KnownHostsStore` TOFU lookups against a real OpenSSH server (SSH-INT-001 to SSH-INT-012). These tests cannot run in a standard CI environment without a live sshd process.

**Why a custom image:** The test container runs `sshd` on port 2222 with a pre-configured test user (`tauterm`), a known password, and a pre-authorized ED25519 key pair. A second user (`tauterm-noauth`) has no valid credentials, allowing auth-failure tests without configuring PAM policy.

**Image:** `Containerfile.ssh-test` (project root) — single-stage `rust:1.94.1-slim-bookworm` with `openssh-server`. The test binary is pre-compiled during image build.

**Key materials:** The ED25519 key pair is generated at image build time and embedded in the image at `/root/.ssh-test-keys/`. The private key path is exported as `TAUTERM_TEST_PUBKEY_PATH`. These are ephemeral test credentials only — never used in production.

**nextest profile:** `ssh` (defined in `src-tauri/.config/nextest.toml`) — `test-threads = 4`, `slow-timeout = 30s`, `fail-fast = false`.

**Running:**
```bash
./scripts/run-ssh-tests.sh             # build image + run
./scripts/run-ssh-tests.sh --no-build  # reuse existing image
./scripts/run-ssh-tests.sh --dry-run   # print commands only
```

**Coverage:**

| Test ID     | FS reference | Scenario                                                    |
|-------------|--------------|-------------------------------------------------------------|
| SSH-INT-001 | FS-SSH-012   | Password auth succeeds                                      |
| SSH-INT-002 | FS-SSH-012   | Pubkey (ED25519) auth succeeds                              |
| SSH-INT-003 | FS-SSH-012   | Keyboard-interactive auth succeeds                          |
| SSH-INT-004 | FS-SSH-012   | Wrong password → Ok(false), not Err                        |
| SSH-INT-005 | FS-SSH-011   | First connection → `Unknown` TOFU trigger                  |
| SSH-INT-006 | FS-SSH-011   | Trusted host key → connection accepted                      |
| SSH-INT-007 | FS-SSH-011   | Mismatched key → `Mismatch`, connection rejected            |
| SSH-INT-008 | FS-SSH-011   | Accept host key, store, reconnect succeeds                  |
| SSH-INT-009 | FS-CRED-006  | Path traversal in identity_file → Err                      |
| SSH-INT-010 | FS-CRED-006  | Directory path as identity_file → Err                      |
| SSH-INT-011 | FS-SSH-020   | Keepalive constants: 30 s interval, 3 max misses            |
| SSH-INT-012 | FS-SSH-013   | PTY request with `xterm-256color` + RFC 4254 modes succeeds |

These tests are **not** part of the default `cargo nextest run` gate. They are an optional step, run on-demand or in a dedicated CI job.

#### Isolation rules

- Temp directories via `tempfile::TempDir` for all filesystem-touching tests
- `SessionRegistry::new()` and `PreferencesStore::load(path)` receive injected paths — no hardcoded `~/.config/tauterm/`
- No port binding in integration tests
- nextest process isolation by default; shared mutable state within a binary → test-scoped `Mutex`
- SecretService integration tests run single-threaded in the `keyring` nextest profile; each test uses a unique attribute key (`tauterm:integration-test:<name>`) and a RAII `Cleanup` guard that deletes the key in `Drop`, preventing keyring pollution across test runs

---

### 14.4 VT Conformance Tests

VT conformance tests are organized as inline unit tests within the `VtProcessor`
implementation, grouped into thematic modules under
`src-tauri/src/vt/processor/tests/`:

| Module | Coverage |
|--------|----------|
| `basic.rs` | Basic character output, UTF-8 handling (FS-VT-010–016) |
| `editing.rs` (+ `editing/` submodules) | Character insert/delete, cursor movement, line operations, text composition, wrap mode, scrolling, reverse index |
| `features.rs` | SGR attributes (FS-VT-020–025), OSC 52 policy (FS-VT-075–076), bell (FS-VT-090–092) |
| `modes.rs` | Terminal mode flags: DECCKM, DECKPAM, mouse, bracketed paste, focus events (FS-VT-080–086) |
| `security.rs` | Terminal injection prevention: read-back sequence rejection (FS-VT-063), OSC 52 read rejection (FS-VT-076) |
| `cursor_dirty.rs` | Cursor dirty-tracking: cursor position updates trigger dirty flags correctly |
| `resize_full_redraw.rs` | Full-redraw flag on resize and alternate screen switch |

Each test function is named after its FS requirement where applicable (e.g.
`test_sgr_truecolor_semicolon`, `osc52_write_blocked_by_default_policy`).
Tests construct a `VtProcessor` directly, feed it byte sequences via `process()`,
and assert the resulting screen buffer state and side-effect flags.

All VT conformance tests are included in the standard `cargo nextest run` invocation
(they are inline unit tests, not integration tests in `src-tauri/tests/`).
No external fixture files or data-driven runner framework is used; sequences are
inline Rust byte literals.

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
| `lib/terminal/selection.ts` | Selection state machine transitions |
| `lib/terminal/keyboard.ts` | Keydown → byte encoding |
| `lib/terminal/mouse.ts` | Mouse event routing decision (PTY vs TauTerm UI); X10/SGR-1006/URXVT-1015 encoding given button, modifiers, row, col; mode-to-encoding arbitration |
| `lib/terminal/color.ts` | Color index → CSS token mapping |
| `lib/terminal/screen.ts` | Screen buffer diffing and snapshot handling |
| `lib/terminal/paste.ts` | Paste handling and confirmation logic |
| `lib/terminal/cell-dimensions.ts` | Cell dimension calculations |
| `lib/theming/validate.ts` | Token presence, contrast ratio enforcement |
| `lib/theming/apply-theme.ts` | Theme application |
| `lib/theming/built-in-themes.ts` | Built-in theme integrity |
| `lib/preferences/shortcuts.ts` | Conflict detection, key combo normalization |
| `lib/preferences/applyUpdate.ts` | Preference update application |
| `lib/layout/split-tree.ts` | `buildFromPaneNode()`, `updateRatio()`, `findLeaf()` |
| `lib/state/session.svelte.ts` | Delta merge, `getPane()` traversal |
| `lib/state/locale.svelte.ts` | `setLocale()` writes to preferences via IPC; `getLocale()` returns current locale; unknown locale code from backend defaults to `"en"` (FS-I18N-006) |
| `lib/state/fullscreen.svelte.ts` | Fullscreen state management |
| `lib/state/notifications.svelte.ts` | Notification state management |
| `lib/utils/tab-title.ts` | `resolveTabTitle()` priority chain (user label > OSC 0/2 title > OSC 7 CWD basename); `getRootPane()` tree traversal (leaf extraction from split tree) |
| `lib/ipc/commands.ts` | Correct command name and parameter shape passed to `invoke()`; `TauTermError` propagated as thrown value; each wrapper calls the right command string |
| `lib/ipc/types.ts` | IPC type shape validation |
| `lib/ipc/ssh-events.ts` | SSH event listener registration and dispatch |
| `lib/ipc/osc-title.ts` | OSC title event handling |
| `lib/ipc/notification-events.ts` | Notification event listeners |
| `lib/i18n/catalogue-parity` | en/fr catalogue key parity |
| `lib/terminal/frame-ack.ts` | Frame-ack IPC call timing and rAF integration (ADR-0027) |

#### `lib/terminal/frame-ack.ts` detail (ADR-0027)

| Test ID | Scenario |
|---------|----------|
| ACK-FE-001 | `flushRafQueue()` completion triggers exactly one `frame_ack` invoke per pane with correct `paneId` |
| ACK-FE-002 | No `frame_ack` call when rAF queue is empty (no pending screen-update for the pane) |
| ACK-FE-003 | Multiple panes active → each pane receives its own `frame_ack` call (no cross-pane ack) |
| ACK-FE-004 | `frame_ack` invoke rejection (e.g., pane closed) → error is silently ignored, no unhandled rejection |
| ACK-FE-005 | Pane unmount → no further `frame_ack` calls for that pane ID |

Mock setup: `vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))`. Tests exercise the rAF queue flush callback, asserting that `invoke('frame_ack', { paneId })` is called with correct timing and pane isolation.

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

**Verdict: criterion 3 not met → no component test file.** The one non-extractable behavior (blink timer lifecycle) is too thin to warrant a full component test setup. Backend correctness for DECSCUSR shapes is covered by VT conformance tests (§14.4).

#### `KeyboardShortcutRecorder`

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

Require a release build with the E2E feature flag: `pnpm tauri build --no-bundle -- --features e2e-testing`. Run on merge to `dev`; gate for promotion `dev` → `main`.

> **`e2e-testing` feature:** enables `InjectablePtyBackend` (replaces the real PTY with an in-process mpsc channel) and registers the `inject_pty_output` Tauri command, which allows tests to push synthetic bytes directly into the VT pipeline. Without this flag the command is absent, injections are silently dropped, and PTY round-trip tests fail.

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
| OSC 0 sequence sets active tab title (TEST-OSC-E2E-001) | FS-VT-060, FS-TAB-006 |
| Successive OSC 0 sequences — last title wins in DOM (TEST-OSC-E2E-002) | FS-VT-060 |
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

One `cargo-fuzz` target in `src-tauri/fuzz/fuzz_targets/` (cargo-fuzz convention: adjacent to the target crate):

- `vt_processor.rs`: 80×24 processor, arbitrary bytes → no panic, no unbounded allocation

Fuzzing is not in the `nextest run` gate. Runs:
- Manually before declaring VT feature complete (`-max_total_time=300`)
- Weekly CI scheduled job (10 min/target)
- Any crash → minimized reproducer → deterministic nextest regression test before fix

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

**Governing rule:** tests live as close as possible to the code they test, except when they cross module boundaries. Do not maintain an exhaustive file inventory here — use `find` or `tree` for that. This section defines **conventions** so new tests land in the right place.

#### Placement rules

| Layer | Location | Example |
|-------|----------|---------|
| Rust unit (small) | Inline `#[cfg(test)] mod tests { }` in source file | `vt/cell.rs`, `session/lifecycle.rs` |
| Rust unit (large) | Separate `tests.rs` file when module exceeds ~150 lines or source exceeds ~400 lines. Declared as `#[cfg(test)] mod tests;`; file at `<module>/tests.rs` | `vt/processor/tests.rs`, `vt/screen_buffer/tests.rs` |
| Rust VT conformance | Thematic sub-modules under `vt/processor/tests/` | `tests/basic.rs`, `tests/security.rs`, `tests/modes.rs` |
| Rust integration | `src-tauri/tests/<domain>.rs` — one file per domain or concern | `vt_processor_integration.rs`, `session_registry_topology.rs` |
| Rust containerized | `src-tauri/tests/` — run via dedicated Podman scripts, not `cargo nextest run` | `credentials_integration.rs`, `ssh_integration.rs` |
| Rust fuzz | `src-tauri/fuzz/fuzz_targets/` | `vt_processor.rs` |
| Frontend unit (TS) | Co-located `<module>.test.ts` | `lib/terminal/keyboard.test.ts` |
| Frontend component | `__tests__/` directory alongside the components | `lib/components/__tests__/SearchOverlay.test.ts` |
| Frontend composable | `__tests__/` directory alongside the composables | `lib/composables/__tests__/cursorBlink.test.ts` |
| E2E | `tests/e2e/<scenario>.spec.ts` — flat, one file per scenario | `tab-lifecycle.spec.ts`, `ssh-reconnect.spec.ts` |
| E2E helpers | `tests/e2e/helpers/` | `helpers/selectors.ts` |

#### Naming conventions

| Scope | Suffix |
|-------|--------|
| Rust unit/integration | Standard `#[test]` / `#[cfg(test)]` — no file suffix convention |
| Frontend unit + component | `.test.ts` |
| E2E | `.spec.ts` |

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

*This document is maintained by the TauTerm software architect. Every structural change to the test strategy, pyramid rationale, coverage targets, or CI gate definition requires updating this document.*
