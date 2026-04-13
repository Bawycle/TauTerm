<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — Full-Screen Mode

> Part of the [Functional Specifications](README.md). See also: [00-overview.md](00-overview.md), [01-terminal-emulation.md](01-terminal-emulation.md), [02-ui-navigation.md](02-ui-navigation.md), [03-remote-ssh.md](03-remote-ssh.md), [04-config-system.md](04-config-system.md), [05-scope-constraints.md](05-scope-constraints.md)

---

## 3.17 FS-FULL: Full-Screen Mode

*Traces to: UR-3.4*

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-FULL-001 | TauTerm MUST support a full-screen mode in which the application window expands to fill the entire display and all window chrome (title bar, window decorations) is hidden. | Must |
| FS-FULL-002 | The user MUST be able to enter full-screen mode via the keyboard shortcut F11. This shortcut MUST be intercepted by TauTerm before any input is forwarded to the active PTY — F11 is consumed by the application and not transmitted to the running process. | Must |
| FS-FULL-003 | The user MUST be able to exit full-screen mode and return to the windowed state via the same F11 shortcut. Exiting MUST always be possible regardless of the state of the running process. | Must |
| FS-FULL-004 | In addition to the F11 keyboard shortcut, entering and exiting full-screen MUST be accessible via a discoverable UI action (e.g., a menu item or a visible control). The UI action MUST be reachable without requiring prior knowledge of any keyboard shortcut. | Must |
| FS-FULL-005 | The F11 full-screen shortcut MUST be user-configurable (per FS-KBD-002). Removing the binding makes F11 available to the PTY. | Should |
| FS-FULL-006 | After the full-screen transition completes, TauTerm MUST send a resize event (SIGWINCH) to all active PTY sessions so that each session's dimensions reflect the new terminal size. | Must |
| FS-FULL-007 | After exiting full-screen, TauTerm MUST send a resize event (SIGWINCH) to all active PTY sessions so that each session's dimensions reflect the restored window size. | Must |
| FS-FULL-008 | Entering or exiting full-screen mode MUST NOT affect application functionality: all tabs, panes, preferences, and SSH connections MUST remain fully accessible and functional throughout the transition and in both window states. | Must |
| FS-FULL-009 | The full-screen state (entered or exited at last application close) MUST be persisted in user preferences and restored on next launch. | Should |
| FS-FULL-010 | On Linux, where full-screen is implemented as a window manager hint and the transition may be asynchronous, TauTerm MUST send SIGWINCH only after the window geometry has stabilized (i.e., after the WM has acknowledged the full-screen state), not at the time the request is issued. | Must |
| FS-FULL-011 | The user SHOULD be able to choose between auto-hiding chrome (`autoHide`) and always-visible chrome (`alwaysVisible`) in full-screen mode via an appearance preference (`fullscreenChromeBehavior`). The default value is `autoHide`. When set to `alwaysVisible`, the tab bar and status bar remain visible and in the normal layout flow; hover zones and the exit badge are not rendered. | Should |

**Acceptance criteria:**

- FS-FULL-001: Pressing F11 causes the TauTerm window to occupy the entire display with no visible title bar or window border.
- FS-FULL-002: In a PTY session running `cat`, pressing F11 does not produce any character output — the event is consumed by TauTerm and not forwarded to the process.
- FS-FULL-003: Pressing F11 a second time while in full-screen mode restores the application to a normal window. This works even when a full-screen terminal application (e.g., `vim`) is running in the active pane.
- FS-FULL-004: A user who does not know the F11 shortcut can still enter and exit full-screen mode using a visible UI element (e.g., a menu item) without consulting documentation.
- FS-FULL-006 / FS-FULL-007: After entering full-screen, `tput cols` and `tput lines` in the active shell reflect the new terminal dimensions. The same holds after exiting full-screen.
- FS-FULL-008: With three tabs open and an active SSH session, entering full-screen mode: (a) does not close or disrupt any tab or pane; (b) does not disconnect the SSH session; (c) keeps the preferences panel accessible via Ctrl+,.
- FS-FULL-009: Closing TauTerm while in full-screen mode, then relaunching, opens the application in full-screen mode.
- FS-FULL-010: On a tiling window manager that applies full-screen asynchronously, the PTY dimensions reported by `tput cols` / `tput lines` match the full-screen geometry, not the pre-transition geometry.
- FS-FULL-011: (a) Setting `fullscreenChromeBehavior` to `alwaysVisible` and entering full-screen mode: the tab bar and status bar remain continuously visible; no hover zones are active; no exit badge is rendered. (b) Setting `fullscreenChromeBehavior` to `autoHide` and entering full-screen mode: the tab bar and status bar hide automatically; hovering the top/bottom edges recalls them; the exit badge is visible in the top-right corner. (c) The default value of the preference is `autoHide`.
