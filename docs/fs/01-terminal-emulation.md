<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — Terminal Emulation

> Part of the [Functional Specifications](README.md). See also: [00-overview.md](00-overview.md), [02-ui-navigation.md](02-ui-navigation.md), [03-remote-ssh.md](03-remote-ssh.md), [04-config-system.md](04-config-system.md), [05-scope-constraints.md](05-scope-constraints.md)

---

## 3.1 FS-VT: Terminal Emulation

### 3.1.1 Target Standard

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

### 3.1.2 Character Set Handling

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

### 3.1.3 ANSI Color Codes

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

### 3.1.4 Cursor

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-030 | TauTerm MUST support cursor shapes 0–6 via DECSCUSR (CSI Ps SP q): default, blinking block, steady block, blinking underline, steady underline, blinking bar, steady bar. | Must |
| FS-VT-031 | Cursor visibility MUST be controllable via DECTCEM (CSI ?25h to show, CSI ?25l to hide). | Must |
| FS-VT-032 | Cursor blink rate MUST be configurable in user preferences. The default MUST be 530ms on/off. | Must |
| FS-VT-033 | DECSC and DECRC MUST save and restore cursor state per screen buffer, independently. | Must |
| FS-VT-034 | When the terminal pane loses focus, the cursor SHOULD visually indicate the inactive state. When focus returns, the cursor MUST resume its normal appearance. | Should |

**Acceptance criteria:**
- FS-VT-030: A script emitting each DECSCUSR value produces the corresponding cursor shape.
- FS-VT-031: `tput civis` hides the cursor; `tput cnorm` restores it.
- FS-VT-032: Changing the blink rate in preferences visibly changes the cursor blink speed without restart.
- FS-VT-033: In vim (alternate screen), saving/restoring the cursor does not affect the normal screen cursor position, and vice versa.
- FS-VT-034: Clicking outside the terminal pane changes the cursor appearance to indicate focus loss; clicking back restores its normal appearance.

### 3.1.5 Screen Modes

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

### 3.1.6 Scrolling Regions

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

### 3.1.7 Title / OSC Sequences

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

### 3.1.8 Hyperlinks

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

### 3.1.9 Clipboard Control Sequences (OSC 52)

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-075 | OSC 52 clipboard write (setting clipboard content from PTY output) MUST be disabled by default. It MAY be enabled per connection: each saved connection (local or SSH) MAY independently enable or disable OSC 52 write. A local PTY session (no saved connection) uses the global default (disabled). This prevents enabling OSC 52 write for trusted local sessions from inadvertently enabling it for untrusted SSH sessions. | Must |
| FS-VT-076 | OSC 52 clipboard read (querying clipboard content) MUST be permanently rejected. No configuration option MUST allow enabling it. This prevents clipboard exfiltration by malicious programs or remote servers. | Must |

**Acceptance criteria:**
- FS-VT-075: By default, `printf "\033]52;c;$(echo -n 'malicious' | base64)\007"` does not modify the system clipboard. In a saved connection with OSC 52 write enabled, the same sequence updates the clipboard. An SSH session with OSC 52 write disabled ignores the sequence even if local sessions have it enabled.
- FS-VT-076: `printf "\033]52;c;?\007"` never produces a response containing clipboard content, regardless of configuration.

### 3.1.10 Mouse Reporting

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-080 | TauTerm MUST support mouse tracking modes: X10 (9), Normal (1000), Button-event (1002), and Any-event (1003). | Must |
| FS-VT-081 | TauTerm MUST support mouse encodings: default X10 and SGR mode 1006. URXVT mode 1015 SHOULD be supported. | Must / Should |
| FS-VT-082 | When mouse reporting is active, mouse events MUST be sent to the PTY. When inactive, mouse events MUST be handled by TauTerm (selection, scrolling, etc.). | Must |
| FS-VT-083 | Shift+Click MUST bypass mouse reporting and perform TauTerm selection regardless of reporting mode. | Must |
| FS-VT-084 | Focus events (mode 1004) MUST be supported. | Must |
| FS-VT-085 | Mouse wheel events when reporting is active MUST be sent to the PTY as button 4/5 events. Shift+Wheel MUST scroll the scrollback instead. | Must |
| FS-VT-086 | When the alternate screen is exited (RM ?1049), all mouse reporting modes MUST be reset to their defaults. This ensures that applications crashing without sending `?1000l`/`?1003l` do not leave the terminal in a broken mouse reporting state. TauTerm destroying the `VtProcessor` on pane close provides equivalent cleanup for session termination. | Must |

**Acceptance criteria:**
- FS-VT-080: vim with `set mouse=a` responds to click-to-position correctly.
- FS-VT-083: With vim mouse capture active, Shift+Click selects text in TauTerm.
- FS-VT-085: In vim with mouse enabled, wheel scrolls the vim buffer; Shift+Wheel scrolls TauTerm's scrollback.
- FS-VT-086: After vim exits, clicking in the terminal performs TauTerm selection (not mouse reporting).

### 3.1.11 Bell

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

## 3.2 FS-PTY: PTY Lifecycle

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-PTY-001 | Each pane MUST have its own independent PTY pair (master + slave). | Must |
| FS-PTY-002 | The child process MUST be spawned with the slave PTY as its controlling terminal. | Must |
| FS-PTY-003 | PTY I/O MUST NOT block the async runtime. | Must |
| FS-PTY-004 | File descriptors MUST be properly managed: each process retains only the file descriptors necessary for its role; no descriptors are leaked across the fork boundary. | Must |
| FS-PTY-005 | Shell process exit MUST be detected (via SIGCHLD/waitpid). The pane MUST transition to a "terminated" state displaying the exit status. The pane MUST NOT auto-close. | Must |
| FS-PTY-006 | A terminated pane MUST offer the user two actions: close the pane, or restart the shell. | Must |
| FS-PTY-007 | Closing a tab or pane MUST close the master fd, sending SIGHUP to the child process group. | Must |
| FS-PTY-008 | If a foreground process is running when the user attempts to close a tab, a pane, or the application window, a confirmation dialog MUST be displayed. When closing the window, the dialog MUST indicate how many tabs/panes have active processes. | Must |
| FS-PTY-009 | Pane resize MUST trigger `ioctl(TIOCSWINSZ)` and deliver SIGWINCH to the foreground process group. The resize MUST include pixel dimensions (xpixel, ypixel). | Must |
| FS-PTY-010 | Resize events SHOULD be debounced (≤ 100ms, typically 50ms). The final size MUST always be sent. | Should |
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

## 3.7 FS-SB: Scrollback Buffer

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

## 3.8 FS-SEARCH: Search in Output

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
