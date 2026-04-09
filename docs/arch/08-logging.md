<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Logging and Observability Strategy

> Part of the [Architecture](README.md).

---

## 15. Logging and Observability

### 15.1 Instrumentation Library

TauTerm uses the [`tracing`](https://docs.rs/tracing) crate as its sole logging/instrumentation facade. Concrete output is wired at startup in `lib.rs` via `tracing_subscriber::fmt()`. No other logging crate (`log`, `env_logger`, `simple_logger`) may be introduced — all new diagnostic output must go through `tracing` macros.

### 15.2 Semantic Level Convention

Each level has a specific semantic contract; violating it degrades log signal quality.

| Level | Semantic contract | Default visibility |
|-------|------------------|--------------------|
| `error!` | Irrecoverable state, data corruption, or unexpected panic boundary. **Always** visible in production. | prod + dev |
| `warn!` | Anomaly that was recovered from, or a degraded service path (e.g. fallback to defaults, failed optional operation). **Always** visible in production. | prod + dev |
| `info!` | Significant one-time startup/shutdown event with operational value (file loaded, initial tab created). **Restricted to startup events** — not repeated on hot paths. | dev only by default |
| `debug!` | Internal state transitions useful during development (auth step entered, PTY EOF, SSH channel closed). | dev only |
| `trace!` | Maximum granularity (per-byte, per-frame). Reserved for isolated opt-in debugging; must never appear in production. | opt-in only |

**Rule:** any message that fires on a user action (keypress, preference change, SSH connect/disconnect) must be `debug!` or lower, not `info!`. The `info!` level is reserved for startup-time events that are emitted once per process lifetime.

### 15.3 Default Filter by Build Profile

The tracing filter is set at runtime in `lib.rs`. The correct strategy by build profile:

| Build | `RUST_LOG` set? | Effective filter |
|-------|----------------|-----------------|
| debug (`debug_assertions = true`) | No | `tau_term_lib=info,warn` — own crate at INFO, all deps at WARN |
| release | No | `warn` — ERROR + WARN only, from all crates |
| Any | Yes | `RUST_LOG` value takes precedence (always honored) |

**Rationale:** dependencies (`russh`, `tokio`, `tungstenite`, `zbus`) are extremely verbose at DEBUG/INFO. A global `info` default in release builds emits noise from these crates. The correct scoped filter for dev is `tau_term_lib=info,warn`; for release it is `warn`.

**ADR:** See [ADR-0018](adr/ADR-0018-logging-filter-per-build-profile.md).

**Implementation note for `rust-dev`:** the filter initialization in `lib.rs` must use `cfg!(debug_assertions)` to select the default:

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

### 15.4 Level Classification of Existing Call Sites

#### Correctly levelled

| File | Call | Level | Verdict |
|------|------|-------|---------|
| `lib.rs:120` | `Failed to create initial tab during setup` | `error!` | Correct — unrecoverable startup failure |
| `lib.rs:136` | `Could not restore fullscreen state` | `warn!` | Correct — recovered anomaly |
| `events.rs:34–125` | `Failed to emit <event>` | `error!` | Correct — IPC emission failure is unexpected |
| `ssh/manager/connect.rs:138` | `Failed to save password to keychain` | `warn!` | Correct — optional operation failure |
| `ssh/manager/connect.rs:326` | `Pubkey auth error` | `warn!` | Correct — auth method failure |
| `ssh/manager/connect.rs:345` | `Keyboard-interactive auth error` | `warn!` | Correct — auth method failure |
| `ssh/known_hosts/store.rs:124,139,232` | Various known-hosts read errors | `warn!` | Correct |
| `session/pty_task/reader.rs:74,85,141` | PTY reader/writer mutex poisoned, PTY read error | `error!` | Correct |
| `session/pty_task/reader.rs:133` | PTY emit task anomaly | `warn!` | Correct |
| `preferences/store/io.rs:30,47,58,86,98,122,131,143` | Preferences read/parse errors and fallbacks | `warn!` | Correct |
| `preferences/store/validation.rs:23–105` | Field clamping on load | `warn!` | Correct |
| `commands/session_cmds.rs:43,107` | SSH close failed on tab/pane close | `warn!` | Correct |
| `commands/ssh_prompt_cmds.rs:90` | Host key prompt anomaly | `warn!` | Correct |
| `session/registry/shell.rs:36` | Invalid `$SHELL`, fallback | `warn!` | Correct |
| `commands/ssh_cmds.rs:59` | Keychain lookup failed | `warn!` | Correct |
| `preferences/store.rs:95` | Preferences path undetermined | `warn!` | Correct |
| `ssh/auth.rs:95` | Exceeded keyboard-interactive rounds | `warn!` | Correct |
| `session/pty_task/reader.rs:80,148,154,237` | PTY EOF, task finished | `debug!` | Correct |
| `ssh/manager/connect.rs:92,105,322,338` | Auth step debug | `debug!` | Correct |
| `vt/search/api.rs:52` | Invalid search regex | `debug!` | Correct |
| `platform/notifications_linux.rs:35` | D-Bus unavailable | `debug!` | Correct |
| `session/ssh_task.rs:147,159` | SSH channel closed / shell exited | `debug!` | Correct |

#### Incorrectly levelled — must be reclassified

| File | Line | Message | Current | Should be | Reason |
|------|------|---------|---------|-----------|--------|
| `preferences/store/io.rs:40` | 40 | `"Migrating preferences from preferences.json to preferences.toml…"` | `info!` | `warn!` | Migration signals a legacy state that the user may not be aware of. Qualifies as a `warn!` (one-time anomaly path). |
| `preferences/store/io.rs:54` | 54 | `"No preferences file found, using defaults."` | `info!` | `debug!` | Normal first-run path. Not an anomaly. Not operationally significant in production. |
| `preferences/store/io.rs:139` | 139 | `"Loaded preferences from preferences.toml"` | `info!` | `debug!` | Normal hot path at startup. Expected steady-state. No value in production. |
| `ssh/manager/connect.rs:152` | 152 | `"connect_task: emitting credential-prompt event"` | `info!` | `debug!` | Per-connection flow step. Fires on every SSH connect that requires credentials. Verbose in production for multi-tab use. |
| `ssh/manager/connect.rs:176` | 176 | `"connect_task: auth input received from user prompt"` | `info!` | `debug!` | Internal state transition in auth flow. |
| `ssh/manager/connect.rs:181` | 181 | `"connect_task: credential prompt cancelled (sender dropped)"` | `info!` | `warn!` | Sender dropped is an unexpected cancellation path. Qualifies as `warn!` — it means the auth pipeline was aborted in an unclean way. |
| `ssh/manager/connect.rs:186` | 186 | `"connect_task: credential prompt timed out"` | `info!` | `warn!` | Timeout is an anomaly (no user response), not a normal flow step. |
| `commands/preferences_cmds.rs:60` | 60 | `"scrollback_lines preference updated to {effective}; applies to new panes…"` | `info!` | `debug!` | Fires on every preference update that touches `scrollback_lines`. Not operationally significant in production. |

### 15.5 Security Constraints

These rules apply at all log levels:

1. **No credentials in logs.** `Credentials` structs (password, private key path) must never be formatted by `tracing` macros at any level. The `Credentials` type must either not implement `Debug`, or its `Debug` implementation must redact sensitive fields. See FS-CRED-004.
2. **No full filesystem paths.** Log the filename or a generic label, never `/home/<user>/…` or any path containing a username. See CLAUDE.md security constraints.
3. **No PII in structured fields.** `tracing` field syntax (e.g., `pane_id = %pane_id`) is acceptable; user-controlled content (username, hostname) is acceptable as it is operational data. Passwords and key material are not.

See security test protocol: `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md` §SEC-CRED-005.

### 15.6 Performance Considerations

In the `tracing` crate, a log call at a level that is filtered out still constructs a `Record` object before the filter is checked at the callsite. For hot-path code (PTY read loop, per-frame VT processing), even disabled `debug!` or `trace!` calls carry a small overhead.

**Rule:** `trace!` must not appear inside the PTY read loop or the VT `process()` call. If fine-grained PTY tracing is temporarily needed during development, it must be guarded with a compile-time feature flag or removed before merge.

`debug!` on hot paths (PTY reader, SSH I/O) is acceptable because the PTY loop runs on a dedicated blocking thread and is not on the async critical path.

The `tracing` crate supports `max_level` static filtering via the `STATIC_MAX_LEVEL` feature. TauTerm does not currently use it (it would require a Cargo feature to compile out trace/debug calls). This is a future optimization if profiling reveals measurable overhead from disabled log record construction.

### 15.7 Future: Structured Logging for Security Events

M2 from the security review (`docs/fs/05-scope-constraints.md` §8.1) recommends that the following events be logged with structured fields for forensic purposes:

- SSH authentication failure (host, username, method, attempt count)
- Host key accepted/rejected after TOFU change (host, old fingerprint, new fingerprint, pane_id)
- OSC 52 write rejected (pane_id, reason)
- CSI 21t read-back blocked (pane_id)

These are already partially implemented via `warn!` calls in `ssh/known_hosts/store.rs` and the security command handlers. When implementing M2 fully, use `tracing` structured fields rather than `format!` strings so log aggregators can index them.

---

*This section is owned by the TauTerm software architect. Any change to the default filter, the level convention, or the `tracing_subscriber` initialization strategy requires updating this document and, if the change is architectural, a new ADR.*
