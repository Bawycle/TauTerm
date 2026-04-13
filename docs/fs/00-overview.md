<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — Overview

> Part of the [Functional Specifications](README.md). See also: [01-terminal-emulation.md](01-terminal-emulation.md), [02-ui-navigation.md](02-ui-navigation.md), [03-remote-ssh.md](03-remote-ssh.md), [04-config-system.md](04-config-system.md), [05-scope-constraints.md](05-scope-constraints.md)

> **Version:** 1.0.0-draft
> **Date:** 2026-04-04
> **Input documents:** [User Requirements (UR.md)](../UR.md)

---

## 1. Purpose and Scope

### 1.1 Purpose

This document translates the user needs expressed in [UR.md](../UR.md) into precise, testable functional requirements for TauTerm v1. It serves as the contract between stakeholders and the development team: every feature that TauTerm v1 delivers is specified here, and every requirement here is expected to be implemented, tested, and validated.

### 1.2 What This Document Specifies

- Observable behavior of TauTerm from the user's and the system's perspective
- Acceptance criteria that can be verified through testing
- Priority classification (MoSCoW) for each requirement
- Traceability to the originating user requirement

**This is the only document in the project that uses normative language (`MUST`, `SHALL`, `SHOULD`, `MAY`).** Every functional constraint written as a requirement lives here and nowhere else — other documents reference FS IDs, they do not restate requirements.

### 1.3 What This Document Does Not Specify

- Internal architecture, module decomposition, or data structures (see `docs/adr/`)
- Implementation technology choices beyond what is externally observable
- Visual design details (layout, spacing, color values, component states) — see `docs/UXD.md`
- Interaction design (animations, transitions, feedback patterns) — see `docs/UXD.md`
- Implementation details (API names, CSS properties, algorithm choices) — see source code and comments

**Corollary — no duplication with UXD.md:** if information belongs in UXD.md, it must not appear here. When a requirement has a visible design consequence, FS.md states the requirement; UXD.md specifies the design that satisfies it and references the FS ID. Writing the same constraint in both documents creates a SSOT violation and a future contradiction risk.

### 1.4 Conventions

| Term | Meaning |
|------|---------|
| MUST / SHALL | Mandatory for v1 delivery. Failure to meet this requirement is a release blocker. |
| SHOULD | Recommended. Expected for v1 unless a documented technical constraint prevents it. |
| MAY | Optional. Desirable but acceptable to defer beyond v1. |

Requirement identifiers follow the pattern `FS-<AREA>-<NNN>` where `<AREA>` is a feature area code and `<NNN>` is a sequential number.

---

## 2. Glossary

| Term | Definition |
|------|------------|
| **Alternate screen buffer** | A secondary screen buffer used by full-screen applications (vim, less, htop). It has no scrollback history. Switching to it preserves the normal screen content; returning restores it. |
| **ANSI escape sequence** | A sequence of bytes beginning with ESC (0x1B) that controls terminal behavior: cursor movement, text styling, screen clearing, mode changes. |
| **Bell (BEL)** | The byte 0x07, which triggers a notification (visual or audible) in the terminal emulator. |
| **Bracketed paste** | A mode (DECSET 2004) where pasted text is wrapped in escape sequences so the application can distinguish typed input from pasted text. |
| **C0 control** | The control characters in the range 0x00–0x1F (e.g., BEL, BS, TAB, LF, CR, ESC). |
| **Cell** | A single character position in the terminal grid, defined by its row and column. A cell holds one character (or one half of a wide character) plus its attributes. |
| **CJK** | Chinese, Japanese, Korean — languages whose characters are typically rendered as double-width (2 cells). |
| **Combining character** | A Unicode character that modifies the preceding base character (e.g., diacritical marks) without advancing the cursor. |
| **CSI** | Control Sequence Introducer: ESC [ — the prefix for most ANSI control sequences. |
| **DECCKM** | DEC Cursor Key Mode — when set, arrow keys emit application-mode sequences (ESC O A/B/C/D) instead of normal-mode sequences (ESC [ A/B/C/D). |
| **DECSCUSR** | DEC Set Cursor Style — a sequence that changes the cursor shape (block, underline, bar) and blink state. |
| **DECSET / DECRST** | DEC Private Mode Set / Reset — sequences that enable or disable terminal modes (e.g., alternate screen, mouse reporting, focus events). |
| **DECSTBM** | DEC Set Top and Bottom Margins — defines the scrolling region within the terminal. |
| **Design token** | A named value (color, spacing, size, radius) that serves as the single source of truth for a visual property across the entire UI. |
| **DCS** | Device Control String — an escape sequence framing protocol for device-specific data. |
| **Focus event** | A notification sent to the application when the terminal gains or loses input focus (mode 1004). |
| **IME** | Input Method Editor — system component for composing characters in languages that require multi-keystroke input (CJK, etc.). |
| **MoSCoW** | Prioritization method: Must have, Should have, Could have, Won't have (this time). |
| **Mouse reporting** | Terminal modes that cause mouse events (clicks, movement, wheel) to be encoded and sent to the application via the PTY. |
| **Normal screen buffer** | The primary screen buffer where shell output appears. It is connected to the scrollback buffer. |
| **OSC** | Operating System Command — escape sequences for setting terminal properties such as window title and hyperlinks. |
| **Pane** | A subdivision of a tab that hosts an independent terminal session. Panes are created by splitting a tab horizontally or vertically. |
| **PRIMARY selection** | The X11/Wayland selection buffer populated by selecting text. Pasted with middle-click. Distinct from the CLIPBOARD selection. |
| **PTY** | Pseudo-terminal — a kernel-level pair (master + slave) that provides a terminal interface to a child process. |
| **Scroll region** | A subset of terminal rows (defined by DECSTBM) within which scrolling operations are confined. |
| **Scrollback buffer** | The history of lines that have scrolled off the top of the visible terminal area. Users can navigate it to review past output. |
| **SGR** | Select Graphic Rendition — the CSI sequence that sets text attributes: colors, bold, italic, underline, etc. |
| **SIGCHLD** | The Unix signal sent to a parent process when a child process terminates. |
| **SIGHUP** | The Unix signal sent to a process group when its controlling terminal is closed. |
| **SIGWINCH** | The Unix signal sent to the foreground process group when the terminal window size changes. |
| **Soft wrap** | A line break introduced by the terminal emulator when output exceeds the terminal width. It is not part of the original output and is transparent to search. |
| **SSH** | Secure Shell — a protocol for encrypted remote login and command execution. |
| **Tab** | A top-level container in TauTerm that holds one or more panes. Each tab is represented by a clickable element in the tab bar. |
| **TOFU** | Trust On First Use — a security model where a host's identity is accepted on first connection and verified on subsequent connections. |
| **Truecolor** | 24-bit color (16.7 million colors), specified as RGB triplets in SGR sequences. |
| **VT100 / VT220** | DEC video terminal models whose escape sequence protocols form the basis of modern terminal emulation. |
| **Wide character** | A character that occupies two cells in the terminal grid (e.g., CJK ideographs, certain emoji). |
| **xterm-256color** | A terminal type identifier that indicates support for 256-color SGR, xterm control sequences, and associated capabilities. |
| **ZWJ** | Zero-Width Joiner — a Unicode character (U+200D) used to combine multiple codepoints into a single glyph (common in emoji sequences). |
