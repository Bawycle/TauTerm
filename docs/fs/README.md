<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — Index

This directory contains the Functional Specifications for TauTerm, split by feature area for navigability. **FS requirement IDs (e.g. `FS-VT-023`) do not change — only the file location changes.**

> **Version:** 1.0.0-draft
> **Date:** 2026-04-04
> **Status:** Draft
> **Input documents:** [User Requirements (UR.md)](../UR.md), Domain Expert Technical Analysis

---

## Files

| File | Description |
|------|-------------|
| [00-overview.md](00-overview.md) | Purpose, scope, conventions (§1), and full glossary (§2) |
| [01-terminal-emulation.md](01-terminal-emulation.md) | VT parser, PTY lifecycle, scrollback buffer, and search |
| [02-ui-navigation.md](02-ui-navigation.md) | Tabs, panes, keyboard input, clipboard, notifications, and UX cross-cutting requirements |
| [03-remote-ssh.md](03-remote-ssh.md) | SSH session management and credential security |
| [04-config-system.md](04-config-system.md) | Theming, preferences, accessibility, security hardening, and internationalisation |
| [05-scope-constraints.md](05-scope-constraints.md) | Distribution, out-of-scope items, domain constraints, traceability matrix, and review notes |
| [06-fullscreen.md](06-fullscreen.md) | Full-screen mode: enter/exit, keyboard shortcut, PTY resize, persistence |

---

## FS Area Code → File

| FS Area Code | Description | File |
|---|---|---|
| FS-VT | Terminal Emulation (VT parser, colors, cursor, screen modes, scrolling regions, titles, hyperlinks, clipboard sequences, mouse, bell) | [01-terminal-emulation.md](01-terminal-emulation.md) |
| FS-PTY | PTY Lifecycle (spawn, exit, resize, environment, shell fallback) | [01-terminal-emulation.md](01-terminal-emulation.md) |
| FS-SB | Scrollback Buffer | [01-terminal-emulation.md](01-terminal-emulation.md) |
| FS-SEARCH | Search in Output | [01-terminal-emulation.md](01-terminal-emulation.md) |
| FS-TAB | Multi-Tab Management | [02-ui-navigation.md](02-ui-navigation.md) |
| FS-PANE | Multi-Pane (Split View) | [02-ui-navigation.md](02-ui-navigation.md) |
| FS-KBD | Keyboard Input Handling | [02-ui-navigation.md](02-ui-navigation.md) |
| FS-CLIP | Clipboard Integration | [02-ui-navigation.md](02-ui-navigation.md) |
| FS-NOTIF | Activity Notifications | [02-ui-navigation.md](02-ui-navigation.md) |
| FS-UX | UX Cross-Cutting Requirements | [02-ui-navigation.md](02-ui-navigation.md) |
| FS-SSH | SSH Session Management | [03-remote-ssh.md](03-remote-ssh.md) |
| FS-CRED | Credential Security | [03-remote-ssh.md](03-remote-ssh.md) |
| FS-THEME | Theming System | [04-config-system.md](04-config-system.md) |
| FS-PREF | User Preferences | [04-config-system.md](04-config-system.md) |
| FS-A11Y | Accessibility | [04-config-system.md](04-config-system.md) |
| FS-SEC | Security Hardening | [04-config-system.md](04-config-system.md) |
| FS-I18N | Internationalisation | [04-config-system.md](04-config-system.md) |
| FS-DIST | Distribution | [05-scope-constraints.md](05-scope-constraints.md) |
| FS-FULL | Full-Screen Mode | [06-fullscreen.md](06-fullscreen.md) |

---

## Conventions

- Normative language (`MUST`, `SHALL`, `SHOULD`, `MAY`) appears **only** in this specification — never in `UXD.md` or other documents.
- Requirement IDs follow the pattern `FS-<AREA>-<NNN>`. IDs are stable: moving content between files does not change IDs.
- UXD.md references FS IDs (e.g. `(FS-TAB-009)`) and describes only the design expression — it never restates requirements.
- For the glossary of domain terms (PTY, OSC, DECSTBM, etc.), see [00-overview.md](00-overview.md#2-glossary).
