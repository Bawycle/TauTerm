# TODO

---

## Scoring System

Each item is scored on four axes (1–3), combined into a weighted composite:

| Axis | 1 | 2 | 3 |
|---|---|---|---|
| **R** Release blocker | not blocking | desirable for v1 | hard blocker — no release without it |
| **S** Security / Correctness | cosmetic / improvement | real bug / architectural debt | security flaw / data corruption |
| **U** User impact | marginal / edge case | common workflow affected | every user / app unusable |
| **E** Effort (inverted) | weeks / major refactor | days | hours / quick win |

**Composite score = R×3 + S×2 + U×1 + E×1** (max 21)

| Band | Score | Label |
|---|---|---|
| 17–21 | Critical — block the release |
| 13–16 | High — must land in v1 |
| 9–12 | Medium — v1 quality target |
| ≤ 8 | Low — post-v1 or nice-to-have |

Items marked **[v1.1]** are scoped to the next minor release.
Items in the **Post-v1 / Roadmap** section are out of scope for v1.

---

## Critical — Release Blockers (score 17–21)

---

## High Priority — Must Land in v1 (score 13–16)

### Distribution

- [ ] **Distribution — GPG signing + SHA256SUMS** `[Score: 13 | R:2, S:2, U:1, E:2]` (FS-DIST-006)
  No signing script exists in CI/CD. Implement in the release pipeline: `SHA256SUMS` generation, GPG signing, publication of signed artifacts.

---

## Medium Priority — v1 Quality Target (score 9–12)

---

## Low Priority — Post-v1 Nice-to-Have (score ≤ 8)

---

## v1.1 — Roadmap Minor Release

*Features absent from current specs, validated by comparative analysis of Tabby, Alacritty, and Hyper. Must be specified in `docs/UR.md` and `docs/fs/` before implementation.*

- [ ] **[v1.1] Clickable hints + OSC 8 (hyperlinks in the terminal)** `[Score: 9 | R:1, S:1, U:3, E:1]`
  The killer feature from Alacritty absent from TauTerm. Two levels:
  - **Passive OSC 8**: recognize OSC 8 sequences (`ESC ] 8 ; params ; uri ST`) emitted by tools like `ls --hyperlink`, `git log`, `delta`, and make the URI clickable (Ctrl+click → open in configured browser/editor). IETF standard, WCAG-compatible, aligned with AD.md §1.3.
  - **Active hints**: on a configurable shortcut, display an overlay of short labels on all URLs/paths detected by regex in the current view — pressing a label triggers the action (open, copy). vim-hints / Alacritty hints style.
  Concerned personas: Alex (stack traces, file paths), Jordan (URLs in logs).
  **Alacritty architecture** (`alacritty_terminal/src/term/cell.rs` for storage, `alacritty/src/display/hint.rs` for the hints overlay) — `CellExtra` with `Option<Arc<HyperlinkInner>>`: URI is lazily stored in cells — `Cell` has `extra: Option<Box<CellExtra>>` allocated only if the cell has non-standard attributes. Ordinary cells have zero memory cost for this field. To adopt: add `hyperlink: Option<Arc<HyperlinkUri>>` (lazy) in TauTerm's `Cell`, without performance impact on normal content.
  **Recommended sequencing:**
  1. **Phase 1 (passive OSC 8)**: parse OSC 8 in `osc.rs` → store URI in `Cell` → expose via IPC → display decorated underline on frontend with Ctrl+click.
  2. **Phase 2 (active hints)**: DOM overlay generated on demand by regex scan of visible buffer → short labels → configurable action.
  Actions required: specify in FS + UXD, implement in two phases.

- [ ] **[v1.1] OSC 1337 — inline images (iTerm2 Inline Images Protocol)** `[Score: 9 | R:1, S:1, U:2, E:2]`
  Absent from `docs/UR.md` and `docs/fs/`. Enables image display in the terminal: used by file managers (`yazi`, `ranger`), Python/Julia plotting tools, and `chafa`. Unlike sixel or TGP, OSC 1337 is well-suited to TauTerm's WebView architecture — the rendering side is trivial (the browser handles PNG/JPEG/GIF natively), the complexity lies in the parser and cell positioning.
  **Protocol**: `ESC ] 1337 ; File=[params]:[base64data] BEL/ST`. Key params: `inline=1` (display vs. download), `width`/`height` (chars, pixels, or percent), `preserveAspectRatio`, `doNotMoveCursor`. Supported formats: PNG, JPEG, GIF, WebP.
  **WezTerm reference** (MIT — compatible with MPL-2.0):
  - Parser: `wezterm-escape-parser/src/osc.rs` — `ITermFileData::parse()`, `ITermProprietary` enum, `ITermDimension`. Clean, well-tested API. The crate is published separately (`wezterm-escape-parser 0.1.0`) — TauTerm can depend on it directly or adapt the parsing logic.
  - Cell assignment: `term/src/terminalstate/iterm.rs` (`set_image()`) + `term/src/terminalstate/image.rs` (`assign_image_to_cells()`) — not reusable as-is (coupled to WezTerm's GPU renderer and `ImageCell` model), but the UV-coordinate-per-cell approach is the right reference for TauTerm's overlay model.
  **TauTerm implementation** (simpler than native terminals):
  1. Parse OSC 1337 in `src-tauri/src/vt/osc.rs` → decode base64 → emit a `inline-image` Tauri event with decoded bytes, MIME type, cell dimensions, cursor position, and `do_not_move_cursor` flag. Validate size before decoding (cap at 100 MB, following WezTerm).
  2. Frontend: position an `<img>` element as an overlay over the terminal grid at the anchor cell. The image scrolls with the terminal content (`position: absolute` relative to the scrollback container). Cells occupied by the image are marked as reserved.
  **Implementation caveats**:
  - OSC 1337 can arrive as multi-chunk `put()` calls — accumulate in `Vec<u8>` before decoding (do not decode prematurely).
  - `inline=0` means download, not display — define TauTerm behavior upfront.
  - Large images sent via IPC as raw `Vec<u8>` may be costly — consider storing in backend and exposing via Tauri asset protocol with a key.
  Actions required: specify in `docs/UR.md` + `docs/fs/`, implement parser in `osc.rs`, add `inline-image` Tauri event, implement frontend overlay renderer.

- [ ] **[v1.1] Session persistence — restore tabs on relaunch** `[Score: 9 | R:1, S:1, U:3, E:1]`
  Absent from `docs/UR.md`. Daily pain point for Alex (4 tabs: frontend, backend, logs, git) and Jordan (10+ SSH sessions). Highly upvoted on Hyper (#311) and expected behavior in Tabby ("Tabby remembers your tabs").
  Target behavior: on close, serialize the tab list (type local/SSH, title, associated connection profile). On relaunch, offer "Restore previous session?" — opt-in, not imposed (Sam won't always want this).
  Note: local PTYs are not restorable (process dead) — only metadata is restored. Saved SSH connections can be relaunched automatically. **Never serialize the VT buffer** — potentially sensitive data + memory cost.
  **Tabby architecture** (`tabby-core/src/services/tabRecovery.service.ts`, `tabby-core/src/api/tabRecovery.ts`, `tabby-core/src/services/app.service.ts`, `tabby-ssh/src/recoveryProvider.ts`) — `TabRecoveryProvider` pattern — discriminated Rust enum:
  ```rust
  #[derive(Serialize, Deserialize)]
  #[serde(tag = "type")]
  enum TabSnapshot {
      LocalPty { title: String, working_dir: Option<String>, shell: String },
      Ssh { connection_id: String, title: String },
  }
  ```
  Stored in `~/.config/tauterm/session.json`. On startup: deserialize → show restore dialog → recreate tabs from snapshots. `working_dir` comes from OSC 7 CWD tracking (dependency: OSC 7 item above).
  Actions required: specify in `docs/UR.md §4.1` + `docs/fs/`, implement `SessionSnapshot` in Rust with `#[serde(tag = "type")]`, add restore dialog on startup.

- [ ] **[v1.1] Pane maximized — enlarge a pane without destroying the split** `[Score: 9 | R:1, S:1, U:2, E:2]`
  Absent from `docs/uxd/03-components.md §7.2`. Alex's workflow: 3 panes open, need temporary focus on one without losing context of the others. Close and recreate destroys VT history.
  Target behavior: shortcut `Ctrl+Shift+Enter` (configurable) toggles the active pane to "maximized" state — it occupies the full split area, others are hidden but not destroyed. A `--color-accent` border + discrete badge signals the state. Same shortcut or `Escape` restores the layout.
  Aligned with AD.md §1.3 "Durability Over Novelty": no state lost, no PTY killed.
  **Tabby reference** (`tabby-core/src/components/splitTab.component.ts`) — state `maximizedTab: BaseTabComponent|null`, method `maximize(tab)` that calls `layout()`.
  **TauTerm translation (Svelte 5, declarative):**
  - State: `let maximizedPaneId = $state<PaneId | null>(null)` in `useTerminalView.core.svelte.ts`
  - `SplitPane.svelte`: each leaf checks `node.paneId === maximizedPaneId` → conditional CSS (`position: absolute; inset: 0; z-index: 6` + `backdrop-filter`, `box-shadow` from design tokens).
  - Shortcut: add `pane-maximize` in `handleGlobalKeydown()` of `useTerminalView.io-handlers.svelte.ts`.
  Actions required: specify in `docs/uxd/03-components.md §7.2` + `docs/uxd/04-interaction.md`, implement layout state in `SplitPane.svelte`.

- [ ] **[v1.1] Jump hosts / ProxyJump SSH in the connection manager** `[Score: 8 | R:1, S:1, U:2, E:1]`
  Absent from `docs/UR.md §9`. Jordan's standard use case: accessing servers on a private network via a bastion. Without ProxyJump in the UI, Jordan configures `~/.ssh/config` manually and TauTerm connections don't match his actual infrastructure.
  Tabby handles jump hosts natively: each saved connection can reference a "jump host" profile, with targeted error messages per chain link.
  `russh` supports ProxyJump — it's a data model problem (add a "via jump host" field in `SshConnectionConfig`) + UI form, not a transport problem.
  **Tabby architecture** (`tabby-ssh/src/components/sshTab.component.ts`, fn `setupOneSession`, `tabby-ssh/src/session/ssh.ts`, data model in `tabby-ssh/src/api/interfaces.ts`) — `direct-tcpip` channel (RFC 4254 §7.2):
  1. Authenticate the SSH session on the bastion normally (host key + auth).
  2. Open a `direct-tcpip` channel via `session.channel_open_direct_tcpip(target_host, target_port, originator, originator_port)`.
  3. Use this channel as TCP transport for a second `russh::client::connect_stream` session toward the final target.
  4. Data model: `jump_host_id: Option<String>` in `SshConnectionConfig` (references another saved connection profile). Limit to 1 hop for v1.
  Actions required: specify in `docs/UR.md §9` + `docs/fs/03-remote-ssh.md`, extend `SshConnectionConfig`, implement the `direct-tcpip` sequence in `src-tauri/src/ssh/manager/connect.rs`, add the "Via jump host" field in the connection form.

- [ ] **[v1.1] Run-merging — group adjacent same-style cells into a single `<span>`** `[Score: 8 | R:1, S:1, U:2, E:1]`
  **Prerequisite for ligature support.** The current span-per-cell model renders one `<span>` per character, which fragments the CSS shaping context and prevents ligature formation. Run-merging groups consecutive cells with identical style (fg, bg, bold, italic, dim, underline, inverse) into a single `<span>` containing multiple characters, restoring the shaping context.
  Secondary benefit: fewer DOM nodes → smaller layout tree → marginal repaint reduction (not a frame-budget lever, but a structural improvement).
  **Boundary constraints** — spans must not cross:
  - Hyperlink boundaries (OSC 8): each distinct URI must be in its own span. **Depends on OSC 8 item above being implemented first.**
  - Search match boundaries: highlighted ranges require span splitting at match edges.
  - Selection boundaries: selected regions require span splitting at selection edges.
  **P12a compatibility**: cell-level dirty tracking (already implemented) writes `gridRows[r][c]` individually. With run-merging, a dirty cell invalidates the entire run it belongs to — the dirty-tracking granularity must be lifted to run level, or runs must be recomputed per row on any cell change.
  **TauTerm implementation:**
  - `TerminalPaneViewport.svelte`: replace `{#each row as cell}` with a derived `runs` array — `$derived.by()` computing `RunCell[]` per row by scanning for style boundaries.
  - `RunCell`: `{ content: string; style: string; strikethrough: boolean; blink: boolean; hyperlink?: string; width: number }` — one entry per contiguous run.
  - Cursor: rendered as a separate absolutely-positioned overlay (unchanged).
  - Selection/search highlighting: computed as run overlays or via CSS `::selection` if feasible.
  Actions required: specify in `docs/uxd/` (rendering model note), implement `computeRuns()` utility + update `TerminalPaneViewport.svelte`, add vitest tests for run boundary detection.

---

## Post-v1 / Roadmap v2+

*Out of scope for v1. Do not start implementation without first updating `docs/UR.md` and `docs/fs/`.*

### Platform Support

- [ ] **Windows 11 support** `[Score: 10 | R:1, S:2, U:2, E:1]`

  **Current state — architecture is ready, implementation is not.**
  ADR-0005 (PAL) is in place: `PtyBackend`, `CredentialStore`, `ClipboardBackend`, `NotificationBackend` traits are defined, factory functions have `#[cfg(target_os = "windows")]` dispatch, and Windows stubs exist in `platform/pty_windows.rs`, `credentials_windows.rs`, `clipboard_windows.rs`, `notifications_windows.rs`. `portable-pty` (already in Cargo.toml) supports ConPTY. `russh` is pure Rust and cross-platform. `arboard` supports Win32 clipboard natively.

  **Prerequisite — an ADR for the Windows porting strategy is required before starting.** No ADR currently governs the porting approach, maintenance model (single cross-platform binary vs platform-specific builds), or test policy per platform.

  #### Phase 1 — Make the project compile on Windows (3–5 days)

  Six issues currently prevent compilation or full functionality on Windows:

  1. **`secret-service` (D-Bus) is an unconditional dependency** in `Cargo.toml`. Move to `[target.'cfg(target_os = "linux")'.dependencies]`.
  2. **`notify-rust` feature `"d"` (D-Bus) is unconditional**. Same fix as above; use `notify-rust` with feature `"windows"` under `[target.'cfg(target_os = "windows")'.dependencies]`.
  3. **`ssh/known_hosts/store.rs`** imports `std::os::unix::fs::OpenOptionsExt` and calls `.mode(0o600)` without `#[cfg(unix)]` guards. Add `#[cfg(unix)]` around these blocks; Windows fallback can skip POSIX permission enforcement (Windows ACLs on the user directory are already restrictive).
  4. **`platform/validation.rs`** imports `std::os::unix::fs::PermissionsExt` without guard. Same fix — skip POSIX mode validation on Windows or implement a Windows ACL equivalent.
  5. **`session/registry/pane_state.rs`** imports `libc` and uses `libc::pid_t` — a POSIX type that leaks outside the PAL. Replace with `i32` (the concrete type on all supported platforms).

  6. **`webview_data_dir.rs` stale cleanup** uses `/proc/<pid>/` to detect dead processes — Linux-only (`#[cfg(target_os = "linux")]`). On Windows, implement an equivalent via `OpenProcess` + `GetExitCodeProcess` (or `windows-sys::Win32::System::Threading`). The core path resolution and `TAUTERM_DATA_DIR` override are already cross-platform.

  Note: file locking (`fs4`) and file watching (`notify`) for preferences safety are already cross-platform — no Windows-specific work needed.

  Additionally, all four Windows stubs currently `unimplemented!()` (panic at runtime). Replace with `Err(...)` or no-op before shipping any Windows build.

  #### Phase 2 — PTY via ConPTY (5–10 days)

  Implement `WindowsPtyBackend` in `platform/pty_windows.rs` using `portable_pty::native_pty_system()` (which resolves to `ConPtySystem` on Windows). The trait surface is already defined.

  Known gaps with no POSIX equivalent:
  - **`foreground_pgid()` / `foreground_process_name()`** — `tcgetpgrp` and `/proc/{pgid}/comm` do not exist on Windows. Return `None`/`Err` — the frontend must handle these gracefully (used for tab title and close-confirmation). If close-confirmation is affected, evaluate `EnumProcesses`-based heuristics, but they are fragile.
  - **Process termination on session close** — no `SIGHUP` equivalent. ConPTY sends `CTRL_CLOSE_EVENT` via the console handler; closing the input pipe causes the child process to receive EOF. Behavior differs from SIGHUP — validate with PowerShell 7 and cmd.exe.
  - **`SIGWINCH`** — not sent by ConPTY on resize. `ResizePseudoConsole` is the correct call; `portable-pty` abstracts this correctly. Shells ported via MSYS2 (Git Bash) emulate `SIGWINCH` on top — validate empirically.

  Shell defaults: `$SHELL` does not exist on Windows. Implement a discovery heuristic: check `$SHELL` (Git Bash / WSL), then `pwsh.exe` in `$PATH`, fallback to `cmd.exe`. Read `$COMSPEC` as the authoritative cmd path.

  Encoding: force UTF-8 before spawning the child shell. For PowerShell 7: pass `-InputFormat Text -OutputFormat Text` or set `$OutputEncoding`. For cmd.exe: prefix the command with `chcp 65001`. Inject `TERM=xterm-256color` into the child environment (absent by default on Windows).

  #### Phase 3 — Credentials (Windows Credential Manager) (2–3 days)

  Implement `WindowsCredentialStore` in `platform/credentials_windows.rs`. Use the `keyring` crate (v3+, cross-platform: Secret Service on Linux, DPAPI/Credential Manager on Windows) or `windows-sys` bindings to `CredWrite`/`CredRead`/`CredDelete`. The `keyring` crate is preferred as it would allow unifying all platforms under a single abstraction.

  #### Phase 4 — SSH Agent on Windows (5–10 days, uncertain)

  **This is the largest functional gap.** The Windows native SSH Agent (OpenSSH Agent service, included in Windows 11) exposes a named pipe at `\\.\pipe\openssh-ssh-agent`, not a Unix socket. `russh` does not support named pipes for agent forwarding — it expects `$SSH_AUTH_SOCK` (Unix socket).

  Options:
  - **Implement named pipe agent support** in TauTerm's SSH layer (`src-tauri/src/ssh/`). Non-trivial but correct.
  - **Document the limitation** and recommend workarounds: Pageant + socat (MSYS2), or using WSL2 where `$SSH_AUTH_SOCK` works natively.
  - **Defer agent support** to a follow-up and ship without it — passphrase-protected keys and password auth still work.

  Other SSH specifics on Windows:
  - `known_hosts` path: `%USERPROFILE%\.ssh\known_hosts` — the `dirs` crate already returns the correct path on Windows. Validate.
  - OpenSSH is included in Windows 11 by default — users already have keys in `%USERPROFILE%\.ssh\`. No additional setup required for key-based auth (excluding agent).

  #### Phase 5 — Distribution and signing (3–5 days)

  Tauri 2 supports NSIS (`.exe` installer) and MSI (WiX) on Windows. NSIS is the recommended starting point for open-source distribution.

  **Code signing (Authenticode) is required** to avoid the Windows SmartScreen "Unknown Publisher" warning, which is a hard barrier for most users. Options:
  - **SignPath.io** (free for open source): signs with an EV certificate using their HSM. Integrates with GitHub Actions. Recommended path for an open-source project without a signing budget.
  - OV certificate (~200–400€/year): reduces SmartScreen reputation friction but does not eliminate it immediately for new binaries.
  - EV certificate (~300–600€/year + physical HSM token): eliminates SmartScreen on first download. Complicates CI/CD unless a cloud signing service (SignPath) is used.

  #### Phase 6 — WebView2 considerations

  On Windows, Tauri uses WebView2 (Chromium-based Edge) instead of WebKitGTK. Key differences for TauTerm:
  - **Data directory isolation already works**: `WebviewWindowBuilder::data_directory()` (ADR-0025) is supported on both WebKitGTK and WebView2. The PID-based isolation mechanism in `webview_data_dir.rs` is cross-platform — only the stale cleanup needs a Windows-specific implementation (see Phase 1, item 6).
  - `contain: strict` and `will-change` are supported (WebView2 is Chromium-based, no WebKitGTK-specific behaviour).
  - **Font metrics differ**: WebView2 uses DirectWrite + ClearType. Default monospace resolves to Consolas/Courier New instead of Liberation Mono/DejaVu. The `ch` unit and `getBoundingClientRect()` cell-width calculations must be validated on Windows — glyph metric differences may produce misaligned cell grids.
  - **CSP hardening opportunity**: WebView2 does not have the WebKitGTK `tauri://localhost` CORS restriction that forces `bundleStrategy: "inline"` and `style-src 'unsafe-inline'`. On Windows, `bundleStrategy: "split"` + `style-src` without `unsafe-inline` is achievable — see ADR-0022 exit criteria.
  - **Performance**: WebView2 is generally faster than WebKitGTK for JS/DOM rendering. The SCROLL frame budget (currently 23 ms on WebKitGTK) is expected to be lower on WebView2 — run P12a benchmark on Windows before concluding on renderer strategy.
  - WebView2 is pre-installed on Windows 11. No bootstrapper needed.

  #### Phase 7 — CI (2–3 days)

  Add a `windows-latest` GitHub Actions runner:
  - `cargo clippy -- -D warnings` (cross-platform)
  - `cargo nextest run` (excluding Linux-only integration tests)
  - `pnpm check` + `pnpm vitest run` (frontend — no changes needed)
  - `pnpm tauri build --no-bundle` (Windows E2E binary)
  - Keyring/SSH container tests are Linux-only (Podman + Xvfb) — explicitly scope them to `ubuntu-latest` runners.

  #### Effort summary

  | Component | Effort |
  |---|---|
  | Phase 1 — Compilation blockers | 3–5 days |
  | Phase 2 — PTY / ConPTY | 5–10 days |
  | Phase 3 — Credentials | 2–3 days |
  | Phase 4 — SSH Agent | 5–10 days |
  | Phase 5 — Distribution / signing | 3–5 days |
  | Phase 6 — WebView2 validation / font metrics | 3–5 days |
  | Phase 7 — CI | 2–3 days |
  | QA and VT compatibility testing | 5–10 days |
  | **Total** | **~4–8 weeks** |

  The wide range reflects uncertainty in SSH Agent implementation (named pipe support) and VT compatibility issues discovered during QA with real applications (Neovim, lazygit, fzf in WSL2).

---

### Performance — Renderer Rewrite

*Context: v1 decision — ~23 ms SCROLL stress test is acceptable. Interactive latency (keystrokes, ncurses) is 1.2 ms. WebKitGTK repaint (78%) is the structural ceiling of the DOM renderer. All CSS/JS optimisations (P-OPT-1 through P-OPT-4) are exhausted. P12b is the only remaining lever to reach 12 ms on SCROLL.*

- [ ] **P12b — WebGL2** *(only if SCROLL < 15 ms becomes a hard requirement post-v1)*
  **All prerequisites met:**
  1. ✓ P-DIAG-1: repaintTime = 78% SCROLL. WebKitGTK repaint confirmed as dominant bottleneck.
  2. ✓ P-OPT-1 (rAF batching): −40% repaints on RAPID-FIRE. SCROLL avg 27.54 ms — budget still exceeded.
  3. ✓ P-OPT-3 (CSS containment): SCROLL avg 23.33 ms, repaintTime 18.13 ms (−15.4%). Insufficient.
  4. ✓ P-OPT-4 (`will-change` layer promotion): no measurable effect in WebKitGTK — removed.
  **Canvas 2D is excluded**: loses DOM accessibility without GPU gains — xterm.js abandoned Canvas addon in v6.0 for this reason.
  WebGL2 constraints on WebKitGTK/Linux:
  - Mandatory fallback on `onContextLoss` (GPU driver crash, tab backgrounded)
  - Background transparency incompatible with WebGL (forces opaque background)
  - Ligatures structurally incompatible (cannot draw beyond cell boundaries)
  - Parallel DOM layer for AT-SPI2 accessibility mandatory (`role="list"` + `aria-live="assertive"`, fed from buffer, `aria-hidden="true"` on canvas) — ~17 KB of code at xterm.js for this component alone
  - Text selection to re-implement entirely (logical model `[col, row]`, mouse tracking, extraction from Rust buffer, clipboard via Tauri)

- [ ] **P5 — Flat buffer for `ScreenBuffer`**
  Replace `Vec<Vec<Cell>>` with a single `Vec<Cell>` of size `rows × cols` with access via `row * cols + col`. **Prerequisite**: after P12b, once the WebGL renderer no longer dominates the frame budget, `build_screen_update_event` (698 µs) may re-emerge as a bottleneck — re-evaluate then. **Risk**: breaking change on all APIs that expose `&[Cell]` by row (`get_row`, `scroll_up`, etc.).

### Terminal Features

- [ ] **Kitty keyboard protocol** (explicitly deferred — ADR-0003, FS-05-scope-constraints.md)
  Enabled by default in Alacritty; required for Neovim 0.10+ (Shift+Enter, Ctrl+I vs Tab, Ctrl+M vs Enter). Natural extension: flag in `ModeState`, dispatch `CSI > 4 ; flags m` (enable) / `CSI < u` (disable) in `Perform::csi_dispatch`, frontend encoding per active mode.
  Note: non-trivial implementation — Alacritty had 6 bug fixes between v0.13 and v0.16 (Shift+number, C0/C1 in associated text).

- [ ] **Vi mode — keyboard navigation in scrollback**
  Alacritty killer feature. Integrated modal mode in the terminal: vi movements (`w`, `b`, `{`, `}`), search (`/`), block selection, yank to clipboard. Not a tmux wrapper — a state managed by the terminal with an independent vi cursor. Power user (Alex who lives in neovim).
  Cost: additional VT state machine + frontend. Substantial — do not underestimate.

- [ ] **Real-time keyword highlighting in terminal output**
  Highlight patterns (errors, IPs, filenames) in real-time output via configurable regexes. Tabby has a highly upvoted request (#632). Strong differentiator for Jordan scanning logs.
  Distinction with existing search: search is punctual and retroactive; highlighting is continuous and prospective.

- [ ] **Integrated SFTP — contextual panel in the SSH session**
  Tabby's biggest differentiator for ops. CWD-aware side panel in the same tab as the active SSH session, with filter bar, folder download, drag-and-drop upload. Eliminates the need for FileZilla or manual `scp`.
  Backend cost: full SFTP client implementation on the Rust side. Substantial — realistic for v2.

- [ ] **Mosh support**
  Highly upvoted request in Tabby (#593). Solves Jordan's pain point: SSH sessions that die on network reconnection (laptop sleep, unstable wifi). Mosh maintains the session via UDP even after a disconnection.
  Cost: integration of the mosh lib or spawn of an external `mosh-client` process. Complex — to investigate.

### Tab / Pane Management

- [ ] **Tab detachment and inter-window movement**
  Detach a tab from its window to create a new one (like Firefox), and move a tab between windows by drag-and-drop.

  **Tab detachment → new window**
  - [ ] Tauri command `detach_tab(tab_id)`: create a new Tauri window, transfer the existing PTY session (without closing or recreating it) in the new window's registry, and close the tab in the origin window
  - [ ] Expose detachment in the tab context menu ("Detach to new window")
  - [ ] Configurable keyboard shortcut (unassigned by default)
  - [ ] Edge case: detaching the last tab of a window must close the origin window after the new one opens

  **Inter-window tab movement (drag-and-drop)**
  - [ ] Drag initiated from the tab bar: detect a drag leaving the tab bar toward an area outside the window → trigger `detach_tab` and open a new window positioned at the cursor (like Firefox)
  - [ ] Drop on another window's tab bar: inter-window transfer protocol (Tauri multi-window messaging or dedicated IPC) to move the PTY session without interruption
  - [ ] Visual indicator during drag (ghost tab, drop zone highlight on other tab bars)
  - [ ] Edge case: cancelled drop (Escape or release outside valid target) → no state change

  **Rust backend**
  - [ ] Abstract the session registry (`SessionRegistry`) so that a **set of sessions** (all panes of the tab — local PTYs and SSH) is transferable between window contexts without being destroyed/recreated
  - [ ] Transfer operates at the tab level, not the individual session: all panes (including split layouts) migrate atomically
  - [ ] SSH sessions (TCP connection + SSH channel + remote PTY) must be treated the same as local PTYs: channel remains open, only the session's window ownership changes in the registry
  - [ ] IPC event `tab-transferred { tab_id, source_window_id, target_window_id }` emitted after successful transfer (discriminated payload, `#[serde(tag = "type")]`)
  - [ ] nextest tests: multi-pane tab transfer, transfer with active SSH session, last tab detachment, transfer cancellation

  **Constraints**
  - No session (local PTY or SSH) **must be interrupted** during transfer — no kill/respawn, no SSH disconnection
  - The complete VT state of each pane (screen buffer, scrollback, cursor) must be fully preserved
  - The pane layout (splits, ratios) must be reproduced identically in the destination window
  - Each Tauri window must have a stable identifier for IPC event routing

### Claude Code Integration

- [ ] **Claude Code Agent Teams — multi-pane support via `CustomPaneBackend`**
  **Prerequisite: [anthropics/claude-code#26572](https://github.com/anthropics/claude-code/issues/26572)**

  Claude Code currently exposes multi-pane to agents only via tmux and iTerm2. A `CustomPaneBackend` extension proposal defines a JSON-RPC 2.0 protocol allowing any terminal to register as a pane backend. This ticket is not yet merged.

  **If and when this ticket is implemented**, implement support in TauTerm:
  - [ ] Rust daemon exposing the `CustomPaneBackend` protocol (JSON-RPC 2.0, stdio or Unix socket): `initialize`, `spawn_agent`, `write`, `capture`, `kill`, `list`, `context_exited`
  - [ ] Pane management primitives on the Rust backend side (split, resize, kill, scrollback)
  - [ ] Automatic registration of `CLAUDE_PANE_BACKEND` on TauTerm launch
  - [ ] Integration tests for the protocol (nextest)

  **Benefit:** TauTerm becomes a first-class terminal for Claude Code Agent Teams, without depending on tmux or iTerm2.

- [ ] **Claude Code Agent Teams — tmux control mode (interim alternative)**
  **Context:** While `CustomPaneBackend` is not implemented and merged (see above), Claude Code uses tmux on Linux. Without integration, tmux runs *inside* TauTerm like any other emulator — double multiplexing layer, visible tmux status bar, conflicting keybindings.

  **Solution:** Implement **tmux control mode** (`tmux -CC`). In this mode, tmux no longer draws its own UI — it sends structured messages (DCS protocol) to the emulator, which creates its own native panes in response. This is the mechanism iTerm2 uses on macOS.

  Reference: `man tmux`, section *CONTROL MODE*. Precedent: [iTerm2 tmux integration](https://iterm2.com/documentation-tmux-integration.html).

  - [ ] Parse the tmux control DCS protocol (`\eP...ST`, messages `%begin`/`%end`, `%output`, `%window-add`, `%pane-*`, etc.)
  - [ ] Map tmux control mode events to TauTerm primitives (tab/pane split, resize, close, scrollback)
  - [ ] Auto-detect control mode on tmux session launch in TauTerm
  - [ ] Integration tests (nextest) covering essential control mode messages

  **Benefit:** Claude Code Agent Teams panes display as native TauTerm panes — no double multiplexing, consistent UX. Superseded by `CustomPaneBackend` if/when available.
