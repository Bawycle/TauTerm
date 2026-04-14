<!-- SPDX-License-Identifier: MPL-2.0 -->

# Changelog

All notable changes to TauTerm are documented in this file.

The format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-beta.2] - 2026-04-14

### Added

- Terminal now responds to capability queries from tmux, Claude Code, and other tools — faster tmux attach, better feature detection by TUI frameworks

### Fixed

- Typing in one tab no longer freezes all other tabs when a shell is slow to read (e.g. under PTY backpressure)
- Reverse-video text (SGR 7) with default colors now displays correctly — fixes invisible cursor in Claude Code and other Ink-based CLI tools
- Release downloads now include individual SHA-256 checksum files for integrity verification

## [0.1.0-beta] - 2026-04-13

### Added

#### Terminal

- Configurable scrollback buffer with regex search (`Ctrl+Shift+F`)
- Three cursor shapes: block, underline, bar — with configurable blink rate (or no blink)
- Bell notification: none, visual flash, audio, or both
- Multiline paste confirmation dialog (can be permanently dismissed)
- Applications can write to the clipboard (opt-in, off by default)
- Mouse cursor auto-hides while typing (toggleable in Preferences)

#### Tabs

- Multi-tab interface: create (`Ctrl+Shift+T`), close (`Ctrl+Shift+W`), switch (`Ctrl+Tab` / `Ctrl+Shift+Tab`)
- Tab rename via double-click, F2, or context menu
- Drag-and-drop tab reorder
- Tab overflow scroll with activity badges on scroll arrows for off-screen tabs
- Activity indicators: output badge, bell badge, process exit (success/error)
- Tab title auto-updates to reflect current directory or foreground process name

#### Panes

- Horizontal split (`Ctrl+Shift+D`) and vertical split (`Ctrl+Shift+E`)
- Keyboard navigation between panes (`Ctrl+Shift+Arrow`)
- Per-pane title bar showing process name or directory (toggleable in Preferences)
- New split opens in the same directory as the current pane
- New tab inherits the working directory of the active pane (FS-VT-064)
- `/proc` fallback for CWD detection when the shell does not emit OSC 7

#### SSH

- Connect to remote hosts via SSH from the tab bar
- SSH credential dialog with password/passphrase entry and keychain save
- Trust-on-first-use host key dialog with fingerprint display (accept, reject, changed-key warning)
- Deprecated algorithm warning banner
- Saved connection management: create, edit, duplicate, delete
- SSH reconnection from the pane

#### Theming

- Three built-in themes: Umbra (dark), Solstice (light), Archipel (dark)
- User-defined themes: create, edit, duplicate, delete
- Full color palette editor (foreground, background, cursor, selection background, and all 16 terminal colors)
- Per-theme line height override

#### Preferences

- Preferences panel (`Ctrl+,`) with five sections: Keyboard, Appearance, Terminal, Themes, Connections
- Customizable keyboard shortcuts for all 16 actions via key recorder
- Font family and size
- Background opacity
- Language switching: English and French
- Cross-instance live-reload: changes saved in one window apply immediately to all open instances

#### Fullscreen

- Fullscreen mode (F11) with auto-hide chrome, persisted across launches

#### Accessibility

- Keyboard navigation, screen reader support, and reduced motion support across all UI
