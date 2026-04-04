<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0005 — Platform Abstraction Layer for OS primitives

**Date:** 2026-04-04
**Status:** Accepted

## Context

TauTerm v1 targets Linux only. However, the project brief explicitly states that Windows and macOS support are planned for future versions, and that the v1 architecture must avoid requiring fundamental refactoring for the port. The OS-specific primitives used by TauTerm are:

1. **PTY allocation and child process spawning** — `posix_openpt` / `openpty` on Linux/macOS; ConPTY on Windows
2. **Credential storage** — Secret Service D-Bus API (`libsecret`) on Linux; Keychain on macOS; Windows Credential Manager on Windows
3. **Clipboard** — X11 `PRIMARY`/`CLIPBOARD` selections and Wayland `wp_primary_selection_v1` on Linux; NSPasteboard on macOS; Win32 Clipboard on Windows
4. **Notifications** — D-Bus desktop notifications on Linux; `NSUserNotification` on macOS; Win32 toast on Windows (used for bell and background activity)

These primitives are structurally different across platforms. Without a deliberate abstraction layer, the Linux-specific code would be scattered throughout the backend modules, making porting a surgical refactoring exercise rather than an additive one.

## Decision

Define a **Platform Abstraction Layer (PAL)** as a dedicated Rust module (`src-tauri/src/platform/`) that exposes trait-based interfaces for each OS primitive. Each trait has exactly one implementation per supported platform. All code outside `platform/` uses only the trait interfaces — never platform-specific types or function calls directly.

The four traits are:

```rust
// PTY backend (see also ADR-0002)
pub trait PtyBackend: Send + Sync {
    fn open(&self, config: PtyConfig) -> Result<Box<dyn PtySession>>;
}

pub trait PtySession: Send {
    fn write(&mut self, data: &[u8]) -> Result<()>;
    fn resize(&self, cols: u16, rows: u16, xpixel: u16, ypixel: u16) -> Result<()>;
    fn as_raw_fd(&self) -> RawFd;
    fn pid(&self) -> u32;
}
// Note: `read` is intentionally absent from this trait. The hot-path PTY read
// is performed by the PtyReadTask via a `tokio::io::unix::AsyncFd` constructed
// from `as_raw_fd()`. A synchronous `read(&mut [u8])` method on the trait would
// block the Tokio async thread if called from an async context. Callers that need
// async reads extract the AsyncFd directly from the raw fd; the trait does not
// expose a synchronous read to prevent misuse.

// Credential store
pub trait CredentialStore: Send + Sync {
    fn store(&self, service: &str, account: &str, secret: &[u8]) -> Result<()>;
    fn retrieve(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>>;
    fn delete(&self, service: &str, account: &str) -> Result<()>;
    fn is_available(&self) -> bool;
}

// Clipboard
pub trait ClipboardBackend: Send + Sync {
    fn set_clipboard(&self, text: &str) -> Result<()>;
    fn get_clipboard(&self) -> Result<String>;
    fn set_primary(&self, text: &str) -> Result<()>; // no-op on non-X11 platforms
}
```

These traits are registered in Tauri's `State<T>` at application startup, with the correct platform implementation injected. All command handlers receive the traits via dependency injection.

Platform-specific module layout:
```
src-tauri/src/platform/
  mod.rs                  (trait definitions, factory functions; #[cfg(target_os = ...)]
                           dispatch lives here, not in sub-files)
  pty_linux.rs            (UnixPtySystem wrapper; AsyncFd extraction; O_CLOEXEC)
  credentials_linux.rs    (SecretService D-Bus adapter; SecVec<u8> zeroizing; fallback)
  clipboard_linux.rs      (arboard adapter; X11 PRIMARY; Wayland fallback)
  notifications_linux.rs  (D-Bus org.freedesktop.Notifications adapter; no-op fallback)
  pty_macos.rs            (stub: unimplemented!())
  credentials_macos.rs    (stub)
  clipboard_macos.rs      (stub)
  notifications_macos.rs  (stub)
  pty_windows.rs          (stub: unimplemented!())
  credentials_windows.rs  (stub)
  clipboard_windows.rs    (stub)
  notifications_windows.rs (stub)
```

The layout is **per-functionality, not per-OS**. Each file contains a single OS implementation of a single trait (e.g., `pty_linux.rs` contains only the Linux PTY implementation). This structure avoids monolithic per-OS files that would accumulate all platform code, makes imports precise, and aligns with the Rust convention of small cohesive modules. The alternative (one `linux.rs` for all Linux implementations) was considered but rejected: it creates large files with unrelated concerns and makes it harder to review or test a single capability in isolation.

For the PTY specifically, the `portable-pty` crate (ADR-0002) already implements cross-platform PTY behind a trait. TauTerm's `PtyBackend` trait wraps it.

## Alternatives considered

**Feature flags (`#[cfg(target_os = "linux")]`) scattered in place**
Each module would have conditional compilation blocks wherever it touches OS primitives. This is the simplest approach but produces spaghetti: the PTY module, SSH module, credential module, and clipboard module all have platform branches scattered throughout their logic. When porting to macOS, there is no single place to add the implementation — changes are spread across the codebase. Not chosen.

**A single `Platform` struct with all capabilities**
Instead of four separate traits, one `Platform` struct with all OS-facing methods. This reduces injection boilerplate but creates a god-object that is harder to test (mocking the clipboard in PTY tests requires a full `Platform` mock). Not chosen; separate traits enable targeted mocking.

**Dynamic dispatch only at Tauri plugin boundaries**
Using Tauri's plugin system to encapsulate each OS primitive. More overhead, more indirection, and Tauri plugins are designed for distribution — internal platform abstractions don't benefit from the plugin distribution model. Not chosen for v1.

## Consequences

**Positive:**
- The Linux implementation is the only concrete code in v1. macOS and Windows stubs can be added incrementally, module by module, without touching any other code.
- Each trait can be mocked independently in unit tests: test the SSH state machine with a mock `CredentialStore`; test the PTY lifecycle with a mock `PtySession`.
- The abstraction cost is minimal: one trait per primitive, one `Box<dyn Trait>` in `State<T>`. No performance-sensitive paths go through the PAL (PTY I/O bypasses the trait boundary in the hot path via Tokio's `AsyncFd` constructed from `PtySession::as_raw_fd()`).
- The `PtySession` trait omits `read` deliberately: the hot-path read is done asynchronously via `AsyncFd`, so a synchronous `read` method on the trait would either block the Tokio runtime thread or be dead code. The raw fd is exposed via `as_raw_fd()` so the `PtyReadTask` can wrap it in an `AsyncFd` directly. This is consistent with ARCHITECTURE.md §7.1.

**Negative / risks:**
- The trait definitions must be designed carefully enough that they are genuinely implementable on all target platforms. A trait method that is fundamentally X11-specific (e.g., `set_primary` for X11 PRIMARY selection) becomes a no-op on macOS, which is acceptable — but must be documented as such, not treated as an error.
- The `*_macos.rs` and `*_windows.rs` stub files will be empty or panic-stub modules in v1. They must not be left in a state where they compile but silently misbehave. Stubs should `unimplemented!()` or return a descriptive `Err` with a message indicating the platform is not yet supported.

**Debt:**
The macos and windows stubs are intentional placeholder debt, scoped and bounded. They are not hidden — they are explicit files in the module tree, each with a comment stating their status.
