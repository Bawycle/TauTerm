# User Requirements — TauTerm

## 1. Overview

TauTerm is a terminal emulator targeting Linux (x86, x86_64, ARM32, ARM64, RISC-V). It supports multiple tabs and multiple screens within a single window, with a studied UX/UI and persistent user preferences.

---

## 2. Terminal Multiplexing

### 2.1 Multi-tab

- The user can open multiple terminal tabs in a single window.
- Each tab hosts an independent PTY session.
- Tabs can be created, closed, and reordered.
- Tabs display a configurable title (process name or user-defined label).

### 2.2 Multi-screen (panes)

- Within a tab, the user can split the view into multiple panes (horizontal and/or vertical splits).
- Each pane hosts an independent PTY session.
- Panes can be resized, closed, and navigated between via keyboard or mouse.

---

## 3. User Preferences

### 3.1 Persistence

- User preferences are persisted across sessions (stored locally on disk).
- Preferences survive application restarts.

### 3.2 Preferences UI

- A dedicated UI panel (e.g., settings screen or modal) allows the user to view and edit all preferences.
- Changes are applied immediately without requiring a restart, where technically feasible.
- Preferences are organized into logical sections: Keyboard, Appearance, Terminal Behavior.

---

## 4. Keyboard Shortcuts

All shortcuts listed below are **default values**. Every shortcut is user-configurable in the preferences UI.

| Action | Default Shortcut |
|---|---|
| Paste from clipboard | `Ctrl+Shift+V` |
| Clear screen | `Ctrl+L` |
| Newline without executing (line continuation) | `Shift+Enter` |

### 4.1 Clipboard

- **Copy**: selecting text with the mouse automatically copies it to the clipboard (standard terminal behavior).
- **Paste**: keyboard shortcut pastes the clipboard content into the active terminal (default: `Ctrl+Shift+V`).

### 4.2 Command History Navigation

- **Up arrow**: recalls the previous command in the shell history.
- **Down arrow**: navigates forward in the shell history (toward the current input).
- This behavior is delegated to the shell running inside the PTY; TauTerm transmits the appropriate escape sequences.

### 4.3 Line Continuation

- `Shift+Enter` sends a line-continuation sequence equivalent to `\` followed by a newline in a classic terminal — allowing the user to compose multi-line commands before execution.

### 4.4 Clear Screen

- `Ctrl+L` sends the clear-screen sequence to the active PTY, equivalent to typing `clear` or pressing `Ctrl+L` in a standard terminal.

---

## 5. Theming

### 5.1 Default Theme

- TauTerm ships with a **single, carefully designed default theme** produced by the UX/UI designer.
- The default theme reflects a deliberate artistic direction (typography, color palette, spacing, iconography) and studied UX/UI decisions. It is not a generic placeholder.
- The default theme cannot be deleted, but it can be overridden by a user-created theme set as active.

### 5.2 User-Created Themes

- The user can create one or more custom themes.
- A theme defines at minimum: background color, foreground color, cursor color, selection color, and the 16 ANSI palette colors.
- Themes may also define: font family, font size, line height, border/panel colors, and UI accent colors.
- The active theme can be switched at any time from the preferences UI.
- Themes are stored persistently alongside other user preferences.

### 5.3 Design Tokens

- The theming system is based on design tokens (colors, spacing, sizing, radius). No hardcoded visual values are allowed in the UI layer.
- User-created themes map to those same tokens, ensuring visual consistency across all UI surfaces.

---

## 6. SSH Session Management

### 6.1 Opening SSH Sessions

- The user can open a new tab or a new pane as an SSH session (rather than a local PTY).
- SSH sessions behave identically to local sessions from a terminal UX standpoint (same multiplexing, theming, shortcuts).

### 6.2 Saved Connections

- The user can configure and save SSH connections (host, port, username, identity file or password, optional label/group).
- Saved connections are listed in a dedicated UI (e.g., connection manager panel or quick-open dialog).
- From that list, the user can open a connection in a new tab or a new pane with a single action.
- Connections are stored persistently as part of user preferences.
- The user can create, edit, duplicate, and delete saved connections.

### 6.3 Security

- Credentials (passwords, passphrases) are stored using the OS keychain or an equivalent secure store — never in plain text.
- Identity files (private keys) are referenced by path; TauTerm does not copy or embed them.

---

## 7. Out of Scope (v1)

- Plugin or extension system.
- Cloud sync of preferences, themes, or saved connections.
- Windows and macOS support.
