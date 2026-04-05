# Functional Specifications — TauTerm

> **Version:** 1.0.0-draft
> **Date:** 2026-04-04
> **Status:** Draft
> **Input documents:** [User Requirements (UR.md)](UR.md), Domain Expert Technical Analysis

---

## 1. Purpose and Scope

### 1.1 Purpose

This document translates the user needs expressed in [UR.md](UR.md) into precise, testable functional requirements for TauTerm v1. It serves as the contract between stakeholders and the development team: every feature that TauTerm v1 delivers is specified here, and every requirement here is expected to be implemented, tested, and validated.

### 1.2 What This Document Specifies

- Observable behavior of TauTerm from the user's and the system's perspective
- Acceptance criteria that can be verified through testing
- Priority classification (MoSCoW) for each requirement
- Traceability to the originating user requirement

### 1.3 What This Document Does Not Specify

- Internal architecture, module decomposition, or data structures (see Architecture Decision Records in `docs/adr/`)
- Implementation technology choices beyond what is externally observable
- Visual design details (layout, spacing, color values) — those are defined by the UX/UI designer and the design token system

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

---

## 3. Functional Requirements

### 3.1 FS-VT: Terminal Emulation

#### 3.1.1 Target Standard

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-001 | TauTerm MUST advertise itself as `xterm-256color` by setting the `TERM` environment variable to `xterm-256color` in every PTY session. | Must |
| FS-VT-002 | TauTerm MUST set `COLORTERM=truecolor` in every PTY session. | Must |
| FS-VT-003 | TauTerm MUST implement the VT100/VT220 control sequence subset: DECSC, DECRC, DECSTBM, DECSET/DECRST. | Must |
| FS-VT-004 | TauTerm MUST implement xterm extensions: 256-color SGR, truecolor SGR, mouse reporting, focus events, bracketed paste, and window title sequences. | Must |
| FS-VT-005 | TauTerm MUST parse ECMA-48 CSI, OSC, and DCS sequences via a conformant state machine. | Must |

**Acceptance criteria:**
- FS-VT-001: `echo $TERM` in a new session outputs `xterm-256color`.
- FS-VT-002: `echo $COLORTERM` in a new session outputs `truecolor`.
- FS-VT-003: Programs that rely on VT100/VT220 sequences (e.g., vim, less, htop) render correctly.
- FS-VT-004: 256-color and truecolor test scripts display correct colors; mouse-aware applications respond to clicks; bracketed paste wraps pasted content.
- FS-VT-005: Malformed sequences are discarded without corrupting subsequent output; no sequence fragment persists across reads.

#### 3.1.2 Character Set Handling

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-010 | TauTerm MUST handle UTF-8 natively, including multi-byte sequences split across read boundaries. | Must |
| FS-VT-011 | CJK wide characters MUST occupy exactly 2 cells. A wide character at the last column of a row MUST wrap to the next line. | Must |
| FS-VT-012 | Combining characters MUST attach to the preceding base character without advancing the cursor. | Must |
| FS-VT-013 | Zero-width characters MUST NOT advance the cursor or consume a cell. | Must |
| FS-VT-014 | Single-codepoint wide emoji MUST occupy 2 cells. ZWJ emoji sequences SHOULD render as a single glyph occupying 2 cells when the font supports it. | Should |
| FS-VT-015 | DEC Special Graphics character set (SI/SO, ESC ( 0) MUST be supported for line-drawing characters. | Must |
| FS-VT-016 | Invalid UTF-8 sequences (e.g., overlong encodings) MUST be replaced with U+FFFD (REPLACEMENT CHARACTER). | Must |

**Acceptance criteria:**
- FS-VT-010: A program outputting a multi-byte character split across two write() calls renders the character correctly.
- FS-VT-011: Chinese/Japanese characters are correctly positioned; `echo -e "\xe4\xb8\xad"` at the last column wraps to the next line.
- FS-VT-012: `echo -e "e\xcc\x81"` (e + combining acute accent) displays as a single accented character in one cell.
- FS-VT-013: Zero-width space (U+200B) does not create a visible gap or shift subsequent characters.
- FS-VT-015: `mc` (Midnight Commander) line-drawing borders render correctly.
- FS-VT-016: Overlong UTF-8 byte `0xC0 0xAF` renders as U+FFFD.

#### 3.1.3 ANSI Color Codes

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-020 | TauTerm MUST support standard ANSI colors: SGR 30–37, 40–47 (normal), 90–97, 100–107 (bright). | Must |
| FS-VT-021 | TauTerm MUST support 256-color mode: SGR 38;5;N / 48;5;N for N in 0–255. Colors 0–7 map to ANSI palette, 8–15 to bright variants, 16–231 to the 6x6x6 color cube, 232–255 to the grayscale ramp. | Must |
| FS-VT-022 | TauTerm MUST support truecolor: SGR 38;2;R;G;B and 48;2;R;G;B. The colon variant (38:2:R:G:B, ITU T.416) MUST also be supported. | Must |
| FS-VT-023 | ANSI palette colors 0–15 MUST be remappable via the active theme. Truecolor values are absolute and not affected by the theme palette. | Must |
| FS-VT-024 | SGR 0 MUST reset all attributes. The following attributes MUST be independently settable and resettable: bold (1/22), dim (2/22), italic (3/23), underline (4/24), blink (5/25), inverse (7/27), hidden (8/28), strikethrough (9/29). | Must |
| FS-VT-025 | Extended underline styles (SGR 4:0 through 4:5) and underline color (SGR 58) SHOULD be supported. | Should |

**Acceptance criteria:**
- FS-VT-020: A test script cycling through SGR 30–37 and 90–97 displays 16 distinct foreground colors.
- FS-VT-021: A 256-color test pattern (e.g., `256colors.pl`) displays all colors correctly with smooth gradients in the cube and ramp regions.
- FS-VT-022: `printf "\033[38;2;255;100;0mTruecolor\033[0m"` displays orange text. The colon variant produces the same result.
- FS-VT-023: Changing the theme's color 1 (red) changes the color displayed by SGR 31, but has no effect on `\033[38;2;255;0;0m`.
- FS-VT-024: Each attribute can be turned on and off independently without affecting other active attributes.
- FS-VT-025: Neovim diagnostic underlines (curly, dotted) render with the correct style and color.

#### 3.1.4 Cursor

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-030 | TauTerm MUST support cursor shapes 0–6 via DECSCUSR (CSI Ps SP q): default, blinking block, steady block, blinking underline, steady underline, blinking bar, steady bar. | Must |
| FS-VT-031 | Cursor visibility MUST be controllable via DECTCEM (CSI ?25h to show, CSI ?25l to hide). | Must |
| FS-VT-032 | Cursor blink rate MUST be configurable in user preferences. The default MUST be 530ms on/off. | Must |
| FS-VT-033 | DECSC and DECRC MUST save and restore cursor state per screen buffer, independently. | Must |
| FS-VT-034 | When the terminal pane loses focus, the cursor SHOULD display as an outline variant of its current shape. When focus returns, the cursor MUST resume its normal appearance. | Should |

**Acceptance criteria:**
- FS-VT-030: A script emitting each DECSCUSR value produces the corresponding cursor shape.
- FS-VT-031: `tput civis` hides the cursor; `tput cnorm` restores it.
- FS-VT-032: Changing the blink rate in preferences visibly changes the cursor blink speed without restart.
- FS-VT-033: In vim (alternate screen), saving/restoring the cursor does not affect the normal screen cursor position, and vice versa.
- FS-VT-034: Clicking outside the terminal pane changes the cursor to an outline; clicking back restores it.

#### 3.1.5 Screen Modes

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-040 | TauTerm MUST support alternate screen buffer activation and deactivation via DECSET/DECRST modes 1049, 47, and 1047. | Must |
| FS-VT-041 | Mode 1049 MUST save the cursor, switch to the alternate screen, and clear it. DECRST 1049 MUST switch to the normal screen and restore the cursor. | Must |
| FS-VT-042 | The alternate screen buffer MUST NOT have scrollback history. | Must |
| FS-VT-043 | Each screen buffer (normal and alternate) MUST maintain independent state: cursor position, text attributes, saved cursor, scroll region, and active character set. | Must |
| FS-VT-044 | On return to the normal screen, the content MUST be restored exactly as it was before the switch to the alternate screen. | Must |

**Acceptance criteria:**
- FS-VT-040: Opening and closing vim, less, or htop switches between screen buffers without artifacts.
- FS-VT-041: Exiting vim restores the cursor to its pre-vim position.
- FS-VT-042: While vim is open, scrolling produces no scrollback content.
- FS-VT-043: Running a full-screen application does not corrupt the normal screen's scroll region or cursor state.
- FS-VT-044: After exiting vim, the scrollback contains only the shell output that preceded and followed the vim session — no vim screen content.

#### 3.1.6 Scrolling Regions

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-050 | TauTerm MUST support DECSTBM (CSI Pt ; Pb r) to define a scrolling region. All scroll operations MUST be restricted to the active region. | Must |
| FS-VT-051 | A line feed at the bottom margin of a scroll region MUST scroll only the region, not the full screen. | Must |
| FS-VT-052 | CSI Ps S (scroll up) and CSI Ps T (scroll down) MUST respect the active scroll region. | Must |
| FS-VT-053 | Lines scrolled out of a partial scroll region (one that does not span the full screen) MUST NOT enter the scrollback buffer. Only lines scrolled out of a full-screen region enter scrollback. | Must |
| FS-VT-054 | DECSTBM with parameters 0;0 or no parameters MUST reset the scroll region to the full screen. | Must |

**Acceptance criteria:**
- FS-VT-050: tmux with a status bar: the status bar remains fixed while the main area scrolls.
- FS-VT-051: Output within a partial scroll region does not affect lines outside the region.
- FS-VT-053: After running tmux and scrolling its main pane, the scrollback does not contain the tmux status bar.
- FS-VT-054: After tmux exits, the scroll region is reset and normal scrolling resumes.

#### 3.1.7 Title / OSC Sequences

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-060 | OSC 0, OSC 1, and OSC 2 MUST set the tab title. | Must |
| FS-VT-061 | OSC 22 (push title) and OSC 23 (pop title) SHOULD be supported as a title stack. | Should |
| FS-VT-062 | Tab titles MUST be sanitized: C0 and C1 control characters stripped; maximum length of 256 characters (truncated if exceeded). | Must |
| FS-VT-063 | Terminal property read-back sequences MUST be silently discarded. This includes: CSI 21t (report window title), OSC queries that would inject the response into the PTY input stream, and any DECRQSS/DECRPM response that echoes terminal state into the input buffer. These sequences are a known injection vector and MUST NOT produce any response. | Must |

**Acceptance criteria:**
- FS-VT-060: `printf "\033]0;My Title\007"` changes the tab title to "My Title".
- FS-VT-061: A program that pushes and pops titles restores the previous title correctly.
- FS-VT-062: A title containing control characters displays without them; a 300-character title is truncated to 256 characters.
- FS-VT-063: `printf "\033[21t"` produces no response in the input stream. A malicious title set via OSC 0 followed by a CSI 21t read-back does not inject the title into the PTY input.

#### 3.1.8 Hyperlinks

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-070 | TauTerm MUST support OSC 8 hyperlinks (start and end). | Must |
| FS-VT-071 | Hovering over a hyperlink MUST display the URI. Ctrl+Click MUST open the URI in the system default browser. | Must |
| FS-VT-072 | The `id` parameter for multi-line hyperlinks MUST be supported. | Must |
| FS-VT-073 | URI scheme validation MUST allow only: `http`, `https`, `mailto`, `ssh`. The `file` scheme MUST only be allowed for local PTY sessions (not SSH sessions). All other schemes (including `javascript:`, `data:`, `blob:`, `vbscript:`, and unknown custom schemes) MUST be rejected. URIs MUST be limited to 2048 characters. URIs containing C0/C1 control characters, or null bytes, MUST be rejected. | Must |

**Acceptance criteria:**
- FS-VT-070: `printf "\033]8;;https://example.com\033\\Link\033]8;;\033\\"` renders "Link" as a hyperlink.
- FS-VT-071: Hovering shows `https://example.com`; Ctrl+Click opens the system browser.
- FS-VT-072: A hyperlink spanning two wrapped lines is treated as a single link.
- FS-VT-073: A link with `javascript:alert(1)` is not clickable. A `data:text/html,...` URI is not clickable. A URI exceeding 2048 characters is not rendered as a hyperlink. A `file:///etc/passwd` link in an SSH session is not clickable.

#### 3.1.9 Clipboard Control Sequences (OSC 52)

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-075 | OSC 52 clipboard write (setting clipboard content from PTY output) MUST be disabled by default. It MAY be enabled per connection: each saved connection (local or SSH) MAY independently enable or disable OSC 52 write. A local PTY session (no saved connection) uses the global default (disabled). This prevents enabling OSC 52 write for trusted local sessions from inadvertently enabling it for untrusted SSH sessions. | Must |
| FS-VT-076 | OSC 52 clipboard read (querying clipboard content) MUST be permanently rejected. No configuration option MUST allow enabling it. This prevents clipboard exfiltration by malicious programs or remote servers. | Must |

**Acceptance criteria:**
- FS-VT-075: By default, `printf "\033]52;c;$(echo -n 'malicious' | base64)\007"` does not modify the system clipboard. In a saved connection with OSC 52 write enabled, the same sequence updates the clipboard. An SSH session with OSC 52 write disabled ignores the sequence even if local sessions have it enabled.
- FS-VT-076: `printf "\033]52;c;?\007"` never produces a response containing clipboard content, regardless of configuration.

#### 3.1.10 Mouse Reporting

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-080 | TauTerm MUST support mouse tracking modes: X10 (9), Normal (1000), Button-event (1002), and Any-event (1003). | Must |
| FS-VT-081 | TauTerm MUST support mouse encodings: default X10 and SGR mode 1006. URXVT mode 1015 SHOULD be supported. | Must / Should |
| FS-VT-082 | When mouse reporting is active, mouse events MUST be sent to the PTY. When inactive, mouse events MUST be handled by TauTerm (selection, scrolling, etc.). | Must |
| FS-VT-083 | Shift+Click MUST bypass mouse reporting and perform TauTerm selection regardless of reporting mode. | Must |
| FS-VT-084 | Focus events (mode 1004) MUST be supported. | Must |
| FS-VT-085 | Mouse wheel events when reporting is active MUST be sent to the PTY as button 4/5 events. Shift+Wheel MUST scroll the scrollback instead. | Must |
| FS-VT-086 | On application exit, all mouse reporting modes MUST be reset. | Must |

**Acceptance criteria:**
- FS-VT-080: vim with `set mouse=a` responds to click-to-position correctly.
- FS-VT-083: With vim mouse capture active, Shift+Click selects text in TauTerm.
- FS-VT-085: In vim with mouse enabled, wheel scrolls the vim buffer; Shift+Wheel scrolls TauTerm's scrollback.
- FS-VT-086: After vim exits, clicking in the terminal performs TauTerm selection (not mouse reporting).

#### 3.1.11 Bell

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-090 | BEL (0x07) MUST trigger a notification. The notification type MUST be configurable: visual (flash or tab highlight), audible (system sound), or disabled. | Must |
| FS-VT-091 | The default notification type MUST be visual bell. | Must |
| FS-VT-092 | Bell notifications MUST be rate-limited to at most one action per 100ms. | Must |
| FS-VT-093 | A bell in a non-active tab or pane MUST produce a visual indicator on that tab or pane. | Must |

**Acceptance criteria:**
- FS-VT-090: `printf "\007"` triggers the configured notification.
- FS-VT-091: Out of the box, the bell produces a visual flash rather than a sound.
- FS-VT-092: A rapid sequence of BEL characters (e.g., `yes $'\a' | head -100`) does not produce 100 separate notifications.
- FS-VT-093: A bell in a background tab causes a visible indicator on that tab.

---

### 3.2 FS-PTY: PTY Lifecycle

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-PTY-001 | Each pane MUST have its own independent PTY pair (master + slave). | Must |
| FS-PTY-002 | The child process MUST be spawned with the slave PTY as its controlling terminal (via `setsid()` + `TIOCSCTTY`). | Must |
| FS-PTY-003 | The master file descriptor MUST be operated in non-blocking mode for asynchronous I/O. | Must |
| FS-PTY-004 | File descriptors MUST be properly managed after fork: the slave fd closed in the parent process, the master fd closed in the child process. | Must |
| FS-PTY-005 | Shell process exit MUST be detected (via SIGCHLD/waitpid). The pane MUST transition to a "terminated" state displaying the exit status. The pane MUST NOT auto-close. | Must |
| FS-PTY-006 | A terminated pane MUST offer the user two actions: close the pane, or restart the shell. | Must |
| FS-PTY-007 | Closing a tab or pane MUST close the master fd, sending SIGHUP to the child process group. | Must |
| FS-PTY-008 | If a foreground process is running when the user attempts to close a tab, a pane, or the application window, a confirmation dialog MUST be displayed. When closing the window, the dialog MUST indicate how many tabs/panes have active processes. | Must |
| FS-PTY-009 | Pane resize MUST trigger `ioctl(TIOCSWINSZ)` and deliver SIGWINCH to the foreground process group. The resize MUST include pixel dimensions (xpixel, ypixel). | Must |
| FS-PTY-010 | Resize events SHOULD be debounced (16–33ms). The final size MUST always be sent. | Should |
| FS-PTY-011 | The following environment variables MUST be set in the child process: `TERM=xterm-256color`, `COLORTERM=truecolor`, `LANG` (UTF-8 locale — inherited or fallback), `LINES`, `COLUMNS`, `SHELL`, `HOME`, `USER`, `LOGNAME`, `PATH`, `TERM_PROGRAM=TauTerm`, `TERM_PROGRAM_VERSION=<version>`. | Must |
| FS-PTY-012 | The environment variables `DISPLAY`, `WAYLAND_DISPLAY`, and `DBUS_SESSION_BUS_ADDRESS` MUST be inherited from the parent environment when present. | Must |
| FS-PTY-013 | The initial tab MUST launch a login shell. Subsequent tabs and panes MUST launch interactive non-login shells. | Must |
| FS-PTY-014 | If `$SHELL` is invalid or unset, TauTerm MUST fall back to `/bin/sh`. | Must |

**Acceptance criteria:**
- FS-PTY-001: Two panes run independent shell sessions; input in one does not affect the other.
- FS-PTY-005: Running `exit` in a shell displays the exit status (0) in the pane; the pane remains visible.
- FS-PTY-006: A terminated pane shows "Close" and "Restart" actions.
- FS-PTY-008: Running `sleep 3600` then pressing the close-tab shortcut shows a confirmation dialog.
- FS-PTY-009: Resizing a pane while vim is open causes vim to redraw at the new size.
- FS-PTY-011: `echo $TERM_PROGRAM` outputs `TauTerm`.
- FS-PTY-013: The initial tab sources `~/.bash_profile` (login shell); a second tab does not.
- FS-PTY-014: Setting `SHELL=/nonexistent` before launch results in a `/bin/sh` session.

---

### 3.3 FS-TAB: Multi-Tab Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-TAB-001 | The user MUST be able to open multiple terminal tabs within a single window. | Must |
| FS-TAB-002 | Each tab MUST host at least one independent PTY session (the root pane). | Must |
| FS-TAB-003 | The user MUST be able to create a new tab via a keyboard shortcut (default: Ctrl+Shift+T) and via a visible UI control (e.g., a "+" button). | Must |
| FS-TAB-004 | The user MUST be able to close the active tab via a keyboard shortcut (default: Ctrl+Shift+W) and via a visible UI control on each tab. | Must |
| FS-TAB-005 | Tabs MUST be reorderable by the user (e.g., drag-and-drop). | Must |
| FS-TAB-006 | Each tab MUST display a title. The title MUST be settable by the running process (via OSC sequences) and overridable by the user with a custom label. The user MUST be able to rename a tab via: (a) double-clicking the tab title, which makes it editable inline; (b) a "Rename" action in the tab's context menu (right-click). Pressing Enter or clicking outside the field confirms the new label; pressing Escape cancels. A user-defined label takes precedence over OSC-set titles for that tab. Clearing the label reverts to process-name or OSC-driven titles. | Must |
| FS-TAB-007 | When a non-active tab produces output or a process terminates within it, the tab MUST display a visual activity notification without switching focus. | Must |
| FS-TAB-008 | Closing the last tab MUST close the application window. | Must |

**Acceptance criteria:**
- FS-TAB-001: Opening 10 tabs results in 10 independent terminal sessions.
- FS-TAB-003: Both Ctrl+Shift+T and the UI button create a new tab.
- FS-TAB-004: Closing a tab with running processes triggers the confirmation dialog (FS-PTY-008).
- FS-TAB-005: Dragging a tab to a new position reorders it.
- FS-TAB-006: Running `printf "\033]0;Custom\007"` changes the tab title. Double-clicking the tab title makes it editable inline; typing a new name and pressing Enter sets the custom label. Right-clicking the tab shows a "Rename" option that achieves the same result. Clearing the label reverts to the process-driven title.
- FS-TAB-007: Output produced in a background tab causes a visible indicator on that tab's header.

---

### 3.4 FS-PANE: Multi-Pane (Split View)

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-PANE-001 | Within a tab, the user MUST be able to split the view horizontally (top/bottom) and vertically (left/right). | Must |
| FS-PANE-002 | Each pane MUST host an independent PTY session. | Must |
| FS-PANE-003 | Panes MUST be resizable by the user (e.g., dragging the separator). | Must |
| FS-PANE-004 | The user MUST be able to close any pane. Closing the last pane in a tab MUST close the tab. | Must |
| FS-PANE-005 | The user MUST be able to navigate between panes via keyboard shortcuts (defaults defined in FS-KBD) and via mouse click. | Must |
| FS-PANE-006 | The active pane MUST be visually distinguishable from inactive panes. | Must |

**Acceptance criteria:**
- FS-PANE-001: A horizontal split produces two panes stacked vertically, each with its own shell session.
- FS-PANE-003: Dragging the separator resizes both adjacent panes and triggers SIGWINCH in each.
- FS-PANE-004: Closing the only pane in a tab closes the tab.
- FS-PANE-005: A keyboard shortcut cycles focus between panes; clicking a pane gives it focus.
- FS-PANE-006: The focused pane has a visually distinct border or highlight.

---

### 3.5 FS-KBD: Keyboard Input Handling

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-KBD-001 | Application shortcuts MUST be intercepted before any input reaches the PTY. A matched shortcut MUST be consumed by TauTerm and not transmitted to the PTY. An unmatched key combination MUST be encoded and written to the PTY master. | Must |
| FS-KBD-002 | All application shortcuts MUST be user-configurable in the preferences UI. Removing a shortcut MUST make that key combination available to the PTY. | Must |
| FS-KBD-003 | Default application shortcuts MUST use Ctrl+Shift prefix to avoid conflict with standard terminal Ctrl+key sequences. The following default shortcuts MUST be provided (all user-configurable per FS-KBD-002): New tab (Ctrl+Shift+T), Close tab (Ctrl+Shift+W), Paste (Ctrl+Shift+V), Search (Ctrl+Shift+F), Open preferences (Ctrl+,), Next tab (Ctrl+Tab), Previous tab (Ctrl+Shift+Tab), Rename active tab (F2). Application shortcuts MUST also exist for pane management; defaults are defined in UXD.md §11.2: split horizontal left/right (Ctrl+Shift+D), split vertical top/bottom (Ctrl+Shift+E), navigate to next/previous pane (Ctrl+Shift+Right / Ctrl+Shift+Left / Ctrl+Shift+Up / Ctrl+Shift+Down), close active pane (Ctrl+Shift+Q). | Must |
| FS-KBD-004 | Ctrl+letter MUST encode as the corresponding C0 control character (Ctrl+A = 0x01 through Ctrl+Z = 0x1A, Ctrl+[ = 0x1B, Ctrl+\ = 0x1C, Ctrl+] = 0x1D, Ctrl+^ = 0x1E, Ctrl+_ = 0x1F). | Must |
| FS-KBD-005 | Alt+key MUST encode as ESC prefix followed by the key (e.g., Alt+A = 0x1B 0x61). 8-bit encoding MUST NOT be used (it breaks UTF-8). | Must |
| FS-KBD-006 | Function keys F1–F12 MUST emit the standard xterm sequences (F1 = ESC OP through F12 = ESC [24~). | Must |
| FS-KBD-007 | Arrow keys MUST be mode-dependent: normal mode = ESC [A/B/C/D; application cursor mode (DECCKM set) = ESC OA/B/C/D. | Must |
| FS-KBD-008 | Home, End, Insert, Delete, Page Up, Page Down MUST emit standard xterm sequences. | Must |
| FS-KBD-009 | Modified keys MUST use CSI 1;Mod X encoding (Mod: 2=Shift, 3=Alt, 4=Shift+Alt, 5=Ctrl, 6=Ctrl+Shift, 7=Ctrl+Alt, 8=Ctrl+Shift+Alt). | Must |
| FS-KBD-010 | Keypad application mode (DECKPAM, ESC =) and numeric mode (DECKPNM, ESC >) MUST be supported. | Must |
| FS-KBD-011 | IME input MUST be supported. The composition window MUST appear at the current cursor position. | Must |
| FS-KBD-012 | Compose key and dead key sequences MUST be handled by the platform input layer. The final composed character MUST be sent to the PTY. | Must |
| FS-KBD-013 | Shift+Enter is indistinguishable from Enter in standard xterm encoding (both = 0x0D). This is a known v1 limitation — see Domain Constraints (section 5). | N/A (constraint) |

**Acceptance criteria:**
- FS-KBD-001: Pressing Ctrl+Shift+T opens a new tab; Ctrl+C sends 0x03 to the PTY.
- FS-KBD-002: Removing the Ctrl+Shift+T binding in preferences causes that key combination to be sent to the PTY.
- FS-KBD-003 (F2): Pressing F2 while a tab is active activates inline rename mode on that tab's title.
- FS-KBD-003 (pane shortcuts): Ctrl+Shift+D splits the active pane horizontally (left/right); Ctrl+Shift+E splits vertically (top/bottom); Ctrl+Shift+Right/Left/Up/Down navigates between panes; Ctrl+Shift+Q closes the active pane. All defaults match UXD.md §11.2.
- FS-KBD-005: Alt+A in bash triggers the expected readline shortcut (e.g., `Meta-a`).
- FS-KBD-007: In vim, arrow keys navigate; after `set t_ku=\eOA` (application mode), arrows still work.
- FS-KBD-011: Typing Chinese characters via an IME produces correct input in the terminal; the composition window tracks the cursor.

---

### 3.6 FS-CLIP: Clipboard Integration

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-CLIP-001 | Text selection MUST operate on character cell boundaries, not pixel boundaries. | Must |
| FS-CLIP-002 | Click-and-drag MUST select a range of cells. Double-click MUST select a word. Triple-click MUST select a line. | Must |
| FS-CLIP-003 | The word delimiter set MUST be configurable in user preferences. The default delimiter set MUST be: space, `"`, `'`, `` ` ``, `(`, `)`, `[`, `]`, `{`, `}`, `\|`, `&`, `;`, `<`, `>`, `!`. The characters `/`, `.`, `-`, `_`, `:`, `@`, `=` MUST NOT be delimiters by default, so that paths (`./my_app`), URLs (`user@host:port`), and hyphenated names (`mon-container`) are selectable as single words with a double-click. | Must |
| FS-CLIP-004 | On Linux/X11, selecting text MUST copy to the PRIMARY selection (enabling middle-click paste). | Must |
| FS-CLIP-005 | Ctrl+Shift+V MUST paste from the CLIPBOARD selection (not PRIMARY). | Must |
| FS-CLIP-006 | An explicit "Copy" action (e.g., via context menu) MUST copy to the CLIPBOARD selection. | Must |
| FS-CLIP-007 | On Wayland, PRIMARY selection MUST use `wp_primary_selection_v1`. TauTerm MUST fall back gracefully if the protocol is unsupported. | Must |
| FS-CLIP-008 | Bracketed paste mode (DECSET 2004) MUST be supported: pasted text MUST be wrapped with ESC [200~ and ESC [201~. Before wrapping, any occurrences of the bracketed paste end sequence (ESC [201~) within the pasted text MUST be stripped to prevent premature termination of the bracketed paste. Pasted text MUST NOT be interpreted as escape sequences. | Must |
| FS-CLIP-009 | When bracketed paste mode is NOT active, text MUST be pasted directly (legacy behavior). If the pasted text contains newlines, a confirmation dialog SHOULD be displayed. The dialog MUST be configurable (can be disabled). | Must / Should |

**Acceptance criteria:**
- FS-CLIP-001: Selecting text never selects partial characters; wide characters are selected as a unit.
- FS-CLIP-002: Double-clicking on `/home/user/project` selects the entire path.
- FS-CLIP-004: Selecting text in the terminal, then middle-clicking in another application pastes the selected text.
- FS-CLIP-005: Copying text via a browser (CLIPBOARD), then pressing Ctrl+Shift+V in TauTerm pastes it.
- FS-CLIP-008: In zsh with bracketed paste enabled, pasting multi-line text does not auto-execute.
- FS-CLIP-009: Pasting a multi-line command when bracketed paste is off shows a confirmation dialog.

---

### 3.7 FS-SB: Scrollback Buffer

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SB-001 | Each pane MUST have its own scrollback buffer. | Must |
| FS-SB-002 | The scrollback buffer size MUST be configurable by the user (number of lines). The default MUST be 10,000 lines. There is no artificially imposed maximum: the user may set any value, constrained only by available system memory. The preferences UI MUST display an estimated memory consumption for the configured value (e.g., "~200 MB per pane at 100,000 lines"), updated as the user adjusts the setting. | Must |
| FS-SB-003 | The scrollback buffer MUST store text content AND cell attributes (colors, bold, italic, underline, etc.). | Must |
| FS-SB-004 | Only lines scrolled off the top of a full-screen scroll region MUST enter the scrollback buffer. Lines scrolled out of a partial scroll region (DECSTBM-restricted) MUST NOT populate the scrollback. | Must |
| FS-SB-005 | When the alternate screen buffer is active, the scrollback buffer MUST be frozen: no new lines added. Scrollback navigation SHOULD be disabled. | Must / Should |
| FS-SB-006 | When returning to the normal screen buffer, the scrollback MUST be navigable with all previously accumulated content intact. | Must |
| FS-SB-007 | The user MUST be able to scroll the scrollback using the mouse wheel, keyboard shortcuts, and a visible scrollbar. | Must |
| FS-SB-008 | The scrollback buffer MUST distinguish between hard newlines (actual line breaks in the output) and soft wraps (line breaks introduced by terminal width). | Must |
| FS-SB-009 | When `scroll_offset > 0` and the PTY process produces new output, the viewport position MUST be maintained. The system MUST NOT auto-scroll to the bottom. | Must |
| FS-SB-010 | When the user sends keyboard input to the PTY while `scroll_offset > 0`, the backend MUST reset `scroll_offset` to 0 and emit a `scroll-position-changed` event. The frontend MUST scroll the viewport to the bottom upon receiving this event. | Must |

**Acceptance criteria:**
- FS-SB-001: Two panes have independent scrollback histories.
- FS-SB-002: The scrollback size preference field accepts any positive integer value. The preferences UI displays a real-time memory estimate that updates as the user types. Setting the value to 100,000 and running `seq 1 100001` causes the first line to be evicted.
- FS-SB-003: Colored output in scrollback retains its colors when scrolled into view.
- FS-SB-004: Running tmux (which uses a partial scroll region for the status bar), scrolling in the tmux pane does not add the tmux status bar to TauTerm's scrollback.
- FS-SB-005: With vim open, scrolling in TauTerm does not navigate the scrollback.
- FS-SB-006: After exiting vim, all pre-vim scrollback content is accessible.
- FS-SB-009: While scrolled 50 lines into the scrollback, a command producing output does not move the viewport. The user's reading position is preserved.
- FS-SB-010: While scrolled 50 lines into the scrollback, pressing any key that sends input to the PTY causes the viewport to jump to the bottom instantly. No additional scroll action is required.

---

### 3.8 FS-SEARCH: Search in Output

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SEARCH-001 | The user MUST be able to search for text within the scrollback buffer. The search MUST operate on plain text content (escape sequences stripped). | Must |
| FS-SEARCH-002 | Search MUST work across soft-wrapped lines (a word split by soft wrap MUST be found). | Must |
| FS-SEARCH-003 | Case-insensitive literal string matching MUST be supported. A case-sensitive toggle SHOULD be available. Regex search SHOULD be supported. | Must / Should |
| FS-SEARCH-004 | Search MUST NOT operate on alternate screen content. | Must |
| FS-SEARCH-005 | On a scrollback buffer of 100,000 lines, the first search result MUST appear in under 100ms on a mid-range system. | Must |
| FS-SEARCH-006 | All matches MUST be visually highlighted. The current match MUST be visually distinct from other matches. Next/previous navigation MUST scroll to center the current match in the viewport. | Must |
| FS-SEARCH-007 | The user MUST be able to initiate search via a keyboard shortcut (default: Ctrl+Shift+F) and via a visible UI control or context menu entry. | Must |

**Acceptance criteria:**
- FS-SEARCH-001: Searching for "error" finds occurrences in the scrollback regardless of their SGR attributes.
- FS-SEARCH-002: A word that wraps across two visual lines is found by a search for that word.
- FS-SEARCH-003: Searching for "error" matches "Error", "ERROR", and "error" in case-insensitive mode. If regex search is implemented, searching for `err(or|eur)` matches both "error" and "erreur".
- FS-SEARCH-004: With vim open, search does not find text displayed by vim.
- FS-SEARCH-005: After `seq 1 100000`, searching for "99999" returns results in under 100ms.
- FS-SEARCH-006: Matches are highlighted; pressing next/previous moves between them, centering each in view.

---

### 3.9 FS-NOTIF: Activity Notifications

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-NOTIF-001 | When a non-active tab or pane produces output, its tab header MUST display a visual activity indicator. | Must |
| FS-NOTIF-002 | When a process terminates in a non-active tab or pane, its tab header MUST display a distinct visual indicator (different from the output activity indicator). | Must |
| FS-NOTIF-003 | The activity indicator MUST be cleared when the user switches to that tab or pane. | Must |
| FS-NOTIF-004 | Bell events (FS-VT-093) in non-active tabs or panes MUST produce a visual indicator on the corresponding tab or pane. | Must |
| FS-NOTIF-005 | When the tab bar is in scrolled state (tabs overflow the available width), scroll navigation arrows MUST display a dot badge if one or more scrolled-out tabs have pending notifications (output activity or bell). Bell takes visual priority over output activity. Navigating to the scrolled-out tabs clears the badge. | Should |

**Acceptance criteria:**
- FS-NOTIF-001: Running a long command in a background tab causes the tab to show an activity indicator.
- FS-NOTIF-002: A shell exiting in a background tab shows a distinct "process ended" indicator.
- FS-NOTIF-003: Switching to a tab with an activity indicator clears the indicator.
- FS-NOTIF-005: When a tab with unread activity is outside the visible tab bar viewport, the scroll arrow pointing toward it displays a dot badge. If multiple notification types exist, bell badge takes priority. The badge clears when the user scrolls to reveal the tab.

---

### 3.10 FS-SSH: SSH Session Management

#### 3.10.1 Session Integration

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-001 | The user MUST be able to open an SSH session in a new tab or a new pane. The SSH session MUST be visually integrated within TauTerm's tab/pane model. | Must |
| FS-SSH-002 | The user MUST be able to distinguish at a glance whether a tab or pane hosts a local or remote (SSH) session. | Must |
| FS-SSH-003 | All terminal emulation requirements (FS-VT-*) apply equally to SSH sessions as to local PTY sessions. | Must |

**Acceptance criteria:**
- FS-SSH-001: The user can open an SSH connection from a UI control, and it appears as a regular tab/pane.
- FS-SSH-002: An SSH tab/pane displays a visual indicator (e.g., icon, badge, or label) distinguishing it from local sessions.

#### 3.10.2 Connection Lifecycle

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-010 | SSH sessions MUST have distinct lifecycle states with visual representation: Connecting, Authenticating, Connected, Disconnected, Closed. State definitions: **Connecting** — TCP connection in progress. **Authenticating** — TCP established, SSH handshake and credential exchange in progress. **Connected** — session fully established and interactive. **Disconnected** — the session was interrupted unexpectedly (network drop, keepalive timeout, or remote process exit with non-zero code); reconnection is possible. **Closed** — the user has explicitly closed the pane or tab hosting the session, or the remote process exited normally (exit code 0 with no unexpected disconnect); the session is no longer active and no reconnection is possible — a new session must be opened to reconnect. | Must |
| FS-SSH-011 | Host key verification MUST follow the TOFU model. **First connection:** the prompt MUST display (a) a human-readable explanation in plain language (e.g., "TauTerm is connecting to `<host>` for the first time. To confirm you are connecting to the right server, verify the fingerprint below with your server administrator. If you are unsure, click Reject."), (b) the host key fingerprint in SHA-256 format, and (c) the key type (e.g., ED25519, RSA). **Key change:** the connection MUST be blocked immediately. A prominent warning dialog MUST be shown displaying: the stored fingerprint, the new fingerprint, a clear warning that a key change may indicate a man-in-the-middle attack, and an explanation of what to do (e.g., "Contact your server administrator to verify this change before accepting."). The default action MUST be rejection. Acceptance MUST require a deliberate non-default action. Accepted keys MUST be stored in TauTerm's own known-hosts file (`~/.config/tauterm/known_hosts`), in OpenSSH-compatible format. TauTerm MUST NOT read from or write to `~/.ssh/known_hosts`. The preferences UI MUST offer an import action to copy entries from `~/.ssh/known_hosts` into TauTerm's known-hosts file. | Must |
| FS-SSH-012 | Authentication MUST be attempted in this order: publickey, keyboard-interactive, password. A saved connection MAY specify a preferred method. | Must |
| FS-SSH-013 | The SSH PTY request MUST include: `TERM=xterm-256color`, terminal dimensions (cols, rows, xpixel, ypixel), and standard terminal modes encoded per RFC 4254 §6.2 and Annex A. The `encoded terminal modes` field MUST contain the following opcode/value pairs (TTY_OP_END = 0 terminates the list): VINTR (opcode 1, value 3 = ^C), VQUIT (opcode 2, value 28 = ^\), VERASE (opcode 3, value 127 = DEL), VEOF (opcode 4, value 4 = ^D), VKILL (opcode 5, value 21 = ^U), VSUSP (opcode 10, value 26 = ^Z), ISIG (opcode 50, value 1), ICANON (opcode 51, value 1), ECHO (opcode 53, value 1). Note: these opcodes are the RFC 4254 Annex A numbering — they are NOT the `termios` struct field indices from the Linux kernel header. | Must |
| FS-SSH-014 | If the negotiated host key algorithm is deprecated (specifically: `ssh-rsa` with SHA-1, or `ssh-dss`), TauTerm MUST display a non-blocking warning in the pane after connection is established. The warning MUST name the deprecated algorithm and state that the server should be updated. The connection MUST NOT be refused. The warning MUST be dismissible by the user. | Must |

**Acceptance criteria:**
- FS-SSH-010: Each lifecycle state is reflected in the pane UI (e.g., status bar, overlay, or icon change). When the user closes an SSH pane or the remote shell exits cleanly (exit code 0), the pane or tab enters the Closed state: no reconnection control is shown and no error indicator is shown. When the connection drops unexpectedly (network interruption, keepalive timeout, or non-zero exit), the pane enters the Disconnected state and displays a reconnection control.
- FS-SSH-011: Connecting to a new host shows a plain-language prompt with the SHA-256 fingerprint and key type. Connecting to a host whose key has changed: connection is blocked, both fingerprints are shown side by side, a MITM warning and actionable instructions are displayed, default action is Reject, acceptance requires a non-default deliberate action.
- FS-SSH-012: A connection using a key file authenticates without prompting for a password.
- FS-SSH-014: Connecting to a server that only offers `ssh-rsa` (SHA-1) shows a visible, dismissible warning in the pane naming the algorithm. The connection is established and functional.

#### 3.10.3 Connection Health

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-020 | SSH keepalive MUST be enabled by default, with an interval of 30 seconds. Three consecutive missed keepalives MUST trigger a transition to the Disconnected state. Keepalive interval and threshold MUST be configurable per connection. | Must |
| FS-SSH-021 | Pane resize MUST trigger an SSH `window-change` channel request with the new dimensions (debounced, same as local PTY). | Must |
| FS-SSH-022 | Connection drop MUST be detected via TCP keepalive, SSH keepalive, or write failure. The Disconnected state MUST be entered within 1 second of detection, with the reason displayed. | Must |

**Acceptance criteria:**
- FS-SSH-020: Blocking the network for 90 seconds triggers the Disconnected state.
- FS-SSH-021: Resizing a pane with an SSH session causes the remote terminal to redraw.
- FS-SSH-022: Disconnection shows a reason (e.g., "Connection timed out").

#### 3.10.4 Saved Connections

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-030 | The user MUST be able to save SSH connections with at minimum: host, port, username, authentication method (identity file path or password reference), and optional label/group. | Must |
| FS-SSH-031 | Saved connections MUST be listed in a dedicated UI (e.g., connection manager panel or quick-open dialog). | Must |
| FS-SSH-032 | From the saved connections list, the user MUST be able to open a connection in a new tab or pane with a single action. | Must |
| FS-SSH-033 | The user MUST be able to create, edit, duplicate, and delete saved connections. | Must |
| FS-SSH-034 | Saved connections MUST be stored persistently as part of user preferences. | Must |

**Acceptance criteria:**
- FS-SSH-030: A saved connection stores host, port, username, and auth method.
- FS-SSH-031: A connection manager UI lists all saved connections.
- FS-SSH-032: Clicking a saved connection opens an SSH session in a new tab.
- FS-SSH-033: All CRUD operations and duplication work from the UI.

#### 3.10.5 Reconnection

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SSH-040 | When an SSH session is interrupted, the user MUST be able to reconnect to the same connection without reconfiguring. | Must |
| FS-SSH-041 | The reconnection action MUST be accessible directly from the affected tab or pane. | Must |
| FS-SSH-042 | On reconnection, scrollback MUST be preserved. A visual separator MUST be displayed at the reconnection boundary. | Must |

**Acceptance criteria:**
- FS-SSH-040: After disconnection, clicking "Reconnect" re-establishes the SSH session.
- FS-SSH-041: The reconnection button/action is visible in the disconnected pane.
- FS-SSH-042: After reconnection, previous scrollback is intact, with a clear separator line.

---

### 3.11 FS-CRED: Credential Security

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-CRED-001 | Credentials (passwords, passphrases) MUST be stored using the OS keychain via the Secret Service D-Bus API (e.g., `libsecret` / `keyring` crate). They MUST NOT be stored in plain text, in environment variables, or in the preferences file. | Must |
| FS-CRED-002 | Identity files (private keys) MUST be referenced by file path. TauTerm MUST NOT copy, embed, or read private key file content beyond what is needed for authentication. | Must |
| FS-CRED-003 | Credentials retrieved from the keychain for authentication MUST be cleared from process memory as soon as authentication completes or fails. Credentials MUST NOT be cached in application state beyond the duration of the authentication handshake. | Must |
| FS-CRED-004 | Credentials (passwords, passphrases, key material) MUST NOT appear in log output, error messages, IPC payloads, or debug traces, at any log level. | Must |
| FS-CRED-005 | If the OS keychain is unavailable (no Secret Service provider running), TauTerm MUST NOT fall back to insecure storage. Instead, it MUST prompt the user for credentials on each connection attempt and inform the user that credential persistence is unavailable. | Must |
| FS-CRED-006 | Identity file paths stored in saved connections MUST be validated at connection time: the path MUST be resolved to an absolute path, MUST NOT contain path traversal components (e.g., `../`), and MUST point to a regular file. Symlinks MAY be followed. | Must |

**Acceptance criteria:**
- FS-CRED-001: Inspecting the preferences file on disk reveals no plaintext passwords. Credentials are retrievable via `secret-tool lookup`.
- FS-CRED-002: The saved connection configuration contains a path string, not key content.
- FS-CRED-003: After an SSH connection is established, a memory dump of the TauTerm process does not contain the password used for authentication.
- FS-CRED-004: Enabling maximum log verbosity and connecting with a password does not log the password.
- FS-CRED-005: With no keychain available, TauTerm prompts for the password each time and displays a notice about unavailable credential persistence.
- FS-CRED-006: A saved connection with identity path `../../etc/shadow` is rejected at connection time with a clear error.

---

### 3.12 FS-THEME: Theming System

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-THEME-001 | TauTerm MUST ship with a single, carefully designed default theme that reflects a deliberate artistic direction. | Must |
| FS-THEME-002 | The default theme MUST NOT be deletable. It MAY be overridden by a user-created theme set as active. | Must |
| FS-THEME-003 | The user MUST be able to create one or more custom themes. | Must |
| FS-THEME-004 | A theme MUST define at minimum: background color, foreground color, cursor color, selection color, and the 16 ANSI palette colors. | Must |
| FS-THEME-005 | A theme MAY also define: font family, font size, line height, border/panel colors, and UI accent colors. | May |
| FS-THEME-006 | The active theme MUST be switchable at any time from the preferences UI. | Must |
| FS-THEME-007 | Themes MUST be stored persistently alongside other user preferences. | Must |
| FS-THEME-008 | The theming system MUST be based on design tokens (colors, spacing, sizing, radius). No hardcoded visual values are allowed in the UI layer. | Must |
| FS-THEME-009 | User-created themes MUST map to the same design tokens as the default theme, ensuring visual consistency across all UI surfaces. | Must |
| FS-THEME-010 | A user theme MAY override the terminal line height. The configurable token is `--line-height-terminal` (default: 1.2). UI chrome line height (tab bar, status bar, panels) is not themeable and is fixed by the design system. | Should |

**Acceptance criteria:**
- FS-THEME-001: On first launch, TauTerm displays a polished default theme.
- FS-THEME-002: The default theme has no "Delete" option in the UI.
- FS-THEME-003: The user can create a theme, name it, and switch to it.
- FS-THEME-004: A custom theme that changes only background and foreground colors applies correctly; ANSI palette is visible in `ls --color` output.
- FS-THEME-006: Switching themes in preferences applies the new theme immediately.
- FS-THEME-008: No UI component uses hardcoded color or spacing values; all reference tokens.
- FS-THEME-010: A user theme that sets `--line-height-terminal: 1.5` causes the terminal to render lines with 1.5× line height. UI elements (tab bar, status bar, panels) are unaffected.

---

### 3.13 FS-PREF: User Preferences

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-PREF-001 | User preferences MUST be persisted locally on disk and survive application restarts. | Must |
| FS-PREF-002 | A dedicated UI panel MUST allow the user to view and edit all preferences. | Must |
| FS-PREF-003 | Changes to preferences MUST be applied immediately without requiring a restart, where technically feasible. When a preference change requires a restart to take effect, the UI MUST inform the user. | Must |
| FS-PREF-004 | Preferences MUST be organized into logical sections. At minimum: Keyboard, Appearance, Terminal Behavior. | Must |
| FS-PREF-005 | The preferences UI MUST be accessible via a keyboard shortcut (default: Ctrl+,) and via a visible UI control. | Must |
| FS-PREF-006 | The following settings MUST be configurable: scrollback buffer size (with real-time memory estimate display per FS-SB-002), cursor blink rate, cursor shape, bell notification type, word delimiter set, font family, font size. | Must |

**Acceptance criteria:**
- FS-PREF-001: Changing a preference, quitting, and relaunching TauTerm shows the preference retained.
- FS-PREF-002: All configurable settings are accessible from the preferences UI.
- FS-PREF-003: Changing font size in preferences immediately changes the terminal font size.
- FS-PREF-004: The preferences UI has labeled sections for Keyboard, Appearance, and Terminal Behavior.
- FS-PREF-005: Ctrl+, opens the preferences panel.

---

### 3.14 FS-A11Y: Accessibility

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-A11Y-001 | Color contrast MUST meet WCAG 2.1 AA standards: at least 4.5:1 for normal text, at least 3:1 for large text and UI components. | Must |
| FS-A11Y-002 | All interactive UI elements (buttons, tabs, inputs) MUST have a minimum touch/click target of 44x44 pixels. | Must |
| FS-A11Y-003 | All interactive UI elements MUST be navigable and operable via keyboard. | Must |
| FS-A11Y-004 | Information MUST NOT be conveyed by color alone. A secondary indicator (shape, icon, text, pattern) MUST supplement color-based distinctions. | Must |
| FS-A11Y-005 | All TauTerm UI features MUST be accessible via both keyboard and mouse, per the dual modality principle (UR 3.1). The terminal content area is excepted per UR 3.3. | Must |
| FS-A11Y-006 | A context menu MUST be available in the terminal area (e.g., right-click). It MUST expose at minimum: Copy, Paste, Search, and pane/tab management actions. This is the primary discoverability mechanism for users who do not know keyboard shortcuts. | Must |
| FS-A11Y-007 | During theme editing, the editor's own chrome (labels, controls, buttons, inputs) MUST always render using the current active Umbra system tokens, not the custom theme being edited. Only a designated preview area (terminal viewport sample) reflects the custom theme in real time. This ensures the editor remains accessible even when the user is authoring a non-compliant theme. | Must |

**Acceptance criteria:**
- FS-A11Y-001: The default theme passes WCAG AA contrast checks for all text and UI components.
- FS-A11Y-002: No interactive element has a click target smaller than 44x44px.
- FS-A11Y-003: Tab key cycles through all interactive elements; Enter/Space activates them.
- FS-A11Y-004: Tab activity indicators use an icon or text badge in addition to color change.
- FS-A11Y-005: Every feature reachable by mouse is also reachable by keyboard (and vice versa, excluding PTY input).
- FS-A11Y-006: Right-clicking in the terminal area opens a context menu with Copy, Paste, Search, and split/close actions.
- FS-A11Y-007: A user creating a theme with foreground color identical to background color: the editor controls and labels remain fully legible. Only the preview terminal sample reflects the low-contrast custom theme.

---

### 3.15 FS-UX: User Experience Cross-Cutting Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-UX-001 | Every user-visible error or warning message MUST (a) identify what happened in plain language, without raw system error codes as the primary text; (b) indicate what the user can do next, or explicitly state that no action is required; (c) display any technical detail (errno, exit code, system message) as a secondary, collapsible or visually subordinate element only. | Must |
| FS-UX-002 | On first launch, TauTerm MUST display a non-intrusive, non-blocking indication that right-clicking in the terminal area opens a context menu. This indication MUST disappear automatically after the user has performed a right-click in the terminal area at least once. It MUST NOT block or delay the user from using the terminal. | Must |

**Acceptance criteria:**
- FS-UX-001: When the configured shell is not found, the message reads in terms the user can understand (e.g., "Shell not found: `/usr/local/bin/zsh`. TauTerm has fallen back to `/bin/sh`. You can update your shell in Preferences → Terminal Behavior."), not a raw system error string.
- FS-UX-001: When an SSH connection drops, the pane shows a human-readable reason (e.g., "Connection lost: server did not respond to keepalive. Click Reconnect to try again."), not a raw errno or SSH error code alone.
- FS-UX-002: On a fresh install, a hint referencing the right-click context menu is visible in the terminal area. After the user right-clicks once, the hint is gone and does not reappear on subsequent launches.

---

### 3.16 FS-SEC: Security Hardening

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SEC-001 | The Tauri Content Security Policy MUST be configured to restrict resource loading. At minimum: `default-src 'self'`; `script-src 'self'`; `style-src 'self' 'unsafe-inline'` (to be tightened when feasible); `connect-src ipc: http://ipc.localhost`; `img-src 'self' asset: http://asset.localhost`. The CSP MUST NOT allow `script-src 'unsafe-inline'` or `script-src 'unsafe-eval'`. | Must |
| FS-SEC-002 | All PTY master file descriptors MUST be opened with the `O_CLOEXEC` flag (or equivalent `CLOEXEC` at creation time) to prevent file descriptor leaks to child processes spawned by the shell. | Must |
| FS-SEC-003 | The preferences file MUST be validated against a schema on load. Invalid entries MUST be replaced with defaults. File paths within preferences (identity file paths, shell path) MUST be validated: resolved to absolute paths, no path traversal, must reference existing regular files. | Must |
| FS-SEC-004 | SSH agent forwarding MUST NOT be supported in v1. | Must |
| FS-SEC-005 | Individual OSC and DCS sequences MUST be limited to 4096 bytes. Sequences exceeding this limit MUST be discarded. This prevents memory exhaustion from malicious or malformed sequences. | Must |

**Acceptance criteria:**
- FS-SEC-001: The WebView does not execute inline scripts. A `<script>` tag injected into the DOM via devtools is blocked by CSP.
- FS-SEC-002: A child process (e.g., `ls -la /proc/self/fd`) does not show open file descriptors belonging to other panes' PTY masters.
- FS-SEC-003: A preferences file with an invalid JSON structure or out-of-range values loads with defaults applied; no crash occurs.
- FS-SEC-004: No SSH agent forwarding channel is opened during an SSH session.
- FS-SEC-005: A sequence of `\033]0;` followed by 10,000 characters does not consume unbounded memory; the sequence is discarded.

---

### 3.17 FS-I18N: Internationalisation

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-I18N-001 | All TauTerm UI strings MUST be internationalised. No UI string visible to the user MAY be hardcoded in source; every string MUST be looked up from a locale catalogue. | Must |
| FS-I18N-002 | TauTerm v1 MUST ship with two locales: English (`en`, default and fallback) and French (`fr`). | Must |
| FS-I18N-003 | The active UI language MUST be selectable by the user in the Preferences UI. | Must |
| FS-I18N-004 | A language change in Preferences MUST be applied immediately to all UI elements without requiring a restart. | Must |
| FS-I18N-005 | The selected language MUST be persisted and restored on next launch. | Must |
| FS-I18N-006 | If the persisted language preference contains an unknown locale code (e.g., a value not in the supported set), TauTerm MUST silently fall back to English. | Must |
| FS-I18N-007 | TauTerm MUST NOT modify PTY session environment variables (`LANG`, `LC_*`, `LANGUAGE`) based on the UI language selection. The shell locale is fully determined by the user's login environment. | Must |

**Acceptance criteria:**
- FS-I18N-001: No UI label, button text, error message, or tooltip is hardcoded in source; all are retrieved from the active locale catalogue.
- FS-I18N-002: Switching to French in Preferences renders all UI labels in French; switching to English renders them in English. No untranslated key (raw key string) is visible in either locale.
- FS-I18N-003: The Preferences UI includes a language selector listing at least English and French.
- FS-I18N-004: Changing the language in Preferences immediately updates all visible UI strings without any page reload or restart.
- FS-I18N-005: Setting the language to French, quitting, and relaunching shows the UI in French.
- FS-I18N-006: Setting `preferences.json` to contain an unknown locale code (e.g., `"language": "de"`) and launching TauTerm shows the UI in English with no crash or error dialog.
- FS-I18N-007: Inspecting `$LANG` and `$LC_ALL` inside a new PTY session after changing the UI language shows the original system values, not TauTerm's UI language.

---

### 3.18 FS-DIST: Distribution

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-DIST-001 | TauTerm v1 MUST be distributed as an AppImage. | Must |
| FS-DIST-002 | The AppImage MUST be self-contained: it MUST bundle all application dependencies (Rust runtime, frontend assets, shared libraries not guaranteed to be present on a target system). It MUST NOT require the user to install external packages beyond a standard Linux desktop environment (display server, `libwebkit2gtk-4.1` on Ubuntu 22.04+ or `libwebkit2gtk-4.0` on older distributions). | Must |
| FS-DIST-003 | The AppImage MUST run on the following architectures: x86 (i686), x86_64, ARM32 (armhf), ARM64 (aarch64), RISC-V (riscv64). A separate AppImage binary MAY be produced per architecture. | Must |
| FS-DIST-004 | The AppImage MUST be executable directly after download (no installation step required). A user who downloads and `chmod +x`es the AppImage MUST be able to run TauTerm immediately. | Must |
| FS-DIST-005 | The AppImage SHOULD integrate with the host desktop environment: it SHOULD provide a `.desktop` entry and application icon accessible via the AppImage integration daemon (`appimaged`) or equivalent. | Should |
| FS-DIST-006 | TauTerm release artefacts MUST be cryptographically signed. Each AppImage MUST be accompanied by a detached GPG signature (`.asc`) and a SHA-256 checksum file (`SHA256SUMS`). The `SHA256SUMS` file itself MUST also be GPG-signed. The public signing key MUST be published on a separate trusted channel (project website or a public keyserver). | Must |

**Acceptance criteria:**
- FS-DIST-001: The release artefact is an `.AppImage` file.
- FS-DIST-002: On a clean minimal Linux installation (e.g., Ubuntu Server with a desktop environment added but no TauTerm-specific dependencies), the AppImage runs without prompting for package installation.
- FS-DIST-003: The AppImage (or per-architecture variant) launches and passes a basic smoke test (`pnpm wdio`) on each of the five target architectures.
- FS-DIST-004: `chmod +x TauTerm-x86_64.AppImage && ./TauTerm-x86_64.AppImage` opens the application on a clean x86_64 system.
- FS-DIST-005: After running the AppImage with `appimaged` active, TauTerm appears in the desktop application launcher with its icon.
- FS-DIST-006: Each release includes a `.asc` detached GPG signature and a signed `SHA256SUMS` file alongside every AppImage artefact. Running `gpg --verify TauTerm-x86_64.AppImage.asc TauTerm-x86_64.AppImage` succeeds with the published public key. Running `sha256sum --check SHA256SUMS` passes for every listed artefact.

---

## 4. Out of Scope (v1)

The following are explicitly excluded from TauTerm v1:

| Item | Rationale |
|------|-----------|
| Plugin or extension system | Complexity; no persona requires it in v1. (UR 9) |
| Cloud sync of preferences, themes, or saved connections | Scope control; security implications. (UR 9) |
| Windows and macOS support | Linux-only first version. (UR 9) |
| Session persistence (restoring tabs/panes after restart) | Acknowledged need, deferred to future version. (UR 9) |
| Kitty keyboard protocol | Deferred to future version. v1 uses standard xterm key encoding. Shift+Enter is a known limitation. |
| URXVT mouse encoding (mode 1015) | SHOULD-level; may be deferred if schedule requires. |
| Serial port / local connection types | No persona requires it. |

---

## 5. Domain Constraints

The following constraints arise from the terminal/PTY domain and are non-negotiable. They affect multiple requirements and must be understood by all stakeholders.

| Constraint | Implication |
|------------|-------------|
| PTY is a byte stream with no framing | VT sequences can be split across read boundaries. The parser must handle partial sequences. (Affects FS-VT-005, FS-VT-010) |
| Applications control terminal mode | TauTerm must track all mode state set by applications (cursor mode, mouse reporting, screen buffer, etc.) and behave accordingly. (Affects FS-VT-030–086) |
| SIGWINCH is asynchronous | TauTerm cannot force an application to redraw after a resize. It can only signal the resize and wait. (Affects FS-PTY-009) |
| Alternate screen is opaque | Scrollback, search, and selection operate on the normal screen only while the alternate screen is active. (Affects FS-SB-005, FS-SEARCH-004) |
| SSH adds only transport layer | All VT behavior specifications apply equally to local and SSH sessions. (Affects FS-SSH-003) |
| Character width is a rendering problem | Disagreement between the terminal and the application on character width (e.g., wcwidth) causes cursor positioning errors. TauTerm must use a consistent and up-to-date width table. (Affects FS-VT-011, FS-VT-014) |
| Ctrl+key encoding is lossy | Ctrl+A and Ctrl+Shift+A both produce 0x01 in standard xterm encoding. TauTerm cannot distinguish them at the PTY level. v1 known limitation. (Affects FS-KBD-004, FS-KBD-013) |

---

## 6. Traceability Matrix

This matrix maps every functional specification to its originating user requirement in [UR.md](UR.md).

| FS Requirement | UR Source |
|----------------|-----------|
| **FS-VT: Terminal Emulation** | |
| FS-VT-001, FS-VT-002 | UR 1 (terminal emulator); Domain expert |
| FS-VT-003, FS-VT-004, FS-VT-005 | UR 1; Domain expert (xterm-256color compatibility) |
| FS-VT-010 – FS-VT-016 | UR 1; Domain expert (character set handling) |
| FS-VT-020 – FS-VT-025 | UR 8 §8.2 (ANSI palette in themes); Domain expert (color codes) |
| FS-VT-030 – FS-VT-034 | UR 1; Domain expert (cursor modes) |
| FS-VT-040 – FS-VT-044 | UR 1; Domain expert (screen modes) |
| FS-VT-050 – FS-VT-054 | UR 1; Domain expert (scrolling regions) |
| FS-VT-060 – FS-VT-062 | UR 4 §4.1 (tab titles); Domain expert (OSC sequences) |
| FS-VT-070 – FS-VT-073 | Domain expert (hyperlinks, security) |
| FS-VT-080 – FS-VT-086 | Domain expert (mouse reporting) |
| FS-VT-090 – FS-VT-093 | UR 4 §4.1 (activity notification); Domain expert (bell) |
| **FS-PTY: PTY Lifecycle** | |
| FS-PTY-001 – FS-PTY-004 | UR 4 §4.1, §4.2 (independent PTY per tab/pane); Domain expert |
| FS-PTY-005 – FS-PTY-006 | Domain expert (exit handling) |
| FS-PTY-007 – FS-PTY-008 | UR 4 §4.1 (close tabs); Domain expert (SIGHUP, confirmation) |
| FS-PTY-009 – FS-PTY-010 | UR 4 §4.2 (pane resize); Domain expert (SIGWINCH) |
| FS-PTY-011 – FS-PTY-014 | Domain expert (environment, shell fallback) |
| **FS-TAB: Multi-Tab** | |
| FS-TAB-001 – FS-TAB-007 | UR 4 §4.1 (multi-tab); UR 6 §6.1 (shortcuts) |
| **FS-PANE: Multi-Pane** | |
| FS-PANE-001 – FS-PANE-006 | UR 4 §4.2 (multi-screen/panes); UR 3.1 (dual modality) |
| **FS-KBD: Keyboard Input** | |
| FS-KBD-001 – FS-KBD-003 | UR 6 (keyboard shortcuts, distinction app vs PTY) |
| FS-KBD-004 – FS-KBD-012 | UR 6 §6.2 (PTY passthrough); Domain expert (key encoding) |
| FS-KBD-013 | Domain expert (known limitation) |
| **FS-CLIP: Clipboard** | |
| FS-CLIP-001 – FS-CLIP-009 | UR 6 §6.3 (clipboard); Domain expert (selection, bracketed paste) |
| **FS-SB: Scrollback** | |
| FS-SB-001 – FS-SB-008 | UR 7 §7.1 (scrollback); Domain expert (buffer behavior) |
| **FS-SEARCH: Search** | |
| FS-SEARCH-001 – FS-SEARCH-007 | UR 7 §7.2 (search in output); Domain expert (search constraints) |
| **FS-NOTIF: Notifications** | |
| FS-NOTIF-001 – FS-NOTIF-004 | UR 4 §4.1 (activity notification); UR 2.2 (Jordan — notifications) |
| FS-NOTIF-005 | UR 4 §4.1 (tab bar overflow, activity notification) |
| **FS-SSH: SSH Sessions** | |
| FS-SSH-001 – FS-SSH-003 | UR 9 §9.1 (SSH integration) |
| FS-SSH-010 – FS-SSH-014 | UR 9 §9.1 (connection state, notification); Domain expert (lifecycle, PTY request); Security review (deprecated algorithms) |
| FS-SSH-020 – FS-SSH-022 | UR 9 §9.1 (interruption detection); Domain expert (keepalive, drop detection) |
| FS-SSH-030 – FS-SSH-034 | UR 9 §9.2 (saved connections) |
| FS-SSH-040 – FS-SSH-042 | UR 9 §9.4 (reconnection) |
| **FS-CRED: Credentials** | |
| FS-CRED-001 – FS-CRED-002 | UR 9 §9.3 (security, keychain, key paths) |
| **FS-THEME: Theming** | |
| FS-THEME-001 – FS-THEME-002 | UR 8 §8.1 (default theme) |
| FS-THEME-003 – FS-THEME-007 | UR 8 §8.2 (user-created themes) |
| FS-THEME-008 – FS-THEME-009 | UR 8 §8.3 (design tokens) |
| FS-THEME-010 | UR 8 §8.2 (user-created themes, font/line height customisation) |
| **FS-PREF: Preferences** | |
| FS-PREF-001 | UR 5 §5.1 (persistence) |
| FS-PREF-002 – FS-PREF-004 | UR 5 §5.2 (preferences UI, sections) |
| FS-PREF-005 | UR 6 §6.1 (Ctrl+, shortcut); UR 3.1 (dual modality) |
| FS-PREF-006 | UR 5 §5.2; UR 7 §7.1 (scrollback config); Domain expert (cursor, bell, delimiters) |
| **FS-A11Y: Accessibility** | |
| FS-A11Y-001 – FS-A11Y-004 | CLAUDE.md (WCAG 2.1 AA, contrast, targets, keyboard, color-only) |
| FS-A11Y-005 | UR 3.1 (dual modality); UR 3.3 (PTY exception) |
| FS-A11Y-006 | UR 3.1 (dual modality); UR 3.2 (discoverable UI) |
| FS-A11Y-007 | UR 8 §8.2 (theme editing); UR 5 §5.2 (preferences UI accessibility) |
| **FS-UX: UX Cross-Cutting** | |
| FS-UX-001 | UR 2 (personas, esp. Sam §2.3); UR 3.2 (discoverable UI) |
| FS-UX-002 | UR 2 §2.3 (Sam — no config required for basic use); UR 3.2 (discoverable UI) |
| **FS-SEC: Security Hardening** | |
| FS-SEC-001 – FS-SEC-005 | Security review; CLAUDE.md (security constraints) |
| **FS-VT-063** | Security review (title read-back injection) |
| **FS-VT-075 – FS-VT-076** | Security review (OSC 52 clipboard control) |
| **FS-CRED-003 – FS-CRED-006** | Security review (credential lifecycle) |
| **FS-I18N: Internationalisation** | |
| FS-I18N-001 – FS-I18N-006 | UR 10 §10.1 (language support, selection, persistence, fallback) |
| FS-I18N-007 | UR 10 §10.2 (PTY locale env vars not modified) |
| **FS-DIST: Distribution** | |
| FS-DIST-001 – FS-DIST-005 | UR 11 §11.1 (AppImage, self-contained, multi-arch) |
| FS-DIST-006 | UR 11 §11.1 (release artefact integrity); Security review |

---

## 7. Review Notes

> **Reviewer:** user-rep (User Representative)
> **Date:** 2026-04-04
> **Status:** Open issues requiring team discussion

### 7.1 Open Issues

**RN-001: FS-CLIP-003 — RESOLVED.**
Default delimiter set specified (Option B): space and punctuation delimiters; `/`, `.`, `-`, `_`, `:`, `@`, `=` are non-delimiters by default. User-configurable. Decision: accepted 2026-04-04.

**RN-002: FS-SSH-011 — RESOLVED.**
Prompt: plain-language explanatory text required (Option B). Fingerprint format: SHA-256 only (Option A). Key change behavior: block by default with explanation and actionable instructions (Option A + explanation). Decision: accepted 2026-04-04.

**RN-003: RESOLVED.**
Added FS-UX-001 (section 3.15): cross-cutting requirement on error message quality — plain language, actionable next step, technical details as secondary element only. Decision: accepted 2026-04-04.

**RN-004: RESOLVED.**
Added FS-UX-002 (section 3.15): on first launch, a non-intrusive hint signals the context menu (right-click); disappears after first right-click. Option B. Decision: accepted 2026-04-04.

**RN-005: FS-VT-091 — RESOLVED.**
`SHOULD` → `MUST`: visual bell is the mandatory default. Decision: accepted 2026-04-04.

**RN-006: FS-SB-002 — RESOLVED.**
No artificial maximum: user-configurable without upper limit. Preferences UI displays real-time memory estimate per pane. Default remains 10,000 lines. Decision: Option D, accepted 2026-04-04.

**RN-007: RESOLVED.**
UR.md fully renumbered: sections 4–9 corrected, all subsections aligned. Traceability matrix updated accordingly. Decision: accepted 2026-04-04.

**RN-008: FS-TAB-006 — RESOLVED.**
Interaction specified: double-click for inline editing (primary) + context menu "Rename" (discoverable). Enter confirms, Escape cancels. Clearing label reverts to process-driven title. Option C. Decision: accepted 2026-04-04.

**RN-009: FS-KBD-003 — RESOLVED.**
Ctrl+Tab / Ctrl+Shift+Tab retained as fixed defaults (universal convention). Split and pane navigation shortcuts: defaults now resolved from UXD.md §11.2 (Ctrl+Shift+D, Ctrl+Shift+E, Ctrl+Shift+Right/Left/Up/Down, Ctrl+Shift+Q). F2 added as default shortcut for inline tab rename. Decision: accepted 2026-04-04, updated 2026-04-04.

**RN-010: FS-SSH-010 Closed state — RESOLVED.**
Added definition for the Closed state (intentional/clean termination) and clarified distinction from Disconnected (unexpected interruption). Decision: accepted 2026-04-04.

**RN-011: FS-THEME-010 line height — RESOLVED.**
Added FS-THEME-010 (Should): `--line-height-terminal` token is user-overridable; UI chrome line height is fixed. Decision: accepted 2026-04-04.

**RN-012: FS-NOTIF-005 scroll arrow badges — RESOLVED.**
Added FS-NOTIF-005 (Should): scroll arrows display a dot badge when scrolled-out tabs have pending notifications; bell takes priority over activity. Decision: accepted 2026-04-04.

**RN-013: FS-A11Y-007 theme editor isolation — RESOLVED.**
Added FS-A11Y-007 (Must): theme editor chrome renders with active Umbra system tokens; only the preview area reflects the custom theme. Decision: accepted 2026-04-04.

---

## 7. Security Review Notes

> **Reviewed by:** security-expert
> **Date:** 2026-04-04
> **Scope:** Full FS document, all sections

### 7.1 Changes Applied (Critical and High)

The following security issues were identified and corrected directly in this document:

| ID | Severity | Issue | Section Modified |
|----|----------|-------|------------------|
| C1 | Critical | OSC 52 clipboard manipulation not addressed -- enables clipboard exfiltration and poisoning by remote/malicious programs | Added FS-VT-075, FS-VT-076 (section 3.1.9) |
| C2 | Critical | Title read-back sequences (CSI 21t, OSC queries) not blocked -- enables input injection attacks | Added FS-VT-063 (section 3.1.7) |
| C3 | Critical | FS-CRED incomplete -- no spec for in-memory credential lifetime, logging prohibition, keychain absence fallback, or identity path validation | Added FS-CRED-003 through FS-CRED-006 (section 3.11) |
| C4 | Critical | TOFU host key change specification too weak -- did not require connection blocking, fingerprint comparison, or safe default action | Strengthened FS-SSH-011 (section 3.10.2) |
| H1 | High | OSC 8 URI validation insufficient -- did not address file: in SSH context, URI length, control characters, or unknown schemes | Strengthened FS-VT-073 (section 3.1.8) |
| H2 | High | Bracketed paste did not specify stripping of end-sequence within pasted text -- enables paste injection | Strengthened FS-CLIP-008 (section 3.6) |
| H4 | High | No CSP specification -- WebView open to XSS | Added FS-SEC-001 (section 3.15) |
| H5 | High | PTY fd O_CLOEXEC not specified -- fd leak to child processes | Added FS-SEC-002 (section 3.15) |
| H3 | High | Preferences file not validated -- potential injection via tampered config | Added FS-SEC-003 (section 3.15) |

### 7.2 Medium Items (Recommended for Architecture/Implementation Phase)

These items do not require FS-level specification changes but SHOULD be addressed during architecture design or implementation:

| ID | Issue | Recommendation |
|----|-------|----------------|
| M1 | TIOCSTI injection | On Linux kernels < 6.2, a child process can inject keystrokes into the terminal via `ioctl(TIOCSTI)`. The architecture should document whether TauTerm relies on kernel 6.2+ `TIOCSTI` restriction or implements its own mitigation (e.g., verifying input source). |
| M2 | Security event logging | SSH authentication failures, host key changes (accepted or rejected), and rejected OSC sequences (OSC 52, CSI 21t) should be logged at INFO level for forensic purposes. No credentials in logs (per FS-CRED-004). |
| M3 | OSC rate-limiting | Title change sequences (OSC 0/1/2) and hyperlink creation (OSC 8) should be rate-limited similarly to bell (FS-VT-092) to prevent UI disruption attacks and memory exhaustion from rapid hyperlink generation. |
| M4 | Known-hosts file validation | The known-hosts file should be parsed defensively: invalid lines skipped with a warning, file corruption should not prevent new connections (degrade to prompting). |
| M5 | SSH agent forwarding | Explicitly declared out of scope (FS-SEC-004). If added in a future version, it requires per-connection opt-in with clear risk disclosure to the user. |

### 7.3 Low Items (Hardening Suggestions)

| ID | Issue | Recommendation |
|----|-------|----------------|
| L1 | VT sequence size limit | Addressed by FS-SEC-005 (4096 byte limit on individual OSC/DCS). Implementation should also enforce a limit on the number of CSI parameters (suggested: 32). |
| L2 | Scrollback memory | Addressed by FS-SB-002 (RN-006 resolved 2026-04-04): preferences UI displays real-time memory estimate. |
| L3 | Environment variable sanitization | FS-PTY-011/012 specify what to set/inherit, but the child process environment should be constructed from scratch (allow-list) rather than inherited wholesale. Dangerous variables (`LD_PRELOAD`, `LD_LIBRARY_PATH`) MUST NOT be inherited. This is an architecture decision. |

### 7.4 Open Questions (Require Team Decision)

1. **OSC 52 write -- RESOLVED.** Per-connection setting (Option B): each saved connection has its own OSC 52 write toggle; unsaved local sessions use the global default (disabled). FS-VT-075 updated. Decision: accepted 2026-04-04.
2. **Known-hosts file location — RESOLVED.** Separate file `~/.config/tauterm/known_hosts` (OpenSSH-compatible format). TauTerm never touches `~/.ssh/known_hosts`. Preferences UI offers import from `~/.ssh/known_hosts`. FS-SSH-011 updated. Option B. Decision: accepted 2026-04-04.
3. **Minimum SSH key strength — RESOLVED.** Added FS-SSH-014: non-blocking, dismissible warning in the pane when deprecated algorithms are negotiated (`ssh-rsa` SHA-1, `ssh-dss`). Connection proceeds. Option C. Decision: accepted 2026-04-04.
