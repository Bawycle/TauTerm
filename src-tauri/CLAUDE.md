# CLAUDE.md — src-tauri/

Rust backend for TauTerm. PTY management, VT parser, screen buffer, Tauri commands and events.

## Module structure

- `lib.rs` — app setup: plugin registration, `generate_handler![]`, `run()` entrypoint
- `main.rs` — thin entrypoint delegating to `tau_term_lib::run()`
- New modules go in `src-tauri/src/` and are declared in `lib.rs`
- Module pattern: use `module_name.rs` + `module_name/` subdirectory for submodules — no `mod.rs`

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
