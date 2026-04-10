<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — UI & Navigation

> Part of the [Functional Specifications](README.md). See also: [00-overview.md](00-overview.md), [01-terminal-emulation.md](01-terminal-emulation.md), [03-remote-ssh.md](03-remote-ssh.md), [04-config-system.md](04-config-system.md), [05-scope-constraints.md](05-scope-constraints.md)

---

## 3.3 FS-TAB: Multi-Tab Management

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
| FS-TAB-009 | When a new tab is created while the tab bar is in overflow (horizontal scroll) mode, the tab bar MUST automatically scroll to make the newly created tab fully visible, without requiring any manual scroll action from the user. | Must |

**Acceptance criteria:**
- FS-TAB-001: Opening 10 tabs results in 10 independent terminal sessions.
- FS-TAB-003: Both Ctrl+Shift+T and the UI button create a new tab.
- FS-TAB-004: Closing a tab with running processes triggers the confirmation dialog (FS-PTY-008).
- FS-TAB-005: Dragging a tab to a new position reorders it.
- FS-TAB-006: Running `printf "\033]0;Custom\007"` changes the tab title. Double-clicking the tab title makes it editable inline; typing a new name and pressing Enter sets the custom label. Right-clicking the tab shows a "Rename" option that achieves the same result. Clearing the label reverts to the process-driven title.

  > **Note — title resolution priority (highest first):** (1) user-defined label, (2) OSC 0/2 title set by the running process, (3) basename of the OSC 7 CWD (FS-VT-064), (4) foreground process name from `/proc/{pgid}/comm`, (5) empty string.
- FS-TAB-007: Output produced in a background tab causes a visible indicator on that tab's header.
- FS-TAB-009: With enough tabs open to trigger the scroll arrows, pressing Ctrl+Shift+T creates a new tab and the tab bar scrolls so that the new tab is fully visible without any user scroll action.

---

## 3.4 FS-PANE: Multi-Pane (Split View)

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-PANE-001 | Within a tab, the user MUST be able to split the view horizontally (top/bottom) and vertically (left/right). | Must |
| FS-PANE-002 | Each pane MUST host an independent PTY session. | Must |
| FS-PANE-003 | Panes MUST be resizable by the user (e.g., dragging the separator). | Must |
| FS-PANE-004 | The user MUST be able to close any pane. Closing the last pane in a tab MUST close the tab. | Must |
| FS-PANE-005 | The user MUST be able to navigate between panes via keyboard shortcuts (defaults defined in FS-KBD) and via mouse click. | Must |
| FS-PANE-006 | The active pane MUST be visually distinguishable from inactive panes. | Must |
| FS-PANE-007 | When a tab contains two or more panes and the `showPaneTitleBar` preference is enabled, each pane MUST display a slim title bar at its top showing the resolved title for that pane (same priority as FS-TAB-006). The title bar MUST NOT render when the tab has only one pane, or when the preference is disabled. The active pane's title bar MUST be visually distinguishable from inactive panes' title bars using typography and/or opacity only — color MUST NOT be the sole distinguishing property. The tab title MUST follow the active pane's resolved title, not the root pane's title, regardless of the `showPaneTitleBar` preference. | Should |
| FS-PANE-008 | The application MUST expose a `showPaneTitleBar` boolean preference (default: `true`) that controls whether the pane title bar is shown in multi-pane layouts. This preference MUST be persisted and MUST be accessible in the Appearance section of the preferences panel. | Should |

**Acceptance criteria:**
- FS-PANE-001: A horizontal split produces two panes stacked vertically, each with its own shell session.
- FS-PANE-003: Dragging the separator resizes both adjacent panes and triggers SIGWINCH in each.
- FS-PANE-004: Closing the only pane in a tab closes the tab.
- FS-PANE-005: A keyboard shortcut cycles focus between panes; clicking a pane gives it focus.
- FS-PANE-006: The focused pane has a visually distinct border or highlight.
- FS-PANE-007: With two panes open and `showPaneTitleBar: true`, both panes display a title bar showing their respective process titles. With `showPaneTitleBar: false`, no title bar is visible regardless of pane count. With a single pane, no title bar is visible regardless of the preference.
- FS-PANE-007: Running `printf "\033]0;MyTitle\007"` in the non-active pane: (a) that pane's title bar immediately shows "MyTitle"; (b) clicking that pane updates the tab title to "MyTitle".
- FS-PANE-007: The active pane's title bar is visually distinct from inactive panes' title bars (different opacity or font weight).
- FS-PANE-008: Toggling "Show pane title bar" in Appearance preferences persists across app restarts. Title bars appear/disappear immediately upon toggle.

---

## 3.5 FS-KBD: Keyboard Input Handling

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
| FS-KBD-013 | Shift+Enter is indistinguishable from Enter in standard xterm encoding (both = 0x0D). This is a known v1 limitation — see [Domain Constraints](05-scope-constraints.md#5-domain-constraints). | N/A (constraint) |

**Acceptance criteria:**
- FS-KBD-001: Pressing Ctrl+Shift+T opens a new tab; Ctrl+C sends 0x03 to the PTY.
- FS-KBD-002: Removing the Ctrl+Shift+T binding in preferences causes that key combination to be sent to the PTY.
- FS-KBD-003 (F2): Pressing F2 while a tab is active activates inline rename mode on that tab's title.
- FS-KBD-003 (pane shortcuts): Ctrl+Shift+D splits the active pane horizontally (left/right); Ctrl+Shift+E splits vertically (top/bottom); Ctrl+Shift+Right/Left/Up/Down navigates between panes; Ctrl+Shift+Q closes the active pane. All defaults match UXD.md §11.2.
- FS-KBD-005: Alt+A in bash triggers the expected readline shortcut (e.g., `Meta-a`).
- FS-KBD-007: In vim, arrow keys navigate; after `set t_ku=\eOA` (application mode), arrows still work.
- FS-KBD-011: Typing Chinese characters via an IME produces correct input in the terminal; the composition window tracks the cursor.

---

## 3.6 FS-CLIP: Clipboard Integration

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

## 3.9 FS-NOTIF: Activity Notifications

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

## 3.15 FS-UX: User Experience Cross-Cutting Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-UX-001 | Every user-visible error or warning message MUST (a) identify what happened in plain language, without raw system error codes as the primary text; (b) indicate what the user can do next, or explicitly state that no action is required; (c) display any technical detail (errno, exit code, system message) as a secondary, collapsible or visually subordinate element only. | Must |
| FS-UX-002 | On first launch, TauTerm MUST display a non-intrusive, non-blocking indication that right-clicking in the terminal area opens a context menu. This indication MUST disappear automatically after the user has performed a right-click in the terminal area at least once. It MUST NOT block or delay the user from using the terminal. | Must |
| FS-UX-003 | When a terminal pane becomes the active pane — on application launch, on new tab creation, or on tab switch — the terminal viewport MUST automatically receive keyboard focus without requiring a mouse click from the user. | Must |

**Acceptance criteria:**
- FS-UX-001: When the configured shell is not found, the message reads in terms the user can understand (e.g., "Shell not found: `/usr/local/bin/zsh`. TauTerm has fallen back to `/bin/sh`. You can update your shell in Preferences → Terminal Behavior."), not a raw system error string.
- FS-UX-001: When an SSH connection drops, the pane shows a human-readable reason (e.g., "Connection lost: server did not respond to keepalive. Click Reconnect to try again."), not a raw errno or SSH error code alone.
- FS-UX-002: On a fresh install, a hint referencing the right-click context menu is visible in the terminal area. After the user right-clicks once, the hint is gone and does not reappear on subsequent launches.
- FS-UX-003: On application launch, the user can type a command immediately without clicking the terminal area first.
- FS-UX-003: After Ctrl+Shift+T creates a new tab, the user can type immediately in the new session without clicking.
- FS-UX-003: After clicking a tab in the tab bar, the user can type immediately without clicking the terminal area.

---

## 3.16 FS-UX: Focus Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-UX-010 | The terminal viewport MUST be the permanent keyboard focus sink. Any mouse click on a toolbar element MUST preserve keyboard focus on the active terminal viewport. | Must |
| FS-UX-011 | Toolbar buttons that do not require keyboard input (SSH toggle, fullscreen toggle, tab bar scroll arrows, scroll-to-bottom button) MUST NOT capture DOM focus on mouse click. Implementation pattern: `onmousedown.preventDefault()`. | Must |
| FS-UX-012 | Tab bar keyboard navigation (Arrow keys within tablist): pressing Escape MUST return keyboard focus to the active terminal viewport without changing the active tab. | Must |
| FS-UX-013 | When a non-modal overlay or panel closes (search overlay, SSH connection manager panel), keyboard focus MUST automatically return to the active terminal viewport without additional user action. | Must |
| FS-UX-014 | Exception — elements that require keyboard input (search input, inline tab rename input, preferences panel fields, Bits UI dialog content) are permitted to temporarily capture focus. They MUST return focus to the active terminal viewport when dismissed. | Must |
| FS-UX-015 | In a multi-pane split layout, focus always returns to the pane identified as active (`activePaneId` of the current tab). There is no separate "last focused pane" tracking. | Must |

**Acceptance criteria:**
- FS-UX-010: Clicking the SSH toggle button in the toolbar does not remove keyboard focus from the terminal viewport; typing immediately after the click sends characters to the PTY.
- FS-UX-010: Clicking the fullscreen toggle does not require the user to click the terminal area to resume typing.
- FS-UX-011: After clicking a tab bar scroll arrow, `document.activeElement` remains the terminal viewport element (not the arrow button).
- FS-UX-012: With the tab bar focused via keyboard navigation, pressing Escape moves focus to the active terminal viewport and the active tab does not change.
- FS-UX-013: Closing the search overlay (Escape or close button) returns focus to the terminal viewport; the user can type immediately.
- FS-UX-013: Closing the SSH connection manager panel returns focus to the terminal viewport without a click.
- FS-UX-014: Typing in the search input field works normally. Pressing Escape in the search input closes the overlay and returns focus to the terminal viewport.
- FS-UX-014: After confirming or cancelling an inline tab rename (Enter or Escape), focus returns to the terminal viewport.
- FS-UX-014: Closing the preferences dialog returns focus to the terminal viewport.
- FS-UX-015: In a two-pane split, clicking a toolbar button while the left pane is active returns focus to the left pane (not the right pane). The `activePaneId` determines which pane receives focus.
