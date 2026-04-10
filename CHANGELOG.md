<!-- SPDX-License-Identifier: MPL-2.0 -->

# Changelog

All notable changes to TauTerm are documented in this file.

The format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
