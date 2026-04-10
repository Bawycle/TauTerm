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

### Tests & CI

- [ ] **Set up CI pipeline — GitHub Actions** `[Score: 17 | R:3, S:2, U:2, E:2]`
  - Minimum jobs: `cargo clippy -- -D warnings`, `cargo nextest run`, `pnpm check`, `pnpm vitest run`
  - Add `cargo audit` and `cargo deny` (with `deny.toml`) to block advisories and non-compliant licenses
  - Trigger: push to `dev` and `main`, PR to `main`

---

## High Priority — Must Land in v1 (score 13–16)

### Distribution

- [ ] **Distribution — GPG signing + SHA256SUMS** `[Score: 13 | R:2, S:2, U:1, E:2]` (FS-DIST-006)
  No signing script exists in CI/CD. Implement in the release pipeline: `SHA256SUMS` generation, GPG signing, publication of signed artifacts.

---

---

## Low Priority — Post-v1 Nice-to-Have (score ≤ 8)

### Performance — Benchmarking Blindspots

The only existing bench (`src-tauri/benches/vt_throughput.rs`) covers only the VT parser on ASCII and a few `DirtyRows` primitives. The entire downstream pipeline and the latency dimension are blind.

**Throughput axis**

- [ ] **Realistic VT content** `[Score: 8 | R:1, S:1, U:2, E:1]` — Add benchmarks on CSI, SGR, OSC sequences, cursor movement, wide chars (typical content: `htop`, `vim`, `ls --color`). A regression in CSI dispatch is currently undetectable.
- [ ] **Unicode/emoji hot path** `[Score: 8 | R:1, S:1, U:2, E:1]` — Benchmark `write_char()` on wide codepoints and Regional Indicators (U+1F1E6–), which activate `pending_ri`/`pending_emoji` and `CompactStr` allocations. Zero current coverage.
- [ ] **Scrollback eviction** `[Score: 8 | R:1, S:1, U:2, E:1]` — Benchmark `scroll_up` with `scrollback.len() >= scrollback_limit` to measure the cost of `pop_front()` + evicted `Vec<Cell>` deallocation on the hot path.
- [ ] **`build_screen_update_event` full redraw** `[Score: 8 | R:1, S:1, U:2, E:1]` — The most vulnerable point: ~11,000 `String` allocations per event on a 220×50 viewport (`.to_string()` per cell). Triggered on every resize, clear-screen, alt-screen toggle, first display. No existing bench.
- [ ] **JSON serialization of IPC payload** `[Score: 8 | R:1, S:1, U:2, E:1]` — The `ScreenUpdateEvent → serde_json` payload reaches 500 KB–1 MB on full redraw. Benchmark `serde_json::to_string()` on these structures to quantify cost and evaluate impact of a more compact codec.
- [ ] **`ProcessOutput::merge()` in burst** `[Score: 8 | R:1, S:1, U:2, E:1]` — Benchmark repeated coalescence of `DirtyRegion` over 50+ messages in a 12 ms debounce window, not a single isolated mark+iterate.

**Latency axis**

- [ ] **Full process→emit cycle** `[Score: 8 | R:1, S:1, U:2, E:1]` — Benchmark the path `process() → take_dirty() → build_screen_update_event()` in isolation to measure the Rust-side share of the 12 ms budget. The only measurement enabling a realistic budget and detecting latency regressions.
- [ ] **`build_scrolled_viewport_event`** `[Score: 8 | R:1, S:1, U:2, E:1]` — Benchmark the composite viewport reconstruction scrollback + live screen (more expensive than a full redraw). A regression here manifests directly as choppy scrolling.
- [ ] **RwLock Task1/Task2 contention** `[Score: 8 | R:1, S:1, U:2, E:1]` — Task1 acquires `vt.write()` per PTY chunk, Task2 acquires `vt.read()`. Benchmark average acquisition time under simulated concurrent load.
- [ ] **Hot path partial update allocations** `[Score: 8 | R:1, S:1, U:2, E:1]` — Benchmark the partial `build_screen_update_event` path including the 220 `String::to_string()` per dirty line, under `vt.read()`, on the async Tokio thread.

**Cross-terminal comparison**

- [ ] **vtebench script** `[Score: 7 | R:1, S:1, U:1, E:2]` ([alacritty/vtebench](https://github.com/alacritty/vtebench))
  Write a `scripts/bench-vtebench.sh` script that: clones/installs vtebench if absent or detects an existing binary in `$PATH`; launches TauTerm headless or in an Xvfb; runs the standard vtebench suite against TauTerm; produces a comparative report (JSON or Markdown) with date and git commit; documents covered bench cases and the comparison methodology with other terminals (Alacritty, foot).

### Performance — Architectural Optimizations

These two optimizations were explicitly descoped from the initial performance campaign. The `write_1mb_ascii` benchmark establishes the current baseline at **~19.6 MiB/s** — compare after each change.

- [ ] **P5 — Flat buffer for `ScreenBuffer`** `[Score: 8 | R:1, S:1, U:2, E:1]`
  Replace `Vec<Vec<Cell>>` with a single `Vec<Cell>` of size `rows × cols` with access via `row * cols + col`. Eliminates one level of indirection and improves cache locality during sequential reads (snapshot, partial update). Estimated impact: 3–10× on `write_1mb_ascii`. **Risk**: breaking change on all APIs that expose `&[Cell]` by row (`get_row`, `scroll_up`, etc.) — non-trivial refactor.

- [ ] **P12a — DOM with dirty tracking + cell recycling** `[Score: 9 | R:1, S:1, U:2, E:2]` *(step 1 — low risk)*
  The real problem is that 11,000 `<span>` elements are globally recreated or modified on every update, not that the renderer is DOM-based. xterm.js validated in production (v6.0, Dec. 2024) that a DOM renderer with dirty tracking can be competitive with Canvas — which is why they abandoned the Canvas addon.
  Actions required:
  - Only traverse dirty lines (`DirtyRows`) for DOM updates
  - Recycle existing `<span>` elements (modify their properties rather than recreating them)
  - Measure via Criterion benchmarks (latency axis) before and after
  **Success condition:** if P12a brings render latency within the target budget, P12b is unnecessary.

- [ ] **P12b — WebGL2** `[Score: 7 | R:1, S:1, U:1, E:1]` *(step 2 — only if P12a insufficient after measurement)*
  If post-P12a benchmarks show DOM is still the bottleneck on wide viewports, consider WebGL2 via an addon. **Canvas 2D is excluded**: it concentrates the drawbacks of both approaches (loses DOM accessibility like WebGL, without GPU gains) — this is xterm.js's conclusion after abandoning its Canvas addon in v6.0 for exactly this reason.
  WebGL2 constraints on WebKitGTK/Linux:
  - Mandatory fallback on `onContextLoss` (GPU driver crash, tab backgrounded)
  - Background transparency incompatible with WebGL (forces opaque background)
  - Ligatures structurally incompatible (cannot draw beyond cell boundaries)
  - Parallel DOM layer for AT-SPI2 accessibility mandatory (`role="list"` + `aria-live="assertive"`, fed from buffer, `aria-hidden="true"` on canvas) — ~17 KB of code at xterm.js for this component alone
  - Text selection to re-implement entirely (logical model `[col, row]`, mouse tracking, extraction from Rust buffer, clipboard via Tauri)
  **Do not commit to P12b without post-P12a benchmark data.**

### Ligatures — Investigation

- [ ] **Ligatures — verify feasibility in WebKitGTK** `[Score: 7 | R:1, S:1, U:1, E:2]` *(investigate — likely architecturally blocked)*
  Font ligatures (FiraCode, Cascadia Code) are the most upvoted Alacritty feature request (issue #50, 1031 👍, open since 2017 — refused for OpenGL architectural reasons). In TauTerm, rendering goes through WebKitGTK: ligatures could be enabled via CSS `font-feature-settings: "liga" 1; font-variant-ligatures: contextual`.
  **Likely architectural blocker:** TauTerm's span-per-cell model (each character in an individual `<span>`) breaks the CSS shaping context — the render engine does not have access to adjacent glyphs to form ligatures. CSS shaping context requires adjacent glyphs to be in the same text node. This is not a CSS problem, it's a DOM tree constraint. Tabby and Hyper work around this via `@xterm/addon-ligatures` (`tabby-terminal/src/frontends/xtermFrontend.ts`), which uses HarfBuzz compiled to WASM to measure glyphs on canvas — not transposable to a DOM renderer.
  Action: test with FiraCode and Cascadia Code in the current renderer to confirm or refute the blocker. If blocked, document explicitly and link to P12a (dirty tracking with grouping of same-style `<span>` elements into contiguous text).

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

---

## Post-v1 / Roadmap v2+

*Out of scope for v1. Do not start implementation without first updating `docs/UR.md` and `docs/fs/`.*

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
