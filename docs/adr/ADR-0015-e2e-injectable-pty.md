<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0015 — E2E Testability via Injectable PTY Backend

**Date:** 2026-04-05
**Status:** Accepted

## Context

TauTerm's E2E test suite (WebdriverIO + tauri-driver) validates terminal rendering end-to-end: bytes enter the VT pipeline, the screen buffer is updated, and the frontend renders the result. The current test infrastructure relies on a real shell process running inside a real PTY, which introduces three problems:

1. **Non-determinism.** Shell startup output (prompts, motd, init scripts) varies across CI environments, distributions, and locale settings. A test that expects a specific string on the terminal grid cannot reliably distinguish it from shell noise.
2. **Timing fragility.** Tests must wait for the shell to reach a stable state before injecting input. Fixed `sleep` calls are brittle; polling on DOM state is better but still depends on shell output timing.
3. **Coupling to the execution environment.** The real PTY spawns a shell process, which requires a valid `$SHELL`, working locale, a writable home directory, and correct environment forwarding. CI containers regularly violate one or more of these assumptions.

The functional test protocol already specifies a test `TEST-PTY-RT-002` (PTY round-trip) that must verify bytes written into the pipeline appear on the terminal grid. This test is currently implemented by sending keystrokes to a real shell — an approach that fails as soon as the shell prompt deviates from the expected format.

What is needed is a way to push synthetic, controlled byte sequences directly into the VT pipeline, bypassing the real PTY, so that E2E tests observe deterministic screen output.

## Decision

Introduce a Cargo feature flag `e2e-testing` that activates two artefacts:

1. **`InjectablePtyBackend`** — a `PtyBackend` implementation that, instead of spawning a real shell, creates an in-process `tokio::sync::mpsc` channel per pane. Output bytes are delivered through this channel, which is backed by a synchronous blocking adapter that satisfies the `std::io::Read` interface already consumed by `spawn_pty_read_task`. The write path (`PtySession::write`) discards bytes silently — send_input calls from the frontend have no effect.

2. **`inject_pty_output` Tauri command** — a feature-gated command that accepts a `pane_id` and a `Vec<u8>` payload, retrieves the mpsc sender for that pane from a dedicated `InjectableRegistry` Tauri state entry, and sends the bytes into the channel. The PTY read task picks them up immediately and feeds them to `VtProcessor`, which emits `screen-update` events to the frontend exactly as it would with real PTY output.

When the `e2e-testing` feature is **not** active (i.e., in all production builds), `InjectablePtyBackend`, `InjectableRegistry`, and `inject_pty_output` are entirely absent from the compiled binary. The production code path — `create_pty_backend()` returning `LinuxPtyBackend` — is unchanged.

The binary produced by `cargo build --features e2e-testing` (from `src-tauri/`) is used as the `binaryPath` in `wdio.conf.ts`. Standard production builds (`pnpm tauri build`) never pass this feature.

## Alternatives considered

**Full IPC mock — stub all Tauri commands in the test harness**
Replace `invoke()` on the frontend with a mock that returns pre-scripted responses, bypassing the backend entirely. This would make tests fast and deterministic but would not validate the actual VT pipeline, screen buffer, or frontend rendering. The entire point of an E2E test is to exercise the real path. Rejected.

**Fixed delays before assertion**
Add `await browser.pause(500)` after sending input to allow the real shell time to respond. This is the current approach and is the source of the fragility we are solving. Does not address locale or environment variance. Rejected as a design target (acceptable as a temporary measure in individual tests where injection is not yet used).

**Sentinel markers in real shell output**
Craft shell scripts that write unique markers (`echo TAUTERM_E2E_MARKER_abc123`) and wait for those markers in the DOM. This reduces — but does not eliminate — sensitivity to environment (locale, shell init files, shell version). It also requires the test to control which shell command runs, which requires solving the environment coupling problem first. This approach is appropriate for integration tests at the PTY level (`src-tauri/tests/`) but is insufficient for E2E tests that must work in a hermetic CI container. Rejected for E2E.

**Separate test binary that bypasses Tauri entirely**
Build a headless Rust binary that exercises the VT pipeline without a WebView. This tests the backend in isolation but does not validate the frontend renderer or the IPC boundary — again, not an E2E test. Rejected.

**`tokio::sync::watch` or `broadcast` instead of `mpsc`**
`watch` is designed for single-value state, not byte stream delivery. `broadcast` requires receivers to be registered before sending and drops messages when no receiver is listening. `mpsc::unbounded_channel` is the correct primitive: it is a reliable, ordered, single-producer single-consumer channel where the sender can push bytes at any time and the receiver (the PTY read task) drains them in order. Rejected in favour of `mpsc::unbounded_channel`.

## Consequences

**Positive:**
- E2E tests gain full control over the bytes entering the VT pipeline. `TEST-PTY-RT-002` and similar rendering tests become deterministic and environment-independent.
- No production code is changed. The feature flag guarantees zero surface area in shipped binaries.
- The pattern is composable: future E2E tests for ANSI sequences, scrollback, search, and resize can all use `inject_pty_output` without spawning a shell.
- The injectable backend exercises the same `spawn_pty_read_task` / `VtProcessor` / `screen-update` path as the production backend, so the test coverage is genuine.

**Negative / risks:**
- A new Cargo feature must be maintained. Any future refactor that changes `PtyBackend`, `PtySession`, or `spawn_pty_read_task` must also update `InjectablePtyBackend`.
- E2E specs that use `inject_pty_output` depend on the `e2e-testing` feature being active. Running those specs against a production binary will fail at the IPC level with "command not found". The `wdio.conf.ts` `beforeSession` hook must enforce this.
- `InjectableRegistry` is a second Tauri state entry. Command handlers that need it must request it explicitly. It is not a replacement for `SessionRegistry`.
- `create_tab` still runs through `SessionRegistry::create_tab` with the `InjectablePtyBackend`. The injectable session does not spawn a shell, so there is no child process, no `SIGHUP` on close, and no `SIGWINCH` on resize — this is correct for E2E tests but must not be confused with the real behaviour when writing test assertions about process lifecycle.

## Notes

The `InjectableRegistry` (a `DashMap<PaneId, mpsc::UnboundedSender<Vec<u8>>>`) is populated by `InjectablePtyBackend::open_session` and consumed by `inject_pty_output`. It must be registered as Tauri state before `SessionRegistry` is created (both are set up in the `setup` closure in `lib.rs`). The two state entries are independent: `SessionRegistry` owns pane lifecycle and the VT processor; `InjectableRegistry` owns only the injection channel senders.
