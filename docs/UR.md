# User Requirements — TauTerm

> **Nature of this document.** A User Requirements document expresses what users need to accomplish — not how the system implements it. Each requirement is stated from the user's perspective ("the user can…", "the user needs to…") and is independent of any technical solution. Functional specifications, architecture decisions, and implementation choices are handled in separate documents.

---

## 1. Overview

TauTerm is a terminal emulator. It supports multiple tabs and multiple screens within a single window, with a studied UX/UI and persistent user preferences.

---

## 2. Personas

The following personas anchor prioritization decisions. They represent realistic usage patterns, not stereotypes.

### 2.1 Alex — Software Developer (primary persona)

Alex is a backend or full-stack developer who spends most of their working day in a terminal emulator. They run build systems, watch logs, connect to remote servers via SSH, and switch frequently between local and remote contexts. They are comfortable with terminal conventions but use a GUI terminal rather than a tiling window manager — they want multiplexing without the cognitive overhead of configuring tmux from scratch.

Key needs: fast tab/pane switching, SSH saved connections, reliable copy-paste, a clean low-distraction interface, configurable shortcuts.

### 2.2 Jordan — Systems Administrator (secondary persona)

Jordan manages a fleet of Linux servers. They open many SSH sessions simultaneously, often to heterogeneous environments (different distros, old kernels, constrained shells). They need to know at a glance which sessions are active or have produced output, and they need to recover quickly from dropped SSH connections without losing context.

Key needs: SSH reconnection, background activity notifications, session persistence awareness, trustworthy credential handling.

### 2.3 Sam — Occasional Terminal User (edge persona)

Sam is a developer who primarily works in a GUI IDE and opens a terminal a few times per day. They are not a terminal power user. They need the application to behave predictably and not require configuration to be usable out of the box. They may not know all terminal conventions.

Key needs: sensible defaults, discoverable preferences UI, no configuration required for basic use.

---

## 3. Interaction Model

### 3.1 Dual Modality

Mouse and keyboard are **equally valid** primary interaction modalities. No action exposed by TauTerm's UI should require keyboard-only access — every feature must be reachable via mouse (clicks, context menus, visible controls). Keyboard shortcuts are an efficiency layer on top of a discoverable UI, not a substitute for it.

This principle ensures that Sam (occasional user, §2.3) can use TauTerm effectively without learning any shortcuts, while Alex and Jordan (§2.1, §2.2) can optimize their workflow with the keyboard.

### 3.2 Discoverable UI

- TauTerm may use standard UI chrome — navigation bars, menu bars, sidebars, toolbars, context menus — wherever they improve discoverability and usability.
- These elements are not in tension with the terminal experience; they are the means by which non-power users navigate the application.
- The choice to include or omit any UI chrome element is a UX/UI design decision, not a constraint imposed by these requirements.

### 3.3 PTY Input Exception

Inside the terminal area itself, keyboard input is necessarily the primary modality — it is transmitted directly to the running process. This is inherent to how terminals work and is not a design choice by TauTerm.

### 3.4 Full-Screen Mode

- The user can switch TauTerm to full-screen mode, expanding the terminal area to fill the entire display with no window chrome visible.
- The user can exit full-screen mode and return to the windowed state at any time.
- Entering and exiting full-screen must be accessible via both keyboard shortcut and a discoverable UI action.
- The full-screen state does not affect the application's functionality: tabs, panes, preferences, and SSH connections remain fully accessible.

---

## 4. Terminal Multiplexing

### 4.1 Multi-tab

- The user can open multiple terminal tabs in a single window.
- Each tab hosts an independent PTY session.
- Tabs can be created, closed, and reordered.
- Tabs display a configurable title (process name or user-defined label).
- When a tab that is not currently active produces output or a process terminates within it, the user receives a visual indication on that tab (activity notification), without switching away from the current tab.

### 4.2 Multi-screen (panes)

- Within a tab, the user can split the view into multiple panes (horizontal and/or vertical splits).
- Each pane hosts an independent PTY session.
- Panes can be resized, closed, and navigated between via keyboard or mouse.

---

## 5. User Preferences

### 5.1 Persistence

- User preferences are persisted across sessions (stored locally on disk).
- Preferences survive application restarts.

### 5.2 Preferences UI

- A dedicated UI panel (e.g., settings screen or modal) allows the user to view and edit all preferences.
- Changes are applied immediately without requiring a restart, where technically feasible.
- Preferences are organized into logical sections: Keyboard, Appearance, Terminal Behavior.

---

## 6. Keyboard Shortcuts

Keyboard input in a terminal emulator serves two distinct purposes that users experience differently, and which must not be conflated:

- **Application shortcuts**: key combinations that act on TauTerm itself (open a tab, switch panes, open preferences). These are intercepted by TauTerm and never reach the PTY.
- **PTY passthrough sequences**: key sequences that are transmitted as-is to the active PTY session and interpreted by the shell or running program (e.g., `Ctrl+C`, `Ctrl+L`, arrow keys). TauTerm's role is to transmit them faithfully, not to intercept them.

All application shortcuts listed below are **default values**. Every application shortcut is user-configurable in the preferences UI.

### 6.1 Application Shortcuts (intercepted by TauTerm)

| Action | Default Shortcut |
|---|---|
| New tab | `Ctrl+Shift+T` |
| Close active tab | `Ctrl+Shift+W` |
| Paste from clipboard | `Ctrl+Shift+V` |
| Search in terminal output | `Ctrl+Shift+F` |
| Open preferences | `Ctrl+,` |

### 6.2 PTY Passthrough Sequences (transmitted to the active PTY)

These sequences are not intercepted by TauTerm. They are listed here for documentation purposes only — their behavior is defined by the shell or program running inside the PTY.

| Sequence | Conventional meaning |
|---|---|
| `Ctrl+C` | Interrupt running process |
| `Ctrl+L` | Clear screen |
| `Ctrl+D` | Send EOF / exit shell |
| Arrow Up / Down | Navigate shell command history |
| `Shift+Enter` | Line continuation (shell-dependent) |

### 6.3 Clipboard

- **Copy**: selecting text with the mouse automatically copies it to the clipboard (standard terminal behavior).
- **Paste**: the user can paste clipboard content into the active terminal using the application shortcut (default: `Ctrl+Shift+V`).

---

## 7. Terminal Navigation

### 7.1 Scrollback

- The user needs to scroll back through the terminal's output history beyond what is currently visible on screen.
- The user can scroll using the mouse wheel, keyboard shortcuts, or the scrollbar.
- The scrollback buffer retains a configurable amount of output history per pane.

### 7.2 Search in Output

- The user needs to search for text within the terminal's output history (scrollback buffer).
- The user can initiate a search from a keyboard shortcut (default application shortcut: `Ctrl+Shift+F`).
- Search results are visually highlighted in the terminal output.
- The user can navigate between matches (next / previous).

---

## 8. Theming

### 8.1 Default Theme

- TauTerm ships with a **single, carefully designed default theme** produced by the UX/UI designer.
- The default theme reflects a deliberate artistic direction (typography, color palette, spacing, iconography) and studied UX/UI decisions. It is not a generic placeholder.
- The default theme cannot be deleted, but it can be overridden by a user-created theme set as active.

### 8.2 User-Created Themes

- The user can create one or more custom themes.
- A theme defines at minimum: background color, foreground color, cursor color, selection color, and the 16 ANSI palette colors.
- Themes may also define: font family, font size, line height, border/panel colors, and UI accent colors.
- The active theme can be switched at any time from the preferences UI.
- Themes are stored persistently alongside other user preferences.

### 8.3 Design Tokens

- The theming system is based on design tokens (colors, spacing, sizing, radius). No hardcoded visual values are allowed in the UI layer.
- User-created themes map to those same tokens, ensuring visual consistency across all UI surfaces.

---

## 9. SSH Session Management

### 9.1 Opening SSH Sessions

- The user can open a new tab or a new pane as an SSH session (rather than a local PTY).
- The user needs the SSH session to be visually integrated within TauTerm's tab/pane model — not in a separate window.
- The user needs to know at a glance that a given tab or pane hosts a remote SSH session (as opposed to a local session).
- When an SSH connection is interrupted (network drop, server-side timeout, etc.), the user needs to be notified clearly — the session must not silently appear to still be running.

### 9.2 Saved Connections

- The user can configure and save SSH connections (host, port, username, identity file or password, optional label/group).
- Saved connections are listed in a dedicated UI (e.g., connection manager panel or quick-open dialog).
- From that list, the user can open a connection in a new tab or a new pane with a single action.
- Connections are stored persistently as part of user preferences.
- The user can create, edit, duplicate, and delete saved connections.

### 9.3 Security

- Credentials (passwords, passphrases) are stored using the OS keychain or an equivalent secure store — never in plain text.
- Identity files (private keys) are referenced by path; TauTerm does not copy or embed them.

### 9.4 Reconnection

- When an SSH session is interrupted, the user needs to be able to reconnect to the same saved connection without reconfiguring it from scratch.
- The reconnection action must be accessible directly from the affected tab or pane (not only from the connection manager).

---

## 10. Internationalisation (UI)

### 10.1 Language Support

- TauTerm's own UI (menus, preferences, labels, notifications, dialogs) is fully internationalised.
- The v1 release supports two languages: **English** (default and fallback) and **French**.
- The user can select their preferred language in the preferences UI; the change is applied immediately without requiring a restart.
- The selected language is persisted across sessions alongside other user preferences.

### 10.2 Scope of Internationalisation

- Only TauTerm's own UI strings are subject to internationalisation. The content of terminal sessions (shell output, program output, locale settings of remote systems) is entirely outside TauTerm's control and is not affected by the UI language setting.
- TauTerm does not modify or override the locale environment variables passed to spawned PTY sessions.

---

## 11. Distribution

### 11.1 AppImage (v1)

- For the v1 release, TauTerm is distributed as an **AppImage** for Linux.
- The AppImage must be self-contained: the user must be able to run it without installing system dependencies beyond what a standard Linux desktop provides.
- The AppImage must run on the supported architectures: x86, x86_64, ARM32, ARM64, RISC-V.

---

## 12. Out of Scope (v1)

- Plugin or extension system.
- Cloud sync of preferences, themes, or saved connections.
- Windows and macOS support.
- **Session persistence** (restoring open tabs and panes after application restart): this need is acknowledged but explicitly excluded from v1 scope. It may be addressed in a future version.
