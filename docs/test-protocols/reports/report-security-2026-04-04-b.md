# Security Test Report — 2026-04-04-b

**Protocol:** `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md` v1.0
**Executed by:** security-expert / rust-dev
**Date:** 2026-04-04
**Branch:** dev
**Follows:** `report-security-2026-04-04.md` (session a)
**Test runner:** `cargo nextest run`
**Result:** 167 tests run — **167 PASS**, 0 FAIL, 0 SKIP

---

## Executive Summary

All 4 security findings from session-a have been addressed in this session. 3 are fully resolved; 1 is partially resolved (FINDING-004 shell path wiring deferred pending `create_tab` shell field implementation).

13 new security-related tests were added. All 167 tests pass. No regression.

---

## Finding Resolution

### FINDING-001 — private_key_path emitted in Debug output

**Status: RESOLVED**

`Credentials::fmt` now redacts `private_key_path`:
```rust
.field("private_key_path", &self.private_key_path.as_deref().map(|_| "<redacted>"))
```
Test `sec_cred_003_private_key_path_redacted_in_debug` verifies the output contains `"<redacted>"` and does NOT contain the actual path. Previously the test pinned the visible-path behaviour; the assertion has been inverted.

**Decision rationale (moe arbitration):** A file path like `/home/alice/.ssh/id_ed25519` reveals username, existence of SSH keys, and filesystem layout. In a terminal emulator where logs may be collected for diagnostics, this constitutes information disclosure that narrows attacker search space post-compromise. Redaction is the correct default; if a developer needs the path for debugging, they can log it explicitly at a controlled callsite.

---

### FINDING-002 — CSP is null

**Status: RESOLVED**

`src-tauri/tauri.conf.json` now has a production-grade CSP:
```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
connect-src ipc: http://ipc.localhost;
img-src 'self' data: asset: http://asset.localhost;
font-src 'self' asset: http://asset.localhost
```

Notes:
- `style-src 'unsafe-inline'` is retained for Tailwind 4 dynamic style injection in dev. Must be reviewed when production build is available — if Tailwind produces a static stylesheet, `'unsafe-inline'` can be dropped.
- `data:` in `img-src` allows inline icons. Acceptable surface — does not enable script execution.
- `ipc:` and `http://ipc.localhost` both included for WebKitGTK compatibility on Linux (Tauri 2 uses `http://ipc.localhost` as the IPC handler scheme on Linux).

Tests `sec_csp_002_unsafe_eval_absent_from_tauri_conf` and `sec_csp_002_script_src_unsafe_inline_absent_or_csp_null` now test against the actual CSP string, not the null case. SEC-CSP-001 and SEC-IPC-001 can be re-evaluated once the app builds end-to-end.

---

### FINDING-003 — send_input has no payload size limit

**Status: RESOLVED**

`validate_input_size(data: &[u8]) -> Result<(), TauTermError>` extracted as a pure, testable function. Called at the top of `send_input` before any registry interaction. Limit: 65,536 bytes (64 KiB).

Three tests added:
- `sec_ipc_006_send_input_oversized_payload_rejected` — 65,537 bytes → Err
- `sec_ipc_006_send_input_at_size_limit_accepted` — 65,536 bytes → Ok
- `sec_ipc_006_empty_payload_accepted` — 0 bytes → Ok

SEC-IPC-006 is now unblocked and passing.

---

### FINDING-004 — Path traversal validation absent

**Status: PARTIAL**

`src-tauri/src/platform/validation.rs` implements:
- `validate_ssh_identity_path(raw: &str) -> Result<PathBuf, TauTermError>` — absolute path required, canonicalize (file must exist), must be within `$HOME/.ssh/`
- `validate_shell_path(raw: &str) -> Result<PathBuf, TauTermError>` — absolute path required, canonicalize, must have executable bit set. **No whitelist** (terminal emulator must support any shell, including fish, nushell, custom builds)

`validate_ssh_identity_path()` is wired into `ssh_cmds.rs::open_ssh_connection()`.

`validate_shell_path()` is **not yet wired** into `create_tab()` because `CreateTabConfig` does not yet have a `shell` field. This will be wired as part of the PTY implementation pass when the shell field is added.

8 tests cover both functions:
- `ssh_identity_path_valid_file_in_ssh_dir` ✓
- `ssh_identity_path_rejects_relative_path` ✓
- `ssh_identity_path_rejects_traversal_even_if_absolute` ✓
- `ssh_identity_path_rejects_path_outside_ssh_dir` ✓
- `ssh_identity_path_rejects_nonexistent_path` ✓
- `shell_path_valid_executable` ✓
- `shell_path_rejects_non_executable_file` ✓
- `shell_path_rejects_relative_path` ✓
- `shell_path_rejects_nonexistent_path` ✓

SEC-PATH-001 (identity_file) is now **unblocked and passing**.
SEC-PATH-002 (shell path) validation logic is implemented and tested, wiring into `create_tab()` deferred to PTY pass — test remains BLOCKED at the integration level.

---

## Updated Threat Coverage Matrix (changes only)

| Threat | SEC-* ID | Status change |
|---|---|---|
| Credentials debug leaks password | SEC-CRED-003 | PASS (was PASS — test updated to verify redaction) |
| `{private_key_path}` in Debug | FINDING-001 | RESOLVED |
| CSP null / no script-src | SEC-CSP-001/002 | RESOLVED (CSP configured); SEC-CSP-001 re-evaluate at E2E |
| send_input oversized payload | SEC-IPC-006 | **PASS** (was BLOCKED) |
| Path traversal in identity_file | SEC-PATH-001 | **PASS** (was BLOCKED) |
| Path traversal in shell path | SEC-PATH-002 | PARTIAL (validation implemented, wiring deferred) |

---

## Remaining Open Findings

None — all 4 session-a findings are resolved or on a tracked milestone.

---

## Blocked Tests (unchanged)

All tests blocked in session-a for infrastructure reasons (PTY stubs, SSH stubs, cargo-fuzz, E2E) remain blocked. See `report-security-2026-04-04.md` §Blocked Tests. Status unchanged.

Newly actionable:
- **SEC-IPC-001** — now that CSP is set, this can be verified once `pnpm tauri build` is functional
- **SEC-PATH-002** — wiring exists, activate when `CreateTabConfig.shell` field is added

---

## Recommendations (updated)

### Immediate (before next feature)

All 4 prior immediate recommendations are resolved. New item:

5. **Wire `validate_shell_path()` into `create_tab()`** when `CreateTabConfig` gains a `shell` field. The validation function exists and is tested — wiring is the only remaining step.

### Pre-release (unchanged)

6. Set up `cargo-fuzz` for VtProcessor (SEC-PTY-008)
7. Run full §3 manual pentest checklist
8. Add `cargo audit` to CI

### For SSH integration pass (unchanged)

9. Implement `known_hosts.rs` TOFU logic (SEC-SSH-001/002)
10. Permanently disable SSH agent forwarding (SEC-SSH-003)
11. Apply `zeroize` to `Credentials` fields (SEC-CRED-002)

### Style-src note

`style-src 'unsafe-inline'` in the current CSP is a known limitation of Tailwind 4 in dev mode. Track whether production `pnpm tauri build` output requires it. If Tailwind produces a fully static stylesheet, replace with a nonce-based CSP or remove `'unsafe-inline'` from `style-src` in the production config.
