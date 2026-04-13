# CLAUDE.md — src-tauri/

Rust backend for TauTerm. PTY management, VT parser, screen buffer, Tauri commands and events.

For build/test/lint commands, see the root [`CLAUDE.md`](../CLAUDE.md#commands).

## Module structure

- `lib.rs` — app setup: plugin registration, `generate_handler![]`, `run()` entrypoint
- `main.rs` — thin entrypoint delegating to `tau_term_lib::run()`
- Module pattern: use `module_name.rs` + `module_name/` subdirectory for submodules — no `mod.rs`

### Modules

| Module | Responsibility |
|---|---|
| `session.rs` | PTY session lifecycle (spawn, resize, close) |
| `vt.rs` | VT parser — ANSI/xterm escape sequence processing |
| `commands.rs` + `commands/` | Tauri `#[command]` handlers: `session_cmds`, `input_cmds`, `preferences_cmds`, `connection_cmds`, `ssh_cmds`, `ssh_prompt_cmds`, `system_cmds`, `testing` |
| `events.rs` + `events/types.rs` | Backend→frontend event definitions and emission |
| `ssh.rs` | SSH connection management |
| `credentials.rs` | Credential storage (SecretService/keyring) |
| `preferences.rs` | User preferences load/save/defaults |
| `error.rs` | Shared error types (`#[derive(Serialize)]`) |
| `platform.rs` + `platform/` | Platform-specific code: PTY (`pty_linux`, `pty_macos`, `pty_windows`), clipboard, credentials, notifications |
| `security_load.rs` | Security policy loading |
| `security_static_checks.rs` | Compile-time security invariants |
| `webview_data_dir.rs` | WebKitGTK data directory isolation |

## Safety rules

- Never cast `char as u8` — use `u8::try_from(c)` (returns `Err` on non-ASCII instead of silently truncating). If an `is_ascii()` guard is already present upstream, `as u8` is acceptable but `try_from` is still preferred. Truncation corrupts VT parsing.
- Never downcast a trait object via raw pointer (`*mut dyn Trait as *mut ConcreteType`). Add the required accessor method to the trait instead. (`Any::downcast_mut` is not a drop-in — it requires the trait to inherit `Any` and expose `as_any(&self)`; prefer the trait-method approach.)
- Never mutate a `DashMap` entry with `remove` + `insert` — use `get_mut()` for in-place mutation (atomicity + no dangling-reference window). For insert-or-update semantics, use `entry().and_modify(…).or_insert(…)`.
- Never call `.expect()` on `try_lock()` — it panics whenever the lock is contended. If blocking is acceptable, use `.lock().await`. If non-blocking is intentional, handle the `Err` explicitly.
- Guard against `usize` underflow before any subtraction: check `value >= delta` before `value - delta`, or use `saturating_sub` when clamping to zero is the correct semantics.
- Every `unsafe` block must carry a `// SAFETY:` comment explaining the invariant that makes it sound. `unsafe` is only acceptable in platform-specific modules (`platform/pty_*.rs`, `platform/clipboard_*.rs`); it must not appear in business-logic modules.
- No `unwrap()` on user-facing data — use `?` or explicit error handling.

## Constraints

- Rust edition: **2024** — prefer its idioms (precise capturing, `impl Trait` in closures, etc.)
- When bumping `rust-toolchain.toml` channel version, verify that a matching `rust:<version>-slim-bookworm` image exists on Docker Hub **before** editing — both `Containerfile.ssh-test` and `Containerfile.keyring-test` must be updated in the same commit.
- Tests: `cargo nextest run` exclusively — not `cargo test`
- Lint: `cargo clippy -- -D warnings` must pass clean
- Keep IPC commands serializable with `serde` — no raw pointers or OS handles across the boundary
- Never log filesystem paths that include usernames or home directories. Log the filename or a generic label only (e.g. `"preferences.json"`, not `path.display()`).
