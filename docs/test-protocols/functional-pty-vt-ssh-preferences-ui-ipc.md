# Test Protocol — TauTerm

> **Document status:** Initial revision — 2026-04-04
> **Author:** test-engineer
> **Based on:** FS.md (all sections), ARCHITECTURE.md §3–§8, docs/testing/TESTING.md §14, UXD.md §5–§7
> **Bootstrap state:** 50 Rust unit tests passing, 24 frontend unit tests passing, E2E not yet runnable (requires production build)

---

## 1. Strategy & Scope

### 1.1 Objectives

This protocol defines what is tested, how it is tested, and the standard that must be met before any feature is considered done. It is the authoritative reference for all test work on TauTerm v1.

### 1.2 Test Pyramid Rationale

TauTerm is a native application with a Rust backend (PTY, VT parser, state machines) and a Svelte frontend (rendering, IPC client). The test distribution follows the classic pyramid:

- **Unit tests (majority):** Fast, deterministic, no external dependencies. Cover all pure logic: VT parser state machine, screen buffer operations, SGR parsing, preferences schema validation, keyboard encoding, selection logic, design token application.
- **Integration tests (middle):** Cross-module behavior without a real PTY or SSH server. Cover IPC contract (command → state → event round-trips), preferences load/save, session registry topology.
- **E2E tests (minority):** A running TauTerm application driven by WebdriverIO via tauri-driver. Cover user-visible acceptance criteria that require the full stack.

### 1.3 Scope

| In scope | Out of scope |
|---|---|
| All FS requirements marked **Must** | Windows and macOS platform paths (stubs only in v1) |
| All FS requirements marked **Should** (tested where implementable) | Plugin or extension system (out of scope per FS §4) |
| Security test cases derived from FS-SEC-* and FS-VT-063/075/076 | Cloud sync |
| WCAG 2.1 AA accessibility verification for all UI chrome | Kitty keyboard protocol (v1 constraint) |
| AppImage smoke test on each target architecture | Session persistence across restarts |

### 1.4 Blocked Tests

Tests marked **[BLOCKED: stub]** cannot be executed until the corresponding implementation replaces the current `todo!()` stubs. These tests must be tracked as open items and unblocked as features land. They are not skipped — they are pending implementation.

Stub state as of bootstrap (2026-04-04):

| Component | Stub nature | Unblocked when… |
|---|---|---|
| `LinuxPtySession::write/resize` | `todo!()` | PTY I/O implemented |
| `SshManager` auth, known_hosts, keepalive | Full stub | SSH lifecycle implemented |
| `CredentialStore::is_available()` = false | Secret Service disabled | Platform integration completed |
| Screen buffer → event pipeline | No real event emission | PtyReadTask implemented |
| `copy_selection`, `paste_to_pane`, `set_locale`, `get_locale` | IPC not registered | Commands implemented |
| E2E test suite | Requires `pnpm tauri build` + `pnpm wdio` | Production build functional |

### 1.5 No-Regression Policy

- All tests in the passing set must remain green at every commit.
- A test that becomes flaky is treated as a bug. It is not skipped or retried silently — it is investigated and fixed.
- New features must not break existing passing tests. This is a merge blocker.
- Test sign-off by the test-engineer is required before a feature is considered complete.

---

## 2. Test Layers

### 2.1 Unit Tests — Rust (nextest)

**Runner:** `cargo nextest run` exclusively. `cargo test` is prohibited.

**Location:** `src-tauri/src/` — inline `#[cfg(test)]` modules within each module file.

**Coverage targets:**
- `vt/` module: all VT parsing state machine transitions, SGR attributes, OSC dispatch, mouse encoding, character width, screen buffer operations.
- `session/` module: ID generation, lifecycle state machine transitions, pane tree topology operations.
- `preferences/` module: schema validation, patch application, load/save round-trips.
- `ssh/` module: connection state machine transitions, known-hosts parse/lookup (unit), keepalive timeout logic.
- `platform/` module: unit tests for path validation, URI scheme validation.
- `commands/` module: input validation at the IPC boundary (not full command invocation).

**Async tests:** `#[tokio::test]` for any test involving Tokio primitives (debounce, async tasks).

**Conventions:**
- Each test file has a `#[cfg(test)] mod tests { ... }` at the bottom of the source file.
- Test helper fixtures (e.g., `make_screen_buffer(80, 24)`) are private helpers within the `tests` module.
- No `unwrap()` in test assertions — use `assert_eq!`, `assert_matches!`, or explicit `Result` unwrapping with context.

### 2.2 Unit Tests — Frontend (vitest)

**Runner:** `pnpm vitest`.

**Location:** `src/` — colocated `*.test.ts` files alongside the modules under test.

**Coverage targets:**
- `lib/ipc/types.ts`: type shape validation (already partially covered in bootstrap).
- `lib/state/locale.svelte.ts`: reactive locale switching, persistence calls (already partially covered).
- `lib/terminal/keyboard.ts`: key encoding logic for all FS-KBD-004 through FS-KBD-012 sequences.
- `lib/terminal/selection.ts`: word boundary detection with default and custom delimiter sets.
- `lib/terminal/grid.ts`: `applyDiff()` and `applySnapshot()` with synthetic payloads.
- `lib/terminal/hyperlinks.ts`: URI scheme validation (whitelist, length limit, control characters).
- `lib/preferences/contrast.ts`: WCAG contrast ratio calculations.
- `lib/preferences/memory-estimate.ts`: line-count to MB estimation.
- `lib/preferences/shortcuts.ts`: conflict detection, key combo normalization.
- `lib/theming/validate.ts`: theme token validation, required fields, CSS.supports checks.
- `lib/layout/split-tree.ts`: tree construction, ratio update, leaf lookup.

**Tauri mocking:** `invoke()` and `listen()` are mocked via vitest's module mock system. Tests do not require a running Tauri process.

### 2.3 Integration Tests

**Runner:** `cargo nextest run` (Rust integration tests in `src-tauri/tests/`).

**Location:** `src-tauri/tests/` — one file per integration domain.

**Coverage targets:**
- `preferences_roundtrip.rs`: write preferences to a temp file, load them back, verify all fields survive the round-trip.
- `preferences_schema_validation.rs`: load a preferences file with out-of-range or invalid values, verify defaults are applied field-by-field.
- `session_registry_topology.rs`: create tabs, split panes, close panes, verify the `PaneNode` tree structure after each mutation.
- `vt_processor_integration.rs`: feed multi-read byte sequences (split across boundaries) to a `VtProcessor`, verify screen state. Does not require a real PTY.
- `ipc_type_coherence.rs`: verify that Rust `serde` serialization of all IPC types produces JSON structures that match the expected TypeScript shape (key names, nesting).

These tests run in CI without a display server or D-Bus session.

### 2.4 E2E Tests — WebdriverIO + tauri-driver

**Runner:** `pnpm wdio` (after `pnpm tauri build`).

**Location:** `tests/` — one file per functional domain.

**Prerequisites:** A production build (`pnpm tauri build`) must exist. E2E tests do not run in unit CI — they run in a separate E2E CI job that first builds the app.

**Strategy:**
- Each E2E scenario drives the full TauTerm application via tauri-driver.
- A mock SSH server (in-process, based on `russh` or `ssh2`) is started as a test fixture for SSH E2E scenarios. It listens on a localhost port and accepts a known test key.
- Scenarios use `waitForDisplayed` and `waitUntil` for async UI updates. No fixed `sleep()` calls are permitted.
- Element selection uses `data-testid` attributes where ARIA roles are insufficient.

**Coverage targets:**
- PTY session open/close lifecycle, tab and pane management, keyboard shortcuts, clipboard, search, preferences persistence, SSH connection flow, accessibility keyboard navigation.

### 2.4.1 E2E Harness Validation — CI Readiness Checklist

Before any E2E test results are considered authoritative, the following checklist must pass:

| Check | How to verify | Required for |
|---|---|---|
| `pnpm tauri build` exits 0 | CI build job log | All E2E scenarios |
| `tau-term` binary exists at `src-tauri/target/debug/tau-term` or `release/tau-term` | File exists check in CI | Tauri-driver startup |
| `tauri-driver` is installed and callable | `~/.cargo/bin/tauri-driver --version` | WebdriverIO session |
| WebKitWebDriver is installed | `WebKitWebDriver --version` | WebdriverIO capabilities |
| X11 or Wayland display server is running | `echo $DISPLAY` or `echo $WAYLAND_DISPLAY` non-empty | App rendering |
| D-Bus session is active | `echo $DBUS_SESSION_BUS_ADDRESS` non-empty | App launch |
| `pnpm wdio` exits with all tests in `passing` or `pending` state | Exit code 0 | CI gate |

**Pass-rate threshold:** ≥ 95% of non-pending E2E scenarios must pass in a qualifying CI run. A scenario marked `pending` due to a stub dependency does not count against the pass rate.

**Timeout expectations:**
- Single scenario timeout: 60 seconds (`mochaOpts.timeout = 60_000` in `wdio.conf.ts`).
- Session connection retry: 3 attempts × 30 second timeout = 90 seconds maximum before hard failure.
- Full suite timeout: no global limit per suite, but individual scenario timeouts prevent indefinite hangs.
- A scenario that exceeds its 60-second timeout is counted as a failure and contributes to pass-rate calculation.

**Deferred execution note:** E2E scenarios are written and reviewed in the test-protocols and `tests/e2e/` directory but are marked `[DEFERRED: build required]` until `pnpm tauri build` succeeds in CI. Written scenarios count toward coverage planning but not toward the pass-rate until the build gate is green.

---

## 3. Coverage Matrix

| Feature Domain | Unit (Rust) | Unit (Frontend) | Integration | E2E | FS Requirements |
|---|---|---|---|---|---|
| VT emulation — target standard | vt/processor, vt/modes | — | vt_processor_integration | — | FS-VT-001–005 |
| VT — character sets | vt/charset, vt/screen_buffer | — | vt_processor_integration | E2E-VT | FS-VT-010–016 |
| VT — ANSI colors & SGR | vt/sgr, vt/cell | — | vt_processor_integration | E2E-VT | FS-VT-020–025 |
| VT — cursor | vt/processor, vt/modes | terminal/grid.ts | — | E2E-VT | FS-VT-030–034 |
| VT — screen modes | vt/processor, vt/screen_buffer | — | vt_processor_integration | E2E-VT | FS-VT-040–044 |
| VT — scrolling regions | vt/screen_buffer | — | vt_processor_integration | — | FS-VT-050–054 |
| VT — OSC title | vt/osc | — | — | E2E-TAB | FS-VT-060–063 |
| VT — hyperlinks | vt/osc, platform/ | terminal/hyperlinks.ts | — | E2E-VT | FS-VT-070–073 |
| VT — OSC 52 clipboard | vt/osc | — | — | E2E-SEC | FS-VT-075–076 |
| VT — mouse reporting | vt/mouse | terminal/mouse.ts | — | E2E-VT | FS-VT-080–086 |
| VT — bell | vt/processor | — | — | E2E-NOTIF | FS-VT-090–093 |
| PTY lifecycle | session/lifecycle | — | session_registry_topology | E2E-PTY | FS-PTY-001–014 |
| Multi-tab management | session/registry | — | session_registry_topology | E2E-TAB | FS-TAB-001–008 |
| Multi-pane (split view) | session/registry | layout/split-tree.ts | session_registry_topology | E2E-PANE | FS-PANE-001–006 |
| Keyboard input | — | terminal/keyboard.ts | — | E2E-KBD | FS-KBD-001–012 |
| Clipboard integration | — | terminal/selection.ts | — | E2E-CLIP | FS-CLIP-001–009 |
| Scrollback buffer | vt/screen_buffer | — | vt_processor_integration | E2E-SB | FS-SB-001–008 |
| Search in output | vt/search | — | vt_processor_integration | E2E-SEARCH | FS-SEARCH-001–007 |
| Activity notifications | — | state/notifications | — | E2E-NOTIF | FS-NOTIF-001–005 |
| SSH lifecycle | ssh/connection, ssh/manager | — | — | E2E-SSH | FS-SSH-001–003, 010–014 |
| SSH health | ssh/keepalive | — | — | E2E-SSH | FS-SSH-020–022 |
| SSH saved connections | preferences/schema | — | preferences_roundtrip | E2E-SSH | FS-SSH-030–034 |
| SSH reconnection | ssh/connection | — | — | E2E-SSH | FS-SSH-040–042 |
| Credential security | credentials.rs | — | — | E2E-SEC | FS-CRED-001–006 |
| Theming | — | theming/validate.ts, theming/tokens.ts | — | E2E-THEME | FS-THEME-001–010 |
| User preferences | preferences/schema, preferences/store | — | preferences_roundtrip, preferences_schema_validation | E2E-PREF | FS-PREF-001–006 |
| Accessibility | — | preferences/contrast.ts | — | E2E-A11Y | FS-A11Y-001–007 |
| UX cross-cutting | — | — | — | E2E-UX | FS-UX-001–002 |
| Security hardening | vt/osc, platform/ | theming/validate.ts | ipc_type_coherence | E2E-SEC | FS-SEC-001–005 |
| i18n | preferences/schema (Language enum) | state/locale.svelte.ts | preferences_roundtrip | E2E-I18N | FS-I18N-001–007 |
| Distribution | — | — | — | E2E-DIST (smoke) | FS-DIST-001–006 |
| Login shell (first tab) | session/lifecycle | — | — | TEST-SPRINT-001 | FS-PTY-013 |
| X11 PRIMARY selection | — | terminal/selection.ts | — | TEST-SPRINT-002 | FS-CLIP-004 |
| Double-click / triple-click select | — | terminal/selection.ts | — | TEST-SPRINT-003 | FS-CLIP-002, FS-CLIP-003 |
| Close confirmation dialog | session/registry | — | — | TEST-SPRINT-004 | FS-PTY-007, FS-PTY-008 |
| Tab inline rename | — | — | — | TEST-SPRINT-005 | FS-TAB-006 |
| Tab drag-and-drop | session/registry | — | session_registry_topology | TEST-SPRINT-006 | FS-TAB-005 |
| Pane shortcuts intercept | — | terminal/keyboard.ts | — | TEST-SPRINT-007 | FS-KBD-003 |
| Shortcuts persistence | preferences/schema | preferences/shortcuts.ts | preferences_roundtrip | TEST-SPRINT-008 | FS-KBD-002 |
| Cursor shape / bell type / blink rate | vt/processor, vt/modes | — | — | TEST-SPRINT-009 | FS-PREF-003, FS-PREF-006 |
| ConnectionManager UI | — | — | — | TEST-SPRINT-010 | FS-SSH-031, FS-SSH-032 |
| Theme editor UI | — | theming/validate.ts | preferences_roundtrip | TEST-SPRINT-011 | FS-THEME-003–006 |
| IPC type coherence (Language, BellType, UserTheme) | preferences/schema | lib/ipc/types.ts | ipc_type_coherence | TEST-SPRINT-012 | FS-I18N-006, FS-PREF-006 |
| Split layout arborescent + draggable dividers | session/registry | layout/split-tree.ts | session_registry_topology | TEST-SPRINT-013 | FS-PANE-001, FS-PANE-003 |

---

## 4. Test Scenarios — by Functional Domain

### 4.1 PTY & Session Management

---

#### TEST-PTY-001
**FS requirements:** FS-PTY-001, FS-PTY-002
**Layer:** Integration
**Priority:** Must

**Preconditions:** Session registry initialized with a mock PTY backend.

**Steps:**
1. Call `SessionRegistry::create_tab(config)` with a local PTY config.
2. Inspect the returned `TabState`.

**Expected result:** `TabState` contains exactly one `PaneNode::Leaf`. The `PaneId` is a valid UUID. The PTY was allocated (mock records the allocation call). The child process was spawned with the slave PTY as controlling terminal.

---

#### TEST-PTY-002
**FS requirements:** FS-PTY-003, FS-PTY-004
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `PtySession` is opened via the Linux PTY backend.

**Steps:**
1. Obtain the master fd from the `PtySession`.
2. Verify via `fcntl(F_GETFL)` that `O_NONBLOCK` is set on the master fd.
3. Verify via `fcntl(F_GETFD)` that `FD_CLOEXEC` is set (FS-SEC-002).

**Expected result:** Both flags are present. The slave fd does not appear in `PtySession`'s owned descriptors after the child has been forked.

**Note:** [BLOCKED: stub] — requires `LinuxPtySession` to be fully implemented.

---

#### TEST-PTY-003
**FS requirements:** FS-PTY-005, FS-PTY-006
**Layer:** Integration
**Priority:** Must

**Preconditions:** A pane is in `Running` state.

**Steps:**
1. Simulate SIGCHLD delivery (mock exit with code 0).
2. Call `SessionRegistry::get_state_snapshot()`.

**Expected result:** The pane's `PaneLifecycleState` is `Terminated { exit_code: 0 }`. The tab still exists. The `SessionStateChanged` event was emitted with `changeType: "pane-metadata-changed"`.

---

#### TEST-PTY-004
**FS requirements:** FS-PTY-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open with one active terminal pane.

**Steps:**
1. In the terminal, run `exit`.
2. Wait for the pane to display the terminated state overlay.
3. Observe the overlay for available actions.

**Expected result:** The pane shows the exit code (0). Two actions are visible: "Restart" and "Close". The pane does not auto-close. Clicking "Restart" opens a new shell in the same pane.

---

#### TEST-PTY-005
**FS requirements:** FS-PTY-007, FS-PTY-008
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open with one pane running `sleep 3600`.

**Steps:**
1. Press Ctrl+Shift+W (close-tab shortcut).
2. Observe whether a confirmation dialog appears.

**Expected result:** A confirmation dialog is displayed indicating that a process is running. Clicking "Cancel" keeps the tab open. Clicking "Close anyway" closes the tab and terminates the process.

**Note:** [BLOCKED: stub] — requires PTY write/process-detection to be implemented.

---

#### TEST-PTY-006
**FS requirements:** FS-PTY-009, FS-PTY-010
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with vim running in a pane.

**Steps:**
1. Resize the TauTerm window by dragging its border to a smaller size.
2. Observe vim's display.

**Expected result:** Vim redraws at the new terminal dimensions. The `resize_pane` IPC command was invoked with correct `cols`, `rows`, `pixel_width`, `pixel_height`. Resize events were debounced (only the final size was applied). Vim's status line appears at the correct bottom row.

**Note:** [BLOCKED: stub] — requires PTY resize to be implemented.

---

#### TEST-PTY-007
**FS requirements:** FS-PTY-011, FS-PTY-012
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with a fresh terminal pane.

**Steps:**
1. Run `echo $TERM` in the terminal.
2. Run `echo $COLORTERM`.
3. Run `echo $TERM_PROGRAM`.
4. Run `echo $TERM_PROGRAM_VERSION`.
5. Run `echo $DISPLAY` (or `echo $WAYLAND_DISPLAY`).

**Expected result:** `$TERM` = `xterm-256color`. `$COLORTERM` = `truecolor`. `$TERM_PROGRAM` = `TauTerm`. `$TERM_PROGRAM_VERSION` contains the application version. `$DISPLAY` (or `$WAYLAND_DISPLAY`) contains the value from the parent environment.

**Note:** [BLOCKED: stub] — requires PTY spawn to be implemented.

---

#### TEST-PTY-008
**FS requirements:** FS-PTY-013, FS-PTY-014
**Layer:** E2E
**Priority:** Must

**Preconditions:** User's `$SHELL` is set to a valid interactive shell (e.g., `/bin/bash`).

**Steps:**
1. Open TauTerm. Check if the initial tab launches a login shell (e.g., `~/.bash_profile` is sourced).
2. Open a second tab. Verify it launches an interactive non-login shell.
3. Set `SHELL=/nonexistent` before launching TauTerm. Verify the fallback.

**Expected result:** First tab: login shell detected (e.g., `~/.bash_profile` sourced). Second tab: interactive non-login shell (profile not re-sourced). With invalid `$SHELL`: TauTerm launches `/bin/sh` without crashing.

**Note:** [BLOCKED: stub] — requires PTY spawn to be implemented.

---

#### TEST-PTY-009
**FS requirements:** FS-SEC-003 (path validation)
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** none.

**Steps:**
1. Pass an identity file path of `../../etc/shadow` to the path validation logic.
2. Pass a path containing a null byte.
3. Pass a relative path `~/.ssh/id_ed25519`.
4. Pass a valid absolute path to an existing regular file.

**Expected result:** Cases 1, 2 are rejected with a clear error. Case 3 is resolved to an absolute path. Case 4 passes validation.

---

### 4.2 VT Parser & Screen Buffer

---

#### TEST-VT-001
**FS requirements:** FS-VT-001, FS-VT-002
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` is initialized.

**Steps:**
1. Create a new `PtySession` configuration.
2. Inspect the environment variable map.

**Expected result:** `TERM = xterm-256color` and `COLORTERM = truecolor` are present in every new PTY session's environment.

---

#### TEST-VT-002
**FS requirements:** FS-VT-005
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with a 80×24 screen buffer.

**Steps:**
1. Feed a CSI sequence split across two byte chunks: first chunk contains `\x1b[` (incomplete), second chunk contains `31m` (rest of the sequence).
2. Feed bytes to `VtProcessor::process()` in two calls.
3. Inspect the cell at position (0, 0) after feeding `A` as a printable character.

**Expected result:** The cell at (0, 0) has SGR foreground color = ANSI red (31). No partial sequence fragment remains. Subsequent output is not corrupted.

---

#### TEST-VT-003
**FS requirements:** FS-VT-010
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed a 3-byte UTF-8 sequence (e.g., `é` = `0xC3 0xA9`) split as `[0xC3]` in the first call and `[0xA9, 'X']` in the second.
2. Inspect the resulting cells.

**Expected result:** The first cell contains the full `é` character. `X` is in the second cell. No U+FFFD replacement character appears.

---

#### TEST-VT-004
**FS requirements:** FS-VT-011
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with a 4-column wide screen buffer (to test edge wrapping).

**Steps:**
1. Position the cursor at column 3 (last column, 0-indexed).
2. Feed a CJK wide character (e.g., `中`, 3-byte UTF-8, width 2).

**Expected result:** The wide character wraps to the next line. Column 3 of the first row is marked as a wide-character placeholder or empty cell. The wide character occupies columns 0–1 of the second row.

---

#### TEST-VT-005
**FS requirements:** FS-VT-016
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed the invalid UTF-8 sequence `[0xC0, 0xAF]` (overlong encoding for `/`).

**Expected result:** The cell contains U+FFFD (REPLACEMENT CHARACTER). No panic. No crash. Subsequent valid UTF-8 continues to parse correctly.

---

#### TEST-VT-006
**FS requirements:** FS-VT-020, FS-VT-021, FS-VT-022
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed `\x1b[31mA` → verify cell A has ANSI red foreground.
2. Feed `\x1b[38;5;196mB` → verify cell B has 256-color index 196.
3. Feed `\x1b[38;2;255;100;0mC` → verify cell C has RGB truecolor (255, 100, 0).
4. Feed `\x1b[38:2:255:100:0mD` (colon ITU T.416 variant) → verify cell D has the same RGB.

**Expected result:** Each cell has the expected `Color` variant: `Ansi16(Red)`, `Ansi256(196)`, `Rgb(255,100,0)`, `Rgb(255,100,0)` respectively.

---

#### TEST-VT-007
**FS requirements:** FS-VT-024
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed `\x1b[1;3;4m` (bold + italic + underline).
2. Verify all three attributes are set.
3. Feed `\x1b[22m` (reset bold/dim).
4. Verify bold is cleared, italic and underline remain.
5. Feed `\x1b[0m` (SGR reset).
6. Verify all attributes are cleared.

**Expected result:** Attributes are independent. Resetting one does not affect others. SGR 0 clears all.

---

#### TEST-VT-008
**FS requirements:** FS-VT-030, FS-VT-031
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed `\x1b[2 q` (CSI 2 SP q = steady block cursor).
2. Verify `ModeState` cursor shape is `SteadyBlock`.
3. Feed `\x1b[?25l` (DECTCEM hide).
4. Verify cursor visibility = false.
5. Feed `\x1b[?25h` (DECTCEM show).
6. Verify cursor visibility = true.

**Expected result:** Cursor shape and visibility are tracked correctly in `ModeState`.

---

#### TEST-VT-009
**FS requirements:** FS-VT-033
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` at position (5, 10).

**Steps:**
1. Feed `\x1b7` (DECSC — save cursor).
2. Feed `\x1b[1049h` (switch to alternate screen).
3. Move cursor to (0, 0) on alternate screen.
4. Feed `\x1b[1049l` (switch back to normal screen).
5. Check cursor position.

**Expected result:** Cursor is restored to (5, 10) on the normal screen. The alternate screen cursor position (0, 0) does not affect the normal screen cursor.

---

#### TEST-VT-010
**FS requirements:** FS-VT-040, FS-VT-041, FS-VT-042, FS-VT-044
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` on the normal screen with some content.

**Steps:**
1. Feed `\x1b[1049h`.
2. Verify active screen = alternate. Verify alternate screen is cleared.
3. Feed some output on the alternate screen.
4. Feed `\x1b[1049l`.
5. Verify normal screen content is intact and cursor position is restored.
6. Verify `get_scrollback_line()` does not contain alternate screen content.

**Expected result:** Normal screen content survives the switch to/from alternate screen. Alternate screen has no scrollback.

---

#### TEST-VT-011
**FS requirements:** FS-VT-050, FS-VT-051, FS-VT-053
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with a 80×10 screen buffer.

**Steps:**
1. Feed `\x1b[2;8r` (DECSTBM: scroll region rows 2–8).
2. Position cursor at row 8. Feed 3 newlines.
3. Verify rows outside region (1 and 9–10) are unchanged.
4. Verify only 3 lines scrolled within region 2–8.
5. Verify scrollback buffer is empty (partial region = no scrollback entry per FS-VT-053).

**Expected result:** Partial scroll region operates correctly. No scrollback lines are added from a partial region scroll.

---

#### TEST-VT-012
**FS requirements:** FS-VT-060, FS-VT-062
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed `\x1b]0;My Title\x07`.
2. Verify the title string is "My Title".
3. Feed `\x1b]0;Title\x01WithControl\x07` (OSC title containing C0 control char).
4. Verify the stored title has control chars stripped.
5. Feed an OSC 0 title of 300 characters.
6. Verify stored title is truncated to 256 characters.

**Expected result:** Titles are stored with C0/C1 stripped and max 256 chars enforced.

---

#### TEST-VT-013
**FS requirements:** FS-VT-063
**Layer:** Unit (Rust)
**Priority:** Must (security)

**Preconditions:** A `VtProcessor` connected to a mock PTY input buffer.

**Steps:**
1. Feed `\x1b[21t` (CSI 21t — report window title).
2. Verify no bytes were written to the PTY master input.
3. Feed an OSC query sequence that would normally inject a response.
4. Verify no bytes were written to the PTY master input.

**Expected result:** All read-back/response sequences are silently discarded. No injection into the PTY input stream occurs.

---

#### TEST-VT-014
**FS requirements:** FS-VT-073
**Layer:** Unit (Rust)
**Priority:** Must (security)

**Preconditions:** URI validation logic (from `vt/osc.rs` or `platform/`).

**Steps:**
1. Validate `javascript:alert(1)` — expect rejection.
2. Validate `data:text/html,<script>` — expect rejection.
3. Validate `blob:https://example.com/x` — expect rejection.
4. Validate a 2049-character `https://` URI — expect rejection.
5. Validate `https://example.com` — expect acceptance.
6. Validate `ssh://user@host` — expect acceptance.
7. Validate `file:///etc/passwd` in an SSH session context — expect rejection.
8. Validate a URI containing `\x00` (null byte) — expect rejection.
9. Validate a URI containing `\x1b` (C0 escape) — expect rejection.

**Expected result:** Only safe schemes and well-formed URIs pass. All attack vectors are rejected.

---

#### TEST-VT-015
**FS requirements:** FS-VT-075, FS-VT-076
**Layer:** Unit (Rust)
**Priority:** Must (security)

**Preconditions:** A `VtProcessor` with `allow_osc52_write = false` (global default).

**Steps:**
1. Feed `\x1b]52;c;<base64 payload>\x07` (OSC 52 clipboard write).
2. Verify clipboard was not modified.
3. Set `allow_osc52_write = true` for this connection.
4. Feed the same sequence.
5. Verify clipboard was modified.
6. Feed `\x1b]52;c;?\x07` (OSC 52 clipboard read query) with `allow_osc52_write = true`.
7. Verify no response is sent and clipboard content is not returned.

**Expected result:** Write respects per-connection policy. Read is permanently rejected regardless of configuration.

---

#### TEST-VT-016
**FS requirements:** FS-VT-080, FS-VT-082, FS-VT-083
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with mouse reporting mode None.

**Steps:**
1. Simulate a mouse click. Verify it is not encoded and sent to PTY.
2. Feed `\x1b[?1000h` (enable X10 normal mouse reporting).
3. Simulate a mouse click at (10, 5). Verify it is encoded and sent to PTY.
4. Simulate a Shift+click at (10, 5). Verify it is NOT sent to PTY (Shift bypasses).

**Expected result:** Mouse event routing respects the reporting mode. Shift+click always bypasses mouse reporting.

---

#### TEST-VT-017
**FS requirements:** FS-VT-090, FS-VT-091, FS-VT-092
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with bell type = visual (default).

**Steps:**
1. Feed `\x07` (BEL) in a non-active pane context.
2. Verify a visual bell notification is triggered.
3. Feed 100 BEL bytes in rapid succession (within 100ms).
4. Verify at most 1 notification action is triggered in that window.

**Expected result:** Visual bell is the default. Bell is rate-limited to one action per 100ms.

---

#### TEST-VT-018
**FS requirements:** FS-SEC-005
**Layer:** Unit (Rust)
**Priority:** Must (security)

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed `\x1b]0;` followed by 10,000 bytes without a terminator.
2. Verify the VtProcessor does not allocate unbounded memory.
3. Verify the sequence is discarded after 4096 bytes.
4. Verify subsequent valid sequences are parsed correctly.

**Expected result:** OCS/DCS sequences exceeding 4096 bytes are discarded. No memory exhaustion. No crash. Parser recovers cleanly.

---

#### TEST-VT-019
**FS requirements:** FS-SB-001, FS-SB-003, FS-SB-004
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with a 80×5 screen buffer and scrollback capacity 10000.

**Steps:**
1. Output 10 lines (scroll 5 lines off the top of the full-screen region).
2. Verify 5 lines are in scrollback with correct text and attributes (color, bold).
3. Set a partial scroll region (rows 2–4). Output enough to scroll within the region.
4. Verify scrollback count did not increase (partial region does not feed scrollback).

**Expected result:** Scrollback only accepts lines from full-screen region scrolls, with attributes preserved.

---

#### TEST-VT-020
**FS requirements:** FS-SB-008
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with a 80-column buffer.

**Steps:**
1. Output a line of 100 characters (causes soft-wrap at column 80).
2. Output a shorter line followed by a hard newline.
3. Inspect the scrollback entries.

**Expected result:** The 100-character output creates a soft-wrap boundary record. The subsequent line has a hard newline mark. The `Cell` metadata or line metadata correctly distinguishes the two.

---

#### TEST-VT-021
**FS requirements:** FS-SEARCH-001, FS-SEARCH-002, FS-SEARCH-003
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` with 100 lines in scrollback containing colored text and a word spanning a soft-wrap boundary.

**Steps:**
1. Call `VtProcessor::search()` with query "error" (case-insensitive).
2. Verify matches include occurrences regardless of SGR attributes.
3. Search for a word that spans a soft-wrap boundary.
4. Verify it is found as a single match.

**Expected result:** Search operates on stripped text, ignores attributes, crosses soft-wrap boundaries.

---

#### TEST-VT-022
**FS requirements:** FS-SEARCH-004
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor` on the alternate screen buffer.

**Steps:**
1. Set up scrollback content on the normal screen.
2. Switch to alternate screen (`\x1b[1049h`).
3. Output distinctive text on the alternate screen.
4. Call `VtProcessor::search()` with a query matching alternate screen text.

**Expected result:** Search returns no matches. Alternate screen content is not searched.

---

#### TEST-VT-023
**FS requirements:** FS-VT-015
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** A `VtProcessor`.

**Steps:**
1. Feed `\x0e` (SO — shift out, activate G1 charset with DEC Special Graphics).
2. Feed `\x6a` (0x6A = `j`, maps to `┘` in DEC Special Graphics).
3. Inspect the resulting cell character.

**Expected result:** The cell contains the line-drawing character `┘` (or equivalent). DEC Special Graphics mapping is correctly applied.

---

#### TEST-VT-024
**FS requirements:** FS-VT-083, FS-VT-080, FS-VT-081, FS-VT-082
**Layer:** Unit (Rust)
**Priority:** Should

**Preconditions:** A `VtProcessor` initialized with no active mouse mode (`mouse_reporting == None`, `mouse_encoding == X10`).

**Steps:**
1. Feed `\x1b[?1003h` (activate AnyEvent reporting).
2. Assert `mouse_reporting == AnyEvent`.
3. Construct a mouse motion event at col=15, row=8, no button pressed (`button=0`), no modifiers, `is_motion=true`.
4. Call `encode(SGR)` on the event.
5. Assert the result is `\x1b[<32;15;8M` (motion bit 32 set, button_bits=0, 1-based coordinates, press trailer `M`).
6. Feed `\x1b[?1003l` (deactivate AnyEvent).
7. Assert `mouse_reporting == None`.

**Expected result:** Mode 1003 activates and deactivates correctly. Motion events carry the motion bit (32) in SGR encoding: `cb = button_bits | 32`.

**Implementation:** `vt::processor::tests::modes::mouse_mode_1003_activate_and_deactivate`, `mouse_round_trip_any_event_sgr`.

---

#### TEST-VT-025
**FS requirements:** FS-VT-080, FS-VT-081, FS-VT-082
**Layer:** Unit (Rust)
**Priority:** Should

**Context:** tmux activates mode 1000 then 1002 (ButtonEvent supersedes Normal), then SGR encoding (1006). Reporting mode and encoding are orthogonal fields; activating one must not clear the other. Reset of mode 1000 clears all reporting.

**Preconditions:** A `VtProcessor` initialized with no active mouse mode.

**Steps:**
1. Feed `\x1b[?1000h` → assert `mouse_reporting == Normal`.
2. Feed `\x1b[?1002h` → assert `mouse_reporting == ButtonEvent` (1002 supersedes 1000).
3. Feed `\x1b[?1006h` → assert `mouse_encoding == Sgr`.
4. Assert `mouse_reporting` is still `ButtonEvent` (encoding change must not affect reporting mode).
5. Construct a button-press event: left button (`button=0`), col=5, row=3, no modifiers, `is_press=true`.
6. Call `encode(Sgr)` → assert result is `\x1b[<0;5;3M`.
7. Feed `\x1b[?1000l` → assert `mouse_reporting == None` (reset clears all reporting).

**Expected result:** tmux sequence activates ButtonEvent + SGR correctly. Reporting mode and encoding are independent. Reset of mode 1000 clears `mouse_reporting` to `None`.

**Implementation:** `vt::processor::tests::modes::mouse_mode_interaction_1000_then_1006`, `mouse_round_trip_button_event_sgr`, `mouse_mode_1002_activate_and_deactivate`.

---

### 4.3 SSH Lifecycle

---

#### TEST-SSH-UNIT-001
**FS requirements:** FS-SSH-010, FS-SSH-020
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/ssh/connection.rs` and `src-tauri/src/ssh/manager.rs`

**Steps:**
1. Create an `SshConnection` — assert initial state is `Connecting`.
2. Transition through: `Connecting → Authenticating → Connected → Disconnected → Closed`.
3. Assert each state transition is reflected by `state()`.
4. Serialize `SshLifecycleState::Connected` to JSON — assert `"type":"connected"` tag.
5. Assert `SSH_KEEPALIVE_MAX_MISSES == 3` (FS-SSH-020).
6. Assert `SshManager::open_connection` returns `Err` on duplicate pane_id.
7. Assert `SshManager::close_connection` returns `Err` on unknown pane_id.
8. Assert `SshManager::reconnect` returns `Err` on unknown pane_id.

**Expected result:** All assertions pass. State machine is correct. Manager guards duplicate and unknown-pane errors.

**Implementation:** Tests already present in `connection.rs` and `manager.rs`. Tests added in sprint 2 for manager error paths.

---

#### TEST-SSH-UNIT-002
**FS requirements:** FS-SSH-011 (known_hosts TOFU)
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/ssh/known_hosts.rs`

**Steps:**
1. Parse an OpenSSH-format known_hosts line (RSA key, ED25519 key).
2. Look up a host — find existing entry and return it.
3. Look up an unknown host — return `None` (TOFU: first time).
4. Add a new host key — write to known_hosts file, read back and verify.
5. Detect a key mismatch for a known host — return `Err(HostKeyMismatch)`.
6. Parse a known_hosts file containing hashed entries (`|1|...`) — skip with count.

**Expected result:** Parse, lookup, add, and mismatch detection all work correctly.

**Status:** [BLOCKED: known_hosts.rs is a stub] — activated when SSH integration pass lands.

---

#### TEST-SSH-UNIT-003
**FS requirements:** FS-SSH-012 (auth order: pubkey → keyboard-interactive → password)
**Layer:** Unit (Rust) — mock transport
**Priority:** Must
**Location:** `src-tauri/src/ssh/auth.rs`

**Steps:**
1. Create a mock `russh` client handle that accepts a public key auth attempt.
2. Run the auth sequence — assert `publickey` is tried first.
3. Simulate public key auth failure — assert `keyboard-interactive` is tried next.
4. Simulate keyboard-interactive failure — assert `password` is tried last.
5. Assert the auth result carries the method used.

**Expected result:** Auth order matches FS-SSH-012. No method is skipped or reordered.

**Status:** [BLOCKED: auth.rs is a stub] — activated when SSH integration pass lands.

---

#### TEST-SSH-UNIT-004
**FS requirements:** FS-SSH-020 (keepalive: 3 misses → Disconnected)
**Layer:** Unit (Rust) — Tokio test with mock channel
**Priority:** Must
**Location:** `src-tauri/src/ssh/keepalive.rs`

**Steps:**
1. Create a keepalive task with a mock channel that drops all pings.
2. Advance time by `3 × keepalive_interval + 1ms` using `tokio::time::advance`.
3. Assert the `SshLifecycleState` transitions to `Disconnected`.
4. Assert exactly 3 keepalive probes were sent before disconnection.

**Expected result:** Connection is declared lost after exactly 3 consecutive missed keepalives.

**Status:** [BLOCKED: keepalive.rs is a stub] — activated when SSH integration pass lands.

---

#### TEST-SSH-UNIT-005
**FS requirements:** FS-CRED-001, FS-CRED-003 (Secret Service store/retrieve)
**Layer:** Unit (Rust) — mock credential store
**Priority:** Must
**Location:** `src-tauri/src/credentials.rs` and `src-tauri/src/platform/credentials_linux.rs`

**Steps:**
1. Call `LinuxCredentialStore::is_available()` — assert it returns `false` when D-Bus is unavailable.
2. Call `CredentialManager::store_password()` — assert it delegates to the credential store.
3. Call `CredentialManager::get_password()` — assert it returns the stored value.
4. Call `CredentialManager::delete_password()` — assert the key is removed.
5. Confirm no password is ever written to `preferences.json`.

**Expected result:** Credential store operations are correctly delegated. No credentials leak to disk.

**Status:** [BLOCKED: credentials_linux.rs is a stub] — activated when SecretService integration lands.

---

#### TEST-SSH-001
**FS requirements:** FS-SSH-001, FS-SSH-002
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open. A saved SSH connection exists pointing to the mock SSH server.

**Steps:**
1. Open the connection manager panel.
2. Click on the saved SSH connection to open it in a new tab.
3. Observe the tab.

**Expected result:** A new tab is created. It displays an SSH state indicator (e.g., SSH badge). The tab enters the "Connecting" state, then "Authenticating", then "Connected". The terminal is interactive.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

#### TEST-SSH-002
**FS requirements:** FS-SSH-010
**Layer:** E2E
**Priority:** Must

**Preconditions:** A connected SSH pane.

**Steps:**
1. Observe the pane in each lifecycle state transition.
2. Interrupt the network connection to force a disconnect.
3. Observe the "Disconnected" state.
4. Click the reconnect action.
5. Observe the reconnection flow.

**Expected result:** Each lifecycle state (Connecting, Authenticating, Connected, Disconnected) has a distinct visual representation. Reconnection transitions through Connecting → Authenticating → Connected.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

#### TEST-SSH-003
**FS requirements:** FS-SSH-011
**Layer:** E2E
**Priority:** Must (security)

**Preconditions:** TauTerm has no known-hosts entry for the mock server.

**Steps:**
1. Initiate an SSH connection to the mock server.
2. Observe the host key dialog.
3. Verify the dialog text contains a plain-language explanation, the SHA-256 fingerprint, and the key type.
4. Accept the key.
5. Change the mock server's host key.
6. Initiate another connection.
7. Observe the key-change warning dialog.

**Expected result:** First connection: plain-language prompt, SHA-256 fingerprint, key type visible. Key change: connection is blocked, both fingerprints shown, MITM warning displayed, default action is Reject, acceptance requires a deliberate non-default action.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

#### TEST-SSH-004
**FS requirements:** FS-SSH-012
**Layer:** E2E
**Priority:** Must

**Preconditions:** Mock SSH server accepts a test private key. Saved connection uses that key.

**Steps:**
1. Open the SSH connection.
2. Observe whether a password prompt appears.

**Expected result:** Authentication proceeds without prompting for a password. Key-based auth is tried first and succeeds.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

#### TEST-SSH-005
**FS requirements:** FS-SSH-013
**Layer:** Integration
**Priority:** Must

**Preconditions:** A mock SSH server that records the PTY request parameters.

**Steps:**
1. Initiate an SSH connection.
2. Inspect the PTY request sent by TauTerm.

**Expected result:** The PTY request includes: `TERM=xterm-256color`, terminal dimensions (`cols`, `rows`, `xpixel`, `ypixel`), and the terminal mode list containing VINTR=3, VQUIT=28, VERASE=127, VEOF=4, VKILL=21, VSUSP=26, ISIG=1, ICANON=1, ECHO=1, TTY_OP_END=0 — using RFC 4254 Annex A opcodes, not kernel termios indices.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

#### TEST-SSH-006
**FS requirements:** FS-SSH-014
**Layer:** E2E
**Priority:** Must

**Preconditions:** Mock SSH server configured to advertise only `ssh-rsa` (SHA-1) as its host key algorithm.

**Steps:**
1. Connect to the mock server.
2. Observe the pane after connection is established.

**Expected result:** The pane displays a non-blocking, dismissible warning naming "ssh-rsa (SHA-1)" as deprecated and recommending server update. The connection is established and the terminal is functional. Clicking "Dismiss" removes the banner.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

#### TEST-SSH-007
**FS requirements:** FS-SSH-020
**Layer:** Integration
**Priority:** Must

**Preconditions:** An `SshConnection` with a mock transport that can simulate dropped keepalives.

**Steps:**
1. Start the keepalive task with 30s interval.
2. Simulate 3 consecutive missed keepalive responses.
3. Observe the `SshLifecycleState`.

**Expected result:** After 3 missed keepalives (90s elapsed), the state transitions to `Disconnected`. The disconnect reason is recorded as keepalive timeout.

---

#### TEST-SSH-008
**FS requirements:** FS-SSH-040, FS-SSH-041, FS-SSH-042
**Layer:** E2E
**Priority:** Must

**Preconditions:** A connected SSH pane with some scrollback history.

**Steps:**
1. Simulate a network drop (force Disconnected state).
2. Observe the reconnect action in the pane.
3. Click "Reconnect".
4. Wait for reconnection to complete.
5. Scroll back in the pane.

**Expected result:** The "Reconnect" action is directly visible in the disconnected pane. After reconnection, all previous scrollback is intact. A visual separator (timestamp or label) appears at the reconnection boundary.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

#### TEST-SSH-009
**FS requirements:** FS-SSH-030, FS-SSH-031, FS-SSH-032, FS-SSH-033, FS-SSH-034
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open.

**Steps:**
1. Open the connection manager UI.
2. Create a new saved connection with host, port, username, and identity file path.
3. Verify it appears in the list.
4. Edit the connection (change the label).
5. Duplicate the connection.
6. Delete the duplicate.
7. Quit and relaunch TauTerm.
8. Verify the original (edited) connection is still present.

**Expected result:** All CRUD operations and duplication work. Connections persist across restarts. The connection list UI shows all saved connections.

---

#### TEST-SSH-010
**FS requirements:** FS-SEC-004
**Layer:** E2E
**Priority:** Must (security)

**Preconditions:** A connected SSH pane. Network monitoring available.

**Steps:**
1. Inspect the SSH connection for any agent forwarding channel requests.

**Expected result:** No SSH agent forwarding channel (`auth-agent@openssh.com`) is ever opened.

**Note:** [BLOCKED: stub] — requires SSH lifecycle to be implemented.

---

### 4.4 Preferences & i18n

---

#### TEST-PREF-001
**FS requirements:** FS-PREF-001
**Layer:** Integration
**Priority:** Must

**Preconditions:** A temporary preferences directory.

**Steps:**
1. Create a `PreferencesStore` with non-default settings (e.g., scrollback = 50000, bell = audible).
2. Call `save()`.
3. Create a new `PreferencesStore` by calling `load()` on the same path.
4. Compare all fields.

**Expected result:** All fields survive the serialization round-trip without loss or transformation.

---

#### TEST-PREF-002
**FS requirements:** FS-PREF-001, FS-SEC-003
**Layer:** Integration
**Priority:** Must

**Preconditions:** A preferences file containing intentionally invalid values.

**Steps:**
1. Write a `preferences.json` with `scrollbackSize: -1` (invalid), `bellType: "laser"` (unknown), and a corrupted JSON fragment.
2. Call `PreferencesStore::load_or_default()`.
3. Inspect the returned preferences.

**Expected result:** Invalid fields are replaced with defaults. No crash. A WARN log entry is emitted for each replaced field. Application starts normally.

---

#### TEST-PREF-003
**FS requirements:** FS-PREF-003
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open.

**Steps:**
1. Open Preferences (Ctrl+,).
2. Change the font size setting.
3. Observe the terminal area immediately.

**Expected result:** The terminal font size changes immediately without requiring a restart. The change is reflected in all open panes.

---

#### TEST-PREF-004
**FS requirements:** FS-PREF-005
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open.

**Steps:**
1. Press Ctrl+, (keyboard shortcut).
2. Close preferences.
3. Click the Settings button in the status bar.

**Expected result:** Both actions open the Preferences panel. The panel is accessible via both modalities.

---

#### TEST-I18N-001
**FS requirements:** FS-I18N-001, FS-I18N-002
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm launched in English (default).

**Steps:**
1. Inspect all visible UI labels (tab bar, status bar, preferences, context menu, dialogs).
2. Switch to French in Preferences.
3. Inspect all visible UI labels again.

**Expected result:** No hardcoded strings are visible in either locale. All labels are present in both English and French. No raw message key (e.g., `"tab.new"`) is visible in either locale.

---

#### TEST-I18N-002
**FS requirements:** FS-I18N-004
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open in English.

**Steps:**
1. Open Preferences.
2. Change language to French.
3. Observe the UI immediately (without closing Preferences, without restart).

**Expected result:** All visible UI strings switch to French immediately. No page reload, no restart required. Preferences panel itself updates to French.

---

#### TEST-I18N-003
**FS requirements:** FS-I18N-005, FS-I18N-006
**Layer:** Integration + E2E
**Priority:** Must

**Preconditions:**

**Steps (persistence):**
1. Set language to French in Preferences.
2. Quit TauTerm.
3. Relaunch TauTerm.
4. Verify the UI is in French.

**Steps (fallback):**
1. Manually set `preferences.json` `appearance.language` to `"de"` (unsupported locale).
2. Launch TauTerm.
3. Verify no crash. Verify the UI is in English.

**Expected result:** Persistence: French is restored on relaunch. Fallback: unknown locale silently defaults to English.

---

#### TEST-I18N-004
**FS requirements:** FS-I18N-006
**Layer:** Unit (Rust)
**Priority:** Must

**Preconditions:** None.

**Steps:**
1. Deserialize the JSON `{"language": "de"}` into the `Language` enum.

**Expected result:** Deserialization succeeds and produces `Language::En` (the default). No error is returned.

---

#### TEST-I18N-005
**FS requirements:** FS-I18N-007
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with a terminal pane.

**Steps:**
1. Set UI language to French in Preferences.
2. In the terminal, run `echo $LANG && echo $LC_ALL`.
3. Compare the output with the values from the parent environment.

**Expected result:** `$LANG` and `$LC_ALL` inside the PTY are unchanged — they reflect the user's login environment, not TauTerm's UI language selection.

---

#### TEST-CRED-001
**FS requirements:** FS-CRED-001
**Layer:** E2E
**Priority:** Must (security)

**Preconditions:** A saved SSH connection with a password configured (stored in keychain).

**Steps:**
1. Inspect the `preferences.json` file on disk.
2. Search for the password string.

**Expected result:** No plaintext password is present in `preferences.json`. The credential is stored in the OS keychain only.

**Note:** [BLOCKED: stub] — requires Secret Service integration to be implemented.

---

#### TEST-CRED-002
**FS requirements:** FS-CRED-004
**Layer:** Unit (Rust) + Integration
**Priority:** Must (security)

**Preconditions:** Logging configured at maximum verbosity (TRACE level).

**Steps:**
1. Perform an SSH authentication using a password.
2. Inspect all log output.

**Expected result:** No password, passphrase, or key material appears in any log line at any log level.

**Note:** [BLOCKED: stub] — requires SSH auth and Secret Service to be implemented.

---

#### TEST-CRED-003
**FS requirements:** FS-CRED-005
**Layer:** E2E
**Priority:** Must

**Preconditions:** No Secret Service provider is running on the system.

**Steps:**
1. Attempt to open an SSH connection using password authentication.
2. Observe the UI.

**Expected result:** TauTerm prompts for the password (without persisting it) and displays a clear notice that credential persistence is unavailable due to missing keychain. The connection proceeds normally with the entered password.

**Note:** [BLOCKED: stub] — requires credential system to be implemented.

---

### 4.5 UI Components & Accessibility

---

#### TEST-TAB-001
**FS requirements:** FS-TAB-001, FS-TAB-003
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open with one tab.

**Steps:**
1. Press Ctrl+Shift+T.
2. Click the "+" button in the tab bar.
3. Verify the tab count.

**Expected result:** Each action creates a new independent tab with its own PTY session. After two actions, there are 3 tabs total.

---

#### TEST-TAB-002
**FS requirements:** FS-TAB-004, FS-PTY-008
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two tabs open; active tab has a running process.

**Steps:**
1. Press Ctrl+Shift+W on the active tab.
2. Observe the confirmation dialog.
3. Confirm close.
4. Verify tab is closed.

**Expected result:** Confirmation dialog appears indicating a running process. After confirmation, the tab is closed. If no process is running, the tab closes immediately without dialog.

---

#### TEST-TAB-003
**FS requirements:** FS-TAB-005
**Layer:** E2E
**Priority:** Must

**Preconditions:** Three tabs open in order: Tab A, Tab B, Tab C.

**Steps:**
1. Drag Tab C to the leftmost position.
2. Verify the new tab order.

**Expected result:** Tab order becomes Tab C, Tab A, Tab B. The tab content (PTY session) follows the tab item.

---

#### TEST-TAB-004
**FS requirements:** FS-TAB-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with one tab.

**Steps:**
1. In the terminal, run `printf "\033]0;My Process Title\007"`.
2. Observe the tab title.
3. Double-click the tab title.
4. Type a custom label and press Enter.
5. Verify the custom label takes precedence over the process title.
6. Clear the custom label (delete all text, press Enter).
7. Verify the tab reverts to the process-driven title.

**Expected result:** OSC-driven title updates the tab. Double-click enables inline editing. User label takes precedence. Clearing reverts to process title. Right-click "Rename" achieves the same result.

---

#### TEST-TAB-005
**FS requirements:** FS-TAB-007
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two tabs open; TauTerm is focused on the first tab.

**Steps:**
1. In the second (background) tab's PTY, generate output (via a long-running command).
2. Observe the second tab's header without switching to it.

**Expected result:** The second tab displays a visual activity indicator. It disappears when the user switches to that tab (TEST-NOTIF-003 covers this).

---

#### TEST-TAB-006
**FS requirements:** FS-TAB-008
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with exactly one tab.

**Steps:**
1. Close the only tab (via close button or Ctrl+Shift+W, confirming any running process dialog).

**Expected result:** The application window closes.

---

#### TEST-PANE-001
**FS requirements:** FS-PANE-001, FS-PANE-002
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with one tab.

**Steps:**
1. Press Ctrl+Shift+D (split horizontal: left/right).
2. Press Ctrl+Shift+E (split vertical: top/bottom).
3. In each pane, run `echo $SHLVL` and compare.

**Expected result:** Each split produces an independent pane with its own PTY session. Each `echo $SHLVL` is independent (same value, but separate sessions).

---

#### TEST-PANE-002
**FS requirements:** FS-PANE-003
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two panes side-by-side; one runs vim.

**Steps:**
1. Drag the pane divider to the left.
2. Observe vim's display in the resized pane.

**Expected result:** Both panes resize. Vim redraws at the new dimensions. The divider's hit area is 8px wide. Hover changes the cursor to `col-resize`.

---

#### TEST-PANE-003
**FS requirements:** FS-PANE-004
**Layer:** E2E
**Priority:** Must

**Preconditions:** A tab with exactly one pane.

**Steps:**
1. Close the only pane (Ctrl+Shift+Q or close button).

**Expected result:** The tab closes (no tab remains with zero panes, per FS-TAB-008 the window closes if it was the last tab).

---

#### TEST-PANE-004
**FS requirements:** FS-PANE-005, FS-PANE-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** Three panes in a tab.

**Steps:**
1. Press Ctrl+Shift+Right to navigate to the next pane.
2. Click a non-active pane with the mouse.
3. Observe the active pane visual indicator.

**Expected result:** Both keyboard navigation and mouse click correctly transfer focus. The active pane has a 2px `--color-pane-border-active` border. Inactive panes have a 1px `--color-pane-border-inactive` border.

---

#### TEST-KBD-001
**FS requirements:** FS-KBD-001
**Layer:** Frontend Unit
**Priority:** Must

**Preconditions:** The keyboard module is initialized.

**Steps:**
1. Simulate Ctrl+Shift+T keydown event.
2. Verify the application shortcut handler fires (new tab action).
3. Verify the keydown event is not transmitted to the PTY encoder.

**Expected result:** Application shortcuts are consumed before PTY encoding. The PTY does not receive the keystroke.

---

#### TEST-KBD-002
**FS requirements:** FS-KBD-004
**Layer:** Frontend Unit
**Priority:** Must

**Preconditions:** The keyboard module, no active application shortcut for Ctrl+C.

**Steps:**
1. Simulate Ctrl+C keydown.
2. Inspect the bytes passed to `send_input`.

**Expected result:** `send_input` is called with `[0x03]` (ETX). Ctrl+A → `[0x01]`, Ctrl+Z → `[0x1A]`, Ctrl+[ → `[0x1B]`.

---

#### TEST-KBD-003
**FS requirements:** FS-KBD-005
**Layer:** Frontend Unit
**Priority:** Must

**Preconditions:** The keyboard module.

**Steps:**
1. Simulate Alt+A keydown.
2. Inspect bytes sent to PTY.

**Expected result:** Bytes are `[0x1B, 0x61]` (ESC + 'a'). Not `[0xE1]` (8-bit encoding is forbidden).

---

#### TEST-KBD-004
**FS requirements:** FS-KBD-007
**Layer:** Frontend Unit
**Priority:** Must

**Preconditions:** The keyboard module with `decckm = false` (normal mode).

**Steps:**
1. Simulate ArrowUp. Verify bytes = `[0x1B, 0x5B, 0x41]` (ESC [ A).
2. Set `decckm = true` (application cursor mode, received via `mode-state-changed` event).
3. Simulate ArrowUp. Verify bytes = `[0x1B, 0x4F, 0x41]` (ESC O A).

**Expected result:** Arrow key encoding is mode-dependent. The frontend correctly tracks DECCKM state from the backend event.

---

#### TEST-CLIP-001
**FS requirements:** FS-CLIP-001, FS-CLIP-002
**Layer:** Frontend Unit + E2E
**Priority:** Must

**Preconditions:** A `TerminalPane` with some text.

**Steps (unit):**
1. Simulate a click-and-drag across cells.
2. Verify the selection starts and ends at cell boundaries (not pixel boundaries).
3. Simulate a double-click on a word.
4. Verify word selection includes the full word using default delimiters.

**Expected result:** Selection is cell-aligned. Double-click selects a full word. Paths like `/home/user/project` are selected as a single word on double-click (`.`, `/`, `-`, `_` are not delimiters).

---

#### TEST-CLIP-002
**FS requirements:** FS-CLIP-004, FS-CLIP-005
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm running on X11 or Wayland.

**Steps:**
1. Select text in TauTerm by click-and-drag.
2. Middle-click in another application.
3. Verify the selected text is pasted (PRIMARY selection).
4. Copy text in a browser (CLIPBOARD).
5. Press Ctrl+Shift+V in TauTerm.
6. Verify the browser text is pasted (not the PRIMARY selection).

**Expected result:** Text selection fills PRIMARY (middle-click paste works). Ctrl+Shift+V pastes from CLIPBOARD, not PRIMARY.

---

#### TEST-CLIP-003
**FS requirements:** FS-CLIP-008
**Layer:** E2E
**Priority:** Must (security)

**Preconditions:** A shell with bracketed paste mode enabled (`\x1b[?2004h`).

**Steps:**
1. Paste multi-line text via Ctrl+Shift+V.
2. Observe what the shell receives.

**Expected result:** Pasted text is wrapped with ESC [200~ and ESC [201~. Multi-line paste does not auto-execute commands. If the pasted text itself contains ESC [201~, it is stripped before wrapping.

---

#### TEST-IPC-CLIP-001
**FS requirements:** FS-CLIP-001 (clipboard write via IPC)
**Layer:** Unit (Rust)
**Priority:** Must
**Location:** `src-tauri/src/commands/system_cmds.rs` — `security_tests` module

**Preconditions:** None (no clipboard hardware required — validation tested in isolation).

**Steps:**
1. Call `copy_to_clipboard` with a payload of `MAX_CLIPBOARD_LEN + 1` bytes.
2. Call `copy_to_clipboard` with an empty string.
3. Call `copy_to_clipboard` with exactly `MAX_CLIPBOARD_LEN` bytes.
4. Call `get_clipboard` (may fail in headless CI — assert no panic and no `CLIPBOARD_TOO_LARGE` error code).

**Expected result:**
- Step 1: `Err` with code `CLIPBOARD_TOO_LARGE`.
- Steps 2–3: Either `Ok` (if display available) or `Err` with a non-`CLIPBOARD_TOO_LARGE` code.
- Step 4: `Ok(String)` or `Err` with code `CLIPBOARD_UNAVAILABLE` or `CLIPBOARD_READ_FAILED`. No panic.

**Implementation:** Tests `ipc_clip_001` through `ipc_clip_004` are implemented and passing in `system_cmds.rs`.

---

#### TEST-SEARCH-001
**FS requirements:** FS-SEARCH-001, FS-SEARCH-003, FS-SEARCH-005, FS-SEARCH-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** A terminal pane with `seq 1 100000` output in scrollback.

**Steps:**
1. Press Ctrl+Shift+F to open search.
2. Type "99999".
3. Measure time to first result.
4. Verify all matches are highlighted.
5. Press Enter/Next to navigate.

**Expected result:** First result appears in under 100ms. All matches are highlighted with a visually distinct current match. Navigation scrolls to center each match in view.

---

#### TEST-SEARCH-002
**FS requirements:** FS-SEARCH-004
**Layer:** E2E
**Priority:** Must

**Preconditions:** A terminal pane with vim open (alternate screen) and scrollback content.

**Steps:**
1. Open search while vim is active.
2. Search for text visible in vim's alternate screen content.

**Expected result:** No matches are found for alternate screen content. Search operates only on normal screen scrollback.

---

#### TEST-NOTIF-001
**FS requirements:** FS-NOTIF-001, FS-NOTIF-002, FS-NOTIF-003
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two tabs open.

**Steps:**
1. Generate output in the background tab.
2. Observe the background tab header for an activity indicator.
3. Have a process exit in the background tab.
4. Observe the background tab header for a termination indicator (visually distinct from activity).
5. Switch to the background tab.
6. Verify both indicators are cleared.

**Expected result:** Activity and termination indicators are visually distinct. Both clear on tab focus. (Secondary indicators beyond color — icon, badge — are required per FS-A11Y-004.)

---

#### TEST-NOTIF-002
**FS requirements:** FS-NOTIF-004
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two tabs open.

**Steps:**
1. In the background tab, send a BEL character (`printf "\007"`).
2. Observe the background tab header.

**Expected result:** The background tab displays a bell notification indicator (visually distinct from plain output activity).

---

#### TEST-THEME-001
**FS requirements:** FS-THEME-001, FS-THEME-002
**Layer:** E2E
**Priority:** Must

**Preconditions:** Fresh TauTerm install.

**Steps:**
1. Open Preferences → Themes.
2. Observe the default theme.
3. Attempt to find a "Delete" action for the default theme.

**Expected result:** The default theme (Umbra) is present and polished on first launch. No delete action is available for it.

---

#### TEST-THEME-002
**FS requirements:** FS-THEME-003, FS-THEME-004, FS-THEME-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open.

**Steps:**
1. Open Preferences → Themes → Create new theme.
2. Set background, foreground, cursor color, selection color, and all 16 ANSI palette colors.
3. Save the theme.
4. Activate the theme.
5. Observe the terminal output of `ls --color`.

**Expected result:** Theme is created and saved. Activating it applies immediately (no restart). ANSI palette colors from the theme are used in `ls --color` output.

---

#### TEST-THEME-003
**FS requirements:** FS-THEME-008, FS-THEME-009
**Layer:** Frontend Unit
**Priority:** Must

**Preconditions:** The theme validation module.

**Steps:**
1. Create a theme with a hardcoded color value that bypasses the token system.
2. Run the theme validator.
3. Create a theme with missing required tokens.

**Expected result:** Any theme that does not map to design tokens or is missing required tokens is flagged by the validator. No hardcoded values are accepted.

---

#### TEST-THEME-004
**FS requirements:** FS-A11Y-007
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm preferences open on the Themes section.

**Steps:**
1. Start editing a new custom theme.
2. Set the foreground color identical to the background color (zero contrast).
3. Observe the theme editor's own chrome (labels, input controls, buttons).

**Expected result:** The theme editor chrome remains fully legible (using Umbra system tokens). Only the designated preview area reflects the low-contrast custom theme.

---

#### TEST-A11Y-001
**FS requirements:** FS-A11Y-001
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm with the default Umbra theme.

**Steps:**
1. Run an automated WCAG contrast ratio check on all visible text elements in the UI chrome (tab bar, status bar, preferences panel, dialogs).
2. For each text element, verify the contrast ratio against its background.

**Expected result:** All normal text meets 4.5:1. All large text and UI components (buttons, inputs, icons used as controls) meet 3:1. (Note: inactive tab labels at ~3.1:1 are intentional and documented in UXD §5.4 — these pass the 3:1 UI component threshold.)

---

#### TEST-A11Y-002
**FS requirements:** FS-A11Y-002
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open.

**Steps:**
1. Measure the bounding box of each interactive element: tab items, tab close buttons, new-tab button, settings button, preferences panel controls, dialog buttons.

**Expected result:** No interactive element has a click/touch target area smaller than 44×44 pixels.

---

#### TEST-A11Y-003
**FS requirements:** FS-A11Y-003, FS-A11Y-005
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open.

**Steps:**
1. Use Tab key to cycle through all interactive elements (tab items, new-tab button, settings button, pane management buttons, preferences panel controls, dialog buttons, context menu items).
2. Use Enter/Space to activate each element.

**Expected result:** All interactive elements are reachable via Tab key. Enter/Space activates them. No interactive element is mouse-only.

---

#### TEST-A11Y-004
**FS requirements:** FS-A11Y-004
**Layer:** E2E
**Priority:** Must

**Preconditions:** Two tabs open; one has activity.

**Steps:**
1. Observe the activity indicator on the background tab.
2. Identify whether the indicator uses only color, or whether a secondary indicator (icon, shape, badge, text) is also present.

**Expected result:** The activity indicator uses at least one non-color indicator (dot badge, icon, or text label) in addition to any color change.

---

#### TEST-A11Y-005
**FS requirements:** FS-A11Y-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with a terminal pane.

**Steps:**
1. Right-click in the terminal area.
2. Observe the context menu.

**Expected result:** A context menu appears with at minimum: Copy, Paste, Search, and pane/tab management actions (split, close). The context menu is the primary discoverability path for mouse users who do not know shortcuts.

---

#### TEST-UX-001
**FS requirements:** FS-UX-001
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm configured with `SHELL=/nonexistent`.

**Steps:**
1. Launch TauTerm.
2. Observe the error message displayed.

**Expected result:** The message is in plain language naming the missing shell and the fallback (`/bin/sh`). It tells the user how to fix it (Preferences → Terminal Behavior). Technical detail (errno) is present but visually subordinate.

---

#### TEST-UX-002
**FS requirements:** FS-UX-002
**Layer:** E2E
**Priority:** Must

**Preconditions:** Fresh TauTerm install (no prior use).

**Steps:**
1. Launch TauTerm.
2. Observe the terminal area for a context menu hint.
3. Right-click in the terminal area.
4. Quit and relaunch TauTerm.
5. Observe whether the hint appears again.

**Expected result:** A non-intrusive, non-blocking hint about right-click is visible on first launch. After the first right-click, the hint disappears. It does not reappear on subsequent launches.

---

### 4.6 IPC Contract

---

#### TEST-IPC-001
**FS requirements:** ARCHITECTURE.md §4.4 (error envelope)
**Layer:** Integration
**Priority:** Must

**Preconditions:** A command handler that returns an error.

**Steps:**
1. Call `close_pane` with an unknown `PaneId`.
2. Inspect the returned `TauTermError`.

**Expected result:** The error contains: `code` (e.g., `"INVALID_PANE_ID"`), `message` (human-readable, non-technical), optional `detail`. No panic. No opaque error.

---

#### TEST-IPC-002
**FS requirements:** ARCHITECTURE.md §4.5.1 (PaneNode tree structure)
**Layer:** Integration
**Priority:** Must

**Preconditions:** A `SessionRegistry` with one tab.

**Steps:**
1. Call `split_pane(root_pane_id, SplitDirection::Vertical)`.
2. Inspect the returned `TabState`.
3. Call `split_pane` again on one of the new panes.
4. Inspect the returned `TabState`.

**Expected result:** After first split: `TabState.layout` is a `PaneNode::Split` with two `PaneNode::Leaf` children. After second split: a correctly nested 3-level tree. The structure unambiguously represents `(A | (B / C))` vs `((A | B) / C)`.

---

#### TEST-IPC-003
**FS requirements:** ARCHITECTURE.md §4.5.3 (close_pane last-pane)
**Layer:** Integration
**Priority:** Must

**Preconditions:** A tab with one pane.

**Steps:**
1. Call `close_pane(only_pane_id)`.
2. Inspect the return value.
3. Call `get_session_state()`.

**Expected result:** `close_pane` returns `None` (last pane was closed; tab removed). `get_session_state()` returns an empty tab list. A `session-state-changed` event with `changeType: "tab-closed"` was emitted.

---

#### TEST-IPC-004
**FS requirements:** ARCHITECTURE.md §4.6 (type coherence)
**Layer:** Integration
**Priority:** Must

**Preconditions:** Rust IPC types serialized to JSON.

**Steps:**
1. Serialize each top-level IPC type (`SessionState`, `TabState`, `PaneNode`, `PaneState`, `SshConnectionConfig`, `Preferences`, `TauTermError`) to JSON.
2. Compare the JSON key names and structure against the TypeScript type definitions in `src/lib/ipc/types.ts`.

**Expected result:** All field names match (camelCase in JSON per serde rename). No missing or extra fields. Union types (e.g., `PaneNode`) use the correct discriminant tag (`type` field, per `serde(tag = "type")`).

---

#### TEST-IPC-005
**FS requirements:** FS-SEC-001 (CSP)
**Layer:** E2E
**Priority:** Must (security)

**Preconditions:** TauTerm is running. Browser devtools accessible.

**Steps:**
1. Via devtools console, attempt to inject a `<script>` tag into the DOM.
2. Attempt to evaluate `eval("alert(1)")`.

**Expected result:** Inline script injection is blocked by the CSP (`script-src 'self'`). `eval()` is blocked (`unsafe-eval` is absent from `script-src`). No alert appears.

---

### 4.7 Security

> **Note:** This section contains placeholders for security-expert collaboration. Security scenarios that arise from threat modeling are added here by the security-expert. The scenarios marked [EXISTING] below derive directly from FS-SEC requirements and the security review notes in FS.md §7.

---

#### TEST-SEC-001
**FS requirements:** FS-SEC-001
**Layer:** E2E
**Priority:** Must (security)

See TEST-IPC-005 above (covers CSP inline script + eval blocking).

---

#### TEST-SEC-002
**FS requirements:** FS-SEC-002
**Layer:** Unit (Rust)
**Priority:** Must (security)

See TEST-PTY-002 above (covers O_CLOEXEC and FD_CLOEXEC on PTY master fds).

---

#### TEST-SEC-003
**FS requirements:** FS-VT-063
**Layer:** Unit (Rust)
**Priority:** Must (security)

See TEST-VT-013 above (covers read-back sequence injection prevention).

---

#### TEST-SEC-004
**FS requirements:** FS-VT-073
**Layer:** Unit (Rust)
**Priority:** Must (security)

See TEST-VT-014 above (covers URI scheme validation for hyperlinks).

---

#### TEST-SEC-005
**FS requirements:** FS-VT-075, FS-VT-076
**Layer:** Unit (Rust)
**Priority:** Must (security)

See TEST-VT-015 above (covers OSC 52 write policy and permanent read rejection).

---

#### TEST-SEC-006
**FS requirements:** FS-CRED-004
**Layer:** Integration
**Priority:** Must (security)

See TEST-CRED-002 above (covers credential non-disclosure in logs).

---

#### Threat-derived scenarios — see security-pty-ipc-ssh-credentials-csp-osc52.md

The threat-derived security scenarios are fully documented in [`docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md`](security-pty-ipc-ssh-credentials-csp-osc52.md), which includes a complete threat model (assets, threat actors, attack surface) and 28 scenarios across 7 domains.

Mapping of pending items above to their authoritative SEC-* IDs:

| Threat category | SEC-* IDs | TEST_PROTOCOL cross-ref |
|---|---|---|
| TIOCSTI injection (Linux < 6.2) | SEC-PTY-001, SEC-PTY-002 | TEST-PTY-002 |
| Clipboard poisoning via OSC 52 (SSH) | SEC-OSC-001, SEC-OSC-002 | TEST-VT-015 |
| Title-based MITM via read-back (CSI 21t) | SEC-PTY-003 | TEST-VT-013 |
| Preference file tampering / path traversal | SEC-PATH-003, SEC-PATH-004 | TEST-PTY-009, TEST-PREF-002 |
| SSH known-hosts injection/corruption | SEC-SSH-003, SEC-SSH-004 | TEST-SSH-009 |
| VT sequence fuzzing (malformed CSI/OSC/DCS) | SEC-PTY-004, SEC-PTY-005 | TEST-VT-023 |
| Multi-line paste injection (bracketed paste bypass) | SEC-IPC-004 | TEST-CLIP-003 |
| Rate-limiting bypass for title sequences | SEC-PTY-003 | TEST-VT-013 |
| Environment variable injection | SEC-PTY-001 | TEST-PTY-001 |
| IPC boundary injection (oversized payloads) | SEC-IPC-001, SEC-IPC-002, SEC-IPC-003 | TEST-IPC-001 |

---

---

## 4.8 Sprint Scenarios — Major Sprint 2026-04-05

New scenarios for the thirteen features shipped in the major sprint of 2026-04-05. Each ID is prefixed `TEST-SPRINT-` to make the sprint boundary unambiguous. Coverage spans all three test layers (Rust unit via nextest, frontend unit via vitest, E2E via WebdriverIO).

---

#### TEST-SPRINT-001
**FS requirements:** FS-PTY-013
**Layer:** E2E
**Priority:** Must

**Preconditions:** User's `$SHELL` env var points to an installed shell (e.g. `/bin/bash`). No `login_shell` override set in preferences.

**Steps:**
1. Launch TauTerm. Observe the initial tab.
2. Inside the initial tab's PTY, run `shopt login_shell` (bash) or equivalent (`[[ -o login ]]` for zsh, `status is-login` for fish).
3. Open a second tab via Ctrl+Shift+T.
4. Run the same login-shell detection command in the second tab.

**Expected result:**
- Initial tab: shell reports it is a login shell (e.g. bash prints `login_shell on`). `~/.bash_profile` (or equivalent) is sourced.
- Second tab: shell reports it is an interactive non-login shell. Profile file is not re-sourced.

**Note:** [BLOCKED: stub] — requires PTY spawn with `login` flag to be implemented (`create_tab` must pass `argv[0] = "-bash"` for the first tab).

---

#### TEST-SPRINT-002
**FS requirements:** FS-CLIP-004
**Layer:** Unit (Rust) + E2E
**Priority:** Must

**Preconditions (unit):** `arboard` is available; a fake X11 display is running (Xvfb in CI).
**Preconditions (E2E):** TauTerm running on X11. A second application that can read the PRIMARY selection (e.g. `xclip -selection primary -o`).

**Steps (unit — Rust):**
1. Call `write_primary_selection("hello primary")` from the clipboard backend.
2. Read the X11 PRIMARY selection via `arboard` or `xclip`. Assert the value equals `"hello primary"`.

**Steps (E2E):**
1. In a TauTerm pane, click and drag to select the text "hello world".
2. In a second terminal (outside TauTerm), run `xclip -selection primary -o`.
3. Verify the output is `hello world`.
4. Middle-click inside another TauTerm pane.
5. Verify the text `hello world` is pasted into that pane.

**Expected result (unit):** X11 PRIMARY selection is written immediately on any text selection. (E2E) Middle-click paste of the TauTerm selection works. Ctrl+Shift+V still pastes from CLIPBOARD (not PRIMARY).

**Note:** [BLOCKED: stub] — requires `write_primary_selection` path in `platform/clipboard_linux.rs` to be wired.

---

#### TEST-SPRINT-003
**FS requirements:** FS-CLIP-002 (word select), FS-CLIP-003 (line select implied by triple-click)
**Layer:** Frontend Unit + E2E
**Priority:** Must

**Preconditions (unit):** `selection.ts` module loaded with a mock grid.
**Preconditions (E2E):** TauTerm open with a line of text `foo bar/baz_qux` in a pane.

**Steps (unit):**
1. Feed the test grid with cells for `foo bar/baz_qux`.
2. Simulate a double-click on the cell containing `b` in `bar`. Assert the selection spans `bar` (space is a delimiter; `/` is not by default).
3. Simulate a double-click on the cell containing `b` in `baz_qux`. Assert the selection spans `baz_qux` (underscore is not a delimiter by default).
4. Simulate a triple-click anywhere on the line. Assert the selection spans the entire line from column 0 to the last non-empty cell.

**Steps (E2E):**
1. Double-click on `bar` in the terminal. Verify only `bar` is selected (highlighted).
2. Double-click on `baz_qux`. Verify the full token `baz_qux` is selected.
3. Triple-click. Verify the entire line `foo bar/baz_qux` is selected.

**Expected result:** Double-click selects a word using the default delimiter set (space, tab, common punctuation; but not `/`, `.`, `-`, `_`). Triple-click selects the full line.

---

#### TEST-SPRINT-004
**FS requirements:** FS-PTY-007, FS-PTY-008
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with one pane running `sleep 3600`.

**Steps:**
1. Attempt to close the tab (Ctrl+Shift+W) or the pane (Ctrl+Shift+Q).
2. Observe whether a confirmation dialog appears.
3. Click "Cancel". Verify the pane is still open with `sleep 3600` still running.
4. Attempt close again. Click "Close anyway".
5. Verify the pane closes and the process is terminated (no zombie process).

**Expected result:** A confirmation dialog always appears when a process is running. Cancel preserves the session. "Close anyway" terminates the process and closes the pane/tab. If no process is running (pane in Terminated state), no dialog is shown — pane closes immediately.

**Note:** [BLOCKED: stub] — requires PTY process-detection to be implemented (reading the child PID state).

---

#### TEST-SPRINT-005
**FS requirements:** FS-TAB-006
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm open with one tab whose title is "bash".

**Steps:**
1. Double-click the tab title. Verify an inline text input appears in place of the label.
2. Type "My Session" and press Enter. Verify the tab now shows "My Session".
3. Run `printf "\033]0;Override\007"` in the terminal. Verify the custom label "My Session" still takes precedence.
4. Double-click the tab title again. Clear all text (Backspace) and press Enter.
5. Verify the tab reverts to the process-driven title ("Override" or "bash").
6. Right-click the tab. Select "Rename". Verify the inline input appears. Press F2 while the tab is focused. Verify the inline input also appears.
7. While the inline input is open, press Escape. Verify the input closes without modifying the label.

**Expected result:** All three entry points (double-click, F2, context-menu Rename) trigger inline rename. User label takes precedence over process title. Clearing the label reverts to process title. Escape cancels without side effects.

---

#### TEST-SPRINT-006
**FS requirements:** FS-TAB-005
**Layer:** E2E
**Priority:** Must

**Preconditions:** Three tabs open in order: Tab A, Tab B, Tab C.

**Steps:**
1. Drag Tab C (by its label area) leftward past Tab B. Verify the drag ghost is visible.
2. Drop Tab C between Tab A and Tab B. Verify the order becomes Tab A, Tab C, Tab B.
3. Drag Tab A to the rightmost position. Verify order becomes Tab C, Tab B, Tab A.
4. Verify that after each reorder, the active pane content (PTY session) follows its tab and remains interactive.
5. Reorder via keyboard: focus Tab B via Tab key, press Alt+Shift+Left. Verify Tab B moves one position left.

**Expected result:** Tab drag-and-drop updates order in `SessionRegistry` and re-renders the tab bar. The PTY session follows its tab. No data loss occurs. Keyboard reorder also works.

---

#### TEST-SPRINT-007
**FS requirements:** FS-KBD-003
**Layer:** Frontend Unit
**Priority:** Must

**Preconditions:** The shortcut-intercept module is active in a simulated TerminalPane context.

**Steps:**
1. Simulate Ctrl+Shift+D keydown. Assert the event is consumed (not sent to PTY). Assert the split-horizontal action fires.
2. Simulate Ctrl+Shift+E keydown. Assert consumed. Assert split-vertical action fires.
3. Simulate Ctrl+Shift+Q keydown. Assert consumed. Assert close-pane action fires.
4. Simulate Ctrl+Shift+Right keydown. Assert consumed. Assert focus-next-pane action fires.
5. Simulate Ctrl+Shift+Left keydown. Assert consumed. Assert focus-prev-pane action fires.
6. Simulate Ctrl+Tab keydown. Assert consumed. Assert next-tab action fires.
7. Simulate F2 keydown while a tab is focused. Assert consumed. Assert inline-rename action fires.
8. For each shortcut above, verify the encoded byte sequence is NOT sent to `send_input`.

**Expected result:** All seven pane-management shortcuts are consumed before PTY encoding. The PTY receives no bytes for any of these key combinations.

---

#### TEST-SPRINT-008
**FS requirements:** FS-KBD-002
**Layer:** Frontend Unit + Integration
**Priority:** Must

**Preconditions (unit):** `preferences/shortcuts.ts` conflict detection module.
**Preconditions (integration):** A `PreferencesStore` with a custom shortcut map.

**Steps (unit — conflict detection):**
1. Define two shortcuts both bound to Ctrl+Shift+D. Assert `detectConflicts()` returns a conflict report naming both actions.
2. Bind the same key to Ctrl+Shift+T for a user-defined action. Assert a conflict is reported against the built-in new-tab shortcut.
3. Resolve conflicts and call `normalizeShortcuts()`. Assert canonical form (sorted modifiers, consistent case).

**Steps (integration — persistence):**
1. Call `update_preferences` with a `shortcuts` patch containing `{ "splitHorizontal": "Ctrl+Shift+H" }`.
2. Call `get_preferences`. Assert `shortcuts.splitHorizontal` equals `"Ctrl+Shift+H"`.
3. Write `preferences.json` directly. Reload via `PreferencesStore::load_or_default()`. Assert the custom shortcut survives the round-trip.
4. Write `preferences.json` with an invalid shortcut value `{ "splitHorizontal": 99 }`. Assert load replaces it with the default and emits a WARN log.

**Expected result:** Conflict detection prevents overlapping bindings. Custom shortcuts persist through the IPC round-trip and through file load. Invalid shortcut values fall back to defaults without crashing.

---

#### TEST-SPRINT-009
**FS requirements:** FS-PREF-003, FS-PREF-006
**Layer:** Integration + E2E
**Priority:** Must

**Preconditions (integration):** A `VtProcessor` initialized with a `TerminalPrefs` struct.
**Preconditions (E2E):** TauTerm open. Preferences panel accessible.

**Steps (integration — cursor shape):**
1. Create a `VtProcessor` with `cursor_shape = CursorShape::Underline`.
2. Inspect `ModeState::cursor_shape`. Assert it equals `Underline`.
3. Feed `\x1b[2 q` (CSI 2 SP q — steady block). Assert `ModeState::cursor_shape` is now `SteadyBlock` (VT sequence overrides preference mid-session).
4. Reset the terminal (soft reset `\x1bc`). Assert cursor shape reverts to the preference value `Underline`.

**Steps (integration — bell type):**
1. Create a `VtProcessor` with `bell_type = BellType::Audible`.
2. Feed `\x07` (BEL). Assert the bell action is `BellAction::Audible`.
3. Create a `VtProcessor` with `bell_type = BellType::None`.
4. Feed `\x07`. Assert no bell action is triggered (neither visual nor audible).

**Steps (integration — cursor blink rate):**
1. Create a `VtProcessor` with `cursor_blink_rate = 400` (ms).
2. Assert the `ModeState::cursor_blink_interval_ms` equals 400.

**Steps (E2E):**
1. Open Preferences → Terminal Behavior. Change Cursor Shape to "Underline". Observe the active pane cursor immediately.
2. Change Bell Type to "Audible". Press Ctrl+G in the terminal. Verify the system emits an audible bell (or at minimum the bell action is fired).
3. Change Bell Type to "Visual". Press Ctrl+G. Verify a visual flash occurs instead.
4. Change Bell Type to "None". Press Ctrl+G. Verify no bell action occurs.
5. Change Cursor Blink Rate to 800ms. Observe the cursor blinking at a noticeably slower rate.

**Expected result:** All three preference settings are wired end-to-end. Changes apply immediately to all open panes without restart. VT sequences can override cursor shape mid-session; soft reset restores the preference value.

---

#### TEST-SPRINT-010
**FS requirements:** FS-SSH-031, FS-SSH-032
**Layer:** E2E
**Priority:** Must

**Preconditions:** TauTerm is open. The ConnectionManager UI component is mounted in TerminalView.

**Steps:**
1. Verify the ConnectionManager panel is accessible from the main TerminalView (e.g. via a sidebar toggle button or the "SSH" entry in the status bar).
2. Open the ConnectionManager. Verify the list of saved connections is displayed.
3. Click "New connection". Fill in host, port, username. Save. Verify the new entry appears in the list.
4. Click "Connect" on the saved entry. Verify a new tab opens and transitions through the SSH connection states.
5. Open the ConnectionManager while a tab is in SSH Connected state. Verify the active connection is visually marked.
6. Close the ConnectionManager. Verify no layout shift in the TerminalView (panes retain their sizes).

**Expected result:** ConnectionManager is correctly mounted and renders without errors. CRUD operations work. Opening a connection creates a new tab. The panel can be opened and closed without disturbing existing panes.

**Note:** [BLOCKED: stub] — SSH connect flow requires full SSH lifecycle implementation. CRUD operations on saved connections can be tested independently.

---

#### TEST-SPRINT-011
**FS requirements:** FS-THEME-003, FS-THEME-004, FS-THEME-005, FS-THEME-006
**Layer:** E2E + Frontend Unit
**Priority:** Must

**Preconditions (E2E):** TauTerm open with Preferences panel accessible.
**Preconditions (unit):** `theming/validate.ts` module.

**Steps (E2E):**
1. Open Preferences → Themes. Verify the Umbra default theme is listed and cannot be deleted.
2. Click "New theme". Verify a theme editor opens with all required token fields (background, foreground, cursor, selection, 16 ANSI palette colors, plus UI chrome tokens).
3. Set background to `#1a1a2e` and foreground to `#eaeaea`. Click "Save as…" with the name "Test Theme". Verify the theme appears in the list.
4. Click "Apply". Verify the terminal rendering updates immediately (background and foreground color change). No restart required.
5. Click "Edit" on "Test Theme". Change the cursor color. Click "Save". Verify the change applies immediately.
6. Click "Duplicate" on "Test Theme". Verify a copy "Test Theme (copy)" appears.
7. Delete "Test Theme (copy)". Verify it disappears from the list.
8. Attempt to delete the Umbra default theme. Verify a delete action is absent or disabled.

**Steps (unit — validator):**
1. Construct a theme object with all required tokens present and valid CSS colors. Assert validation passes.
2. Remove the `--color-bg` token. Assert validation fails with a "missing required token" error.
3. Set `--color-bg` to `not-a-color`. Assert validation fails with an "invalid CSS color" error.
4. Set `--color-bg` to a hardcoded hex `#000000` bypassing the token system. If the validator checks that values reference tokens rather than raw values, assert it fails; otherwise this is a documentation note that the token layer enforces this at design time.

**Expected result:** Theme editor allows full CRUD on user-defined themes. Default theme cannot be deleted. Changes apply immediately. Validator correctly rejects incomplete or malformed themes.

---

#### TEST-SPRINT-012
**FS requirements:** FS-I18N-006 (Language enum), FS-PREF-006 (BellType enum), FS-THEME-001 (UserTheme)
**Layer:** Integration (Rust) + Frontend Unit
**Priority:** Must

**Preconditions:** `ipc_type_coherence.rs` integration test file in `src-tauri/tests/`.

**Steps (Rust — serde serialization coherence):**
1. Serialize `Language::En` → assert JSON is `"en"`.
2. Serialize `Language::Fr` → assert JSON is `"fr"`.
3. Deserialize `"en"` → assert `Language::En`. Deserialize `"de"` → assert deserialization fails (unknown variant, not silently mapped to default).
4. Serialize `BellType::Audible` → assert JSON is `"audible"`.
5. Serialize `BellType::Visual` → assert JSON is `"visual"`.
6. Serialize `BellType::None` → assert JSON is `"none"`.
7. Deserialize `"laser"` into `BellType` → assert deserialization fails with an unknown variant error.
8. Serialize a `UserTheme` struct containing `name = "Test"` and a valid `tokens` map → verify JSON has keys `name` and `tokens` at the top level (no extra nesting, no snake_case/camelCase drift).
9. Deserialize the JSON produced in step 8 back into `UserTheme`. Assert all fields survive the round-trip without mutation.

**Steps (Frontend — TypeScript shape):**
1. Mock `invoke('get_preferences')` to return the JSON from step 8.
2. Assert that the TypeScript type `Language` accepts `"en"` and `"fr"` and rejects anything else at compile time (enforced by the `Language` union type, not a string).
3. Assert that the TypeScript type `BellType` accepts `"audible"`, `"visual"`, `"none"` only.

**Expected result:** All three enum/struct types serialize to their exact expected JSON shape. Unknown variants are rejected at deserialization — no silent coercion. TypeScript types are structurally consistent with the Rust JSON output.

---

#### TEST-SPRINT-013
**FS requirements:** FS-PANE-001, FS-PANE-003
**Layer:** Frontend Unit + E2E
**Priority:** Must

**Preconditions (unit):** `layout/split-tree.ts` loaded with a mock pane state.
**Preconditions (E2E):** TauTerm open with one tab.

**Steps (unit — split-tree.ts):**
1. Create a tree with a single leaf (pane A). Assert `getRootNode()` returns a `LeafNode`.
2. Split pane A horizontally (left/right). Assert the root is now a `SplitNode` with `direction: "horizontal"` and two `LeafNode` children with ratios summing to 1.0.
3. Split the right child (pane B) vertically. Assert the tree is `SplitNode(horizontal) → [LeafNode(A), SplitNode(vertical) → [LeafNode(B), LeafNode(C)]]`.
4. Drag the horizontal divider to ratio 0.3/0.7. Assert `updateRatio(nodeId, 0.3)` updates the split node and clamps to [0.1, 0.9].
5. Close pane C. Assert the parent vertical split is removed and pane B is promoted to a leaf in its place. Assert the resulting tree is `SplitNode(horizontal) → [LeafNode(A), LeafNode(B)]`.
6. Close pane B. Assert the root becomes `LeafNode(A)` (no dangling split node).

**Steps (E2E):**
1. Press Ctrl+Shift+D. Verify two panes appear side-by-side.
2. Focus the right pane. Press Ctrl+Shift+E. Verify the right pane splits into top and bottom.
3. Drag the vertical divider between the two right panes. Verify both panes resize and the PTY sessions within them resize accordingly (verified by running `tput cols` / `tput lines`).
4. Drag the horizontal divider. Verify the left pane and the right pane group resize together.
5. Close the bottom-right pane (Ctrl+Shift+Q). Verify the top-right pane expands to fill the freed space. Verify the left pane is unaffected.
6. Verify all pane borders use the correct design tokens (`--color-pane-border-active` for focus, `--color-pane-border-inactive` otherwise).

**Expected result (unit):** `split-tree.ts` manages the arborescent layout correctly for all mutation operations (split, resize, close, promote). (E2E) The visual layout matches the tree state. Draggable dividers correctly propagate resize to PTY sessions. No orphaned split nodes.

**Note:** [BLOCKED: partial stub] — E2E drag-divider steps require the layout renderer to be wired to `split-tree.ts`. Unit tests on `split-tree.ts` alone are unblocked.

---

## 5. Regression Policy

### 5.1 Pre-commit Gate

Before any commit that touches `src-tauri/` or `src/`:
- `cargo nextest run` must pass with zero failures in `src-tauri/`.
- `pnpm vitest run` must pass with zero failures in `src/`.
- `cargo clippy -- -D warnings` must produce zero warnings.
- `cargo fmt -- --check` must report no formatting differences.
- `pnpm prettier --check src/` must report no formatting differences.

### 5.2 E2E Gate

E2E tests (`pnpm wdio`) must pass in the E2E CI job before a feature branch may be merged. The E2E job requires a production build (`pnpm tauri build` completing successfully).

### 5.3 Coverage Expectations

There are no hard numeric line coverage targets enforced by tooling. Coverage is assessed by judgment: every path in a state machine transition table must have at least one test. Every FS requirement marked **Must** must have at least one test scenario in this document that covers its acceptance criteria.

### 5.4 Flaky Tests

A test that fails non-deterministically is treated as a bug. The failing test is:
1. Temporarily isolated from the pass gate (not skipped — annotated with `#[ignore = "flaky: issue #N"]`).
2. Investigated with priority to determine the root cause (race condition, timing dependency, test isolation violation).
3. Fixed and restored to the pass gate within the same sprint.

No test is permanently skipped without an explicit tracking issue and a milestone for resolution.

### 5.5 Adding New Tests

Every pull request that introduces a new FS requirement or modifies an existing one must include corresponding test updates. The test-engineer reviews the coverage delta as part of the PR review.

---

## 6. CI Integration

### 6.1 Pipeline Structure

```
CI pipeline
│
├─ [unit] Rust unit + integration tests
│    cargo nextest run --all
│    cargo clippy -- -D warnings
│    cargo fmt -- --check
│    Runs on: every push to every branch
│    No display server required
│
├─ [unit] Frontend unit tests
│    pnpm vitest run
│    pnpm prettier --check src/
│    pnpm check (TypeScript/Svelte type checking)
│    Runs on: every push to every branch
│
├─ [build] Production build gate
│    pnpm tauri build
│    Runs on: every push to main and dev branches, and PR merge candidates
│    Produces: AppImage artefact (x86_64 baseline)
│
└─ [e2e] E2E test suite
     Requires: [build] job to have completed successfully
     pnpm wdio
     Runs on: PR merge candidates and main branch
     Environment: Linux desktop with display server (X11 or Wayland), D-Bus session
```

### 6.2 Architecture-specific E2E Jobs

Per FS-DIST-003, the AppImage must run on five architectures. An E2E smoke test job runs per architecture after the release build:

| Architecture | Runner strategy | Smoke test scope |
|---|---|---|
| x86_64 | Native x86_64 | Full E2E suite |
| i686 | x86_64 runner + multilib | Smoke: launch + basic PTY |
| aarch64 | Native ARM64 or QEMU | Smoke: launch + basic PTY |
| armhf | QEMU | Smoke: launch + basic PTY |
| riscv64 | QEMU | Smoke: launch + basic PTY |

The x86_64 job runs the full E2E suite. Other architectures run a reduced smoke test due to QEMU overhead: launch TauTerm, verify a terminal pane opens, type a command and observe output.

### 6.3 Security Fuzzing (cargo-fuzz)

A dedicated fuzzing job runs the `cargo-fuzz` targets (to be defined with `security-expert`):
- `fuzz_vt_processor`: feed arbitrary byte sequences to `VtProcessor::process()`, verify no panics or memory safety violations.
- `fuzz_preferences_load`: feed arbitrary JSON to `PreferencesStore::load()`, verify no panics and always returns a valid `Preferences` struct.
- `fuzz_osc_parser`: feed oversized/malformed OSC sequences, verify 4096-byte limit is enforced.

These jobs run nightly (not on every push) due to run time.

### 6.4 CI Failure Policy

- Any failure in the [unit] jobs blocks all other CI jobs.
- Any failure in the [build] job blocks the [e2e] job.
- E2E failures on non-x86_64 architectures (smoke tests) produce a warning, not a hard block, during the early QEMU integration phase. This tolerance will be tightened to a hard block once QEMU runners are stable.
- Fuzzing failures (panics, sanitizer violations) are immediately escalated to the security-expert and test-engineer. The affected code path is blocked from merging until the fix is verified.

### 6.5 Blocked Test Tracking

All tests marked **[BLOCKED: stub]** are tracked as open CI issues. When the corresponding stub is replaced with a real implementation, the blocked tests are activated and added to the relevant CI job. The test-engineer owns the activation checklist:

| Test ID(s) | Blocked by | Target milestone |
|---|---|---|
| TEST-PTY-002, TEST-PTY-005–008 | PTY I/O implementation | PTY milestone |
| TEST-SSH-001–010 (most) | SSH lifecycle implementation | SSH milestone |
| TEST-CRED-001–003 | Secret Service integration | Credentials milestone |
| All E2E tests | Production build functional | Build milestone |
| TEST-SPRINT-001–004, TEST-SPRINT-007–008 | PTY spawn / process detection | PTY milestone |
| TEST-SPRINT-013 | Split layout arborescent renderer wired | Layout milestone |
