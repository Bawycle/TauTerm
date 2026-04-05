# UI Test Protocol — ProcessTerminatedPane, ContextMenu, PreferencesPanel, ConnectionManager, SearchOverlay

> **Version:** 1.0.0
> **Date:** 2026-04-05
> **Status:** Approved
> **Covers:** Five UI components added in sprint 2026-04-05 session h
> **Input documents:** UXD.md §7.4, §7.6, §7.7, §7.8, §7.17, §7.18 — FS-PTY-005/006/008, FS-SEARCH-001..007, FS-PREF-001..006, FS-SSH-030..034, FS-TAB-006, FS-A11Y-003..006, FS-KBD-002/003

---

## Table of Contents

1. [Scope and Conventions](#1-scope-and-conventions)
2. [ProcessTerminatedPane](#2-processterminatedpane)
3. [ContextMenu](#3-contextmenu)
4. [PreferencesPanel](#4-preferencespanel)
5. [ConnectionManager](#5-connectionmanager)
6. [SearchOverlay](#6-searchoverlay)
7. [Cross-Component Accessibility Audit](#7-cross-component-accessibility-audit)
8. [i18n Coverage](#8-i18n-coverage)
9. [Security Scenarios (summary)](#9-security-scenarios-summary)

---

## 1. Scope and Conventions

### 1.1 Identifier scheme

Scenarios follow the pattern `UITCP-<COMPONENT>-<CATEGORY>-<NNN>` where:

- `COMPONENT`: `PTP` (ProcessTerminatedPane), `CTX` (ContextMenu), `PREF` (PreferencesPanel), `CM` (ConnectionManager), `SO` (SearchOverlay)
- `CATEGORY`: `FN` (functional), `UX` (user-experience / visual), `A11Y` (accessibility), `I18N` (internationalisation), `SEC` (security — cross-reference to security protocol)

### 1.2 Test method codes

| Code | Meaning |
|------|---------|
| `UNIT` | Vitest unit test — isolated, no DOM rendering |
| `COMP` | Vitest component test — `@testing-library/svelte` rendering |
| `E2E` | WebdriverIO end-to-end (full Tauri stack) |
| `E2E-DEFERRED` | Scenario identified but blocked on current E2E infrastructure |
| `MANUAL` | Manual verification required |

### 1.3 Priority

`P0` = release blocker · `P1` = high · `P2` = medium · `P3` = low

---

## 2. ProcessTerminatedPane

Component: `src/lib/components/ProcessTerminatedPane.svelte`
FS references: FS-PTY-005, FS-PTY-006, FS-PTY-008

### 2.1 Functional Scenarios

#### UITCP-PTP-FN-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-FN-001 |
| **Description** | Banner renders with exit code 0 (success state) |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount `ProcessTerminatedPane` with props `exitCode={0}`. |
| **Expected** | Banner is visible. CheckCircle icon is rendered. Text "Process exited" is present. No error color applied. |
| **FS** | FS-PTY-005, FS-PTY-006 |

#### UITCP-PTP-FN-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-FN-002 |
| **Description** | Banner renders with non-zero exit code (error state) |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `exitCode={1}`. |
| **Expected** | XCircle icon rendered. Text contains "code 1". Error color token applied to icon. |
| **FS** | FS-PTY-005 |

#### UITCP-PTP-FN-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-FN-003 |
| **Description** | Banner renders with exit code 127 (command not found) |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Steps** | Mount with `exitCode={127}`. |
| **Expected** | Text contains "code 127". |
| **FS** | FS-PTY-005 |

#### UITCP-PTP-FN-004

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-FN-004 |
| **Description** | "Restart" button emits restart event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount component. Click "Restart" button. |
| **Expected** | `restart` event is emitted. |
| **FS** | FS-PTY-006 |

#### UITCP-PTP-FN-005

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-FN-005 |
| **Description** | "Close" button emits close event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount component. Click "Close" button. |
| **Expected** | `close` event is emitted. |
| **FS** | FS-PTY-006 |

#### UITCP-PTP-FN-006

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-FN-006 |
| **Description** | Banner does not auto-close after render |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `exitCode={0}`. Wait 2 seconds. |
| **Expected** | Banner remains visible. No auto-close. |
| **FS** | FS-PTY-005 ("The pane MUST NOT auto-close") |

#### UITCP-PTP-FN-007

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-FN-007 |
| **Description** | Signal name displayed for signal-killed process |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Steps** | Mount with `exitCode={137}`, `signalName="SIGKILL"`. |
| **Expected** | "SIGKILL" or equivalent signal info shown in secondary text below the exit code. |
| **FS** | FS-PTY-005, UXD §7.18 |

### 2.2 UX / Visual Scenarios

#### UITCP-PTP-UX-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-UX-001 |
| **Description** | Banner uses correct design tokens |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Background `--color-bg-surface`, border-top `--color-border`. No hardcoded color values. |
| **UXD** | §7.18 |

#### UITCP-PTP-UX-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-UX-002 |
| **Description** | Minimum height 44px met |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Rendered height >= 44px. |
| **UXD** | §7.18 |

### 2.3 Accessibility Scenarios

#### UITCP-PTP-A11Y-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-A11Y-001 |
| **Description** | Restart and Close buttons are keyboard-focusable |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Both buttons have `tabindex` (default or explicit). Pressing Tab cycles to each. |
| **FS** | FS-A11Y-003 |

#### UITCP-PTP-A11Y-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-A11Y-002 |
| **Description** | Icon is decorative (aria-hidden) and text carries the meaning |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Lucide icon has `aria-hidden="true"`. Exit status conveyed by text, not icon alone. |
| **FS** | FS-A11Y-004 |

### 2.4 i18n Scenarios

#### UITCP-PTP-I18N-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PTP-I18N-001 |
| **Description** | All visible strings use i18n message accessors |
| **Method** | `UNIT` (static scan) |
| **Priority** | P1 |
| **Expected** | No hardcoded English strings in component template. All labels via `m.*()` accessors. |

---

## 3. ContextMenu

Component: `src/lib/components/ContextMenu.svelte`
FS references: FS-A11Y-006, FS-TAB-006, FS-SEARCH-007, FS-UX-002

### 3.1 Functional Scenarios — Terminal Variant

#### UITCP-CTX-FN-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-001 |
| **Description** | Terminal context menu renders all required items |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount `ContextMenu` with `variant="terminal"`. |
| **Expected** | Items: Copy, Paste, Search, Split Top/Bottom, Split Left/Right, Close Pane. |
| **FS** | FS-A11Y-006 |

#### UITCP-CTX-FN-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-002 |
| **Description** | "Copy" item is disabled when no text is selected |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `variant="terminal"`, `hasSelection={false}`. |
| **Expected** | Copy item has `aria-disabled="true"` or `disabled` attribute. |
| **UXD** | §7.8.1 |

#### UITCP-CTX-FN-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-003 |
| **Description** | "Copy" item is enabled when text is selected |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `variant="terminal"`, `hasSelection={true}`. |
| **Expected** | Copy item is not disabled. |
| **UXD** | §7.8.1 |

#### UITCP-CTX-FN-004

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-004 |
| **Description** | "Close Pane" is omitted when only one pane exists |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Steps** | Mount with `variant="terminal"`, `canClosePane={false}`. |
| **Expected** | No "Close Pane" item in rendered output. |
| **UXD** | §7.8.1 |

#### UITCP-CTX-FN-005

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-005 |
| **Description** | Clicking "Copy" emits copy event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `variant="terminal"`, `hasSelection={true}`. Click Copy. |
| **Expected** | `copy` event emitted. |

#### UITCP-CTX-FN-006

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-006 |
| **Description** | Clicking "Paste" emits paste event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click Paste. |
| **Expected** | `paste` event emitted. |

#### UITCP-CTX-FN-007

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-007 |
| **Description** | Clicking "Search" emits search event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click Search. |
| **Expected** | `search` event emitted. |

#### UITCP-CTX-FN-008

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-008 |
| **Description** | Pressing Escape closes the menu |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Open menu. Press Escape. |
| **Expected** | Menu is not visible in DOM (or `open=false`). |
| **UXD** | §7.8 (standard dropdown behavior) |

### 3.2 Functional Scenarios — Tab Variant

#### UITCP-CTX-FN-010

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-010 |
| **Description** | Tab context menu renders all required items |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `variant="tab"`. |
| **Expected** | Items: New Tab, Rename, Split Top/Bottom, Split Left/Right, Close Tab. |
| **FS** | FS-TAB-006 |

#### UITCP-CTX-FN-011

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-011 |
| **Description** | Clicking "Rename" emits rename event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `variant="tab"`. Click Rename. |
| **Expected** | `rename` event emitted. |
| **FS** | FS-TAB-006 |

#### UITCP-CTX-FN-012

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-012 |
| **Description** | Clicking "New Tab" emits new-tab event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click New Tab. |
| **Expected** | `newTab` event emitted. |

#### UITCP-CTX-FN-013

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-FN-013 |
| **Description** | Clicking "Close Tab" emits close-tab event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click Close Tab. |
| **Expected** | `closeTab` event emitted. |

### 3.3 UX / Visual Scenarios

#### UITCP-CTX-UX-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-UX-001 |
| **Description** | Context menu uses correct design tokens |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Background `--color-bg-raised`, border `--color-border`, radius `--radius-md`, min-width 180px, max-width 280px. |
| **UXD** | §7.8.3 |

#### UITCP-CTX-UX-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-UX-002 |
| **Description** | Each menu item height >= 44px |
| **Method** | `COMP` |
| **Priority** | P1 |
| **FS** | FS-A11Y-002 |

#### UITCP-CTX-UX-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-UX-003 |
| **Description** | Separators render between groups |
| **Method** | `COMP` |
| **Priority** | P2 |
| **Expected** | `hr` or equivalent separator elements present between Copy/Paste, Search, and Split groups. |
| **UXD** | §7.8.1 |

### 3.4 Accessibility Scenarios

#### UITCP-CTX-A11Y-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-A11Y-001 |
| **Description** | Menu root has role="menu" |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Root element has `role="menu"`. |
| **FS** | FS-A11Y-003; UXD §7.8.3 |

#### UITCP-CTX-A11Y-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-A11Y-002 |
| **Description** | Each item has role="menuitem" |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Each actionable item has `role="menuitem"`. |

#### UITCP-CTX-A11Y-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-CTX-A11Y-003 |
| **Description** | Arrow key navigation moves focus between items |
| **Method** | `E2E-DEFERRED` |
| **Priority** | P1 |
| **Note** | Blocked on Bits UI DropdownMenu portal interaction in JSDOM. |

---

## 4. PreferencesPanel

Component: `src/lib/components/PreferencesPanel.svelte`
FS references: FS-PREF-001..006, FS-KBD-002/003, FS-I18N-004/005/006

### 4.1 Functional Scenarios

#### UITCP-PREF-FN-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-001 |
| **Description** | Panel renders with four section tabs |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount `PreferencesPanel` with `open={true}` and mock preferences. |
| **Expected** | Section navigation shows: Keyboard, Appearance, Terminal Behavior, Connections. |
| **FS** | FS-PREF-004 |

#### UITCP-PREF-FN-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-002 |
| **Description** | Clicking a section nav item switches to that section |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click "Appearance" in section nav. |
| **Expected** | Appearance section content is visible. Keyboard section content is hidden. |

#### UITCP-PREF-FN-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-003 |
| **Description** | Appearance section renders font family, font size, language dropdown |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Navigate to Appearance section. |
| **Expected** | Font family input, font size input, and language dropdown are present. |
| **FS** | FS-PREF-006 |

#### UITCP-PREF-FN-004

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-004 |
| **Description** | Terminal Behavior section renders all required controls |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Navigate to Terminal Behavior section. |
| **Expected** | Cursor shape dropdown, cursor blink input, scrollback size input with memory estimate, bell type dropdown, word delimiter input all present. |
| **FS** | FS-PREF-006 |

#### UITCP-PREF-FN-005

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-005 |
| **Description** | Scrollback size field shows memory estimate |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Navigate to Terminal Behavior. Set scrollback to 10000. |
| **Expected** | Helper text below field shows a memory estimate (e.g., "~X MB per pane"). |
| **FS** | FS-SB-002, FS-PREF-006 |

#### UITCP-PREF-FN-006

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-006 |
| **Description** | Language dropdown contains English and Français options |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Navigate to Appearance. Inspect language dropdown. |
| **Expected** | Options include "English" and "Français". |
| **FS** | FS-I18N, UXD §7.6.3 |

#### UITCP-PREF-FN-007

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-007 |
| **Description** | Selecting a language emits update event with correct Language enum value |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Select "Français" from language dropdown. |
| **Expected** | `updatePreferences` called with `{ appearance: { language: 'Fr' } }`. |
| **FS** | FS-I18N-006 (MUST be enum, never free string) |

#### UITCP-PREF-FN-008

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-008 |
| **Description** | Escape closes the panel |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `open={true}`. Press Escape. |
| **Expected** | `close` event emitted or `open` prop becomes false. |
| **UXD** | §7.6.2 |

#### UITCP-PREF-FN-009

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-009 |
| **Description** | Keyboard section renders shortcut recorder rows |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Navigate to Keyboard section. |
| **Expected** | At least one KeyboardShortcutRecorder row rendered per configurable shortcut (FS-KBD-003 default set). |
| **FS** | FS-KBD-002, FS-PREF-004 |

#### UITCP-PREF-FN-010

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-010 |
| **Description** | KeyboardShortcutRecorder: click to enter Recording state |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click a shortcut recorder field. |
| **Expected** | Field enters Recording state: border becomes `--color-accent`, placeholder changes to "Press keys..." text. |
| **UXD** | §7.17 |

#### UITCP-PREF-FN-011

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-011 |
| **Description** | KeyboardShortcutRecorder: Escape while recording cancels and reverts |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click recorder. Press Escape. |
| **Expected** | Field returns to Inactive state. Previous shortcut value displayed. |
| **UXD** | §7.17 |

#### UITCP-PREF-FN-012

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-012 |
| **Description** | KeyboardShortcutRecorder: key capture and confirmation |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click recorder. Press Ctrl+Shift+X. Press Enter. |
| **Expected** | Field shows "Ctrl+Shift+X". `shortcutChanged` event emitted. |
| **UXD** | §7.17 |

#### UITCP-PREF-FN-013

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-FN-013 |
| **Description** | KeyboardShortcutRecorder: conflict detection |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount recorder with existing shortcuts. Enter a shortcut already assigned to another action. |
| **Expected** | Conflict state shown: border `--color-error`, conflicting action name displayed below field. Cannot confirm until conflict resolved. |
| **UXD** | §7.17 |

### 4.2 UX / Visual Scenarios

#### UITCP-PREF-UX-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-UX-001 |
| **Description** | Panel uses correct layout tokens |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Width 640px (`--size-preferences-panel-width`), max-height 80vh, background `--color-bg-raised`, border `--color-border`, radius `--radius-md`, z-index `--z-overlay` (40). |
| **UXD** | §7.6.1 |

#### UITCP-PREF-UX-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-UX-002 |
| **Description** | Active section has left border accent |
| **Method** | `COMP` |
| **Priority** | P2 |
| **Expected** | Active section nav item has `border-left: 2px solid var(--color-accent)`. |
| **UXD** | §7.6.2 |

### 4.3 Accessibility Scenarios

#### UITCP-PREF-A11Y-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-A11Y-001 |
| **Description** | Focus is trapped within the panel when open |
| **Method** | `E2E-DEFERRED` |
| **Priority** | P0 |
| **Note** | Focus trap requires browser-level Tab cycling. Bits UI Dialog provides this natively. |

#### UITCP-PREF-A11Y-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-A11Y-002 |
| **Description** | Panel has role="dialog" and aria-modal="true" |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Root dialog element has `role="dialog"` and `aria-modal="true"`. |

#### UITCP-PREF-A11Y-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-A11Y-003 |
| **Description** | All form controls have associated labels |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Every input/dropdown/toggle has a `<label>` or `aria-label`. |
| **FS** | FS-A11Y-003 |

### 4.4 i18n Scenarios

#### UITCP-PREF-I18N-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-I18N-001 |
| **Description** | Panel title and all section headings use i18n accessors |
| **Method** | `UNIT` (static scan) |
| **Priority** | P1 |

#### UITCP-PREF-I18N-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-PREF-I18N-002 |
| **Description** | Language enum value sent to backend is 'En' or 'Fr', never a locale string |
| **Method** | `COMP` |
| **Priority** | P0 |
| **FS** | FS-I18N-006 |

---

## 5. ConnectionManager

Component: `src/lib/components/ConnectionManager.svelte`
FS references: FS-SSH-030..034, FS-CRED-001..006

### 5.1 Functional Scenarios

#### UITCP-CM-FN-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-001 |
| **Description** | Connection list renders saved connections |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with mock connections list. |
| **Expected** | Each connection item shows label/host:port and user@host. |
| **FS** | FS-SSH-031 |

#### UITCP-CM-FN-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-002 |
| **Description** | Empty state renders when no connections |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Steps** | Mount with empty connections array. |
| **Expected** | A clear empty-state message is displayed. |

#### UITCP-CM-FN-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-003 |
| **Description** | "New Connection" button opens edit form |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click "New Connection" button. |
| **Expected** | Edit form becomes visible (inline or sub-panel). List becomes hidden. |
| **FS** | FS-SSH-033 |

#### UITCP-CM-FN-004

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-004 |
| **Description** | Edit form has all required fields |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Open edit form. |
| **Expected** | Fields: Label (optional), Group (optional), Host (required), Port (default 22), Username (required), Auth method radio, identity file or password field. |
| **FS** | FS-SSH-030 |

#### UITCP-CM-FN-005

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-005 |
| **Description** | Saving a new connection emits save-connection event with correct data |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Fill form: Host=example.com, Port=22, Username=user. Click Save. |
| **Expected** | `saveConnection` event emitted with `{ host: 'example.com', port: 22, username: 'user', ... }`. |
| **FS** | FS-SSH-030 |

#### UITCP-CM-FN-006

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-006 |
| **Description** | Cancel button returns to list without saving |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Open edit form. Click Cancel. |
| **Expected** | List is shown again. No save event emitted. |

#### UITCP-CM-FN-007

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-007 |
| **Description** | Edit action on connection item populates form with existing values |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Hover connection item. Click Edit. |
| **Expected** | Edit form shown with existing host, port, username pre-filled. |
| **FS** | FS-SSH-033 |

#### UITCP-CM-FN-008

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-008 |
| **Description** | Delete action emits delete-connection event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Hover connection item. Click Delete. Confirm in dialog. |
| **Expected** | `deleteConnection` event emitted with connection id. |
| **FS** | FS-SSH-033 |

#### UITCP-CM-FN-009

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-009 |
| **Description** | "Open in new tab" action emits open-connection event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Hover connection item. Click ExternalLink icon. |
| **Expected** | `openConnection` event emitted with `{ connectionId, target: 'tab' }`. |
| **FS** | FS-SSH-032 |

#### UITCP-CM-FN-010

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-010 |
| **Description** | Port field default is 22 |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Steps** | Open new connection form. |
| **Expected** | Port field shows value 22. |

#### UITCP-CM-FN-011

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-011 |
| **Description** | Password field is type="password" (masked) |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Select "Password" auth method. |
| **Expected** | Password input has `type="password"`. Value not visible as plain text. |
| **FS** | FS-CRED-001, UITCP-SEC |

#### UITCP-CM-FN-012

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-FN-012 |
| **Description** | Connection groups render as collapsible sections |
| **Method** | `COMP` |
| **Priority** | P2 |
| **Steps** | Mount with connections having different group values. |
| **Expected** | Connections grouped by `group` field under collapsible section headings. |
| **UXD** | §7.7.3 |

### 5.2 Accessibility Scenarios

#### UITCP-CM-A11Y-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-A11Y-001 |
| **Description** | All action buttons have aria-label or visible text |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Icon-only buttons (Open, Edit, Duplicate, Delete) have `aria-label` or `title`. |
| **FS** | FS-A11Y-003 |

#### UITCP-CM-A11Y-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-CM-A11Y-002 |
| **Description** | Connection list items are keyboard-navigable |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Items reachable via Tab. Action buttons within each item keyboard-accessible. |

---

## 6. SearchOverlay

Component: `src/lib/components/SearchOverlay.svelte`
FS references: FS-SEARCH-001..007
Implementation note: Uses `search_pane` IPC (Rust backend — implemented). Frontend renders match highlights on top of terminal grid.

### 6.1 Functional Scenarios

#### UITCP-SO-FN-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-001 |
| **Description** | Overlay renders in correct position |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount `SearchOverlay` with `open={true}`. |
| **Expected** | Overlay visible, positioned top-right, z-index 20, background `--color-bg-raised`. |
| **UXD** | §7.4.1 |

#### UITCP-SO-FN-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-002 |
| **Description** | Search input has correct placeholder |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Input placeholder: "Search..." in `--color-text-tertiary`. |
| **UXD** | §7.4.2 |

#### UITCP-SO-FN-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-003 |
| **Description** | Typing triggers search event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Type "error" in search input. |
| **Expected** | `search` event emitted with `{ text: 'error', caseSensitive: false, regex: false }`. |
| **FS** | FS-SEARCH-003 |

#### UITCP-SO-FN-004

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-004 |
| **Description** | Match count displays "N of M" format |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `matchCount={42}`, `currentMatch={3}`. |
| **Expected** | Text "3 of 42" displayed in match count area. |
| **UXD** | §7.4.2 |

#### UITCP-SO-FN-005

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-005 |
| **Description** | Match count displays "No matches" when 0 results |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Mount with `matchCount={0}`. |
| **Expected** | Text "No matches" displayed. |
| **UXD** | §7.4.2 |

#### UITCP-SO-FN-006

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-006 |
| **Description** | Next button emits next event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click Next (ChevronDown) button. |
| **Expected** | `next` event emitted. |

#### UITCP-SO-FN-007

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-007 |
| **Description** | Previous button emits prev event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click Prev (ChevronUp) button. |
| **Expected** | `prev` event emitted. |

#### UITCP-SO-FN-008

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-008 |
| **Description** | Close button emits close event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Click X button. |
| **Expected** | `close` event emitted. |

#### UITCP-SO-FN-009

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-009 |
| **Description** | Escape key emits close event |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Focus search input. Press Escape. |
| **Expected** | `close` event emitted. |
| **UXD** | §7.4.4 |

#### UITCP-SO-FN-010

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-010 |
| **Description** | Enter key navigates to next match |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Focus search input. Press Enter. |
| **Expected** | `next` event emitted. |
| **UXD** | §7.4.4 |

#### UITCP-SO-FN-011

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-011 |
| **Description** | Shift+Enter navigates to previous match |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Steps** | Press Shift+Enter. |
| **Expected** | `prev` event emitted. |
| **UXD** | §7.4.4 |

#### UITCP-SO-FN-012

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-FN-012 |
| **Description** | Match count area has min-width to prevent layout shift |
| **Method** | `COMP` |
| **Priority** | P2 |
| **Expected** | Match count element has `min-width: 64px` or equivalent. |
| **UXD** | §7.4.2 |

### 6.2 UX / Visual Scenarios

#### UITCP-SO-UX-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-UX-001 |
| **Description** | Overlay uses correct design tokens |
| **Method** | `COMP` |
| **Priority** | P1 |
| **Expected** | Background `--color-bg-raised`, border `--color-border`, radius `--radius-md`, shadow `--shadow-raised`. |

### 6.3 Accessibility Scenarios

#### UITCP-SO-A11Y-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-A11Y-001 |
| **Description** | Overlay has role="search" |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Container element has `role="search"`. |
| **UXD** | §7.4.1 |

#### UITCP-SO-A11Y-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-A11Y-002 |
| **Description** | Search input has accessible label |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | Input has `aria-label="Search"` or associated `<label>`. |

#### UITCP-SO-A11Y-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-A11Y-003 |
| **Description** | Next and previous buttons have aria-labels |
| **Method** | `COMP` |
| **Priority** | P0 |
| **Expected** | ChevronUp button: `aria-label="Previous match"`. ChevronDown button: `aria-label="Next match"`. |

#### UITCP-SO-A11Y-004

| Field | Value |
|-------|-------|
| **ID** | UITCP-SO-A11Y-004 |
| **Description** | Prev/Next button hit areas >= 44px |
| **Method** | `COMP` |
| **Priority** | P1 |
| **FS** | FS-A11Y-002; UXD §7.4.2 |

---

## 7. Cross-Component Accessibility Audit

#### UITCP-XCOMP-A11Y-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-XCOMP-A11Y-001 |
| **Description** | No hardcoded color values in any new component |
| **Method** | `UNIT` (static scan) |
| **Priority** | P0 |
| **Expected** | grep for `#[0-9a-fA-F]{3,6}` or `rgb(` in component files returns 0 results (excluding comments and test files). |

#### UITCP-XCOMP-A11Y-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-XCOMP-A11Y-002 |
| **Description** | No `{@html}` in any new component |
| **Method** | `UNIT` (static scan) |
| **Priority** | P0 |
| **Expected** | Grep for `{@html` in new component files returns 0 results. |
| **FS** | FS-SEC; UIBC-SEC-014 |

#### UITCP-XCOMP-A11Y-003

| Field | Value |
|-------|-------|
| **ID** | UITCP-XCOMP-A11Y-003 |
| **Description** | All interactive elements have minimum 44×44px touch target |
| **Method** | `MANUAL` / `COMP` |
| **Priority** | P1 |
| **FS** | FS-A11Y-002 |

---

## 8. i18n Coverage

#### UITCP-I18N-001

| Field | Value |
|-------|-------|
| **ID** | UITCP-I18N-001 |
| **Description** | All user-visible strings in new components have EN and FR translations in message catalogues |
| **Method** | `UNIT` |
| **Priority** | P0 |
| **Steps** | Parse `src/lib/i18n/messages/en.json` and `fr.json`. Verify that all keys used by new components are present in both files. |

#### UITCP-I18N-002

| Field | Value |
|-------|-------|
| **ID** | UITCP-I18N-002 |
| **Description** | Language enum sent over IPC is always 'En' or 'Fr' |
| **Method** | `UNIT` |
| **Priority** | P0 |
| **FS** | FS-I18N-006 |

---

## 9. Security Scenarios (summary)

Security scenarios for the new components are detailed in `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md` §2.8 (added in sprint session h). Key identifiers:

| SEC ID | Component | Threat |
|--------|-----------|--------|
| SEC-UI-001 | ConnectionManager | Hostname/username field injection (XSS attempt) |
| SEC-UI-002 | ConnectionManager | Password field not stored in component state |
| SEC-UI-003 | SearchOverlay | Regex ReDoS via crafted search pattern |
| SEC-UI-004 | PreferencesPanel | Font family/size values validated before IPC send |
| SEC-UI-005 | ContextMenu | No clipboard content shown in menu (clipboard not read until Paste action) |
| SEC-UI-006 | ProcessTerminatedPane | Exit code rendered as number, not interpolated HTML |
