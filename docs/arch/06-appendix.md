<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Security Architecture, Future Extensibility, and ADR Index

> Part of the [Architecture](README.md).

---

## 8. Security Architecture

### 8.1 IPC Boundary Validation

Every `#[tauri::command]` that accepts user-provided data applies validation at entry:

- **Path inputs** (identity file path in `SshConnectionConfig`): resolved to absolute path, checked for path traversal components (`..`), verified to point to a regular file (FS-CRED-006, FS-SEC-003).
- **URI inputs** (hyperlink URIs): scheme whitelisted to `http`, `https`, `mailto`, `ssh`; `file` scheme only for local sessions; length ≤ 2048 bytes; no C0/C1 characters (FS-VT-073).
- **Tab titles** (from OSC sequences via the VtProcessor): C0/C1 stripped, truncated to 256 characters (FS-VT-062).
- **IPC sequence length**: OSC and DCS sequences are limited to 4096 bytes in the VtProcessor (FS-SEC-005).
- **Preferences on load**: validated against a schema; out-of-range values replaced with defaults (FS-SEC-003). See [§7.6](04-runtime-platform.md#76-preferencesstore-load-strategy) for the load strategy.

**`PreferencesStore` structure:** The `Preferences` struct (defined in `preferences/schema.rs`) owns the following top-level keys in `preferences.toml`:

| Sub-key | Type | Description |
|---------|------|-------------|
| `appearance` | `AppearancePrefs` | Font, font size, cursor style, theme name, opacity, language |
| `terminal` | `TerminalPrefs` | Scrollback size, `allow_osc52_write`, word delimiters, bell type |
| `keyboard` | `KeyboardPrefs` | Shortcut bindings |
| `connections` | `Vec<SshConnectionConfig>` | Saved SSH connections. **Authoritative source for connection configs** — `SshManager` reads and writes this list via `State<PreferencesStore>`; it holds no independent connection store. |
| `themes` | `Vec<UserTheme>` | User-defined themes |

### 8.2 PTY Isolation

- Master file descriptors opened with `O_CLOEXEC` (FS-SEC-002). The `portable-pty` crate applies this by default; verify at implementation.
- Child processes have no access to other panes' PTY fds, the application's D-Bus connection, or credential memory.
- OSC 52 clipboard read is permanently rejected in the VtProcessor (FS-VT-076). OSC 52 clipboard write policy (FS-VT-075): for local PTY sessions (no saved connection), write is controlled by the global preference `allow_osc52_write: bool` (default: `false`). For saved connections (local or SSH), a per-connection `allow_osc52_write` flag overrides the global default. This prevents a global "enabled" setting from inadvertently enabling OSC 52 write in SSH sessions where it was not explicitly authorized.

### 8.3 SSH Security

- Host key verification is TOFU, stored in `~/.config/tauterm/known_hosts` (OpenSSH-compatible format). TauTerm does **not** read `~/.ssh/known_hosts` automatically at startup or during connection (FS-SSH-011). The Preferences UI offers an explicit "Import from OpenSSH" action that copies entries from `~/.ssh/known_hosts` into TauTerm's own known-hosts file on user request; this is the sole interaction with the OpenSSH file (`ssh/known_hosts.rs`). Once imported, entries are managed independently.
- Deprecated algorithm detection (FS-SSH-014): `ssh-rsa` with SHA-1 and `ssh-dss` trigger a non-blocking in-pane warning.
- SSH agent forwarding is permanently disabled (FS-SEC-004).
- Credentials are never logged, never embedded in IPC payloads, and never cached beyond the authentication handshake (FS-CRED-003, FS-CRED-004).

### 8.4 Content Security Policy

The WebView CSP is configured in `tauri.conf.json` and tightened incrementally:

**v1 minimum (per FS-SEC-001):**
```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
connect-src ipc: http://ipc.localhost;
img-src 'self' asset: http://asset.localhost;
```

`unsafe-inline` for styles is required by Tailwind 4's runtime token injection. `unsafe-eval` and inline scripts are permanently forbidden.

**Future tightening:** As features stabilize, `style-src 'unsafe-inline'` should be replaced with a nonce-based policy if WebKit's support allows. Each capability grant in `capabilities/default.json` is audited when new commands are added.

### 8.5 Terminal Injection Prevention

- Property read-back sequences (CSI 21t, OSC queries that echo into PTY input, DECRQSS responses) are permanently silently discarded in the VtProcessor (FS-VT-063). These are a known injection vector.
- Tab titles set via OSC are sanitized before display (FS-VT-062).
- Multi-line paste confirmation (FS-CLIP-009) prevents accidental command execution from untrusted paste content.

---

## 12. Future Extensibility

This section documents the planned extension points for features that are out of scope for v1 but must not require architectural rework.

### 12.1 Session Persistence (Post-v1)

Session persistence requires serializing the `SessionRegistry` state to disk (tab topology, pane types, working directories) and restoring it on startup. The extension point is the `SessionRegistry`: adding `fn serialize_to_disk() -> Result<()>` and `fn restore_from_disk() -> Result<()>`. The `VtProcessor` screen buffer state (current screen content) cannot be fully restored without replaying PTY output, which is not feasible. Restoration will recreate the session structure but not the terminal content.

Architecture readiness: `SessionRegistry`, `TabState`, `PaneNode`, `PaneState`, `SshConnectionConfig` are all `Serialize`/`Deserialize`. No structural change is required.

### 12.2 Plugin / Extension System (Post-v1)

A plugin system would allow third parties to add new session types (e.g., serial port connections), custom tab title formatters, or additional IPC commands. The extension point is the `PtyBackend` trait (ADR-0005): any new session type that can satisfy `PtySession` can be integrated into the `SessionRegistry` without changing the core. Command registration in `lib.rs` would need to support dynamic command registration, which Tauri currently does not support natively; this may require a different plugin approach (e.g., IPC via a local socket to a plugin process). This is noted as an open design problem for the plugin system version.

### 12.3 Cloud Sync (Post-v1 — explicitly out of scope)

Preferences and saved connections are stored in `~/.config/tauterm/preferences.toml` (TOML format, validated on load). A cloud sync feature would add a sync layer above `PreferencesStore`. The `PreferencesStore` interface (`get`, `apply_patch`, `get_themes`, `save_theme`, `delete_theme`) is the abstraction boundary. No structural change is required; a `SyncedPreferencesStore` could wrap the base store.

### 12.4 Kitty Keyboard Protocol (Post-v1)

The Kitty protocol requires changes to the VT parser (new mode flags) and the key encoding logic in the frontend. The `VtProcessor`'s `Perform` implementation is the extension point; new mode flags would be added to the terminal mode state ([§5.3](03-ipc-state.md#53-vt-terminal-mode-state)). No structural change is required.

### 12.5 Windows / macOS Port (Post-v1)

See ADR-0005. The PAL stubs are in `platform/pty_macos.rs`, `platform/credentials_macos.rs`, `platform/clipboard_macos.rs` and their Windows equivalents. The Tauri framework handles the WebView layer. The SSH library (`russh` or `ssh2-rs`) is already cross-platform. The `portable-pty` crate provides ConPTY on Windows. The primary porting work is implementing the PAL trait implementations for each platform; all other code is platform-agnostic.

---

## 13. ADR Index

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-0001](adr/ADR-0001-tauri-2-as-application-framework.md) | Tauri 2 as application framework | Accepted |
| [ADR-0002](adr/ADR-0002-pty-native-rust.md) | PTY management in native Rust | Accepted |
| [ADR-0003](adr/ADR-0003-vt-parser-library.md) | VT parser: use the `vte` crate | Accepted |
| [ADR-0004](adr/ADR-0004-svelte-5-runes-frontend-state.md) | Svelte 5 runes as frontend state management | Accepted |
| [ADR-0005](adr/ADR-0005-platform-abstraction-layer.md) | Platform Abstraction Layer for OS primitives | Accepted |
| [ADR-0006](adr/ADR-0006-ipc-coarse-grained.md) | Coarse-grained IPC: one command per user action | Accepted |
| [ADR-0007](adr/ADR-0007-ssh-via-rust-ssh-library.md) | SSH implementation via pure-Rust SSH library | Accepted |
| [ADR-0008](adr/ADR-0008-terminal-rendering-strategy.md) | Terminal rendering strategy: DOM-based with row virtualization | Accepted |
| [ADR-0009](adr/ADR-0009-pane-structure-flat-list.md) | Pane layout structure: flat list with split metadata vs. recursive tree | Accepted |
| [ADR-0010](adr/ADR-0010-session-state-delta-events.md) | `session-state-changed` event: complete TabState vs. partial diff | Accepted |
| [ADR-0011](adr/ADR-0011-scrollback-rust-ring-buffer.md) | Scrollback storage: Rust ring buffer in backend | Accepted |
| [ADR-0012](adr/ADR-0012-preferences-json-file.md) | Preferences persistence: JSON file in XDG_CONFIG_HOME | Superseded by ADR-0016 |
| [ADR-0016](adr/ADR-0016-preferences-toml-format.md) | Preferences persistence: TOML with snake_case keys (supersedes ADR-0012) | Accepted |
| [ADR-0013](adr/ADR-0013-i18n-paraglide-js.md) | i18n library: Paraglide JS (Inlang) | Accepted |
| [ADR-0014](adr/ADR-0014-appimage-tauri-bundler.md) | AppImage distribution via Tauri bundler | Accepted |
