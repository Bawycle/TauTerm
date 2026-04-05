<!-- SPDX-License-Identifier: MPL-2.0 -->

# Changelog

All notable changes to TauTerm are documented in this file.

The format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `data-screen-generation` counter attribute on the terminal grid element, incremented on every screen-buffer update. Allows E2E tests to synchronise on DOM changes without polling cell content.
- `data-tab-index` attribute on tab bar elements for deterministic E2E index assertions, avoiding reliance on implicit DOM ordering.
- `InjectablePtyBackend` and `inject_pty_output` Tauri command, compiled exclusively behind the `e2e-testing` Cargo feature flag (ADR-0015). Enables E2E tests to push synthetic bytes directly into the VT pipeline without a real PTY, making output deterministic.
- `tests/e2e/helpers/selectors.ts` — centralised E2E selector constants (`terminalGrid`, `terminalCell`, `terminalPane`, `activeTerminalPane`, `tabBar`, `tab`, `activeTab`) to prevent selector drift across spec files.
- `wdio.conf.ts`: `TAUTERM_BINARY_PATH` environment variable override for the E2E binary path, defaulting to the debug build produced by `cargo build --features e2e-testing`.
- `wdio.conf.ts`: binary existence check in `beforeSession` — fails immediately with an actionable error message if the binary is not found, rather than a cryptic tauri-driver spawn failure.

### Fixed

- E2E spec selectors in `pty-roundtrip.spec.ts` aligned with actual BEM DOM classes (`.terminal-pane__cell`, `.tab-bar__tab`, `.terminal-pane`). Previous selectors did not match the rendered DOM and caused all assertions to fail silently.
- Stale `[DEFERRED: PTY write stub]` comments removed from E2E specs — PTY write is fully implemented and these comments were misleading.
- `TEST-PTY-RT-002` rewritten to use `inject_pty_output` instead of keyboard simulation. The keyboard path depends on shell availability in CI and is non-deterministic; the injectable path is hermetic and reliable.

[Unreleased]: https://github.com/your-org/tau-term/compare/v0.1.0...HEAD
