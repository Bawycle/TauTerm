<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Traceability Matrix & IPC Contract

> Part of the [UX/UI Design](README.md).

---

## 14. Traceability Matrix

This table maps major UX/UI decisions in this document to their source requirements in UR.md and FS.md.

| UXD Section | Decision | UR Source | FS Source |
|-------------|----------|-----------|-----------|
| [§2.1](01-principles.md#21-terminal-content-is-primary) | Terminal content is primary; chrome at lower visual temperature | UR §2.1 (Alex — low-distraction interface) | — |
| [§2.2](01-principles.md#22-every-feature-is-reachable-by-both-modalities) | Every feature reachable by mouse and keyboard | UR §3.1 (dual modality) | FS-A11Y-005 |
| [§2.3](01-principles.md#23-status-is-honest-and-immediate) | Status is honest and immediate | UR §2.2 (Jordan — at-a-glance status) | FS-SSH-022, FS-A11Y-004 |
| [§2.4](01-principles.md#24-sensible-defaults-zero-required-configuration) | Sensible defaults, zero required configuration | UR §2.3 (Sam — no config for basic use) | — |
| [§3](02-tokens.md#3-design-token-system) | Complete design token system | UR §8.3 (design tokens) | FS-THEME-008 |
| [§3.1](02-tokens.md#31-color-tokens--primitives) | Primitive tokens use `--color-` prefix | — | Tailwind 4 namespace collision avoidance |
| [§4.1](02-tokens.md#41-terminal-font) | Terminal font stack with JetBrains Mono primary | UR §8.2 (font family in themes) | FS-PREF-006 (font configurable) |
| [§5.3](02-tokens.md#53-ansi-16-color-palette) | ANSI 16-color palette with 4.5:1+ contrast | UR §8.2 (ANSI palette in themes) | FS-VT-020, FS-VT-023 |
| [§5.4](02-tokens.md#54-accessibility-compliance) | All pairings meet WCAG 2.1 AA | — | FS-A11Y-001 |
| [§6.1](02-tokens.md#61-window-anatomy) | Window anatomy: tab bar, terminal area, status bar | UR §4.1 (multi-tab), UR §4.2 (panes) | FS-TAB-001, FS-PANE-001 |
| [§6.4](02-tokens.md#64-status-bar-region) | Settings button in status bar | UR §3.1 (dual modality — mouse access to settings) | FS-PREF-005 |
| [§6.5](02-tokens.md#65-pane-dividers) | Pane divider: 1px line, 8px hit area | UR §4.2 (pane resize) | FS-PANE-003 |
| [§6.6](02-tokens.md#66-active-pane-indication) | Active pane: 2px blue border | UR §4.2 (pane navigation) | FS-PANE-006 |
| [§6.8](02-tokens.md#68-minimum-and-default-window-size) | Minimum window 640x400 | — | FS-A11Y-002 (target sizes) |
| [§7.1](03-components.md#71-tab-bar) | Tab bar with activity indicators | UR §4.1 (activity notification) | FS-TAB-007, FS-NOTIF-001-004 |
| [§7.1.2](03-components.md#712-tab-item) | Tab reorder via drag | UR §4.1 (tabs reorderable) | FS-TAB-005 |
| [§7.1.3](03-components.md#713-tab-activity-indicators) | Distinct indicators for output, process end, bell; aggregated scroll arrow badges | UR §4.1 (activity notification) | FS-NOTIF-001, FS-NOTIF-002, FS-NOTIF-004 |
| [§7.1.6](03-components.md#716-tab-inline-rename) | Inline rename on double-click, F2, and context menu | UR §4.1 (configurable title) | FS-TAB-006 |
| [§7.1.7](03-components.md#717-ssh-badge-on-tab) | SSH badge on tab with lifecycle states (incl. Closed) | UR §9.1 (at-a-glance SSH distinction) | FS-SSH-002, FS-SSH-010 |
| [§7.2.1](03-components.md#721-pane-activity-indicators-inactive-panes) | Pane activity indicators (border pulse) | UR §4.1 (activity notification for panes) | FS-NOTIF-001, FS-NOTIF-004 |
| [§7.3.1](03-components.md#731-cursor-styles) | Cursor styles: block, underline, bar (steady/blinking) | — | FS-VT-030, FS-VT-032 |
| [§7.3.1](03-components.md#731-cursor-styles) | Unfocused cursor as hollow outline | — | FS-VT-034 |
| [§7.4](03-components.md#74-search-overlay) | Search overlay with match highlighting | UR §7.2 (search in output) | FS-SEARCH-001-007 |
| [§7.5.2](03-components.md#752-disconnection-overlay-in-pane) | SSH disconnection banner with reconnect CTA and reconnecting state | UR §9.1 (clear notification) | FS-SSH-022, FS-SSH-041 |
| [§7.6](03-components.md#76-preferences-panel) | Preferences panel with sections | UR §5.2 (preferences UI) | FS-PREF-002, FS-PREF-004 |
| [§7.6.3](03-components.md#763-preference-sections-fs-pref-004) | Connections section: inline view in Preferences | UR §9.2 (saved connections accessible from preferences) | FS-SSH-030-034 |
| [§7.7](03-components.md#77-connection-manager) | Connection manager (standalone) with grouped list and CRUD operations | UR §9.2 (saved connections) | FS-SSH-030-034 |
| [§7.8](03-components.md#78-context-menu) | Context menu in terminal area | UR §3.1 (dual modality), UR §3.2 (discoverability) | FS-A11Y-006 |
| [§7.9.3](03-components.md#793-destructive-confirmation-dialog) | Destructive confirmation with safe default | UR §2.2 (Jordan — no silent failures) | FS-PTY-008 |
| [§7.9.4](03-components.md#794-ssh-host-key-verification-dialog-fs-ssh-011) | SSH host key verification dialogs; MITM uses ShieldAlert in error color | UR §9.3 (security) | FS-SSH-011 |
| [§7.13](03-components.md#713-first-launch-context-menu-hint-fs-ux-002) | First-launch context menu hint | UR §2.3 (Sam — discoverability) | FS-UX-002 |
| [§7.14](03-components.md#714-button-variants) | Button variants (primary, secondary, ghost, destructive) | UR §3.1 (dual modality — visible controls) | — |
| [§7.17](03-components.md#717-keyboard-shortcut-recorder) | Keyboard shortcut recorder with conflict detection; WebView-level interception note | UR §6 (configurable shortcuts) | FS-KBD-002 |
| [§7.18](03-components.md#718-process-terminated-pane-fs-pty-005-fs-pty-006) | Process terminated pane with restart/close | — | FS-PTY-005, FS-PTY-006 |
| [§7.19](03-components.md#719-ssh-reconnection-separator-fs-ssh-042) | Reconnection separator injected into scrollback at reconnect; full-width rule + left-aligned timestamp label; non-interactive UI overlay | — | FS-SSH-042 |
| [§7.20](03-components.md#720-theme-editor-fs-theme-003-fs-theme-004) | Theme editor with color pickers and contrast advisory | UR §8.2 (user-created themes) | FS-THEME-003, FS-THEME-004 |
| [§7.21](03-components.md#721-deprecated-ssh-algorithm-warning-banner-fs-ssh-014) | Deprecated SSH algorithm warning banner | UR §9.3 (security awareness) | FS-SSH-014 |
| [§8.2](04-interaction.md#82-focus-management) | Focus trap in modals | UR §3.1 (keyboard completeness) | FS-A11Y-003 |
| [§8.5](04-interaction.md#85-clipboard) | Auto-copy to PRIMARY, paste from CLIPBOARD | UR §6.3 (clipboard) | FS-CLIP-004, FS-CLIP-005 |
| [§8.5](04-interaction.md#85-clipboard) | Multi-line paste confirmation | — | FS-CLIP-009 |
| [§8.6](04-interaction.md#86-ssh-connection-interruption-feedback) | SSH interruption feedback within 1 second | UR §9.1 (interruption notification) | FS-SSH-022 |
| [§9](04-interaction.md#9-motion--animation) | All animations respect prefers-reduced-motion | — | FS-A11Y-001 (WCAG compliance) |
| [§10](04-interaction.md#10-iconography) | Lucide icon set with 1.5px stroke | — (CLAUDE.md stack) | — |
| [§11.2](05-accessibility.md#112-keyboard-navigation-map) | Pane navigation shortcuts (Ctrl+Shift+Arrow) | UR §4.2 (pane keyboard navigation) | FS-KBD-003, FS-PANE-005 |
| [§11.2](05-accessibility.md#112-keyboard-navigation-map) | Split shortcuts (Ctrl+Shift+D/E) | UR §4.2 (pane splits) | FS-KBD-003 |
| [§11.5](05-accessibility.md#115-touch-target-minimums) | 44px minimum touch targets | — | FS-A11Y-002 |
| [§11.6](05-accessibility.md#116-non-color-indicators) | Non-color indicators for all status states | — | FS-A11Y-004 |
| [§12](05-accessibility.md#12-responsiveness--window-resizing) | Graceful degradation at narrow widths | UR §2.1 (Alex — tiling WM use) | — |
| [§13](06-themes.md#13-theme-extensibility) | Theme extensibility via token override | UR §8.2 (user themes) | FS-THEME-003, FS-THEME-009 |
| [§13.5](06-themes.md#135-theme-validation-rules) | Contrast validation on user themes; editor chrome accessibility invariant (editor always renders with active system tokens, not work-in-progress theme) | UR §8.3 (visual consistency) | FS-THEME-008, FS-A11Y-001, FS-A11Y-005 (to be added), FS-PREF-003 |
| [§15](#15-ipc-contract) | IPC contract — deferred to ARCHITECTURE.md §4 (authoritative) | — | Cross-cutting (FS-SSH-010, FS-NOTIF, FS-VT, FS-SB) |
| [§2.7](01-principles.md#27-internationalisation-as-a-design-constraint) | i18n as design constraint: all strings are locale-resolved keys, no hardcoded copy | UR 10 §10.1 (language support) | FS-I18N-001 |
| [§7.6.3 Language subsection](03-components.md#763-preference-sections-fs-pref-004) | Language selector dropdown in Appearance section of Preferences; immediate apply with `--duration-fast` opacity transition; persisted; discoverability for Sam (UR §2.3) | UR 10 §10.1; UR §2.3 (Sam — discoverability) | FS-I18N-003, FS-I18N-004, FS-I18N-005, FS-I18N-006 |

---

## 15. IPC Contract

> **This section is superseded by [`docs/ARCHITECTURE.md` §4](../ARCHITECTURE.md#4-ipc-contract)**, which is the single source of truth for all data shapes, command signatures, and event payloads exchanged between the Rust backend and the Svelte frontend.
>
> ARCHITECTURE.md §4 covers: the complete `invoke()` command list (29 commands), all `listen()` events (8 events), TypeScript interfaces, Rust structs, and the authoritative decisions that override earlier drafts (pane layout tree, `SessionStateChanged` shape, `close_pane` return type, `notification-changed` payload).
>
> This document references ARCHITECTURE.md §4 for IPC concerns — it does not restate the contract here.
