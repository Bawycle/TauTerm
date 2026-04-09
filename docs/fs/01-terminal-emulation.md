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
| FS-VT-006 | DECAWM (auto-wrap mode, DECSET/DECRST 7) MUST be supported. DECAWM MUST be enabled by default. When DECAWM is enabled, a character written at the last column MUST cause the next character to wrap to the first column of the next line. When DECAWM is disabled, a character written at the last column MUST overwrite the character in that last column; the cursor MUST NOT advance beyond the last column. DECAWM state MUST be saved and restored by DECSC/DECRC alongside the cursor position. | Must |

**Acceptance criteria:**
- FS-VT-001: `echo $TERM` in a new session outputs `xterm-256color`.
- FS-VT-002: `echo $COLORTERM` in a new session outputs `truecolor`.
- FS-VT-003: Programs that rely on VT100/VT220 sequences (e.g., vim, less, htop) render correctly.
- FS-VT-004: 256-color and truecolor test scripts display correct colors; mouse-aware applications respond to clicks; bracketed paste wraps pasted content.
- FS-VT-005: Malformed sequences are discarded without corrupting subsequent output; no sequence fragment persists across reads.
- FS-VT-006: With DECAWM disabled (`CSI ?7l`), printing 10 characters on an 80-column line leaves the cursor at column 80 and only the last character is visible at that position; the rest of the line is not altered. After `DECSC`, toggling DECAWM and issuing `DECRC` restores the original DECAWM state.

### 3.1.2 Character Set Handling

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-010 | TauTerm MUST handle UTF-8 natively, including multi-byte sequences split across read boundaries. | Must |
| FS-VT-011 | CJK wide characters MUST occupy exactly 2 cells. A wide character at the last column of a row MUST wrap to the next line. | Must |
| FS-VT-012 | Combining characters MUST attach to the preceding base character without advancing the cursor. | Must |
| FS-VT-013 | Zero-width characters MUST NOT advance the cursor or consume a cell. | Must |
| FS-VT-014 | Single-codepoint wide emoji MUST occupy 2 cells. ZWJ emoji sequences SHOULD render as a single glyph occupying 2 cells when the font supports it. | Should |
| FS-VT-017 | The variation selector U+FE0F (emoji presentation) appended to an emoji base codepoint MUST cause the glyph to be treated as wide (2 cells). The variation selector U+FE0E (text presentation) appended to an emoji base codepoint MUST cause the glyph to be treated as narrow (1 cell). The variation selector itself MUST NOT advance the cursor. | Must |
| FS-VT-018 | Emoji skin-tone modifier codepoints (U+1F3FB–U+1F3FF) MUST be treated as combining characters: they MUST attach to the preceding emoji base without advancing the cursor and without consuming an additional cell. The base emoji retains its 2-cell width. | Must |
| FS-VT-019 | Regional Indicator Symbol pairs (U+1F1E6–U+1F1FF followed immediately by another U+1F1E6–U+1F1FF) MUST be treated as a single 2-cell unit (flag emoji). The first Regional Indicator MUST occupy a provisional 2-cell slot; upon receiving the second Regional Indicator, the combined flag is confirmed in those same 2 cells. An unpaired Regional Indicator (followed by a non-Regional Indicator codepoint) MUST be treated as a 1-cell wide character. | Must |
| FS-VT-039 | When a ZWJ emoji sequence is encountered and the font does not contain a composite glyph for the full sequence, TauTerm MUST fall back to rendering the constituent components in sequence, each occupying its normal cell width. The ZWJ codepoints (U+200D) themselves MUST NOT advance the cursor. | Must |
| FS-VT-015 | DEC Special Graphics character set (SI/SO, ESC ( 0) MUST be supported for line-drawing characters. | Must |
| FS-VT-016 | Invalid UTF-8 sequences (e.g., overlong encodings) MUST be replaced with U+FFFD (REPLACEMENT CHARACTER). | Must |

**Acceptance criteria:**
- FS-VT-010: A program outputting a multi-byte character split across two write() calls renders the character correctly.
- FS-VT-011: Chinese/Japanese characters are correctly positioned; `echo -e "\xe4\xb8\xad"` at the last column wraps to the next line.
- FS-VT-012: `echo -e "e\xcc\x81"` (e + combining acute accent) displays as a single accented character in one cell.
- FS-VT-013: Zero-width space (U+200B) does not create a visible gap or shift subsequent characters.
- FS-VT-015: `mc` (Midnight Commander) line-drawing borders render correctly.
- FS-VT-016: Overlong UTF-8 byte `0xC0 0xAF` renders as U+FFFD.
- FS-VT-017: `printf "\U0001F600\uFE0F"` (grinning face + emoji variation selector) occupies 2 cells. `printf "\u0023\uFE0E"` (# + text variation selector) occupies 1 cell.
- FS-VT-018: `printf "\U0001F44B\U0001F3FC"` (waving hand + medium-light skin tone) occupies exactly 2 cells, not 4.
- FS-VT-019: `printf "\U0001F1EB\U0001F1F7"` (FR flag) occupies 2 cells. `printf "\U0001F1EB X"` (unpaired RI) occupies 1 cell for the RI and 1 for the space.
- FS-VT-039: An emoji ZWJ sequence whose composite glyph is absent from the font renders each component separately without corrupting cursor position.

### 3.1.3 ANSI Color Codes

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-020 | TauTerm MUST support standard ANSI colors: SGR 30–37, 40–47 (normal), 90–97, 100–107 (bright). | Must |
| FS-VT-021 | TauTerm MUST support 256-color mode: SGR 38;5;N / 48;5;N for N in 0–255. Colors 0–7 map to ANSI palette, 8–15 to bright variants, 16–231 to the 6x6x6 color cube, 232–255 to the grayscale ramp. | Must |
| FS-VT-022 | TauTerm MUST support truecolor: SGR 38;2;R;G;B and 48;2;R;G;B. The colon variant (38:2:R:G:B, ITU T.416) MUST also be supported. | Must |
| FS-VT-023 | ANSI palette colors 0–15 MUST be remappable via the active theme. Truecolor values are absolute and not affected by the theme palette. | Must |
| FS-VT-024 | SGR 0 MUST reset all attributes. The following attributes MUST be independently settable and resettable: bold (1/22), dim (2/22), italic (3/23), underline (4/24), blink (5/25), inverse (7/27), hidden (8/28), strikethrough (9/29). | Must |
| FS-VT-025 | Extended underline styles (SGR 4:0 through 4:5) and underline color (SGR 58) SHOULD be supported. | Should |
| FS-VT-026 | SGR bold (1) MUST increase the rendering weight of the glyph. When the foreground color is set via SGR 31–37 (standard ANSI foreground, indices 1–7), bold MUST remap that color to the corresponding bright variant (indices 9–15). SGR 30 (ANSI index 0, Black) MUST NOT be remapped — bold black is indistinguishable from bright black on dark backgrounds and promotion is omitted by convention. This "bold colors" behavior MUST be the default. Colors set via SGR 38;5;N (256-color) or SGR 38;2;R;G;B (truecolor) MUST NOT be remapped regardless of the palette index value. This matches xterm's `boldColors` resource behavior. | Must |
| FS-VT-027 | SGR dim (2) MUST reduce the visual intensity of the foreground color. When both bold (1) and dim (2) are active simultaneously, dim MUST take precedence for intensity rendering (the glyph is rendered at reduced intensity, not bold intensity). Bold color remapping (ANSI 1–7 → 9–15) remains active even when dim is also set. This matches the xterm reference behavior. | Must |
| FS-VT-028 | SGR italic (3) MUST select the italic variant of the active font when available. If the font does not provide a distinct italic face, TauTerm MUST use the regular face without synthetic slanting. No fallback font substitution is required. | Must |
| FS-VT-029 | SGR blink (5) and rapid blink (6) MUST both produce a blinking effect. TauTerm is not required to differentiate their blink rate; both MAY use the same configured blink interval. Blinking MUST be suspended (characters remain fully visible) when the `prefers-reduced-motion` system accessibility setting is active. | Must |
| FS-VT-036 | SGR hidden (8, conceal) MUST cause the character to be rendered with its foreground color set to the background color, making it invisible on screen. Hidden characters MUST be included verbatim in selection and clipboard copy operations — the copy result MUST contain the concealed text. | Must |
| FS-VT-037 | SGR strikethrough (9) MUST render a horizontal line through the vertical center of the cell. The exact vertical position is implementation-defined but MUST be visually distinct from underline and overline. | Must |
| FS-VT-038 | SGR reverse video (7) MUST swap the resolved foreground and background colors at render time. "Resolved" means after applying bold color remapping, default-color fallback, and theme palette substitution. When reverse is combined with an explicit truecolor foreground and no explicit background, the truecolor value becomes the cell background and the default terminal background becomes the foreground. | Must |

**Acceptance criteria:**
- FS-VT-020: A test script cycling through SGR 30–37 and 90–97 displays 16 distinct foreground colors.
- FS-VT-021: A 256-color test pattern (e.g., `256colors.pl`) displays all colors correctly with smooth gradients in the cube and ramp regions.
- FS-VT-022: `printf "\033[38;2;255;100;0mTruecolor\033[0m"` displays orange text. The colon variant produces the same result.
- FS-VT-023: Changing the theme's color 1 (red) changes the color displayed by SGR 31, but has no effect on `\033[38;2;255;0;0m`.
- FS-VT-024: Each attribute can be turned on and off independently without affecting other active attributes.
- FS-VT-025: Neovim diagnostic underlines (curly, dotted) render with the correct style and color.
- FS-VT-026: `printf "\033[1;31mBold red\033[0m"` displays text in ANSI bright red (index 9), not ANSI red (index 1). `printf "\033[1;38;5;1mBold 256\033[0m"` is not remapped — it displays palette index 1 (dark red), not index 9 (bright red).
- FS-VT-027: `printf "\033[1;2mBoldDim\033[0m"` renders at reduced intensity. `printf "\033[1;2;31mBoldDimRed\033[0m"` uses bright red for the color (bold remap active) but renders at dim intensity.
- FS-VT-028: If the active font lacks an italic face, italic text is rendered in the regular face without transformation artifacts.
- FS-VT-029: `printf "\033[5mBlink\033[0m"` and `printf "\033[6mRapidBlink\033[0m"` both produce a blinking effect. With system `prefers-reduced-motion` enabled, the text is static.
- FS-VT-036: `printf "\033[8mSecret\033[0m"` renders as invisible text. Selecting and copying that text yields "Secret" in the clipboard.
- FS-VT-037: `printf "\033[9mStrike\033[0m"` renders a horizontal line through the middle of the text.
- FS-VT-038: `printf "\033[38;2;255;0;0m\033[7mInverse\033[0m"` renders with a red background and the terminal default foreground as text color.

### 3.1.4 Cursor

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-VT-030 | TauTerm MUST support cursor shapes 0–6 via DECSCUSR (CSI Ps SP q): default (0), blinking block (1), steady block (2), blinking underline (3), steady underline (4), blinking bar (5), steady bar (6). DECSCUSR 0 MUST be treated as "blinking block" — it is the xterm default shape and does NOT restore user preferences. DECSCUSR emits a `cursor-style-changed` IPC event immediately (see architecture §4.3). | Must |
| FS-VT-031 | Cursor visibility MUST be controllable via DECTCEM (CSI ?25h to show, CSI ?25l to hide). | Must |
| FS-VT-031a | TauTerm MUST support DECSET/DECRST ?12 (ATT610 — cursor blink). `CSI ?12h` enables cursor blinking; `CSI ?12l` disables it. DECSET ?12 is independent of DECSCUSR: DECSCUSR encodes shape and per-shape blink preference; DECSET ?12 is a global blink on/off override. When DECSET ?12 is disabled, all cursor shapes render steady regardless of the DECSCUSR value. The DECSET ?12 state is propagated to the frontend via the `cursor.blink` field of `ScreenUpdateEvent`. | Must |
| FS-VT-032 | Cursor blink rate MUST be configurable in user preferences. The default MUST be 533ms visible (on) / 266ms invisible (off) — an asymmetric 2:1 ratio. The user-configurable value controls the visible phase duration; the invisible phase is half that duration. | Must |
| FS-VT-033 | DECSC and DECRC MUST save and restore cursor state per screen buffer, independently. | Must |
| FS-VT-034 | When the terminal pane loses focus, the cursor MUST visually indicate the inactive state by rendering as a hollow (outline) rectangle regardless of the DECSCUSR shape that was active. Cursor blinking MUST be suspended while the pane is unfocused. When focus returns, the cursor MUST immediately resume its configured shape and blink state. A hidden cursor (DECTCEM off) MUST remain hidden regardless of focus state. | Must |
| FS-VT-035 | The minimum terminal dimensions reported to the child process via `TIOCSWINSZ` MUST be 1 column by 1 line. TauTerm MUST NOT clamp or refuse to report sizes below a UI-imposed minimum to the PTY: the PTY always receives the true cell geometry. If the UI enforces a minimum pane size larger than 1×1 (e.g. 20 columns × 5 lines as defined in UXD), the PTY reports that enforced minimum rather than a size smaller than what is displayed. | Must |

**Acceptance criteria:**
- FS-VT-030: A script emitting each DECSCUSR value (0–6) produces the corresponding cursor shape. `printf '\033[0 q'` (DECSCUSR 0) renders a blinking block — not the user's preference shape.
- FS-VT-031: `tput civis` hides the cursor; `tput cnorm` restores it.
- FS-VT-031a: `printf '\033[?12l'` stops cursor blinking regardless of DECSCUSR shape; `printf '\033[?12h'` re-enables it.
- FS-VT-032: Changing the blink rate in preferences visibly changes the cursor blink speed without restart.
- FS-VT-033: In vim (alternate screen), saving/restoring the cursor does not affect the normal screen cursor position, and vice versa.
- FS-VT-034: Clicking outside the terminal pane changes the cursor to a hollow rectangle and stops blinking; clicking back restores the blinking block (or the application-set shape). A pane with a hidden cursor (`tput civis`) does not reveal the cursor on focus loss.
- FS-VT-035: Resizing a pane to the UI minimum (20×5) causes `stty size` inside the pane to report `5 20`.

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
| FS-VT-055 | When a line feed (LF, VT, FF) occurs and the cursor is positioned OUTSIDE the active scroll region (above the top margin or below the bottom margin), the cursor MUST move down one line without triggering any scroll. If the cursor is already at the last line of the screen and outside the scroll region, the line feed MUST be ignored (no scroll, no cursor movement beyond the screen boundary). | Must |
| FS-VT-056 | DECOM (origin mode, DECSET/DECRST 6) MUST be supported. When DECOM is enabled, cursor addressing (CUP, HVP, and all absolute cursor movement sequences) MUST be relative to the top-left corner of the active scroll region. The cursor MUST be constrained within the scroll region in origin mode. When DECOM is disabled, cursor addressing is relative to the top-left corner of the full screen. DECOM MUST be disabled by default. | Must |
| FS-VT-057 | DECOM state MUST be saved and restored by DECSC/DECRC. Switching screen buffers (DECSET/DECRST 1049) MUST reset DECOM to disabled in the newly active buffer, consistent with each buffer maintaining independent mode state (FS-VT-043). | Must |

**Acceptance criteria:**
- FS-VT-050: tmux with a status bar: the status bar remains fixed while the main area scrolls.
- FS-VT-051: Output within a partial scroll region does not affect lines outside the region.
- FS-VT-053: After running tmux and scrolling its main pane, the scrollback does not contain the tmux status bar.
- FS-VT-054: After tmux exits, the scroll region is reset and normal scrolling resumes.
- FS-VT-055: Setting a scroll region of lines 5–10 on a 24-line screen, then positioning the cursor at line 2 and issuing a LF, moves the cursor to line 3 without scrolling lines 5–10.
- FS-VT-056: With DECOM enabled (`CSI ?6h`) and a scroll region of lines 5–10, `CSI 1;1H` positions the cursor at physical row 5, column 1. With DECOM disabled (`CSI ?6l`), `CSI 1;1H` positions the cursor at physical row 1, column 1.
- FS-VT-057: vim (which uses scroll regions and DECOM internally) exits cleanly with DECOM restored to disabled and the normal screen cursor at its correct position.

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
| FS-PTY-005 | Shell process exit MUST be detected (via SIGCHLD/waitpid). If the exit code is non-zero or the process was terminated by a signal, the pane MUST transition to a "terminated" state displaying the exit status; the pane MUST NOT auto-close. If the exit code is 0 (clean exit), the pane MUST auto-close immediately. | Must |
| FS-PTY-006 | A pane that has transitioned to the "terminated" state (non-zero exit or signal termination, per FS-PTY-005) MUST offer the user two actions: close the pane, or restart the shell. This requirement does not apply to exit code 0, which triggers auto-close (FS-PTY-005). | Must |
| FS-PTY-007 | Closing a tab or pane MUST close the master fd, sending SIGHUP to the child process group. | Must |
| FS-PTY-008 | If a non-shell foreground process is running (i.e., the foreground process group of the PTY is not the shell itself) when the user attempts to close a tab, a pane, or the application window, a confirmation dialog MUST be displayed. A pane with only an idle shell at the prompt MUST NOT trigger a confirmation dialog. When closing the window, the dialog MUST indicate how many tabs/panes have active non-shell processes. | Must |
| FS-PTY-009 | Pane resize MUST trigger `ioctl(TIOCSWINSZ)` and deliver SIGWINCH to the foreground process group. The resize MUST include pixel dimensions (xpixel, ypixel). | Must |
| FS-PTY-010 | Resize events SHOULD be debounced (≤ 100ms, typically 50ms). The final size MUST always be sent. | Should |
| FS-PTY-011 | The following environment variables MUST be set in the child process: `TERM=xterm-256color`, `COLORTERM=truecolor`, `LANG` (UTF-8 locale — inherited or fallback), `LINES`, `COLUMNS`, `SHELL`, `HOME`, `USER`, `LOGNAME`, `PATH`, `TERM_PROGRAM=TauTerm`, `TERM_PROGRAM_VERSION=<version>`. | Must |
| FS-PTY-012 | The environment variables `DISPLAY`, `WAYLAND_DISPLAY`, and `DBUS_SESSION_BUS_ADDRESS` MUST be inherited from the parent environment when present. | Must |
| FS-PTY-013 | The initial tab MUST launch a login shell. Subsequent tabs and panes MUST launch interactive non-login shells. | Must |
| FS-PTY-014 | If `$SHELL` is invalid or unset, TauTerm MUST fall back to `/bin/sh`. | Must |

**Acceptance criteria:**
- FS-PTY-001: Two panes run independent shell sessions; input in one does not affect the other.
- FS-PTY-005: Running `exit 1` in a shell displays the exit status (1) in the pane; the pane remains visible with Close/Restart actions. Running `exit` (code 0) causes the pane to close immediately.
- FS-PTY-006: A pane terminated with a non-zero exit code shows "Close" and "Restart" actions.
- FS-PTY-008: Running `sleep 3600` then pressing the close-tab shortcut shows a confirmation dialog. Pressing the close-tab shortcut on a pane with only an idle shell prompt does NOT show a confirmation dialog.
- FS-PTY-009: Resizing a pane while vim is open causes vim to redraw at the new size.
- FS-PTY-011: `echo $TERM_PROGRAM` outputs `TauTerm`.
- FS-PTY-013: The initial tab sources `~/.bash_profile` (login shell); a second tab does not.
- FS-PTY-014: Setting `SHELL=/nonexistent` before launch results in a `/bin/sh` session.

---

## 3.7 FS-SB: Scrollback Buffer

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SB-001 | Each pane MUST have its own scrollback buffer. | Must |
| FS-SB-002 | The scrollback buffer size MUST be configurable by the user (number of lines). The default MUST be 10,000 lines. The valid range is **100 – 1,000,000 lines** (inclusive). Values below 100 MUST be clamped to 100; values above 1,000,000 MUST be clamped to 1,000,000. The upper bound exists to prevent unbounded memory allocation: at the architectural upper-bound of 5,500 bytes/line (see `docs/arch/07-screen-buffer-data-model.md` §14.2), 1,000,000 lines corresponds to approximately 5.5 GB per pane — a practical safety ceiling. The preferences UI MUST display an estimated memory consumption for the configured value (e.g., "~200 MB per pane at 100,000 lines"), updated as the user adjusts the setting. When the backend clamps the submitted value, the UI MUST display the effective (clamped) value, not the originally submitted value. | Must |
| FS-SB-003 | The scrollback buffer MUST store text content AND cell attributes (colors, bold, italic, underline, etc.). | Must |
| FS-SB-004 | Only lines scrolled off the top of a full-screen scroll region MUST enter the scrollback buffer. Lines scrolled out of a partial scroll region (DECSTBM-restricted) MUST NOT populate the scrollback. | Must |
| FS-SB-005 | When the alternate screen buffer is active, the scrollback buffer MUST be frozen: no new lines added. Scrollback navigation SHOULD be disabled. | Must / Should |
| FS-SB-006 | When returning to the normal screen buffer, the scrollback MUST be navigable with all previously accumulated content intact. | Must |
| FS-SB-007 | The user MUST be able to scroll the scrollback using the mouse wheel, keyboard shortcuts, and a visible scrollbar. | Must |
| FS-SB-008 | The scrollback buffer MUST distinguish between hard newlines (actual line breaks in the output) and soft wraps (line breaks introduced by terminal width). | Must |
| FS-SB-009 | When `scroll_offset > 0` and the PTY process produces new output, the viewport position MUST be maintained. The system MUST NOT auto-scroll to the bottom. | Must |
| FS-SB-010 | When the user sends keyboard input to the PTY while `scroll_offset > 0`, the backend MUST reset `scroll_offset` to 0 and emit a `scroll-position-changed` event. The frontend MUST scroll the viewport to the bottom upon receiving this event. | Must |
| FS-SB-011 | When the user copies selected text from the terminal (viewport or scrollback), soft-wrapped line boundaries MUST NOT produce a newline character in the copied text. The text of a soft-wrapped line and its continuation on the next visual row MUST be joined without any inserted character. Hard newlines (actual LF produced by the application or entered by the user) MUST produce a newline character (U+000A) in the copied text. Trailing space characters at the end of a soft-wrapped line (padding introduced by the terminal to fill the row width) MUST be stripped before joining. | Must |

**Acceptance criteria:**
- FS-SB-001: Two panes have independent scrollback histories.
- FS-SB-002: The scrollback size preference field accepts integer values in the range [100, 1,000,000]. Submitting 50 causes the input to display 100 (clamped). Submitting 2,000,000 causes the input to display 1,000,000 (clamped). The preferences UI displays a real-time memory estimate that updates as the user types. Setting the value to 100,000 and running `seq 1 100001` causes the first line to be evicted.
- FS-SB-003: Colored output in scrollback retains its colors when scrolled into view.
- FS-SB-004: Running tmux (which uses a partial scroll region for the status bar), scrolling in the tmux pane does not add the tmux status bar to TauTerm's scrollback.
- FS-SB-005: With vim open, scrolling in TauTerm does not navigate the scrollback.
- FS-SB-006: After exiting vim, all pre-vim scrollback content is accessible.
- FS-SB-009: While scrolled 50 lines into the scrollback, a command producing output does not move the viewport. The user's reading position is preserved.
- FS-SB-010: While scrolled 50 lines into the scrollback, pressing any key that sends input to the PTY causes the viewport to jump to the bottom instantly. No additional scroll action is required.
- FS-SB-011: In an 80-column pane, `echo "$(python3 -c "print('a'*200)")"` produces a single 200-character 'a' string that wraps across three visual rows. Selecting all three rows and copying yields a single unbroken 200-character string with no newlines. Running `printf "line1\nline2\n"` and copying both lines yields `line1\nline2\n` with two newlines preserved.

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
