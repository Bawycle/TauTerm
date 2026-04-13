<!-- SPDX-License-Identifier: MPL-2.0 -->

# Tauri 2 Capabilities Audit

> **Applies to:** `src-tauri/capabilities/default.json`

---

## 1. Purpose

This document records the result of replacing the opaque `"core:default"` preset with an
explicit minimal permission list in `src-tauri/capabilities/default.json`. The goal is to
apply the principle of least privilege: grant only the built-in Tauri 2 API surface that
TauTerm's frontend actually uses, and nothing more.

---

## 2. Composition of `core:default`

`core:default` is a meta-preset that automatically pulls in nine sub-presets. The full
composition, as of Tauri 2 (`dev` branch, verified 2026-04-09), is:

| Sub-preset | Individual permissions |
|---|---|
| `core:app:default` | allow-version, allow-name, allow-tauri-version |
| `core:event:default` | allow-listen, allow-unlisten, allow-emit, allow-emit-to |
| `core:image:default` | allow-new, allow-from-bytes, allow-from-path, allow-rgba, allow-size |
| `core:menu:default` | allow-new, allow-append, allow-prepend, allow-insert, allow-remove, allow-remove-at, allow-items, allow-get, allow-popup, allow-create-default, allow-set-as-app-menu, allow-set-as-window-menu, allow-text, allow-set-text, allow-is-enabled, allow-set-enabled, allow-set-accelerator, allow-set-as-windows-menu-for-nsapp, allow-set-as-help-menu-for-nsapp, allow-is-checked, allow-set-checked, allow-set-icon |
| `core:path:default` | allow-resolve-directory, allow-resolve, allow-normalize, allow-join, allow-dirname, allow-extname, allow-basename, allow-is-absolute |
| `core:resources:default` | allow-close |
| `core:tray:default` | allow-new, allow-get-by-id, allow-remove-by-id, allow-set-icon, allow-set-menu, allow-set-tooltip, allow-set-title, allow-set-visible, allow-set-temp-dir-path, allow-set-icon-as-template, allow-set-show-menu-on-left-click |
| `core:webview:default` | allow-get-all-webviews, allow-webview-position, allow-webview-size, allow-internal-toggle-devtools |
| `core:window:default` | allow-get-all-windows, allow-scale-factor, allow-inner-position, allow-outer-position, allow-inner-size, allow-outer-size, allow-is-fullscreen, allow-is-minimized, allow-is-maximized, allow-is-focused, allow-is-decorated, allow-is-resizable, allow-is-maximizable, allow-is-minimizable, allow-is-closable, allow-is-visible, allow-is-enabled, allow-title, allow-current-monitor, allow-primary-monitor, allow-monitor-from-point, allow-available-monitors, allow-cursor-position, allow-theme, allow-internal-toggle-maximize |

Total individual permissions granted by `core:default`: approximately 60 distinct
permissions across 9 domains.

---

## 3. What TauTerm Actually Uses

### 3.1 Frontend Tauri API calls (production code only)

The audit covers files under `src/lib/` and `src/routes/`. Test files and mocks are
excluded.

#### `@tauri-apps/api/core` — `invoke()`

Used in `src/lib/ipc/commands.ts`, `src/lib/components/TerminalPane.svelte`, and several
composables (`useTabBarRename.svelte.ts`, `useTabBarDnd.svelte.ts`).

`invoke()` dispatches custom commands registered via `generate_handler![]` in the Rust
backend. In Tauri 2, custom commands do **not** require a capability entry — they are
governed solely by their presence in `generate_handler![]` and the capability's
`permissions` array only controls built-in Tauri APIs. The `invoke()` function itself is
available to any webview by default.

**Built-in Tauri APIs used through this import:** none. All `invoke()` targets are custom
TauTerm commands.

#### `@tauri-apps/api/event` — `listen()`

Used in `src/lib/ipc/events.ts` and `src/lib/components/TerminalPane.svelte`.

`listen()` subscribes to backend events emitted via `AppHandle::emit()`. This requires:
- **`core:event:default`** → `allow-listen`, `allow-unlisten`

`emit()` and `emit-to()` from this preset are not called by the frontend (events flow
backend → frontend only, per architecture rule). However, these are included in the preset
and removing them would require switching to individual permission names.

#### `@tauri-apps/api/window` — `getCurrentWindow()`

Used in:
- `src/lib/composables/useTerminalView.lifecycle.svelte.ts`: `getCurrentWindow().isFullscreen()`, `appWindow.onCloseRequested()`
- `src/lib/composables/useTerminalView.session-handlers.svelte.ts`: `getCurrentWindow().destroy()`

This requires:
- **`core:window:default`** → provides `allow-is-fullscreen` (used by `isFullscreen()`)
- **`core:window:allow-close`** → required by `onCloseRequested()` (already explicit)
- **`core:window:allow-destroy`** → required by `destroy()` (already explicit)

`onCloseRequested()` registers a listener for the OS window-close event and uses Tauri's
internal close coordination mechanism. It relies on `allow-close` to intercept the event.

### 3.2 Tauri plugins

One plugin is registered in `src-tauri/src/lib.rs`:

- **`tauri_plugin_opener`** → `opener:default` in capabilities

`opener:default` grants:
- `opener:allow-open-url` — used by the `open_url` custom command to open URLs in the system browser
- `opener:allow-reveal-item-in-dir` — not used by TauTerm
- `opener:allow-default-urls` — restricts `open-url` to safe URL schemes (http, https, mailto, tel)

### 3.3 Summary of used vs. granted

| Sub-preset | Used | Reason |
|---|---|---|
| `core:app:default` | No | Frontend does not call `app.getName()`, `app.getVersion()`, or `app.getTauriVersion()` |
| `core:event:default` | Partially | `allow-listen` and `allow-unlisten` are used; `allow-emit` and `allow-emit-to` are not (events are backend → frontend only) |
| `core:image:default` | No | No `@tauri-apps/api/image` imports anywhere |
| `core:menu:default` | No | No `@tauri-apps/api/menu` imports; TauTerm renders its own tab bar in the webview |
| `core:path:default` | No | No `@tauri-apps/api/path` imports; path operations are backend-side |
| `core:resources:default` | No | No resource handle management in the frontend |
| `core:tray:default` | No | No system tray; TauTerm has no tray icon |
| `core:webview:default` | No | No `@tauri-apps/api/webview` imports; no devtools toggle in production |
| `core:window:default` | Partially | `allow-is-fullscreen` is used; the 23 other permissions in this preset are not directly called |

---

## 4. Decision: Replace `core:default` with Explicit Minimal List

### 4.1 Rationale

Seven of the nine sub-presets granted by `core:default` are entirely unused. Keeping them
violates the principle of least privilege without any benefit. A compromised renderer
(e.g., via a malicious terminal sequence that injects JS) would have access to tray
manipulation, menu creation, webview enumeration, path resolution, and native image
decoding — none of which TauTerm requires.

The counter-argument for keeping `core:default` is forward compatibility: if a future
feature needs one of these permissions, it is added back explicitly. This is not a valid
reason to grant excess permissions in the present. YAGNI applies symmetrically to security
surface.

### 4.2 What cannot be narrowed further

Two sub-presets are included with permissions that are partially used:

**`core:event:default`**: includes `allow-emit` and `allow-emit-to` beyond what is needed
(`allow-listen`, `allow-unlisten`). Tauri 2 does not expose individual `core:event:*`
permissions — only the `core:event:default` preset. Switching to hypothetical individual
names is not possible without forking Tauri's permission definitions. The preset is
retained as-is; the two unused emit permissions are a known residual.

**`core:window:default`**: includes 23 permissions beyond `allow-is-fullscreen`. As with
events, Tauri 2 does not expose individual `core:window:allow-is-fullscreen` as a
standalone capability identifier in the preset system. The full `core:window:default`
preset must be used to obtain `allow-is-fullscreen`. The unused query permissions
(position, size, monitors, theme, etc.) are read-only introspection — they do not allow
state mutation, which limits the damage from misuse.

**`opener:default`**: includes `allow-reveal-item-in-dir` which TauTerm does not call.
This cannot be narrowed without switching to individual `opener:allow-open-url` +
`opener:allow-default-urls`, which is feasible. However, `reveal-item-in-dir` requires a
filesystem path argument that TauTerm never supplies (no UI exposes it), so the attack
surface is inert. The preset is retained for simplicity; this may be revisited if
`reveal-item-in-dir` is found to accept untrusted input through any path.

### 4.3 Resulting explicit permission list

```
core:event:default         — listen/unlisten for backend→frontend events
core:window:default        — isFullscreen() query on mount
core:window:allow-close    — onCloseRequested() interception
core:window:allow-destroy  — destroy() for programmatic window close
opener:default             — open_url command (http/https/mailto/tel only)
```

Removed from the previous implicit `core:default`:
- `core:app:default`
- `core:image:default`
- `core:menu:default`
- `core:path:default`
- `core:resources:default`
- `core:tray:default`
- `core:webview:default`

---

## 5. Audit Trail

| Date | Author | Change |
|---|---|---|
| 2026-04-09 | architect | Initial audit; replaced `core:default` with explicit minimal list |
