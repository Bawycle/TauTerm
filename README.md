# TauTerm

A terminal emulator for Linux, built with Rust and Tauri 2. Currently Linux-only.

![Version: 0.1.0-beta.3](https://img.shields.io/badge/version-0.1.0--beta.3-yellow)
![MSRV: 1.94.1](https://img.shields.io/badge/MSRV-1.94.1-orange)
![License: MPL-2.0](https://img.shields.io/badge/license-MPL--2.0-blue)

**Status: beta (v0.1.0-beta.3)** — Core features are implemented. Under active stabilization — expect rough edges.

![TauTerm with Umbra theme showing split panes and tab bar](docs/images/screenshot.png)

## Features

- Multi-tab terminal with independent PTY sessions and background activity notifications
- Split panes (horizontal and vertical), each with its own independent session
- Full VT parser with configurable scrollback buffer
- Search in terminal output
- SSH as a first-class citizen: saved connection profiles, secure credential storage via D-Bus Secret Service
- Three built-in themes — Umbra (dark, default), Solstice (light), Archipel (dark, tropical) — plus custom themes via design tokens
- Customizable keyboard shortcuts
- Every feature accessible via both mouse and keyboard
- Full-screen mode
- English and French interface

## Build and Run

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed prerequisites. In short: Rust 1.94.1, Node.js 22, pnpm 10, and a few system packages on Debian/Ubuntu.

```bash
git clone git@github.com:Bawycle/TauTerm.git
cd TauTerm
pnpm install
pnpm tauri dev
```

Production build (`pnpm tauri build`) produces an AppImage.

## Default Shortcuts

All shortcuts are customizable in Preferences (`Ctrl+,`).

| Action | Shortcut |
|---|---|
| New tab | `Ctrl+Shift+T` |
| Close tab | `Ctrl+Shift+W` |
| Next tab | `Ctrl+Tab` |
| Previous tab | `Ctrl+Shift+Tab` |
| Rename tab | `F2` |
| Split pane horizontally | `Ctrl+Shift+D` |
| Split pane vertically | `Ctrl+Shift+E` |
| Close pane | `Ctrl+Shift+Q` |
| Navigate panes | `Ctrl+Shift+Arrow` |
| Paste | `Ctrl+Shift+V` |
| Search | `Ctrl+Shift+F` |
| Toggle full-screen | `F11` |

## Themes

TauTerm ships with three built-in themes. **Umbra** (default) is a dark theme built on warm neutrals and cool steel-blue accents — designed for all-day use without eye fatigue. **Solstice** is a high-contrast light theme with cold, Nordic tones. **Archipel** is a dark theme with saturated tropical accents for users who find neutral palettes visually flat.

Custom themes can be created using design tokens. See [docs/AD.md](docs/AD.md) for the full artistic direction.

## Documentation

| Document | Content |
|---|---|
| [docs/arch/](docs/arch/) | Architecture: modules, IPC, state machines, concurrency |
| [docs/AD.md](docs/AD.md) | Artistic direction and theme design |
| [docs/UR.md](docs/UR.md) | User requirements and personas |
| [docs/fs/](docs/fs/) | Functional specifications |
| [docs/uxd/](docs/uxd/) | UX/UI design: tokens, components, interactions |
| [docs/testing/TESTING.md](docs/testing/TESTING.md) | Test strategy |
| [docs/adr/](docs/adr/) | Architecture Decision Records |
| [CHANGELOG.md](CHANGELOG.md) | Release history |
| [SECURITY.md](SECURITY.md) | Security policy and vulnerability reporting |

## Architecture

The Rust backend handles PTY management, VT parsing, SSH, and terminal state. The Svelte 5 frontend renders the terminal UI. All communication crosses the IPC boundary via Tauri commands and events.

```
src-tauri/src/    Rust backend (PTY, VT parser, SSH, IPC commands)
src/              Svelte 5 frontend (terminal rendering, tabs, panes, preferences)
docs/             Project documentation
scripts/          CI and development scripts
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards, and pull request guidelines.

## Roadmap

Planned for future releases:

- Session persistence across restarts
- Pane maximize/restore without destroying splits
- SSH jump host (ProxyJump) support
- OSC 8 hyperlinks in terminal output
- OSC 1337 inline images
- GPG-signed releases with SHA256SUMS verification
- Windows 11 support

## Acknowledgments

TauTerm is built on the work of many open-source projects and their maintainers:

- [Tauri](https://tauri.app/) — application framework
- [Svelte](https://svelte.dev/) and [SvelteKit](https://svelte.dev/docs/kit) — frontend framework
- [Tailwind CSS](https://tailwindcss.com/) — utility-first CSS
- [Bits UI](https://bits-ui.com/) — headless UI primitives
- [Paraglide JS](https://inlang.com/m/gerre34r/library-inlang-paraglideJs) (inlang) — i18n
- [vte](https://crates.io/crates/vte) — VT parser
- [portable-pty](https://crates.io/crates/portable-pty) — cross-platform PTY management
- [russh](https://crates.io/crates/russh) — pure-Rust SSH implementation
- [Tokio](https://tokio.rs/) — async runtime
- [secret-service](https://crates.io/crates/secret-service) — D-Bus Secret Service client
- [Lucide](https://lucide.dev/) — icon set

And to every crate and package in the dependency tree that makes this possible — thank you.

## License

[MPL-2.0](LICENSE)
