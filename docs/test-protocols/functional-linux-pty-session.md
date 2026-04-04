# Functional Test Protocol — LinuxPtySession

> **Version:** 1.0.0
> **Date:** 2026-04-04
> **Status:** Active
> **Scope:** `platform/pty_linux.rs` — `LinuxPtyBackend::open_session`, `LinuxPtySession::write`, `LinuxPtySession::resize`, `LinuxPtySession::close`; `session/registry.rs` — `create_tab`, `send_input`; PTY read task (`session/pty_task.rs`); shell path validation wiring in `create_tab`.
> **References:** FS-PTY-001 through FS-PTY-014, FS-VT-001/002/011, FS-SEC-003, ARCHITECTURE.md §6.2, §6.3, §7.1, §7.5

---

## 1. Scope and Prerequisites

### 1.1 What This Protocol Covers

- `LinuxPtyBackend::open_session` — PTY allocation, child process spawn, environment setup
- `LinuxPtySession::write` — keyboard input forwarded to PTY master fd
- `LinuxPtySession::resize` — `TIOCSWINSZ` + SIGWINCH delivery, pixel dimensions
- `LinuxPtySession::close` — master fd drop, SIGHUP delivery
- PTY read task — async read loop, VtProcessor integration, `screen-update` event emission
- `SessionRegistry::create_tab` — wiring of `validate_shell_path`, PTY session spawn
- `SessionRegistry::send_input` — forwarding to `PtySession::write`
- Resize debounce — 16–33ms debounce, final size always applied
- Shell fallback — `$SHELL` invalid or unset falls back to `/bin/sh`
- Environment variables — mandatory set per FS-PTY-011, FS-PTY-012

### 1.2 Out of Scope

- SSH sessions (separate protocol)
- VT parser conformance (covered by existing `functional-pty-vt-ssh-preferences-ui-ipc.md`)
- Screen buffer rendering (frontend concern)
- Multi-pane split layout (tested at registry level only)

### 1.3 Test Environment

- Linux x86_64 (CI and developer workstation)
- Rust nextest (`cargo nextest run`)
- Tests that spawn real PTY processes are marked `#[ignore]` for CI unless the environment is verified; they MUST pass on developer machines
- `$HOME` must be set; `$SHELL` may be overridden per test

---

## 2. Test Scenarios

### 2.1 PtySession Trait — write()

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-W-001 | Call `write()` with a small payload (`b"ls\n"`, ≤ 64 bytes) on a running session | Returns `Ok(())` | FS-PTY-003 |
| FPL-W-002 | Call `write()` with maximum allowed payload (64 KiB) | Returns `Ok(())` | FS-SEC-005 |
| FPL-W-003 | Call `write()` after the child process has exited (PTY master fd is broken) | Returns `Err(PtyError::Io(_))` — no panic, no `unwrap` abort | FS-PTY-005 |
| FPL-W-004 | Bytes written via `write()` are readable on the slave side (round-trip) | `read()` on slave yields the exact bytes written | FS-PTY-003 |

### 2.2 PtySession Trait — resize()

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-R-001 | Call `resize(80, 24, 0, 0)` on a running session | Returns `Ok(())` | FS-PTY-009 |
| FPL-R-002 | Call `resize(cols, rows, pixel_width, pixel_height)` with pixel dims set | `TIOCSWINSZ` receives correct `ws_xpixel` and `ws_ypixel` | FS-PTY-009 |
| FPL-R-003 | Call `resize(0, 0, 0, 0)` (degenerate size) | Returns `Ok(())` without panic — degenerate sizes are forwarded to the kernel | — |
| FPL-R-004 | Call `resize()` on a closed session | Returns `Err(PtyError::Io(_))` or `Err(PtyError::Resize(_))` — no panic | — |
| FPL-R-005 | SIGWINCH is delivered to the child process group after `resize()` | Child process receives SIGWINCH signal (verified via signal handler in test child) | FS-PTY-009 |

### 2.3 PtySession Trait — close()

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-C-001 | `close()` (or `Drop`) closes the master fd | Child process receives SIGHUP (slave's controlling terminal goes away) | FS-PTY-007 |
| FPL-C-002 | After `close()`, writing to the closed session produces an error | Subsequent `write()` on the dropped session is not possible (ownership transferred to `close(self: Box<Self>)`) | FS-PTY-007 |
| FPL-C-003 | Dropping `PtyTaskHandle` aborts the read task | The `tokio::task` is cancelled; no further reads are attempted | ARCHITECTURE.md §6.2 |

### 2.4 LinuxPtyBackend::open_session — Spawn

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-S-001 | `open_session("/bin/sh", &[], env)` on Linux | Returns `Ok(Box<dyn PtySession>)` | FS-PTY-001/002 |
| FPL-S-002 | `open_session("/nonexistent_shell", &[], env)` | Returns `Err(PtyError::Spawn(_))` with descriptive message | FS-PTY-014 |
| FPL-S-003 | Spawned session has independent PTY pair (master + slave) | Two concurrent sessions have different master fds | FS-PTY-001 |
| FPL-S-004 | Child process is spawned with slave PTY as controlling terminal | `tty` in the child outputs the slave device path | FS-PTY-002 |
| FPL-S-005 | Child process inherits `TERM=xterm-256color` | Child `echo $TERM` yields `xterm-256color` | FS-VT-001 |
| FPL-S-006 | Child process inherits `COLORTERM=truecolor` | Child `echo $COLORTERM` yields `truecolor` | FS-VT-002 |
| FPL-S-007 | Child process inherits `TERM_PROGRAM=TauTerm` | Child `echo $TERM_PROGRAM` yields `TauTerm` | FS-PTY-011 |
| FPL-S-008 | Child process inherits `LINES` and `COLUMNS` matching the PTY dimensions | `echo $LINES` matches `rows`; `echo $COLUMNS` matches `cols` | FS-PTY-011 |
| FPL-S-009 | `DISPLAY` is forwarded when set in the parent environment | Child `echo $DISPLAY` yields the parent value | FS-PTY-012 |
| FPL-S-010 | `WAYLAND_DISPLAY` is forwarded when set in the parent environment | Child `echo $WAYLAND_DISPLAY` yields the parent value | FS-PTY-012 |
| FPL-S-011 | `DBUS_SESSION_BUS_ADDRESS` is forwarded when set in the parent environment | Child `echo $DBUS_SESSION_BUS_ADDRESS` yields the parent value | FS-PTY-012 |
| FPL-S-012 | First tab is spawned with `--login` flag | Login shell sources `~/.bash_profile` or equivalent (verified via env var set in profile) | FS-PTY-013 |
| FPL-S-013 | Subsequent tabs/panes spawned without `--login` | Non-login shell does not source `~/.bash_profile` | FS-PTY-013 |

### 2.5 Shell Path Validation — create_tab Wiring

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-V-001 | `create_tab` with `shell: None` falls back to `$SHELL` from environment | Session spawns successfully with the default shell | FS-PTY-014 |
| FPL-V-002 | `create_tab` with `shell: None` and `$SHELL` unset falls back to `/bin/sh` | Session spawns `/bin/sh` successfully | FS-PTY-014 |
| FPL-V-003 | `create_tab` with `shell: Some("/bin/sh")` | Session spawns successfully | — |
| FPL-V-004 | `create_tab` with `shell: Some("bash")` (relative path) | Returns `Err` with code `INVALID_SHELL_PATH` | FS-SEC-003 |
| FPL-V-005 | `create_tab` with `shell: Some("/nonexistent/shell")` | Returns `Err` with code `INVALID_SHELL_PATH` | FS-SEC-003 |
| FPL-V-006 | `create_tab` with `shell: Some("/tmp/nonexec.sh")` (not executable) | Returns `Err` with code `INVALID_SHELL_PATH` | FS-SEC-003 |

### 2.6 SessionRegistry::send_input — Write Path

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-I-001 | `send_input(valid_pane_id, b"ls\n".to_vec())` on a running pane | Returns `Ok(())`; bytes reach the PTY | ARCHITECTURE.md §6.3 |
| FPL-I-002 | `send_input(invalid_pane_id, data)` | Returns `Err(SessionError::PaneNotFound(_))` | — |
| FPL-I-003 | `send_input` with payload of exactly 65536 bytes (64 KiB) | Returns `Ok(())` (at the maximum boundary) | FS-SEC-005 |
| FPL-I-004 | `send_input` with payload of 65537 bytes (over 64 KiB limit) | Returns `Err` with code `INPUT_TOO_LARGE` | FS-SEC-005 |

### 2.7 PTY Read Task — Event Emission

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-E-001 | PTY output is read and processed by `VtProcessor` | `VtProcessor::process()` is called with the received bytes | ARCHITECTURE.md §6.2 |
| FPL-E-002 | After processing dirty cells, a `screen-update` event is emitted via `AppHandle` | `app_handle.emit("screen-update", ...)` is called with correct `pane_id` | ARCHITECTURE.md §6.2 |
| FPL-E-003 | PTY EOF (child process exit) causes the read task to stop cleanly | Task loop exits; no panic; `PtyTaskHandle` can be dropped | ARCHITECTURE.md §6.2 |
| FPL-E-004 | PTY read error (non-EOF I/O error) causes the read task to stop cleanly | Task logs the error and exits; no panic | ARCHITECTURE.md §6.2 |
| FPL-E-005 | Multiple reads in rapid succession are coalesced before emitting | Batch of bytes is processed in a single `process()` call when available; one event per read cycle | ARCHITECTURE.md §6.5 |

### 2.8 Resize Debounce

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-D-001 | Single resize call fires callback after 16ms | Callback called once with the scheduled size | FS-PTY-010 |
| FPL-D-002 | Three rapid resize calls within 16ms | Callback called once with the last (most recent) size | FS-PTY-010 |
| FPL-D-003 | Two resize calls separated by > 33ms | Callback called twice, each with the respective size | FS-PTY-010 |
| FPL-D-004 | Final resize size is always applied (never lost) | After debounce, the terminal grid matches the last resize | FS-PTY-010 |

### 2.9 Error Handling

| ID | Scenario | Expected result | FS ref |
|----|----------|-----------------|--------|
| FPL-ERR-001 | `PtyError::Io` converts to `TauTermError` with code `PTY_IO_ERROR` | `From<PtyError>` impl produces correct code | — |
| FPL-ERR-002 | `PtyError::Spawn` converts to `TauTermError` with code `PTY_SPAWN_FAILED` | `From<PtyError>` impl produces correct code | — |
| FPL-ERR-003 | No `unwrap()` on any error path in `write()`, `resize()`, `open_session()` | Confirmed by `cargo clippy -- -D warnings` + code review | CLAUDE.md |

---

## 3. Acceptance Criteria Summary

A LinuxPtySession implementation is accepted when:

1. All FPL-W-*, FPL-R-*, FPL-C-*, FPL-S-* scenarios pass (or are marked `#[ignore]` with documented rationale for CI environment limitations).
2. All FPL-V-*, FPL-I-*, FPL-E-*, FPL-D-* scenarios pass.
3. `cargo nextest run` exits 0 for the session module.
4. `cargo clippy -- -D warnings` exits 0 with no warnings.
5. No `todo!()`, `unimplemented!()`, or `unwrap()` on user-facing data paths remain.
6. Every new source file carries the SPDX header `// SPDX-License-Identifier: MPL-2.0`.

---

## 4. Test Data and Fixtures

- `/bin/sh` — always present on Linux; used as the reference valid shell
- `/bin/bash` — used when a login shell is required (test for `--login` flag)
- `/tmp/tauterm_nonexec_<n>.sh` — temp file without execute bit (created and removed per test)
- Timeout: PTY integration tests use a 2-second timeout via `tokio::time::timeout`
- Signal delivery verification: test child processes use `signal_hook` or `nix` crate signal handlers

---

## 5. Coverage Gaps and Known Limitations

- **FPL-S-005 through FPL-S-013** require spawning a real shell and reading its output — these are integration tests that may be tagged `#[ignore]` in CI environments without a controlling terminal. They must pass on developer workstations.
- **FPL-E-002** requires a real `AppHandle` or a testable mock. The architecture must inject a mock emitter in tests rather than a full Tauri `AppHandle`.
- `SIGWINCH` delivery (FPL-R-005) requires a live process; this is an integration test.
- Rate-limiting / second-level coalescing (FPL-E-005) is a best-effort test — timing-dependent scenarios use sufficiently long timeouts to be deterministic on a loaded CI system.
