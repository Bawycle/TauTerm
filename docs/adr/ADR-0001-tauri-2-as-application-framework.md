<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0001 — Tauri 2 as application framework

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm requires a native desktop application shell that can:
- Host a WebView for the Svelte UI frontend
- Provide a Rust backend with direct access to OS primitives (PTY via `libc`, D-Bus for the Secret Service keychain, X11/Wayland clipboard APIs)
- Expose an IPC boundary between the two layers
- Produce a distributable binary for Linux (x86, x86_64, ARM, RISC-V) in v1, with a credible path to Windows and macOS in future versions
- Be open-source and free of per-distribution royalty concerns

The choice of application framework shapes every layer of the architecture: how backend and frontend communicate, what security model applies to the WebView, and how the application is built and distributed.

## Decision

Use **Tauri 2** as the application framework.

Tauri 2 provides a Rust backend process hosting a system WebView, a typed IPC bridge (`#[tauri::command]`, `invoke()`, events via `AppHandle::emit()`), a capability-scoped permissions model, and first-class support for Linux (AppImage, deb, rpm) with documented cross-platform paths for Windows and macOS.

## Alternatives considered

**Electron**
Bundles Chromium and Node.js with the application, resulting in ~150–200 MB base bundles and significantly higher RAM usage at runtime. The Node.js layer would add a third language (JavaScript) for backend concerns, whereas Tauri keeps all backend logic in Rust. Electron's IPC model is less typed and its security posture requires more explicit hardening. Not chosen due to resource footprint and the availability of a leaner alternative.

**Qt / C++ with WebEngine**
Qt provides mature PTY abstractions and proven cross-platform support, but requires a C++ codebase, commercial licensing for some features, and a significantly larger runtime dependency. The team's expertise is in Rust and TypeScript, not C++. Qt's WebEngine embeds Chromium, similar to Electron. Not chosen due to licensing complexity, language mismatch, and resource footprint.

**GTK direct (Rust via gtk-rs)**
Writing a native GTK application in Rust with a custom terminal widget (e.g., libvte integration) would produce the leanest possible binary, but at the cost of a bespoke terminal rendering stack and limited ability to use modern web frontend tooling. The design system and component library choices (Tailwind 4, Bits UI, Lucide) would be unavailable. Not chosen because it eliminates the frontend tooling ecosystem that enables the targeted UX quality.

**Deno / NAPI-based approaches**
Emerging options with insufficient production track record for a desktop application with strict OS API requirements. Not chosen due to ecosystem maturity.

## Consequences

**Positive:**
- Rust backend has direct, zero-overhead access to all required Linux OS APIs: `libc` for PTY, `libsecret`/`keyring` crate for the Secret Service, `arboard` or platform-specific crates for clipboard.
- WebView is system-provided (WebKitGTK on Linux), keeping binary size minimal.
- Tauri 2's capability system (`capabilities/`) enforces IPC surface restriction at the framework level: the frontend cannot invoke commands it has not been granted.
- Cross-platform portability (Windows via WebView2, macOS via WKWebView) is a documented and supported path, enabling the v2+ roadmap without re-architecting.
- The `AppHandle` + `State<T>` dependency injection pattern allows managed state (preferences, session registry) to be injected into command handlers without global mutable state.

**Negative / risks:**
- System WebView dependency: on Linux, WebKitGTK version varies across distributions. This can cause subtle rendering differences. Mitigation: constrain CSS to widely-supported features; avoid bleeding-edge CSS that WebKitGTK may not implement.
- Tauri's IPC introduces serialization cost on every command call. This is acceptable for coarse-grained commands (one per user action) but would be a bottleneck for high-frequency, fine-grained calls. The IPC design (see ADR-0006) must stay coarse-grained.
- `screen-update` events will be high-frequency. The current design (cell diffs via Tauri events) may require tuning or an alternative transport for very high terminal output rates. This is a known risk flagged in the IPC contract (UXD §15.3) and will require performance validation.

**Debt:**
None introduced. This is a foundational choice with a clear cross-platform roadmap.
