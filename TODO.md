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

### Security

- [ ] **PTY — unfiltered inherited environment (silent secret leak)** `[Score: 19 | R:3, S:3, U:2, E:2]` *(severity: high)*
  `session/registry/tab_ops.rs` and `pane_ops.rs` build an explicit `Vec<(&str, &str)>` of env vars, but `portable-pty`'s `CommandBuilder::env()` **appends** to the inherited environment without clearing it. Any variable present in TauTerm's own process environment (`AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`, `DATABASE_URL`, etc.) is silently forwarded to the child shell and all its subprocesses.
  **Note:** Alacritty has the same gap (`env_clear()` not called). This is a common shortcoming — TauTerm has the opportunity to be more secure than existing references on this point.
  Actions required:
  1. Call `cmd.env_clear()` before `cmd.env(key, val)` calls in `LinuxPtyBackend::open_session()` — verify that `portable-pty::CommandBuilder` exposes `env_clear()`.
  2. Explicitly define `LANG`, `SHELL`, `HOME`, `USER`, `LOGNAME`, `PATH` (currently implicitly inherited, missing vs. FS-PTY-011).
  3. Add a nextest test verifying that a variable absent from the explicit allowlist is not present in the child shell's environment after spawn.
  4. Document the allowlist in FS-PTY-011.


- [ ] **SSH — invariant test "authentication never before host key validation"** `[Score: 18 | R:3, S:3, U:1, E:2]` *(lesson from CVE-2024-48460 Tabby)*
  The invariant is architecturally guaranteed by `russh` (the `check_server_key` callback blocks `connect()`) but is attested by no test. If `russh` changed its behavior, the regression would be invisible.
  Actions required:
  1. Add an integration test (mock SSH server) simulating a server with an `Unknown` host key: verify that `try_authenticate` is never called.
  2. Add a `// SECURITY:` comment in `connect.rs` documenting this invariant.

### Tests & CI

- [ ] **Set up CI pipeline — GitHub Actions** `[Score: 17 | R:3, S:2, U:2, E:2]`
  - Minimum jobs: `cargo clippy -- -D warnings`, `cargo nextest run`, `pnpm check`, `pnpm vitest run`
  - Add `cargo audit` and `cargo deny` (with `deny.toml`) to block advisories and non-compliant licenses
  - Trigger: push to `dev` and `main`, PR to `main`

### VT Correctness

- [ ] **Cursor on phantom cell of a wide char — missing normalization** `[Score: 17 | R:2, S:3, U:2, E:3]` *(correctness bug)*
  When `CUP`, `HVP`, or any absolute cursor movement lands on a phantom cell (width=0, trailing slot of a wide char), the cursor should be normalized to the base cell (col − 1). Neither the code (`csi_cursor.rs`) nor the specs cover this case. `DSR CPR` (CSI 6 n) then returns an incorrect position; editors that write to this position (vim, helix, tmux) silently corrupt the wide char.
  **Alacritty reference** (`alacritty_terminal/src/term/mod.rs`, fn `goto()`, patch #8786): the fix is trivial — after any absolute positioning, insert `if col > 0 && buf[row][col].flags.contains(WIDE_CHAR_SPACER) { cursor.col -= 1; }`. All the logic exists already — it's a 2-line guard to call consistently. The normalization already exists in `RenderableCursor::new` for rendering but does not affect the logical position used by `DSR CPR`.
  Actions required:
  1. Add FS-VT-0xx: "When a cursor positioning command results in the cursor landing on a phantom cell, the cursor MUST be adjusted to the base cell (col - 1)."
  2. Implement `normalize_cursor_position()` in the VT processor (`src-tauri/src/vt/processor/dispatch/csi_cursor.rs`), called after every absolute movement (`cup`, `vpa`, `cha`, `hpa`, `decrc`). Check `cell.flags.contains(WIDE_CHAR_SPACER)` and decrement `col` if true.
  3. Test with vim and helix in CJK locale.

---

## High Priority — Must Land in v1 (score 13–16)

### Security — Capability & Credential Coverage

- [ ] **`capabilities/default.json` — audit of `core:default`** `[Score: 14 | R:2, S:2, U:1, E:3]`
  The `core:default` preset has never been audited. Its exact content is opaque — if it includes capabilities unused by TauTerm (`core:process:allow-terminate`, etc.), the principle of least privilege is violated without the team knowing.
  Action: consult the official Tauri 2 list of `core:default`, document the included capabilities in a comment in `default.json`, and replace with an explicit list if superfluous capabilities are identified.

- [ ] **Credential regression test suite** `[Score: 15 | R:2, S:3, U:1, E:2]` (FS-CRED-001–009, SEC-CRED-001/002/005)
  Three critical security scenarios remain `BLOCKED` in `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md`: SEC-CRED-001 (credentials.json), SEC-CRED-002 (zeroize), SEC-CRED-005 (fallback keyring). No dedicated non-regression suite is defined as a merge criterion.
  Actions required (after stub removal):
  1. Create `src-tauri/tests/credential_regression.rs`: non-regression suite tagged including SEC-CRED-001/002/003/004/005/006.
  2. Add this suite to merge criteria in `docs/testing/TESTING.md`.

### SSH

- [ ] **SSH reconnect — missing credential re-injection** `[Score: 15 | R:2, S:2, U:3, E:2]` (FS-SSH-040, FS-SSH-041)
  The reconnection architecture is in place (button in `TerminalPaneBanners.svelte`, `handleReconnect`, `reconnect_ssh` command). However, `SshManager::reconnect()` returns `Ok(())` without re-injecting credentials (stub documented in the security protocol). For password-auth connections, reconnection is a decoy.
  Actions required:
  1. Implement re-injection in `SshManager::reconnect()`: lookup keychain by (host, port, username), or trigger `credential-prompt` if unavailable (FS-CRED-007).
  2. Coordinate with SSH stub completion (`auth.rs`, `credentials_linux.rs`).
  3. Add a nextest test: reconnection of a password-auth session with mock credential store.

- [ ] **SSH terminal modes RFC 4254 §8 — missing on-the-wire validation** `[Score: 14 | R:2, S:2, U:2, E:2]`
  The terminal modes (`TERMINAL_MODES` in `ssh/manager.rs`) are correctly defined in intent, but no test verifies that `russh` actually encodes the RFC 4254 opcodes (VINTR=1, VQUIT=2, … ECHO=53) rather than the POSIX `termios.h` constants. A mismatch would cause incorrect signals on the remote server (Ctrl+C not killing the process, etc.).
  Action: add an integration test capturing the SSH frame and verifying the opcodes encoded in the `pty-req`, or inspect the `russh` source to confirm the mapping.

### PTY Robustness

- [ ] **PTY teardown — end-to-end test `close_pane` → `ProcessExited`** `[Score: 14 | R:2, S:2, U:1, E:3]` (FS-PTY-lifecycle)
  The teardown sequence (Drop-cascade → SIGHUP → `ProcessExited` event) is implemented but not covered end-to-end. No nextest test verifies that after `close_pane`, the `ProcessExited` event is emitted with the correct `pane_id` before the registry entry is destroyed.
  Action: add an integration test that spawns a real PTY session, calls `close_pane`, and verifies receipt of `ProcessExited`.

- [ ] **PTY EIO — verify behavior on read error** `[Score: 15 | R:2, S:2, U:2, E:3]` *(robustness)*
  On Linux, `read()` returns `EIO` when the master side of the PTY is read after the child process dies. The correct response is to cleanly terminate the read loop and emit `ProcessExited`. An immediate `break` on any `io::Error` (a common but incorrect pattern) can mask transient errors and prematurely terminate the session.
  **Alacritty reference** (`alacritty_terminal/src/tty/unix.rs`, `alacritty_terminal/src/event_loop.rs`, `alacritty_terminal/src/tty/mod.rs`): treats `EIO` as a normal end-of-process signal (not an error) — `continue` on transient errors, `break` only on `EIO` and explicit close. To verify in `src-tauri/src/platform/pty_linux/backend.rs`: does the PTY read loop distinguish `EIO` from other errors?
  Action: inspect the PTY read loop, ensure `EIO` → `ProcessExited` (not `SessionError`) and that transient errors (`EINTR`, `EAGAIN`) do not terminate the loop.

### Accessibility

- [ ] **Accessibility — missing `aria-controls` and `role="tabpanel"` on the tab bar** `[Score: 15 | R:2, S:2, U:2, E:3]` (WCAG 4.1.2)
  Specified in `docs/uxd/05-accessibility.md §11.3` but not implemented. `TabBarItem.svelte` has `role="tab"` + `aria-selected` but is missing `aria-controls={panelId}`. No `role="tabpanel"` + `aria-labelledby` exists in `TerminalPane.svelte` or the pane container.
  Actions required: add `aria-controls` on each tab item + `role="tabpanel"` + `aria-labelledby` on the corresponding pane container.

- [ ] **Accessibility — missing `:focus-visible` on search input** `[Score: 13 | R:2, S:1, U:2, E:3]` (WCAG 2.4.7)
  `SearchOverlay.svelte`: the main input has `outline: none` with no `:focus-visible` substitute. A keyboard user has no visible focus indicator on this input.
  Action: add `:focus-visible { outline: 2px solid var(--color-focus-ring); outline-offset: -1px; }`.

### Security — IPC

- [ ] **Parse Don't Validate — unvalidated text fields at the IPC boundary** `[Score: 13 | R:2, S:2, U:1, E:2]` (SEC-IPC-005)
  Numeric and enum fields are validated at the boundary (`validate_and_clamp`). Free-text fields are not: `font_family`, `theme_name`, `word_delimiters`, `SshConnectionConfig.host`, `SshConnectionConfig.label` accept arbitrary values including control characters and empty strings.
  **Alacritty reference** (`alacritty/src/config/bindings.rs`, `alacritty/src/config/ui_config.rs`, derive macro in `alacritty_config_derive/src/lib.rs`): pattern `RawBinding` → newtypes with `impl TryFrom<String>` performing validation at construction, decorated `#[serde(try_from = "String")]`. Deserialization produces either a valid type or a serde error.
  Action: add validated newtypes (`SshHost`, `FontFamily`) in `preferences/types.rs` with `TryFrom<String>` + `#[serde(try_from = "String")]`. Minimum checks: max length, absence of control characters, non-empty. Prioritize `SshConnectionConfig.host` (moderate risk: feeds SSH logic).

### Security — Tests

- [ ] **OSC52 — test of per-connection SSH flag vs. global flag** `[Score: 13 | R:1, S:3, U:1, E:3]`
  The `allow_osc52_write` flag exists at two levels (global `TerminalPrefs` and per-connection `SshConnectionConfig`). The override logic is documented in `docs/arch/06-appendix.md §8.2` but covered by no test. A regression in `propagate_osc52_allow()` would make the per-connection flag inoperative.
  Action: add two nextest tests: (1) SSH connection with `allow_osc52_write: true` and global `false` → write allowed; (2) inverse.

### Distribution

- [ ] **Distribution — GPG signing + SHA256SUMS** `[Score: 13 | R:2, S:2, U:1, E:2]` (FS-DIST-006)
  No signing script exists in CI/CD. Implement in the release pipeline: `SHA256SUMS` generation, GPG signing, publication of signed artifacts.

---

## Medium Priority — v1 Quality Target (score 9–12)

### VT Correctness & Tests

- [ ] **OSC 7 (CWD reporting) — not implemented, not specified** `[Score: 9 | R:1, S:1, U:2, E:2]` (FS-TAB-006)
  OSC 7 (`ESC ] 7 ; file://hostname/path ST`) is emitted by fish, zsh (oh-my-zsh), bash (`__vte_prompt_command`) to report the current working directory. Currently ignored in `src-tauri/src/vt/osc.rs` (branch `_ => OscAction::Ignore`). Not specified in `docs/fs/`.
  Impact: tab titles do not reflect the CWD without manual shell config; future "open a pane here" cannot use the current CWD.
  Actions required:
  1. Add FS-VT-0xx: "OSC 7 SHOULD be received and stored as the current working directory of the pane. The CWD MUST be used as the initial directory for new panes/tabs opened from the same pane."
  2. Parse OSC 7 in `osc.rs`, store in `PaneSession`, expose via IPC.
  3. Use CWD for the default tab title (priority: user-label > OSC 0/2 > OSC 7 CWD basename > process name).
  4. Use CWD during `split_pane` / `create_tab`.

- [ ] **Tab titles — fallback process name not implemented** `[Score: 10 | R:1, S:1, U:2, E:3]` (FS-TAB-006)
  FS-TAB-006 mentions "process name" as a title fallback, but the mechanism is neither defined nor implemented. `pane_state.rs` exposes `tcgetpgrp` but does not read `/proc/{pgid}/comm`. Without OSC 0/2/7 emitted by the shell, the title remains empty.
  Action: read `/proc/{pgid}/comm` via the PGID returned by `tcgetpgrp`, and use it as the fallback title. Specify the priority chain in FS-TAB-006.

- [ ] **Mouse mode 1003 (AnyEvent) — implementation correct, only test missing** `[Score: 11 | R:1, S:2, U:1, E:3]`
  The backend correctly tracks the `AnyEvent` mode (enum `MouseReportingMode`), and the frontend (`src/lib/composables/useTerminalPane.svelte.ts:576–591`) correctly forwards `mousemove` events without pressed button via `send_mouse_event` when `mouseReportingMode === 'anyEvent'`. **TauTerm is ahead of Tabby and Hyper**, which fully delegate to xterm.js and do not handle 1003 natively.
  Remaining action: add a functional test (scenario in `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`) covering: activate mode 1003 → move mouse without button → verify encoded bytes arrive at PTY.

- [ ] **Mouse mode PTY — missing activation/deactivation and tmux compatibility tests** `[Score: 11 | R:1, S:2, U:1, E:3]` (FS-VT-080–082)
  The three format encodings are tested. Gaps: no test for activation/deactivation of modes 1002/1003/1006/1015 nor their interaction (e.g. `?1000h` + `?1006h` → SGR encoding active). No "tmux" scenario (1000+1002+1006 SGR) in the functional protocol.
  Actions required:
  1. Add in `src-tauri/src/vt/processor/tests/modes.rs`: activation and reset tests for modes 1002h/1003h/1006h/1015h.
  2. Add a round-trip test: set mode → `MouseEvent::encode()` → expected bytes (reporting × encoding matrix).
  3. Add a tmux scenario in `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`.

- [ ] **Pixel dimensions null in SSH `pty-req` and local PTY open** `[Score: 9 | R:1, S:1, U:1, E:3]` (FS-SSH-013)
  `connect.rs` and `pty_linux/backend.rs` pass `pixel_width: 0, pixel_height: 0`. FS-SSH-013 requires `xpixel`/`ypixel` to be transmitted. Needed for applications that compute font sizes or use pixel dimensions (future graphics protocols over SSH).
  Action: the frontend must compute and transmit `cell_pixel_width × cell_pixel_height` to the backend on open and resize. The backend propagates in `TIOCSWINSZ` (local) and `window-change` (SSH).

### Architecture — Documented Debt

- [ ] **Prefs schema — missing versioning** `[Score: 10 | R:1, S:2, U:1, E:2]` *(migration debt)*
  `Preferences` has no `schema_version: u32` field. Forward compatibility relies exclusively on `#[serde(default)]`: a renamed or removed field in a future version silently resets the user value with no warning and no recovery path. ADR-0012 and ADR-0016 acknowledge this debt without owning it.
  **Tabby reference** (`tabby-core/src/services/config.service.ts`, fn `migrate()`): sequential migration v0→v8 operating on `serde_json::Value` *before* deserialization — pattern: `if version < N { transform_raw_json(&mut raw); version = N; }` repeated per step. Each step is testable in isolation.
  Actions required:
  1. Write an ADR documenting the decision (accept the debt with defined limits, or add `schema_version` + sequential migration engine).
  2. If version added: implement `migrate_from(version, raw: serde_json::Value) -> serde_json::Value` in `preferences/store/` with sequential steps. Start at `schema_version: 1` (v0 = absence of the field, handled by `unwrap_or(0)`).

- [ ] **ADR — PTY session teardown strategy** `[Score: 9 | R:1, S:1, U:1, E:3]`
  The Drop-cascade → SIGHUP → `ProcessExited` sequence is implemented and documented in `docs/arch/` §5.1 and §7.1, but without an ADR. The Drop-cascade choice (vs. explicit ordered shutdown) has consequences (no flush guarantee before `abort()`) that deserve to be documented.
  Action: write an ADR documenting the teardown contract, Drop-cascade rationale, and SIGHUP delivery guarantee.

- [ ] **ADR — render coalescing strategy (12 ms debounce, 256-slot channel)** `[Score: 9 | R:1, S:1, U:1, E:3]`
  The strategy is implemented and documented in `docs/arch/` §6.2 and §6.5, but without an ADR. The 12 ms debounce choice, the bounded 256-slot channel, and the two-task pipeline deserve to be formalized with rationale (behavior on channel saturation, latency budget).
  Action: write an ADR documenting these decisions.

### Security — CSP

- [ ] **CSP `style-src 'unsafe-inline'` — feasibility check and removal criterion** `[Score: 11 | R:1, S:2, U:1, E:3]`
  Documented as "future tightening" in `docs/arch/06-appendix.md §8.4` but without a feasibility check (are style nonces supported by WebKit2GTK + Tauri 2?) or a concrete entry criterion.
  Action: experimentally verify nonce support. If not supported, create an ADR documenting this as a permanent v1 constraint (avoid a false architectural promise). If supported, define an exit criterion in FS-SEC-001.

### UI — Minor Fixes

- [ ] **Ghost token `--size-tab-bar-height` in `ConnectionManager.svelte`** `[Score: 9 | R:1, S:1, U:1, E:3]`
  Line 470: `top: var(--size-tab-bar-height, 44px)` — the canonical token is `--size-tab-height`. The hardcoded fallback `44px` will be used if the token changes, silently breaking panel positioning.
  Action: replace with `var(--size-tab-height)`.

- [ ] **`hide_when_typing` — hide mouse cursor during typing** `[Score: 9 | R:1, S:1, U:1, E:3]` *(quick win — v1)*
  Mouse cursor set to `cursor: none` on the first `keydown` in the terminal viewport, restored on `mousemove`. Alacritty implements this as a configurable option. Aligned with AD.md §1.1 ("chrome should disappear during work") — a mouse cursor in the middle of terminal output is unsolicited visual noise.
  Actions required:
  1. Add the behavior in the terminal component (conditional CSS + `keydown`/`mousemove` handlers).
  2. Expose an option in Preferences > Appearance ("Hide mouse cursor while typing", enabled by default).
  3. Specify in `docs/uxd/04-interaction.md §8.1`.

### Documentation — Minor Gaps

- [ ] **`docs/uxd/02-tokens.md §3.4`** `[Score: 9 | R:1, S:1, U:1, E:3]`
  Add the complete table `--term-color-0` to `--term-color-15` (currently referenced back to `AD.md §3.2`) and explicitly document the `--term-dim-opacity` (opacity) choice vs. separate dim color tokens.

- [ ] **`docs/uxd/04-interaction.md §8.4`** `[Score: 9 | R:1, S:1, U:1, E:3]`
  Document why no overlay shim is needed during divider drag (`setPointerCapture` solves the problem; prevent a future regression by adding a useless shim).

- [ ] **`docs/uxd/03-components.md §7.2`** `[Score: 9 | R:1, S:1, U:1, E:3]`
  Explicitly document that the active/inactive pane distinction is conveyed by border color only (not by viewport content opacity), with the justification (WCAG safe-by-default, preserved readability).

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
