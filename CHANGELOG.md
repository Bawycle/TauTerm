<!-- SPDX-License-Identifier: MPL-2.0 -->

# Changelog

All notable changes to TauTerm are documented in this file.

The format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CSS containment (`contain: strict`) is now applied to the terminal viewport element. This confines WebKitGTK repaints and layout recalculations to the terminal subtree, reducing repaint time by 15 % on average (−20 % at p95) on SCROLL-intensive workloads. Scrolling through large command output or log streams is noticeably smoother.
- `will-change: contents` is applied to terminal row elements as a compositing hint. The property has no measurable effect in the current WebKitGTK renderer (delta within noise — < 0.2 ms), but is retained as a forward-compatibility hint for other WebKit-based renderers where row-level compositing may be active.
- `VITE_PERF_INSTRUMENTATION=1` build flag activates `performance.mark/measure` instrumentation (`tauterm:applyOnly`, `tauterm:repaintTime`, `tauterm:frameRender`) in production E2E builds without affecting normal dev or release builds. Enables benchmark-grade decomposition of JS-side vs WebKitGTK-repaint costs in WebdriverIO perf specs.
- `screen-update` events are now batched into a single `requestAnimationFrame` callback (P-OPT-1): N events arriving within one browser frame are coalesced into 1 Svelte reconcile cycle and 1 WebKitGTK repaint. For high-frequency update bursts (e.g. ncurses, rapid scrolling) this reduces WebKitGTK repaint count by up to 40 %. Steady-state scroll workloads (≤ 1 event per 16 ms) are unaffected.
- IPC serialization is now ~56 % faster for typical terminal content: boolean SGR attributes (`bold`, `dim`, `italic`, `blink`, `inverse`, `hidden`, `strikethrough`) and `underline` are omitted from the `screen-update` JSON payload when at their default values (`false`/`0`). For a 220×50 terminal with unformatted text, serialization drops from 1.53 ms to ~660 µs per frame.
- Terminal apps that use cursor-position queries (CPR, `ESC[6n`) — `vim`, `neovim`, `fzf`, `less` — now get their screen-state update immediately rather than waiting up to 12 ms for the debounce window. The debounce is bypassed only when a VT response was generated; ordinary render updates still coalesce normally.
- In multi-pane layouts, each pane now displays a slim title bar showing its process title (OSC 0/2, current directory name, or foreground process). The active pane's bar is visually distinct from inactive panes (font weight and opacity, not color). The title bar can be turned off in Preferences > Appearance ("Show pane title bar"). The tab title also now follows the active pane rather than always the leftmost/top pane.
- Split-pane tabs show a `LayoutPanelLeft` icon in the tab bar to indicate that the tab contains multiple panes.
- Tab titles now update automatically to reflect the current directory when using shells that emit OSC 7 (`fish`, `zsh` with oh-my-zsh, `bash` with `__vte_prompt_command`). When opening a new tab or split from an existing pane, the new pane starts in the same directory.
- When no explicit title is set by the shell, the tab now shows the name of the foreground process (e.g. `vim`, `htop`, `cargo`).
- Mouse cursor hides automatically while typing in the terminal and reappears when you move the mouse. Behaviour is on by default and can be turned off in Preferences > Appearance ("Hide mouse cursor while typing").
- Preferences are now versioned: future TauTerm updates that rename or remove a preference field will migrate existing user settings instead of silently resetting them to defaults.
- Terminal dimensions (cell pixel size) are now correctly reported to the running process at session open, in addition to at resize. Applications that use pixel dimensions to compute font metrics or layout (e.g. some image-rendering tools) will now receive accurate values from the start.
- `data-screen-generation` counter attribute on the terminal grid element, incremented on every screen-buffer update. Allows E2E tests to synchronise on DOM changes without polling cell content.
- `data-tab-index` attribute on tab bar elements for deterministic E2E index assertions, avoiding reliance on implicit DOM ordering.
- `InjectablePtyBackend` and `inject_pty_output` Tauri command, compiled exclusively behind the `e2e-testing` Cargo feature flag (ADR-0015). Enables E2E tests to push synthetic bytes directly into the VT pipeline without a real PTY, making output deterministic.
- `tests/e2e/helpers/selectors.ts` — centralised E2E selector constants (`terminalGrid`, `terminalCell`, `terminalPane`, `activeTerminalPane`, `tabBar`, `tab`, `activeTab`) to prevent selector drift across spec files.
- `wdio.conf.ts`: `TAUTERM_BINARY_PATH` environment variable override for the E2E binary path, defaulting to the debug build produced by `cargo build --features e2e-testing`.
- `wdio.conf.ts`: binary existence check in `beforeSession` — fails immediately with an actionable error message if the binary is not found, rather than a cryptic tauri-driver spawn failure.
- E2E PTY round-trip tests use `inject_pty_output` for deterministic output injection; hermetic and reliable in CI regardless of shell availability.

[Unreleased]: https://github.com/your-org/tau-term/compare/v0.1.0...HEAD
