# Security Test Protocol — LinuxPtySession

> **Version:** 1.0.0
> **Date:** 2026-04-04
> **Status:** Active
> **Scope:** `platform/pty_linux.rs`, `session/registry.rs`, `platform/validation.rs`
> **References:** FS-SEC-001 through FS-SEC-007, FS-PTY-008, FS-PTY-011, FS-PTY-014, ARCHITECTURE.md §8.1, §8.2, CLAUDE.md security constraints

---

## 1. Threat Model Summary

| Threat | Vector | Mitigation |
|--------|--------|-----------|
| Shell path injection | `CreateTabConfig.shell` field accepts arbitrary string | `validate_shell_path()` — absolute path only, executable bit required |
| Path traversal via shell path | `../../bin/malicious` in shell field | `validate_shell_path()` — canonicalization rejects traversal |
| Input size exhaustion | `send_input` with unbounded payload | 64 KiB limit enforced at IPC boundary before write to PTY |
| PTY injection via crafted output | Malicious bytes in PTY output containing escape sequences | VtProcessor sanitizes; OSC/DCS length limit 4096 bytes |
| Resource exhaustion (PTY fd leak) | Tab/pane close without releasing master fd | `close()` / `Drop` must close master fd |
| Privilege escalation via shell spawn | Spawning privileged shell (e.g., `/bin/su`, `/usr/bin/sudo`) | No restriction — spawn is user-level; no privilege escalation possible from user session |
| Error information leakage | Error messages exposing full filesystem paths or internal state | Error messages must not expose absolute paths to untrusted callers |
| Environment variable injection | Untrusted characters in `TERM_PROGRAM_VERSION` or other env vars | Version string must be validated/sanitized before injection into child env |

---

## 2. Security Test Scenarios

### 2.1 Shell Path Validation (path traversal, injection)

| ID | Scenario | Expected result | Threat |
|----|----------|-----------------|--------|
| SPL-PV-001 | `shell: Some("bash")` — relative path | `Err` with code `INVALID_SHELL_PATH`; no spawn attempt | Path injection |
| SPL-PV-002 | `shell: Some("./bash")` — relative path with prefix | `Err` with code `INVALID_SHELL_PATH` | Path injection |
| SPL-PV-003 | `shell: Some("../../etc/bash")` — relative traversal | `Err` with code `INVALID_SHELL_PATH` | Path traversal |
| SPL-PV-004 | `shell: Some("/bin/../bin/sh")` — absolute with traversal component | `validate_shell_path` canonicalizes; if canonical is executable, `Ok`. If not, `Err`. No traversal bypass. | Path traversal |
| SPL-PV-005 | `shell: Some("/etc/passwd")` — non-executable existing file | `Err` with code `INVALID_SHELL_PATH` | Injection |
| SPL-PV-006 | `shell: Some("")` — empty string | `Err` with code `INVALID_SHELL_PATH` | Injection |
| SPL-PV-007 | `shell: Some("/bin/sh\x00extra")` — null byte injection | `Err` — `Path::new` will treat the null byte as part of the path; canonicalize fails or path is rejected | Null byte injection |
| SPL-PV-008 | `shell: Some("/bin/sh; rm -rf /")` — shell metacharacter injection | Not exploitable — command is passed as `argv[0]`, not to a shell interpreter. Canonicalize fails on this non-existent path. | Shell metacharacter |
| SPL-PV-009 | `shell: None` with `$SHELL=/etc/passwd` in environment | Fallback reads `$SHELL` from env, then `validate_shell_path` rejects non-executable | Injection via env |
| SPL-PV-010 | `shell: None` with `$SHELL` unset | Falls back to `/bin/sh`; no error | Robustness |

### 2.2 Input Size Limits

| ID | Scenario | Expected result | Threat |
|----|----------|-----------------|--------|
| SPL-SZ-001 | `send_input` with 65537 bytes (above 64 KiB limit) | `Err` with code `INPUT_TOO_LARGE`; no bytes written to PTY | Resource exhaustion |
| SPL-SZ-002 | `send_input` with exactly 65536 bytes (64 KiB boundary) | `Ok(())` — boundary is inclusive | Boundary correctness |
| SPL-SZ-003 | `send_input` with empty payload (`b""`) | `Ok(())` — no-op write | Edge case |
| SPL-SZ-004 | Rapid sequence of 10 × 64 KiB `send_input` calls | All succeed; no fd exhaustion, no deadlock, no panic. **Timeout: all 10 calls must complete within 5 seconds** (validated in test via `tokio::time::timeout`). | Resource exhaustion |

### 2.3 PTY Injection via Terminal Output

| ID | Scenario | Expected result | Threat |
|----|----------|-----------------|--------|
| SPL-INJ-001 | PTY outputs a sequence exceeding 4096 bytes within a single OSC | `VtProcessor` truncates or discards at the 4096-byte limit | PTY injection |
| SPL-INJ-002 | PTY outputs a DCS sequence with 4097-byte body | `VtProcessor` truncates or discards; no memory exhaustion | PTY injection |
| SPL-INJ-003 | PTY outputs bytes 0x80–0x9F (C1 range) | Treated as UTF-8 multi-byte leading bytes, not as C1 control codes | C1 injection |
| SPL-INJ-004 | PTY outputs crafted OSC 52 (clipboard write) with oversized payload | Rejected or truncated at FS-SEC-005 limit; no write to system clipboard | Clipboard injection |

### 2.4 Resource Management (fd leak, process leak)

| ID | Scenario | Expected result | Threat |
|----|----------|-----------------|--------|
| SPL-RM-001 | Create and immediately close 10 panes | No fd leak: `/proc/self/fd` count returns to baseline. **Baseline measurement procedure:** enumerate `/proc/self/fd` before the test loop; close all 10 panes; enumerate again; assert count ≤ baseline. Run as `#[ignore]` to prevent fd count noise from parallel nextest workers. | fd exhaustion |
| SPL-RM-002 | `LinuxPtySession::close()` drops master fd before function returns | SIGHUP is delivered to child immediately on close | fd leak |
| SPL-RM-003 | `PtyTaskHandle` drop aborts the read task | Tokio task is cancelled; no zombie task accumulation | Resource leak |
| SPL-RM-004 | Open session then immediately drop `LinuxPtySession` (without calling `close()`) | `Drop` must close master fd (kernel guarantees SIGHUP via hangup on last fd close) | fd/process leak |
| SPL-RM-005 | Child process exits; read task detects EOF | Read task exits cleanly; no orphan Tokio task | Zombie task |

### 2.5 Privilege Separation

| ID | Scenario | Expected result | Threat |
|----|----------|-----------------|--------|
| SPL-PS-001 | Child process runs as the same UID as the parent | `getuid()` in child matches `getuid()` of Tauri process | Privilege escalation |
| SPL-PS-002 | Child process does not inherit unexpected file descriptors from the parent | All fds > 2 except the slave PTY fd are closed before exec (FD_CLOEXEC or explicit close) | fd inheritance |

### 2.6 Error Information Leakage

| ID | Scenario | Expected result | Threat |
|----|----------|-----------------|--------|
| SPL-EL-001 | Spawn failure on `/nonexistent` produces error | `TauTermError.detail` may contain OS error string; `TauTermError.message` must not expose full filesystem paths | Info leakage |
| SPL-EL-002 | Shell validation failure produces error | Error code is `INVALID_SHELL_PATH`; message is user-facing plain language; detail contains technical info | Info leakage |
| SPL-EL-003 | PTY I/O error during write | `TauTermError.detail` may contain `errno` description; no full path exposed in `message` | Info leakage |
| SPL-EL-004 | `PtyError::Io` from `std::io::Error` | `to_string()` of `std::io::Error` used only in `detail` field, not in `message` | Info leakage |

### 2.7 Environment Variable Sanitization

| ID | Scenario | Expected result | Threat |
|----|----------|-----------------|--------|
| SPL-ENV-001 | `TERM_PROGRAM_VERSION` is set from the compiled application version string | Version string contains only printable ASCII; no shell metacharacters | Env injection |
| SPL-ENV-002 | `LANG` inherited from parent contains a valid UTF-8 locale string | Accepted; a LANG value containing null bytes or C0 control characters is stripped or replaced with `en_US.UTF-8` | Env injection |

---

## 3. Acceptance Criteria

An implementation passes the security protocol when:

1. All SPL-PV-* scenarios return the correct `Err` variant — no bypass of path validation.
2. All SPL-SZ-* scenarios enforce the 64 KiB input size limit at the `send_input` boundary.
3. SPL-INJ-001 through SPL-INJ-004 confirm the VtProcessor OSC/DCS limit is active.
4. SPL-RM-001 through SPL-RM-005 show no fd leaks and no zombie tasks.
5. SPL-EL-001 through SPL-EL-004 confirm user-facing error messages do not expose sensitive system paths.
6. `cargo clippy -- -D warnings` is clean (no `unwrap()` warnings, no `expect()` on user data).
7. No `todo!()` or `unimplemented!()` remain in any code path reachable from `send_input`, `create_tab`, `open_session`, `write`, `resize`, or `close`.

---

## 4. Security Review Notes

- **FD_CLOEXEC:** `portable-pty` should set `FD_CLOEXEC` on the master fd before fork. This must be verified at implementation time. If not set by the library, an explicit `fcntl(fd, F_SETFD, FD_CLOEXEC)` call must be added.
- **Login shell and `--login`:** passing `--login` as an explicit argument is safe — it is not interpreted as a shell metacharacter because it is passed directly as `argv[1]`, not via a shell command string.
- **`$SHELL` validation:** when reading `$SHELL` from the environment as the default shell, the value must go through `validate_shell_path()` before use. This prevents privilege escalation via a crafted `$SHELL` environment variable.
- **No setuid/setgid:** the PTY spawn must never call `setuid`, `setgid`, or `seteuid`. The child process inherits the effective UID of the Tauri process.
