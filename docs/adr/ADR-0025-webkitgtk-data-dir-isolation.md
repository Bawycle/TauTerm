<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0025 — WebKitGTK data directory isolation for concurrent instances

**Date:** 2026-04-12
**Status:** Accepted

## Context

On Linux, Tauri 2 uses WebKitGTK as the webview engine. The WebKitGTK data
directory (cache, local storage, cookies) is derived from `productName` in
`tauri.conf.json` and defaults to `~/.local/share/tau-term/`.

When two TauTerm instances run concurrently, they share this directory. WebKit's
HTTP cache uses hard links internally, and concurrent access causes `Failed to
create hard link … WebKitCache` errors. The app still runs, but cache corruption
between instances is possible.

## Decision

1. Set `"create": false` in `tauri.conf.json` to suppress automatic window
   creation.
2. Create the main window manually in `setup()` using
   `WebviewWindowBuilder::from_config().data_directory()`.
3. On Linux, inject a PID-based unique data directory:
   `$XDG_DATA_HOME/tau-term/webview/<pid>/`.
4. Support `TAUTERM_DATA_DIR` environment variable override (absolute path only).
5. Clean up stale directories on startup by checking `/proc/<pid>/` existence
   (Linux only, `#[cfg(target_os = "linux")]`).

## Alternatives rejected

- **UUID-based paths**: accumulate stale directories indefinitely — no cleanup
  mechanism without a registry.
- **`data_store_identifier`**: not supported on Linux by Tauri/WebKitGTK.
- **WebKitGTK incognito mode**: loses IndexedDB/localStorage state needed by
  the webview.

## Consequences

- `tauri.conf.json` has `"create": false` permanently — any new window must be
  created manually in `setup()`.
- Each launch gets a fresh WebKitGTK data directory (PID-based). WebKitGTK
  initializes correctly with the `tauri://` protocol regardless.
- Stale cleanup is Linux-only; Windows equivalent (`OpenProcess` +
  `GetExitCodeProcess`) deferred to the Windows porting phase.
- The `TAUTERM_DATA_DIR` env var enables testing and power-user multi-profile
  setups.
