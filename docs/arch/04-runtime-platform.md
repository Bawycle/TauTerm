<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Concurrency Model, Platform Abstraction Layer, and Build Architecture

> Part of the [Architecture](README.md).

---

## 6. Concurrency Model

### 6.1 VT Processing Pipeline

#### C1 control codes (0x80–0x9F)

TauTerm operates in a UTF-8 environment. Bytes in the range 0x80–0x9F are **not** interpreted as 8-bit C1 control codes (i.e., CSI/OSC/DCS 8-bit equivalents). They are treated as the leading bytes of UTF-8 multi-byte sequences. If the `vte` crate exposes an option to disable 8-bit C1 processing, it must be disabled. This avoids conflicts between C1 8-bit code processing and valid UTF-8 multi-byte sequences, which share the same byte range.

#### DCS dispatch

For v1, DCS sequences are handled as follows:
- The `vte::Perform` callbacks `dcs_hook`, `dcs_put`, and `dcs_unhook` are no-ops for all unrecognized DCS sequences — they are silently ignored.
- DECRQSS (Device Control Request Status String) receives an error response: the reply `P0$r<params>` with `P0` indicating "invalid request". This is the correct response for unsupported parameters and prevents applications from hanging waiting for a response.
- No other DCS sequences are recognized in v1.

#### Tokio Runtime

The backend uses a single Tokio multi-threaded runtime (standard `#[tauri::main]` setup). All async operations (PTY reads, SSH I/O, keepalive tasks) run on Tokio's thread pool.

### 6.2 PTY I/O Task

`PaneSession` holds an `Arc<RwLock<VtProcessor>>`. The `PtyReadTask` receives a clone of this `Arc` at creation time. Each `PaneSession` has a dedicated blocking task (`tauri::async_runtime::spawn_blocking`) that runs the PTY read loop on Tokio's blocking thread pool:

```
PtyReadTask (per pane, on a Tokio blocking thread):
  loop {
    let n = reader.lock().read(&mut buf);          // synchronous blocking read (OS or mpsc channel)
    {
      let mut proc = vt_processor.write();          // write lock — held briefly
      proc.process(&buf[..n]);                     // VtProcessor: parse + update ScreenBuffer
      let dirty = proc.take_dirty_cells();         // collect cell diffs
      drop(proc);                                  // release write lock
      if !dirty.is_empty() {
        app_handle.emit("screen-update", ScreenUpdateEvent { ... });
      }
    }
  }
```

**Why `spawn_blocking` and not `tokio::spawn`:** `portable-pty`'s master reader is a synchronous `Box<dyn Read + Send>` that blocks the OS thread. Wrapping it in `spawn_blocking` keeps Tokio's async worker threads unblocked. `tauri::async_runtime::spawn_blocking` is used (rather than `tokio::task::spawn_blocking`) because `setup()` runs on the main GTK thread, which has no Tokio thread-local context (`Handle::current()` panics); Tauri's API uses a stored static handle that works from any thread.

The write lock is held only for the duration of `process()` + `take_dirty_cells()` — a short, CPU-bound window. Command handlers (e.g., `get_pane_screen_snapshot`) acquire the read lock for snapshots or search; there is no structural conflict between the read task and command handlers. The `ScreenBuffer` is never accessed outside the `RwLock`.

### 6.3 Write Path (Input)

`send_input` command handler runs on Tokio's thread pool (Tauri spawns command handlers on the runtime). It acquires a write handle to the `PtySession` and writes synchronously (the PTY write is fast and non-blocking for small payloads). No lock contention with the read task.

### 6.4 SSH I/O

SSH sessions have their own async task structure managed by the SSH library (`russh` / `ssh2-rs`). The SSH channel output is piped to a `VtProcessor` in the same way as local PTY output. The `SshConnection` state machine runs in a separate Tokio task per connection.

### 6.5 Back-pressure

The backend emits `screen-update` events at the rate that PTY output arrives. At high terminal output rates (e.g., `cat /dev/urandom | head -c 10M`), this can produce many events per second. The frontend renderer must be able to process these without queuing unbounded events. Mitigation strategies:

1. **Coalescing:** The backend PTY read task coalesces multiple reads into a single event if reads complete faster than the event loop can process them. This is implemented by processing all available bytes before emitting a single event.
2. **Rate limiting:** If event frequency exceeds a configured threshold (e.g., 60 events/s per pane), the backend coalesces further before emitting.
3. **Frontend rendering:** The frontend does not re-render on every individual event; it uses `requestAnimationFrame` batching to render at most once per frame.

Back-pressure between the PTY read and the Tauri event system is a known performance risk (noted in ADR-0001). It requires profiling during development.

**Resize debounce:** `resize_pane` IPC calls from `TerminalPane`'s `ResizeObserver` are debounced by the backend (`session/resize.rs`): a 16–33ms Tokio timer is reset on each incoming call; `TIOCSWINSZ` + SIGWINCH are only issued after the timer fires (FS-PTY-010). The final size is always applied.

**Scroll follow semantics (`scroll.svelte.ts`):** when new output arrives on the normal screen and the user has scrolled back (viewport is not at bottom), the viewport stays at its current scroll position — it does not follow new output. A visual indicator ("new output below") is shown. The viewport follows output automatically only when already at the bottom. The user returns to the bottom via scroll action or keyboard shortcut. When on the alternate screen buffer, scroll navigation is disabled entirely (FS-SB-005).

**Combining characters and run-merge boundaries (`TerminalRow.svelte`):** combining characters (Unicode codepoints of width 0, categories Mn/Mc/Me) are stored in the `Cell` of the preceding base character, not in an independent cell. During attribute-run merging, a width-0 cell never starts a new `<span>` run — it is always folded into the preceding cell's run. This guarantees that the browser text shaping engine receives grapheme-complete sequences in each `<span>`, ensuring correct glyph rendering for accented characters and diacritics (FS-VT-012).

### 6.6 State Access Patterns

| State | Owner | Access pattern |
|-------|-------|---------------|
| `SessionRegistry` | `State<Arc<SessionRegistry>>` | `Arc` for multi-command access, internal `RwLock` per tab |
| `VtProcessor` (per pane) | `Arc<RwLock<VtProcessor>>` in `PaneSession` | Write lock: `PtyReadTask` (process + take_dirty_cells, brief). Read lock: command handlers (snapshot, search). |
| `ScreenBuffer` (per pane) | `VtProcessor` (internal) | Accessed only via the `RwLock` on `VtProcessor`. Never accessed directly from outside. |
| `PreferencesStore` | `State<Arc<RwLock<PreferencesStore>>>` | Read: many readers. Write: preferences command handler |
| `SshManager` | `State<Arc<SshManager>>` | `Arc` + internal `DashMap` for per-connection state |

---

## 7. Platform Abstraction Layer

See ADR-0005 for the full rationale. This section documents the four PAL traits and their Linux v1 implementations. ADR-0005 lists four OS primitives; the fourth (notifications) is documented in §7.4 below.

### 7.1 PtyBackend / PtySession

**Linux v1 implementation:** `portable-pty` crate (`UnixPtySystem`).

The `PtySession` trait wraps the `portable-pty` `MasterPty` and `Child` handles. Resize is delegated to `MasterPty::resize()`. The master file descriptor is exposed as a `tokio::io::unix::AsyncFd` for the PTY read task.

**SIGHUP delivery on close:** closing a `PtySession` (via `Drop` or explicit close) must close the master file descriptor. This is the kernel mechanism that delivers SIGHUP to the foreground process group (FS-PTY-007). The implementation must verify that `portable-pty`'s ownership model closes the underlying fd on `Drop` of the `MasterPty` handle — not merely dropping a Rust wrapper that leaves the fd open. If `portable-pty` does not guarantee this, an explicit `close(fd)` call must be issued in the `PtySession::Drop` implementation before the `portable-pty` handle is dropped.

**Login shell:** the first tab launches a login shell (FS-PTY-013). Since `portable-pty`'s `CommandBuilder` does not natively support the POSIX argv[0] prefix convention (prepending `-` to the shell name), the v1 mechanism is to pass `--login` as an explicit argument: e.g., `CommandBuilder::new("/bin/bash").args(["--login"])`. Subsequent tabs and panes launch interactive non-login shells (no `--login` flag). This behavior is implemented in `session/spawn.rs`.

**Future (Windows):** `portable-pty`'s `ConPtySystem` provides Windows ConPTY support behind the same API. The `PtySession` implementation switches; no other code changes.

### 7.2 CredentialStore

**Linux v1 implementation:** `secret-service` crate (D-Bus Secret Service API, compatible with GNOME Keyring and KWallet).

If the Secret Service is unavailable (`is_available()` returns `false`), TauTerm prompts for credentials on each connection attempt per FS-CRED-005. Credentials are stored in a `SecVec<u8>` (zeroed on drop) during authentication and cleared immediately after the handshake completes.

**Future (macOS):** `keychain-services` or `security-framework` crate. **Future (Windows):** `windows-credentials` crate.

### 7.3 ClipboardBackend

**Linux v1 implementation:** `arboard` crate handles both X11 and Wayland. For X11 PRIMARY selection (FS-CLIP-004), `arboard`'s `SetExtX11` API or a direct `x11-clipboard` crate integration is used; API availability must be verified at implementation time.

**Future (macOS/Windows):** `arboard` supports both natively.

### 7.4 NotificationBackend

**Trait:**

```rust
pub trait NotificationBackend: Send + Sync {
    fn notify(&self, title: &str, body: &str) -> Result<()>;
}
```

**Linux v1 implementation:** D-Bus `org.freedesktop.Notifications` interface (`notify-rust` crate or direct D-Bus call). If D-Bus is unavailable at startup, `create_notification_backend()` returns a no-op implementation that silently discards notifications (no error returned to callers).

**Usage:** `VtProcessor` triggers `NotificationBackend::notify()` when it receives BEL (0x07) from a pane that is not currently active (FS-VT-090, FS-VT-093). The pane-active check is performed by the `PtyReadTask` before invoking the backend — the `NotificationBackend` itself is stateless.

**Future (macOS):** `NSUserNotification` / `UNUserNotificationCenter`. **Future (Windows):** Win32 toast notifications.

### 7.5 PAL Injection

All four traits are registered in Tauri's managed state at startup in `lib.rs`:

```rust
tauri::Builder::default()
    .manage(platform::create_pty_backend())           // Arc<dyn PtyBackend>
    .manage(platform::create_credential_store())      // Arc<dyn CredentialStore>
    .manage(platform::create_clipboard_backend())     // Arc<dyn ClipboardBackend>
    .manage(platform::create_notification_backend())  // Arc<dyn NotificationBackend>
    .manage(SessionRegistry::new())
    .manage(PreferencesStore::load_or_default())
    .manage(SshManager::new())
    .invoke_handler(tauri::generate_handler![...])
    .run(ctx)
```

**`PreferencesStore::load_or_default()`:** if the preferences file does not exist, loading returns a default instance. If the file exists but is invalid (corrupted TOML, I/O error), the error is logged and a default `PreferencesStore` is returned — TauTerm does not crash on preference corruption. See §7.6.

### 7.6 PreferencesStore Load Strategy

`PreferencesStore::load_or_default()` replaces the original `load().expect(...)` call. The strategy:

1. Attempt to read `~/.config/tauterm/preferences.toml` (XDG_CONFIG_HOME).
2. If the TOML file does not exist, attempt to read `~/.config/tauterm/preferences.json` (legacy migration path). If the JSON file is found and parses successfully, its content is used as the initial preferences; the TOML file will be written on the next `save_to_disk` call (i.e., the first `update_preferences` or theme/connection save after launch). The JSON file is not deleted automatically.
3. If neither file exists: return `PreferencesStore::default()`.
4. If the file exists but cannot be read (I/O error) or cannot be parsed (TOML/JSON error): log the error at `WARN` level and return `PreferencesStore::default()`. The corrupted file is not deleted automatically; the user retains it for inspection.
5. On successful load: validate values against schema ranges (see [§8.1](06-appendix.md#81-ipc-boundary-validation)). Out-of-range values are replaced with defaults, and a `WARN` log entry is emitted per replaced field.

**Format mandate:** preferences must be serialised as **TOML** (`preferences.toml`). JSON is accepted only as a one-way migration source. New code must never write `preferences.json`.

This strategy satisfies [§9.1](02-backend-modules.md#91-rust-backend) (no `unwrap()` on filesystem data) and prevents application startup failure due to preference corruption (FS-SEC-003).

### 7.7 Preference Propagation Model

When `update_preferences` is called, the new `Preferences` value is persisted and the command returns the full updated struct to the frontend. Propagation to existing sessions is field-dependent:

| Category | Propagation mechanism |
|---|---|
| Font (`fontFamily`, `fontSize`) and `opacity` | Handled entirely on the frontend via CSS variable updates. The backend is not involved. |
| `themeName`, `language`, `wordDelimiters`, `bellType`, `confirmMultilinePaste`, `keyboard.bindings` | Frontend-side: reactive state derived from the returned `Preferences` value takes effect on the next relevant event (render, keydown, paste, BEL). |
| `cursorStyle` | Backend propagates to all currently open sessions via `SessionRegistry::set_cursor_style_all()` (or equivalent), called from the `update_preferences` command handler after persisting the change. On the frontend, the cursor shape token updates immediately. DECSCUSR overrides from terminal applications remain in effect until a terminal reset, at which point the shape reverts to the preference value. |
| `allowOsc52Write` | Backend propagates to all currently open sessions via `SessionRegistry::set_osc52_write_all()` (or equivalent), called from the `update_preferences` command handler. Takes effect for the next OSC 52 write sequence received by any session. |
| `scrollbackLines` | **Not propagated to existing sessions.** The `ScreenBuffer` capacity (scrollback line count) is fixed at construction time. Changing this preference only affects panes opened after the change. This is a known architectural constraint — the ScreenBuffer is not designed to support runtime capacity changes. The UI must communicate this constraint to the user (see FS-PREF behavioral constraints table and §7.6.3 of the UXD). **Valid range: [100, 1,000,000].** Values outside this range are silently clamped by `validate_and_clamp` (`preferences/store/validation.rs`) before persistence. The `update_preferences` command returns the effective (post-clamp) `Preferences` struct — the frontend **must** display the value from the returned struct, not the originally submitted value. This is the correct mechanism; no separate error code is emitted for a clamped value. |
| `fullscreen` | Tracks the last fullscreen state for restoration at next launch. The live window state is managed by the OS window manager; `update_preferences` does not trigger a window state change. |

**Design rule:** the `update_preferences` command handler is the only place where preference persistence and session propagation are coupled. Individual Tauri commands must not write to `PreferencesStore` directly — all preference mutations flow through `update_preferences` (or the theme/connection sub-commands for those specific sub-keys).

---

## 10. Build Architecture

### 10.1 Pipeline

```
pnpm tauri build
  │
  ├─ Vite build (frontend)
  │    SvelteKit static adapter → build/
  │    Tailwind 4 CSS processing
  │    TypeScript compilation
  │    Tree-shaking, minification (production)
  │
  └─ Cargo build (src-tauri/)
       Rust edition 2024
       Release profile: opt-level = 3, LTO = thin
       Output: src-tauri/target/release/tau-term
       Tauri bundles: AppImage, .deb, .rpm (Linux)
       AppImage: requires "appimage" in bundle.targets (tauri.conf.json) — see §10.6 and ADR-0014
```

### 10.2 Development Mode

```
pnpm tauri dev
  │
  ├─ Vite dev server (localhost:1420) — HMR for frontend changes
  └─ Cargo incremental build — Rust recompile on change, process restart
```

Frontend-only iteration: `pnpm dev` — Vite dev server only, no Tauri backend. IPC calls fail gracefully (mock stubs required for frontend-only development).

### 10.3 Profiles

| Profile | Rust opt-level | LTO | Debug info | Use |
|---------|---------------|-----|-----------|-----|
| debug | 0 | off | full | Development, fast iteration |
| release | 3 | thin | none | Distribution builds |

### 10.4 Testing

See [../testing/TESTING.md](../testing/TESTING.md) — Testing Strategy for the complete test organization, pyramid rationale, per-layer coverage targets, CI gate definition, and no-regression policy.

### 10.5 Internationalisation (i18n)

**Library:** Paraglide JS (`@inlang/paraglide-sveltekit`) — the idiomatic i18n solution for SvelteKit. It performs compile-time message extraction and generates fully tree-shakeable, zero-runtime-cost string accessor functions. See ADR-0013.

**Locale files:** `src/lib/i18n/messages/en.json` (source, fallback) and `src/lib/i18n/messages/fr.json`. Both are JSON objects mapping message keys to string values. Keys use snake_case and are namespaced by UI area (e.g., `"prefs.language.label"`, `"tab.new"`, `"ssh.state.connecting"`).

**Loading strategy:** Paraglide generates typed accessor functions at build time from the JSON catalogues. The compile step (`pnpm exec paraglide-js compile`) is run automatically via the Vite plugin integration during `pnpm dev` and `pnpm tauri build`. The generated output lives in `src/lib/paraglide/` and must not be hand-edited. At runtime, the active locale is a Svelte 5 reactive value stored in `src/lib/state/locale.svelte.ts`. The locale value is initialised on mount from `preferences.appearance.language` (IPC: `get_preferences`) and defaults to `"en"` if missing or unknown (FS-I18N-006). Locale switching (FS-I18N-004) updates the reactive locale value; all components that consume message accessors re-render automatically via Svelte 5's fine-grained reactivity.

**Frontend string resolution:** Components import message accessor functions from the Paraglide-generated module (e.g., `import * as m from '$lib/paraglide/messages'`) and call them as plain functions (`m.prefs_language_label()`). There is no runtime lookup table and no string interpolation at the framework level — strings are resolved to their target-locale value at the call site.

**Tauri integration:** Locale files are static frontend assets bundled by Vite. No Rust-side i18n is required: all user-visible strings live in the frontend. The backend emits string keys (error codes, status codes) which the frontend maps to locale strings via its own message catalogue. This keeps the IPC contract locale-agnostic. The backend never reads or modifies PTY environment variables (`LANG`, `LC_*`) based on the UI language selection (FS-I18N-007).

**Persistence:** The active locale is saved to `preferences.toml` under `appearance.language` via the standard `update_preferences` command. On next launch, `get_preferences` returns the saved locale; the frontend restores it before first render.

**IPC safety — `language` field:** The `language` field on `AppearancePrefs` MUST NOT be a free `String` across the IPC boundary. It MUST be deserialised on the Rust side to an enum validated against the known allowlist:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Fr,
}
```

With `#[serde(default)]`, any unknown locale code in `preferences.toml` (e.g., `"de"`) deserialises to `Language::En` instead of propagating an arbitrary string through the IPC layer and into the frontend (FS-I18N-006). The serialised form remains the lowercase string (`"en"` / `"fr"`).

**Module map additions:**

```
src/
  lib/
    i18n/
      messages/
        en.json         — English message catalogue (source, fallback) (FS-I18N-001, FS-I18N-002)
        fr.json         — French message catalogue (FS-I18N-002)
    paraglide/          — Paraglide-generated code (build artefact; not hand-edited)
    state/
      locale.svelte.ts  — reactive locale state; setLocale(lang) writes to preferences;
                          getLocale() returns current locale (FS-I18N-003, FS-I18N-004, FS-I18N-005)
```

### 10.6 Distribution: AppImage

**Artefact:** One AppImage binary for x86_64 (FS-DIST-003). Naming convention: `TauTerm-{version}-{arch}.AppImage`. ARM64 (aarch64) is supported for source builds only — no AppImage artefact is distributed for that architecture.

**Bundler:** Tauri's native AppImage bundler. Configured via `bundle.targets: ["appimage"]` in `tauri.conf.json`. No external toolchain (`appimagetool`, `linuxdeploy`) is required. See ADR-0014.

**Runtime dependency:** WebKitGTK (`libwebkit2gtk-4.1` on Ubuntu 22.04+, `libwebkit2gtk-4.0` on older distributions). This is the only dependency not bundled in the AppImage — it is a standard component of any GNOME-compatible Linux desktop environment. All other dependencies (Rust binary, frontend assets, locale JSON files, application icon, `.desktop` entry) are bundled (FS-DIST-002, FS-DIST-005).

**Multi-architecture build strategy:** Cross-compilation is avoided. Tauri's AppImage bundler requires WebKitGTK headers and libraries matching the target architecture at build time, making cross-compilation impractical without a full matching sysroot.

The CI strategy is as follows (FS-DIST-003):

| Architecture | Rust target triple | CI artefact | CI runner strategy |
|---|---|---|---|
| x86_64 | `x86_64-unknown-linux-gnu` | AppImage released | Native x86_64 runner |
| ARM64 (aarch64) | `aarch64-unknown-linux-gnu` | Source build only — no AppImage | Native ARM64 runner (`ubuntu-24.04-arm`) |

The release publishes a single AppImage artefact (`TauTerm-{version}-x86_64.AppImage`). The ARM64 CI job validates that the source builds and tests pass on aarch64, but produces no distributable package.

**Minimum supported WebKitGTK version:** `libwebkit2gtk-4.1 >= 2.38` (Ubuntu 22.04+) or `libwebkit2gtk-4.0 >= 2.38` (older distributions). Version 2.38 is the threshold that introduced post-2022 WebKit security patches addressing multiple CVE-class vulnerabilities. Distributions shipping an older WebKitGTK release are not officially supported; TauTerm may run but security properties are not guaranteed. The CI build environment enforces this minimum by targeting Ubuntu 22.04 as the baseline.
