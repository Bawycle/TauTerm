<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0018 — Logging filter strategy per build profile

**Status:** Accepted
**Date:** 2026-04-09
**Author:** Software Architect — TauTerm team

---

## Context

TauTerm initializes its `tracing_subscriber` in `lib.rs` with a fallback filter of `"info"` when `RUST_LOG` is not set:

```rust
tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
```

This single filter applies identically to debug builds and release builds. The consequence is:

1. **Release builds emit INFO messages**, which violates the industry standard that production binaries default to `WARN` or higher. Users running the AppImage see startup noise (`"Loaded preferences from preferences.toml"`, `"No preferences file found, using defaults."`) that has no operational value.
2. **INFO filter is global**, not scoped to the `tau_term_lib` crate. Dependencies such as `russh`, `tokio`, `zbus`, and `tungstenite` are verbose at INFO and flood the log output in development.

The `RUST_LOG` environment variable is honored when explicitly set — this behavior is correct and must be preserved.

## Options Considered

### Option A — Keep `"info"` as global fallback for all builds (status quo)

**Consequences:**
- Release builds expose INFO messages to end users (noise, minor information disclosure risk).
- Dependencies' INFO messages appear in dev logs, reducing signal-to-noise ratio.
- Simple to reason about.

**Verdict:** Rejected. INFO in release is non-standard and creates avoidable noise.

### Option B — Global `"warn"` fallback for all builds

**Consequences:**
- Release builds correctly default to WARN+.
- Development builds also default to WARN+ — dev loses visibility into own-crate INFO messages (startup events, preferences loading, SSH flow steps).
- Must rely on `RUST_LOG=tau_term_lib=info` for development work, which is a friction point.

**Verdict:** Rejected for dev. Acceptable for release only.

### Option C — Split default by build profile using `cfg!(debug_assertions)` (selected)

Set the default filter based on the build profile:
- **Debug build** (`debug_assertions = true`): `tau_term_lib=info,warn` — own crate at INFO, all other crates at WARN.
- **Release build** (`debug_assertions = false`): `warn` — all crates at WARN+.

`RUST_LOG` always takes precedence when set.

**Consequences:**
- Release builds default to WARN+: correct production behavior, no INFO noise.
- Dev builds see own-crate INFO messages but not dependency noise: better signal-to-noise.
- `cfg!(debug_assertions)` is evaluated at compile time — zero runtime cost.
- Behavioral difference between release and debug is intentional and documented.
- A developer explicitly wanting INFO from deps in dev must set `RUST_LOG=info`.

**Verdict:** Accepted.

## Decision

Option C is adopted. The `lib.rs` tracing initialization must use:

```rust
let default_filter = if cfg!(debug_assertions) {
    "tau_term_lib=info,warn"
} else {
    "warn"
};
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter)),
    )
    .init();
```

## Consequences

### Positive
- Release AppImage no longer emits INFO log lines by default.
- Dev output is scoped: own-crate flow is visible, dependency noise is suppressed.
- `RUST_LOG` override mechanism is unaffected — users and CI can still set any level.

### Negative / Constraints
- A developer working on a dependency integration (e.g., debugging `russh` auth at INFO level) must explicitly set `RUST_LOG=russh=info` or `RUST_LOG=info`. This is documented in `docs/arch/08-logging.md`.
- The behavioral difference between debug and release builds must be kept in mind when interpreting logs from field reports (release users will not see INFO messages by default).

### Implementation responsibility
`rust-dev` is responsible for updating `lib.rs`. No other files require code changes for this ADR. See `docs/arch/08-logging.md` §15.3 for the reference implementation snippet.

### Level corrections
In addition to the filter change, the following `info!` call sites must be reclassified as part of the same work item (see `docs/arch/08-logging.md` §15.4):

| File | Current | Target | Reason |
|------|---------|--------|--------|
| `preferences/store/io.rs:40` | `info!` | `warn!` | Legacy migration path |
| `preferences/store/io.rs:54` | `info!` | `debug!` | Normal first-run, no operational value |
| `preferences/store/io.rs:139` | `info!` | `debug!` | Normal startup, expected steady-state |
| `ssh/manager/connect.rs:152` | `info!` | `debug!` | Per-connection auth flow step |
| `ssh/manager/connect.rs:176` | `info!` | `debug!` | Auth state transition |
| `ssh/manager/connect.rs:181` | `info!` | `warn!` | Unexpected cancellation (sender dropped) |
| `ssh/manager/connect.rs:186` | `info!` | `warn!` | Timeout is an anomaly |
| `commands/preferences_cmds.rs:60` | `info!` | `debug!` | Fires on every pref update — not prod-relevant |
