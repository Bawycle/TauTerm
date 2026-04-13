<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — System Overview and Architectural Principles

> Part of the [Architecture](README.md).

> **Version:** 1.5.0
> **Date:** 2026-04-04

---

## 1. Overview

### 1.1 System Layers

TauTerm is structured as a two-process application separated by Tauri's IPC boundary:

```
┌─────────────────────────────────────────────────────────────────┐
│  Frontend (WebView — WebKitGTK on Linux)                        │
│                                                                 │
│  Svelte 5 + Tailwind 4 + Bits UI + Lucide                       │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │ Tab Bar  │ │ Terminal │ │ SSH/Conn.│ │ Preferences Panel │  │
│  │          │ │ Renderer │ │ Manager  │ │                   │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────────┘  │
└───────────────────────────┬─────────────────────────────────────┘
                            │ invoke() / listen()  [IPC boundary]
┌───────────────────────────┴─────────────────────────────────────┐
│  Backend (Rust process — Tokio async runtime)                   │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ session      │  │ vt / parser  │  │ ssh                  │  │
│  │ (tab, pane,  │  │ (vte crate + │  │ (russh / ssh2-rs)    │  │
│  │  lifecycle)  │  │  ScreenBuf)  │  │                      │  │
│  └──────┬───────┘  └──────┬───────┘  └─────────┬────────────┘  │
│         │                 │                     │               │
│  ┌──────┴─────────────────┴─────────────────────┴────────────┐  │
│  │ platform/ (PAL)                                            │  │
│  │  PtyBackend │ CredentialStore │ ClipboardBackend           │  │
│  └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
         │ PTY master fd      │ Secret Service D-Bus
         ↓                    ↓
    [Shell / SSH]          [Keychain]
```

**Data flow for terminal output:**

```
PTY read (async) → vt/parser (vte crate) → ScreenBuffer mutations
   → dirty cell tracking → screen-update event → IPC → Frontend renderer
```

**Data flow for keyboard input:**

```
Frontend keydown → invoke('send_input', {pane_id, data})
   → session module → PtySession::write() → PTY master fd
```

### 1.2 Technology Stack

| Layer | Technology | Decision |
|-------|-----------|---------|
| Application shell | Tauri 2 | ADR-0001 |
| Backend language | Rust (edition 2024) | — |
| Async runtime | Tokio | — |
| VT parser | `vte` crate | ADR-0003 |
| PTY management | `portable-pty` via PAL | ADR-0002, ADR-0005 |
| SSH client | `russh` (preferred) or `ssh2-rs` | ADR-0007 |
| Frontend framework | Svelte 5 with runes | ADR-0004 |
| Frontend state | Svelte 5 runes (`$state`, `$derived`, `$effect`) | ADR-0004 |
| CSS framework | Tailwind 4 (`@theme` design tokens) | — |
| Component primitives | Bits UI | — |
| Icons | Lucide-svelte | — |
| Terminal renderer | DOM + row virtualization + attribute-run merging | ADR-0008 |
| Build tool (frontend) | Vite via SvelteKit | — |

### 1.3 Platform Targets

| Platform | v1 Status | v2+ Path |
|----------|-----------|---------|
| Linux x86\_64 | Supported (AppImage distributed) | — |
| Linux ARM64 (aarch64) | Supported (source build only — no distributed binary) | — |
| macOS | Not supported | PAL stubs ready; russh cross-platform; Keychain via PAL |
| Windows | Not supported | PAL stubs ready; russh/ssh2-rs cross-platform; ConPTY via portable-pty |

---

## 2. Architectural Principles

These principles govern every module boundary and interface decision in the codebase.

### 2.1 Single Source of Truth

Each piece of state has exactly one authoritative owner:
- **Session topology** (tabs, panes, active pane): the `session` module in the Rust backend. The frontend receives it via events and does not maintain its own authoritative copy.
- **Screen buffer content**: the `ScreenBuffer` in the `vt` module, one instance per pane.
- **PTY state**: the `PtySession` trait object, managed by the `session` module.
- **User preferences**: the `PreferencesStore` in the `preferences` module.
- **SSH lifecycle state**: the `SshSession` state machine in the `ssh` module.

The frontend holds a **replica** of backend state for rendering. It updates this replica in response to events; it never speculatively modifies it.

### 2.2 Unidirectional Data Flow

```
User action → Frontend invoke() → Backend command handler → State change
   → Backend emit() event → Frontend replica update → UI re-render
```

The frontend never mutates shared state directly. The backend never pushes imperative UI instructions. Events carry state, not commands.

### 2.3 Module Isolation

Each Rust module exposes a public API through a small number of types and functions. Implementation details (internal state types, helper functions) are private to the module. Cross-module communication happens through the `session` module as coordinator, or through Tauri's `State<T>` injection — never through direct coupling between sibling modules.

### 2.4 No Global Mutable State

No `static mut`, no `lazy_static!` with interior mutability, no `Arc<Mutex<GlobalSomething>>` accessible from multiple unrelated modules. State is owned by the `session` module and passed to submodules through function parameters or trait method calls.

### 2.5 Parse Don't Validate at the IPC Boundary

Every `#[tauri::command]` function receives strongly-typed inputs. Newtype wrappers (`PaneId`, `TabId`, `ConnectionId`) prevent confusion between IDs of different entity kinds. All validation (path traversal checks, URI scheme validation, sequence length limits) happens at the entry point — not scattered through internal logic.

### 2.6 YAGNI

The architecture is designed to accommodate future features (session persistence, plugin system, cloud sync) without requiring redesign, but it does not implement them. Extension points are trait boundaries and module interfaces, not pre-built infrastructure.
