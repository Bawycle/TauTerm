<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Architecture Index

> **Version:** 1.8.0
> **Status:** Living document — update when architectural decisions change
> **Author:** Software Architect — TauTerm team

The architecture documentation has been split into focused files. See below for the index.

---

## Files

| File | Contents |
|------|----------|
| [01-overview.md](01-overview.md) | §1 System Overview (layers, stack, platform targets) + §2 Architectural Principles (SSOT, data flow, module isolation, no global state, parse-don't-validate, YAGNI) |
| [02-backend-modules.md](02-backend-modules.md) | §3 Rust Module Decomposition (file layout, module map, public interfaces, newtype IDs) + §9 Error Handling Strategy (Rust backend, frontend mapping) |
| [03-ipc-state.md](03-ipc-state.md) | §4 IPC Contract (commands, events, error envelope, pane layout topology, type definitions) + §5 State Machines (PTY lifecycle, SSH lifecycle, VT terminal mode state) |
| [04-runtime-platform.md](04-runtime-platform.md) | §6 Concurrency Model (VT pipeline, PTY I/O task, write path, SSH I/O, back-pressure, state access patterns) + §7 Platform Abstraction Layer (PAL traits, Linux implementations, PAL injection, PreferencesStore load strategy) + §10 Build Architecture (pipeline, dev mode, profiles, i18n, AppImage distribution) |
| [05-frontend.md](05-frontend.md) | §11 Frontend Architecture (module map, TerminalPane component split, keyboard shortcut interception) |
| [06-appendix.md](06-appendix.md) | §8 Security Architecture (IPC boundary validation, PTY isolation, SSH security, CSP, terminal injection prevention) + §12 Future Extensibility (session persistence, plugin system, cloud sync, Kitty protocol, Windows/macOS port) + §13 ADR Index |
| [07-screen-buffer-data-model.md](07-screen-buffer-data-model.md) | §14 Screen Buffer Data Model (cell layout, scrollback memory formula, soft/hard wrap representation, minimum terminal size constraint) |
| [08-logging.md](08-logging.md) | §15 Logging and Observability Strategy (instrumentation library, semantic levels, default filter by build profile, level classification of call sites, security constraints, performance) |
| [capabilities.md](capabilities.md) | Tauri 2 capabilities audit — composition of `core:default`, frontend API surface audit, decision to replace with explicit minimal list |

Testing strategy is in a separate document: [../testing/TESTING.md](../testing/TESTING.md).

---

## Quick Reference — Concern → File

| Concern | File |
|---------|------|
| System layers diagram | [01-overview.md](01-overview.md) §1.1 |
| Technology stack table | [01-overview.md](01-overview.md) §1.2 |
| Platform targets | [01-overview.md](01-overview.md) §1.3 |
| Architectural principles (SSOT, unidirectional flow, etc.) | [01-overview.md](01-overview.md) §2 |
| Rust module file layout convention | [02-backend-modules.md](02-backend-modules.md) §3.1 |
| Full module map (`vt/`, `session/`, `ssh/`, `platform/`, etc.) | [02-backend-modules.md](02-backend-modules.md) §3.2 |
| Public interfaces (`SessionRegistry`, `VtProcessor`, `SshManager`, `PreferencesStore`) | [02-backend-modules.md](02-backend-modules.md) §3.3 |
| Newtype IDs (`TabId`, `PaneId`, `ConnectionId`) | [02-backend-modules.md](02-backend-modules.md) §3.4 |
| Error handling (`thiserror`, `anyhow`, no `unwrap()` policy) | [02-backend-modules.md](02-backend-modules.md) §9 |
| IPC guiding policy | [03-ipc-state.md](03-ipc-state.md) §4.1 |
| Full IPC command table | [03-ipc-state.md](03-ipc-state.md) §4.2 |
| Full IPC event table | [03-ipc-state.md](03-ipc-state.md) §4.3 |
| `TauTermError` envelope | [03-ipc-state.md](03-ipc-state.md) §4.4 |
| Pane layout tree (`PaneNode`, `TabState.layout`) | [03-ipc-state.md](03-ipc-state.md) §4.5.1 |
| `SessionStateChanged` delta granularity | [03-ipc-state.md](03-ipc-state.md) §4.5.2 |
| `close_pane` return value and last-pane behavior | [03-ipc-state.md](03-ipc-state.md) §4.5.3 |
| `NotificationChangedEvent` | [03-ipc-state.md](03-ipc-state.md) §4.5.4 |
| Rust ↔ TypeScript type definitions | [03-ipc-state.md](03-ipc-state.md) §4.6 |
| PTY session lifecycle state machine | [03-ipc-state.md](03-ipc-state.md) §5.1 |
| `PtyLifecycleState` type (running/terminated-clean/terminated-error/closed) | [03-ipc-state.md](03-ipc-state.md) §5.1 |
| Foreground process detection (`tcgetpgrp`, `hasForegroundProcess`) | [03-ipc-state.md](03-ipc-state.md) §5.1 |
| Window close confirmation flow (CloseRequested aggregation) | [03-ipc-state.md](03-ipc-state.md) §5.1 |
| SSH session lifecycle state machine | [03-ipc-state.md](03-ipc-state.md) §5.2 |
| VT terminal mode state (DECCKM, mouse modes, etc.) | [03-ipc-state.md](03-ipc-state.md) §5.3 |
| Tokio runtime, C1 codes, DCS dispatch | [04-runtime-platform.md](04-runtime-platform.md) §6.1 |
| PTY I/O task (`spawn_blocking`, VtProcessor write lock) | [04-runtime-platform.md](04-runtime-platform.md) §6.2 |
| Write path / `send_input` | [04-runtime-platform.md](04-runtime-platform.md) §6.3 |
| Back-pressure, resize debounce, scroll follow semantics | [04-runtime-platform.md](04-runtime-platform.md) §6.5 |
| State access patterns (lock table) | [04-runtime-platform.md](04-runtime-platform.md) §6.6 |
| PTY PAL (`portable-pty`, SIGHUP, login shell) | [04-runtime-platform.md](04-runtime-platform.md) §7.1 |
| Credential store PAL (Secret Service) | [04-runtime-platform.md](04-runtime-platform.md) §7.2 |
| Clipboard PAL (`arboard`, X11 PRIMARY) | [04-runtime-platform.md](04-runtime-platform.md) §7.3 |
| Notification PAL (D-Bus, no-op fallback) | [04-runtime-platform.md](04-runtime-platform.md) §7.4 |
| PAL injection at startup (`lib.rs`) | [04-runtime-platform.md](04-runtime-platform.md) §7.5 |
| PreferencesStore load strategy (TOML, JSON migration, fallback) | [04-runtime-platform.md](04-runtime-platform.md) §7.6 |
| Preference propagation model (per-field: CSS vars, session propagation, scrollbackLines constraint) | [04-runtime-platform.md](04-runtime-platform.md) §7.7 |
| Build pipeline (Vite + Cargo) | [04-runtime-platform.md](04-runtime-platform.md) §10.1 |
| i18n (Paraglide JS, locale files, `Language` enum) | [04-runtime-platform.md](04-runtime-platform.md) §10.5 |
| AppImage distribution (Tauri bundler, multi-arch CI) | [04-runtime-platform.md](04-runtime-platform.md) §10.6 |
| Frontend module map (`lib/`, `components/`, routes) | [05-frontend.md](05-frontend.md) §11.1 |
| `TerminalPane` component split rule (250-line threshold) | [05-frontend.md](05-frontend.md) §11.2 |
| Keyboard shortcut interception (`isRecordingShortcut`) | [05-frontend.md](05-frontend.md) §11.3 |
| IPC boundary validation (paths, URIs, titles, sequence length) | [06-appendix.md](06-appendix.md) §8.1 |
| `PreferencesStore` structure (sub-keys table) | [06-appendix.md](06-appendix.md) §8.1 |
| Built-in theme model (Umbra, Solstice, Archipel — static assets, no backend storage) | [06-appendix.md](06-appendix.md) §8.1 |
| PTY isolation (`O_CLOEXEC`, OSC 52 write policy) | [06-appendix.md](06-appendix.md) §8.2 |
| SSH security (TOFU, known-hosts, agent forwarding disabled) | [06-appendix.md](06-appendix.md) §8.3 |
| Content Security Policy | [06-appendix.md](06-appendix.md) §8.4 |
| Terminal injection prevention (read-back sequences, paste) | [06-appendix.md](06-appendix.md) §8.5 |
| Future extensibility (session persistence, plugins, cloud sync, Kitty, ports) | [06-appendix.md](06-appendix.md) §12 |
| ADR index (ADR-0001 through ADR-0017) | [06-appendix.md](06-appendix.md) §13 |
| Cell struct layout and memory sizing | [07-screen-buffer-data-model.md](07-screen-buffer-data-model.md) §14.1 |
| Scrollback memory estimate formula (5,500 bytes/line upper bound) | [07-screen-buffer-data-model.md](07-screen-buffer-data-model.md) §14.2 |
| Soft wrap / hard newline storage (`soft_wrapped` flag) | [07-screen-buffer-data-model.md](07-screen-buffer-data-model.md) §14.3 |
| `get_scrollback_line` API change (expose `soft_wrapped`) | [07-screen-buffer-data-model.md](07-screen-buffer-data-model.md) §14.3.1 |
| Minimum terminal size (20×5), enforcement layer | [07-screen-buffer-data-model.md](07-screen-buffer-data-model.md) §14.4 |
| Testing strategy (pyramid, unit, integration, VT conformance, E2E, security) | [../testing/TESTING.md](../testing/TESTING.md) |
| Logging/tracing library and level convention | [08-logging.md](08-logging.md) §15.1–15.2 |
| Default filter by build profile (`debug_assertions`) | [08-logging.md](08-logging.md) §15.3 |
| Level audit of existing `tracing!` call sites | [08-logging.md](08-logging.md) §15.4 |
| Security constraints on log content (no credentials, no full paths) | [08-logging.md](08-logging.md) §15.5 |
| Performance constraints on hot-path logging | [08-logging.md](08-logging.md) §15.6 |
| Structured logging for security events (M2 roadmap) | [08-logging.md](08-logging.md) §15.7 |
| Tauri 2 capabilities: `core:default` composition, frontend API audit, minimal explicit list | [capabilities.md](capabilities.md) |

---

*This documentation is maintained by the TauTerm software architect. Every structural change to the **backend module layout, the frontend module layout, the IPC command surface, the pane topology model**, or the platform abstraction layer requires updating the relevant file in this directory and, where appropriate, adding a new ADR.*
