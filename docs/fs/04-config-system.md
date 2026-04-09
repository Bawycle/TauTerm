<!-- SPDX-License-Identifier: MPL-2.0 -->
# Functional Specifications — Configuration & System

> Part of the [Functional Specifications](README.md). See also: [00-overview.md](00-overview.md), [01-terminal-emulation.md](01-terminal-emulation.md), [02-ui-navigation.md](02-ui-navigation.md), [03-remote-ssh.md](03-remote-ssh.md), [05-scope-constraints.md](05-scope-constraints.md)

---

## 3.12 FS-THEME: Theming System

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-THEME-001 | TauTerm MUST ship with three built-in themes that reflect deliberate artistic directions: Umbra (default), Solstice (light), and Archipel (dark). Each built-in theme is permanently available. | Must |
| FS-THEME-002 | Built-in themes MUST NOT be deletable or modifiable. They MAY be overridden by a user-created theme set as active. | Must |
| FS-THEME-003 | The user MUST be able to create one or more custom themes. | Must |
| FS-THEME-004 | A theme MUST define at minimum: background color, foreground color, cursor color, selection color, and the 16 ANSI palette colors. | Must |
| FS-THEME-005 | A theme MAY also define: font family, font size, line height, border/panel colors, and UI accent colors. | May |
| FS-THEME-006 | The active theme MUST be switchable at any time from the preferences UI. | Must |
| FS-THEME-007 | User-created themes MUST be stored persistently alongside other user preferences. Built-in themes are shipped with the application and are not stored in the preferences file. | Must |
| FS-THEME-008 | The theming system MUST be based on design tokens (colors, spacing, sizing, radius). No hardcoded visual values are allowed in the UI layer. | Must |
| FS-THEME-009 | User-created themes MUST map to the same design tokens as the built-in themes, ensuring visual consistency across all UI surfaces. | Must |
| FS-THEME-010 | A user theme MAY override the terminal line height. UI chrome line height (tab bar, status bar, panels) is not themeable and is fixed by the design system. | Should |
| FS-THEME-011 | The three built-in themes (FS-THEME-001) are: Umbra (dark), Solstice (light, Nordic winter), and Archipel (dark, Caribbean). Each has a distinct artistic direction defined in `docs/AD.md` §7–9. | Must |
| FS-THEME-012 | Built-in themes MUST be listed separately from user-created themes in the theme preferences UI, under a distinct label (e.g. "Built-in"). | Must |
| FS-THEME-013 | On first launch (no saved preferences), the active theme MUST be Umbra. After first launch, the active theme is the last user-selected theme, whether built-in or user-created. | Must |

**Acceptance criteria:**
- FS-THEME-001: On first launch, TauTerm displays a polished built-in theme. All three built-in themes are selectable from the preferences UI.
- FS-THEME-002: No built-in theme has a "Delete" or "Edit" option in the UI.
- FS-THEME-003: The user can create a theme, name it, and switch to it.
- FS-THEME-004: A custom theme that changes only background and foreground colors applies correctly; ANSI palette is visible in `ls --color` output.
- FS-THEME-005: A user theme that defines `--font-terminal` applies the custom font to the terminal viewport; UI chrome (tab bar, status bar) retains the system font.
- FS-THEME-006: Switching themes in preferences applies the new theme without requiring a restart or any other user action; the change is visible on the next rendered frame.
- FS-THEME-008: No UI component uses hardcoded color or spacing values; all reference tokens.
- FS-THEME-009: A user-created theme is applied to the terminal viewport, tab bar, status bar, and all modal dialogs — no UI surface retains tokens from the previous theme.
- FS-THEME-010: A user theme that overrides the terminal line height causes the terminal to render with the new spacing. UI elements (tab bar, status bar, panels) are unaffected.
- FS-THEME-011: All three built-in themes (Umbra, Solstice, Archipel) are available on a fresh install with no user action.
- FS-THEME-012: The theme preferences UI shows built-in themes grouped under a "Built-in" label, distinct from the user-created themes list.
- FS-THEME-013: On first launch with no preferences file, the active theme is Umbra. After selecting Solstice, quitting, and relaunching, the active theme is Solstice.

---

## 3.13 FS-PREF: User Preferences

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-PREF-001 | User preferences MUST be persisted locally on disk and survive application restarts. | Must |
| FS-PREF-002 | A dedicated UI panel MUST allow the user to view and edit all preferences. | Must |
| FS-PREF-003 | Changes to preferences MUST be applied immediately without requiring a restart, where technically feasible. When a preference change requires a restart to take effect, the UI MUST inform the user. | Must |
| FS-PREF-004 | Preferences MUST be organized into logical sections. At minimum: Keyboard, Appearance, Terminal Behavior. | Must |
| FS-PREF-005 | The preferences UI MUST be accessible via a keyboard shortcut (default: Ctrl+,) and via a visible UI control. | Must |
| FS-PREF-006 | The following settings MUST be configurable: scrollback buffer size (with real-time memory estimate display per FS-SB-002), cursor blink rate, cursor shape, bell notification type, word delimiter set, font family, font size. | Must |

**Preference field behavioral constraints:**

The table below documents, for each configurable field, when its effect takes place and any architectural constraints that constrain that behavior. These are design-time commitments; implementation must conform to them.

| Field | Effect timing | Behavioral notes |
|---|---|---|
| `fontFamily` | Immediate | Applied via CSS variable update in the frontend. No backend involvement. |
| `fontSize` | Immediate | Applied via CSS variable update in the frontend. No backend involvement. |
| `themeName` | Immediate | Active theme tokens are reloaded in the frontend on change. |
| `opacity` | Immediate | Controls **terminal background transparency only** — text, UI chrome, and the window frame remain fully opaque. Applied via CSS variable update in the frontend. |
| `language` | Immediate | All visible UI strings switch locale without page reload or restart. The frontend reacts to the new `Language` enum value returned by `update_preferences`. |
| `cursorStyle` | Immediate | Sets the **default** cursor shape. Terminal applications may override it at any time via DECSCUSR escape sequences (e.g., `ESC [ 1 q` for blinking block, `ESC [ 5 q` for blinking bar). On a terminal hard reset, the shape reverts to the preference value. The preference is propagated to all currently open sessions at the time of the change. |
| `cursorBlinkMs` | Immediate | Applied to all currently open sessions at the time of the change. |
| `allowOsc52Write` | Immediate | Propagated to all currently open sessions at the time of the change via `SessionRegistry`. |
| `wordDelimiters` | Immediate | Takes effect on the next double-click selection. |
| `bellType` | Immediate | Takes effect on the next BEL character received. |
| `confirmMultilinePaste` | Immediate | Takes effect on the next paste operation. |
| `keyboard.bindings` | Immediate | Takes effect on the next keydown event. |
| `scrollbackLines` | New panes only | The scrollback buffer capacity is fixed at pane construction. Changing this preference does **not** resize existing pane buffers. This is a known architectural constraint of the `ScreenBuffer` design, not a deficiency. The UI MUST display a note to the user that the setting applies to new panes only. |
| `fullscreen` | Startup only | The preference value records whether the window should open in full-screen mode. It is restored at next application launch. It does not control the live window state directly — the live state is managed by the OS window manager. |
| `contextMenuHintShown` | Internal latch | One-shot flag. Once set to `true`, it is never reset to `false` by any user action. It is not exposed as an editable preference in the UI. |

**Acceptance criteria:**
- FS-PREF-001: Changing a preference, quitting, and relaunching TauTerm shows the preference retained.
- FS-PREF-002: All configurable settings are accessible from the preferences UI.
- FS-PREF-003: Changing font size in preferences immediately changes the terminal font size.
- FS-PREF-004: The preferences UI has labeled sections for Keyboard, Appearance, and Terminal Behavior.
- FS-PREF-005: Ctrl+, opens the preferences panel.

---

## 3.14 FS-A11Y: Accessibility

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-A11Y-001 | Color contrast MUST meet WCAG 2.1 AA standards: at least 4.5:1 for normal text, at least 3:1 for large text and UI components. | Must |
| FS-A11Y-002 | All interactive UI elements (buttons, tabs, inputs) MUST have a minimum touch/click target of 44x44 pixels. | Must |
| FS-A11Y-003 | All interactive UI elements MUST be navigable and operable via keyboard. | Must |
| FS-A11Y-004 | Information MUST NOT be conveyed by color alone. A secondary indicator (shape, icon, text, pattern) MUST supplement color-based distinctions. | Must |
| FS-A11Y-005 | All TauTerm UI features MUST be accessible via both keyboard and mouse, per the dual modality principle (UR 3.1). The terminal content area is excepted per UR 3.3. | Must |
| FS-A11Y-006 | A context menu MUST be available in the terminal area (e.g., right-click). It MUST expose at minimum: Copy, Paste, Search, and pane/tab management actions. This is the primary discoverability mechanism for users who do not know keyboard shortcuts. | Must |
| FS-A11Y-007 | During theme editing, the editor's own chrome (labels, controls, buttons, inputs) MUST always render using the active built-in theme's system tokens, not the custom theme being edited. If no built-in theme is active (a user-created theme is active), the editor chrome MUST fall back to Umbra system tokens. Only a designated preview area (terminal viewport sample) reflects the custom theme in real time. This ensures the editor remains accessible even when the user is authoring a non-compliant theme. | Must |

**Acceptance criteria:**
- FS-A11Y-001: All three built-in themes pass WCAG AA contrast checks for all text and UI components.
- FS-A11Y-002: No interactive element has a click target smaller than 44x44px.
- FS-A11Y-003: Tab key cycles through all interactive elements; Enter/Space activates them.
- FS-A11Y-004: Tab activity indicators use an icon or text badge in addition to color change.
- FS-A11Y-005: Every feature reachable by mouse is also reachable by keyboard (and vice versa, excluding PTY input).
- FS-A11Y-006: Right-clicking in the terminal area opens a context menu with Copy, Paste, Search, and split/close actions.
- FS-A11Y-007: A user creating a theme with foreground color identical to background color: the editor controls and labels remain fully legible (rendered using the active built-in theme tokens, or Umbra if no built-in theme is active). Only the preview terminal sample reflects the low-contrast custom theme.

---

## 3.16 FS-SEC: Security Hardening

| ID | Requirement | Priority |
|----|-------------|----------|
| FS-SEC-001 | The Tauri Content Security Policy MUST be configured to restrict resource loading. At minimum: `default-src 'self'`; `script-src 'self'`; `style-src 'self' 'unsafe-inline'` (to be tightened when feasible); `connect-src ipc: http://ipc.localhost`; `img-src 'self' asset: http://asset.localhost`. The CSP MUST NOT allow `script-src 'unsafe-inline'` or `script-src 'unsafe-eval'`. | Must |
| FS-SEC-002 | All PTY master file descriptors MUST be opened with the `O_CLOEXEC` flag (or equivalent `CLOEXEC` at creation time) to prevent file descriptor leaks to child processes spawned by the shell. | Must |
| FS-SEC-003 | The preferences file MUST be validated against a schema on load. Invalid entries MUST be replaced with defaults. File paths within preferences (identity file paths, shell path) MUST be validated: resolved to absolute paths, no path traversal, must reference existing regular files. The valid set of built-in theme IDs (`"umbra"`, `"solstice"`, `"archipel"`) is part of the validated schema for `appearance.theme_name`; user-created theme names are resolved dynamically against the loaded user theme list after validation (see architecture §8.1 — Unknown `theme_name` fallback). Numeric preference fields MUST be clamped to the following canonical ranges on load; values outside the range are silently clamped to the nearest bound (not rejected outright, to handle minor version drift): `font_size` [6.0, 72.0] pt; `cursor_blink_ms` [0, 5000] ms; `opacity` [0.0, 1.0]; `scrollback_lines` [100, 1 000 000] lines; `UserTheme.line_height` [1.0, 2.0]. | Must |
| FS-SEC-004 | SSH agent forwarding MUST NOT be supported in v1. | Must |
| FS-SEC-005 | Individual OSC and DCS sequences MUST be limited to 4096 bytes. Sequences exceeding this limit MUST be discarded. This prevents memory exhaustion from malicious or malformed sequences. | Must |

**Acceptance criteria:**
- FS-SEC-001: The WebView does not execute inline scripts. A `<script>` tag injected into the DOM via devtools is blocked by CSP.
- FS-SEC-002: A child process (e.g., `ls -la /proc/self/fd`) does not show open file descriptors belonging to other panes' PTY masters.
- FS-SEC-003: A preferences file with an invalid JSON structure or out-of-range values loads with defaults (or clamped values) applied; no crash occurs. Specific checks: `font_size: 200.0` is clamped to 72.0; `scrollback_lines: 0` is clamped to 100; `opacity: 1.5` is clamped to 1.0; `UserTheme.line_height: 5.0` is clamped to 2.0.
- FS-SEC-004: No SSH agent forwarding channel is opened during an SSH session.
- FS-SEC-005: A sequence of `\033]0;` followed by 10,000 characters does not consume unbounded memory; the sequence is discarded.

---

## 3.17 FS-I18N: Internationalisation

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
