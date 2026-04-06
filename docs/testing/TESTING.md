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
- **Separate `tests.rs` file**: when the test module exceeds ~150 lines OR the source file exceeds ~400 lines. Declared as `#[cfg(test)] mod tests;` in the source; file at `<module>/tests.rs`. No `mod.rs` ([§3.1](../arch/02-backend-modules.md#31-file-layout-convention) convention).
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

*This document is maintained by the TauTerm software architect. Every structural change to the test strategy, pyramid rationale, coverage targets, or CI gate definition requires updating this document.*
