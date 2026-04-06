<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design Document — TauTerm

> **Version:** 1.4.0
> **Date:** 2026-04-04
> **Status:** Draft
> **Author:** UX/UI Designer — TauTerm team
> **Input documents:** [User Requirements (UR.md)](UR.md), [Functional Specifications (FS.md)](FS.md), [Artistic Direction (AD.md)](AD.md)

---

## Table of Contents

1. [Purpose & Scope](#1-purpose--scope)
2. [Design Principles](#2-design-principles)
3. [Design Token System](#3-design-token-system)
4. [Typography System](#4-typography-system)
5. [Color System](#5-color-system)
6. [Layout & Spatial Model](#6-layout--spatial-model)
7. [Component Specifications](#7-component-specifications)
8. [Interaction Patterns](#8-interaction-patterns)
9. [Motion & Animation](#9-motion--animation)
10. [Iconography](#10-iconography)
11. [Accessibility](#11-accessibility)
12. [Responsiveness & Window Resizing](#12-responsiveness--window-resizing)
13. [Theme Extensibility](#13-theme-extensibility)
14. [Traceability Matrix](#14-traceability-matrix)
15. [IPC Contract](#15-ipc-contract)

---

## 1. Purpose & Scope

This document is the **single source of truth for every visual and interactive decision** in TauTerm. It specifies the complete design token system, component anatomy, interaction behavior, motion rules, accessibility compliance, and layout structure — with sufficient precision that a frontend developer can implement every surface without design judgment calls, and that a UX reviewer can validate conformance against measurable criteria.

### 1.1 Relationship to Other Documents

| Document | Governs | This document's relationship |
|----------|---------|------------------------------|
| **UR.md** | What users need | Every design decision traces to a persona need or interaction model principle from UR.md. |
| **FS.md** | What the system must do | FS.md is the **only** source of functional requirements. This document specifies *how* those requirements are expressed visually and interactively — it never restates them. When a design decision satisfies a specific requirement, a reference `(FS-XXX-NNN)` is included. |
| **AD.md** | Artistic direction, token definitions, palette | This document consumes AD.md's token values and palette verbatim. It extends them with component-level specifications, interaction patterns, and layout rules that AD.md intentionally does not cover. |

### 1.2 Authoritativeness and Scope Rules

- **Token values** (colors, spacing, typography, radii, shadows, motion): AD.md is authoritative. This document references those values; it does not redefine them.
- **Component specifications** (anatomy, states, dimensions, behavior): this document is authoritative.
- **Interaction patterns** (focus management, drag behavior, keyboard navigation): this document is authoritative.
- **Layout structure** (window anatomy, region placement, spatial rules): this document is authoritative.

**What belongs here:** visual properties, interaction design, component anatomy, accessibility implementation, motion rules, UX behavior descriptions.

**What does not belong here:**
- Normative requirements (`MUST`, `SHALL`, `SHOULD`) — those belong exclusively in `FS.md`. Use descriptive language instead: "the tab bar scrolls to show the new tab" rather than "the tab bar MUST scroll".
- Functional requirements — reference the FS ID instead of restating. `(FS-TAB-009)` is sufficient; do not write the requirement again.
- Implementation details (API names, CSS property names, algorithm choices) — those belong in source code and comments.

**Corollary — no duplication with FS.md:** if a behavior is already specified as a requirement in FS.md, this document describes only the *design expression* of that behavior (visual feedback, transition, layout consequence) and links to the FS ID. Writing the same constraint in both documents creates a SSOT violation and a future contradiction risk.

---

## 2. Design Principles

These principles govern every decision in this document. Each is grounded in UR.md persona needs and AD.md artistic intent. They are ordered by priority when principles conflict.

### 2.1 Terminal Content is Primary

The terminal output is the user's work. Every chrome element exists to support that work, not to compete with it. Chrome operates at a lower visual temperature than terminal content (AD.md §1.1). UI elements earn their pixel budget through utility.

**Grounding:** Alex (UR §2.1) spends full working days in TauTerm. Visual fatigue from competing chrome is a productivity cost. AD.md §1.2 resolves the density-vs-calm tension in favor of managed density.

**Verification:** No chrome element uses a font size larger than `--font-size-ui-lg` (16px). No chrome element uses a color with higher luminance than `--color-text-primary` (`#ccc7bc`) outside of explicit emphasis contexts (active tab title, accent on interaction).

### 2.2 Every Feature Is Reachable by Both Modalities

Every action exposed by TauTerm's UI is reachable via mouse (visible controls, context menus) and via keyboard (shortcuts, tab navigation). Neither modality is an afterthought.

**Grounding:** UR §3.1 (dual modality). Sam (UR §2.3) relies on visible controls; Alex (UR §2.1) relies on keyboard shortcuts. AD.md §1.2 states "the visual design expresses both clearly without privileging either."

**Verification:** Every interactive element in §7 specifies both a mouse interaction and a keyboard interaction. Every component spec includes a focus state. The keyboard navigation map in §11 covers all interactive surfaces.

### 2.3 Status Is Honest and Immediate

The interface reflects the system's actual state at all times. Connection drops, process termination, and activity in background tabs are communicated promptly (FS-SSH-022), using both color and a non-color indicator (icon, text, or shape).

**Grounding:** Jordan (UR §2.2) manages many SSH sessions and needs at-a-glance status. FS-SSH-022 requires disconnection detection within 1 second. FS-A11Y-004 requires non-color-only indicators. AD.md §1.3 lists "Honesty" as a design value.

**Verification:** Every status state in §7 specifies a color token AND a secondary indicator (icon or text). SSH lifecycle states (§7.5) each have a distinct visual treatment with both color and icon.

### 2.4 Sensible Defaults, Zero Required Configuration

TauTerm is usable immediately after installation. Default shortcuts, theme, font, and behavior require no adjustment for productive use. Configuration is an optimization layer.

**Grounding:** Sam (UR §2.3) needs "sensible defaults, discoverable preferences UI, no configuration required for basic use." AD.md §2.4 states the theme must read "as a coherent product, not an assembled collection of defaults."

**Verification:** Every configurable value in §7 and §8 specifies an explicit default. The default theme (Umbra) passes all accessibility checks in §11 without user adjustment.

### 2.5 Precision Over Decoration

Spacing, sizing, and color choices are deliberate, consistent, and derived from the token system. No approximate alignment, no decorative elements, no visual effects that do not communicate state.

**Grounding:** AD.md §1.3 ("Precision: nothing is approximately aligned or roughly the right size"). AD.md §1.3 ("Restraint: each new visual element must justify its existence").

**Verification:** Every dimension in §7 is a token reference. No component uses a color value outside the token vocabulary defined in §3. No animation exists without a stated purpose in §9.

### 2.6 Durability Over Novelty

TauTerm is used for hours daily. The visual design must not fatigue the eye, produce surprising behaviors, or require relearning. Consistency across sessions builds trust; familiarity is a feature.

**Grounding:** AD.md §1.3 ("Durability: TauTerm is used for hours daily. The visual design must not fatigue the eye or demand relearning."). Alex (UR §2.1) and Jordan (UR §2.2) are daily users.

**Verification:** No animation exceeds `--duration-slow` (300ms). All hover/focus states follow the same pattern (token pair + timing) across components. The color palette uses warm-shifted neutrals (AD.md §3.1) to reduce luminance extremes that cause long-session fatigue.

### 2.7 Internationalisation as a Design Constraint

All user-visible strings referenced in component specifications throughout this document are logical string keys, not hardcoded text. At render time, each key is resolved to the active locale's value via the i18n message catalogue (see ARCHITECTURE.md §10.5). This constraint applies equally to button labels, section headings, placeholder text, tooltips, error messages, and all other copy.

**Implication for component specs:** when this document specifies text such as "Reconnect" or "THEMES", these are the English-locale display values used as examples. The actual rendered string is always looked up from the active catalogue. No string in component source code may be hardcoded.

**Grounding:** FS-I18N-001 (no hardcoded UI string), FS-I18N-004 (immediate apply on locale change).

**Verification:** No string literal appears in any component `.svelte` file outside of an i18n accessor call. Locale switching in Preferences immediately updates all visible text with no reload.

---

## 3. Design Token System

All tokens are defined in AD.md §7 and expressed as Tailwind 4 `@theme` CSS custom properties. This section provides the complete reference with usage descriptions. Components in §7 reference these tokens exclusively — no hardcoded values.

### 3.1 Color Tokens — Primitives

Primitive tokens are named palette positions with no semantic meaning. Components never consume these directly — they go through semantic tokens (§3.2).

Primitive tokens use the `--color-*` namespace (e.g. `--color-neutral-950`). This shared namespace with Tailwind 4 is safe in practice: Tailwind 4 generates its utility tokens from the palette config, not from custom `@theme` declarations — both sets coexist without collision. The previous `--umbra-` prefix added verbosity and onboarding friction without meaningful benefit and has been removed.

Primitives are distinguished from semantic tokens by their position in the name: `--color-neutral-950` is a primitive (palette position), while `--color-bg-base` is semantic (usage meaning). Components never consume primitives directly — they always go through semantic tokens (§3.2).

#### Neutral Scale (warm-shifted)

| Token | Value | Description |
|-------|-------|-------------|
| `--color-neutral-950` | `#0e0d0b` | Deepest background, window chrome base |
| `--color-neutral-900` | `#16140f` | Terminal background, active tab background |
| `--color-neutral-850` | `#1c1a14` | Unfocused pane background |
| `--color-neutral-800` | `#242118` | Tab bar background, surface |
| `--color-neutral-750` | `#2c2921` | Raised surface (menus, dropdowns, tooltips, hover backgrounds) |
| `--color-neutral-700` | `#35312a` | Border, divider, active background |
| `--color-neutral-600` | `#4a4640` | Scrollbar thumb, inactive tab text background reference |
| `--color-neutral-500` | `#6b6660` | Placeholder text, disabled elements |
| `--color-neutral-400` | `#9c9890` | Secondary text, inactive labels, icons default, inactive tab text |
| `--color-neutral-300` | `#ccc7bc` | Primary UI text, terminal foreground |
| `--color-neutral-200` | `#e8e3d8` | Emphasized text, active tab title |
| `--color-neutral-100` | `#f5f2ea` | High-emphasis text (rare), ANSI bright white |

#### Blue-Steel Scale (primary accent)

| Token | Value | Description |
|-------|-------|-------------|
| `--color-blue-700` | `#1a3a52` | Deep accent background, SSH badge background, selection (unfocused) |
| `--color-blue-600` | `#1e4d6e` | Focus ring base |
| `--color-blue-500` | `#2e6f9c` | Interactive accent hover, selection background (focused) |
| `--color-blue-400` | `#4a92bf` | Primary accent default state, focus ring, active pane border, divider hover |
| `--color-blue-300` | `#7ab3d3` | Accent text on dark, cursor fill, hyperlink text |
| `--color-blue-200` | `#b3d2e6` | Light accent (reserved) |

#### Amber Scale (warning/caution)

| Token | Value | Description |
|-------|-------|-------------|
| `--color-amber-700` | `#4d3000` | Warning background, search match background (non-active) |
| `--color-amber-500` | `#b06a00` | Warning indicator |
| `--color-amber-400` | `#d48a20` | Warning text, bell indicator, SSH connecting state |
| `--color-amber-300` | `#e8b060` | Warning label, search match foreground (non-active) |

#### Red Scale (error/danger)

| Token | Value | Description |
|-------|-------|-------------|
| `--color-red-700` | `#3d1212` | Error background, SSH disconnected badge background |
| `--color-red-500` | `#9c2c2c` | Error indicator |
| `--color-red-400` | `#c44444` | Error text, destructive action, ANSI red normal |
| `--color-red-300` | `#d97878` | Error label, SSH disconnected badge text |

#### Green Scale (success/activity)

| Token | Value | Description |
|-------|-------|-------------|
| `--color-green-600` | `#1a3d1a` | Activity background |
| `--color-green-400` | `#4a9c4a` | Activity indicator, success |
| `--color-green-300` | `#78c078` | Activity text, output-on-inactive-tab indicator |

### 3.2 Color Tokens — Semantic

Semantic tokens are the layer components consume. They map to primitives and can be fully replaced by a user theme.

#### Background Elevation Layers

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-bg-base` | `#0e0d0b` | Window chrome, deepest layer |
| `--color-bg-surface` | `#242118` | Tab bar, sidebar backgrounds |
| `--color-bg-raised` | `#2c2921` | Menus, dropdowns, tooltips, popover panels |
| `--color-bg-overlay` | `#16140f` | Modal scrim base (apply opacity separately at 60%) |

#### Borders and Dividers

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-border` | `#35312a` | Standard structural border |
| `--color-border-subtle` | `#2c2921` | Low-contrast border for nested containers |
| `--color-divider` | `#35312a` | Pane divider visible line (default) |
| `--color-divider-active` | `#4a92bf` | Pane divider on hover or drag |

#### Text Hierarchy

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-text-primary` | `#ccc7bc` | Primary UI text, body copy |
| `--color-text-secondary` | `#9c9890` | Descriptions, subtitles, supplementary labels |
| `--color-text-tertiary` | `#6b6660` | Placeholder text, disabled elements |
| `--color-text-inverted` | `#0e0d0b` | Text on accent or light backgrounds |
| `--color-text-heading` | `#9c9890` | Section heading labels (all-caps, small) |
| `--color-text-muted` | `#6b6660` | Muted text, group headings in connection list, "Closed" SSH state label |

#### Icons

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-icon-default` | `#9c9890` | Icon resting state |
| `--color-icon-active` | `#ccc7bc` | Icon on hover or in active context |

#### Interactive / Accent

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-accent` | `#4a92bf` | Primary interactive accent |
| `--color-accent-subtle` | `#1a3a52` | Tinted background for accent contexts |
| `--color-accent-text` | `#7ab3d3` | Accent-colored text on dark surfaces |
| `--color-hover-bg` | `#2c2921` | Generic hover background |
| `--color-active-bg` | `#35312a` | Generic active/pressed background |
| `--color-focus-ring` | `#4a92bf` | Keyboard focus ring color |
| `--color-focus-ring-offset` | `#0e0d0b` | Gap between focus ring and element edge |

#### Status and Notification

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-activity` | `#78c078` | Output activity on inactive tab |
| `--color-indicator-output` | `#78c078` | Output activity indicator (alias for `--color-activity`, used for scroll arrow badges) |
| `--color-indicator-bell` | `#d48a20` | Bell event indicator (alias for `--color-bell`, used for scroll arrow badges and pane border pulse) |
| `--color-process-end` | `#9c9890` | Process terminated indicator |
| `--color-bell` | `#d48a20` | Bell event indicator |
| `--color-error` | `#c44444` | Error state foreground |
| `--color-error-bg` | `#3d1212` | Error state background |
| `--color-error-text` | `#d97878` | Error text on error background |
| `--color-warning` | `#d48a20` | Warning state foreground |
| `--color-warning-bg` | `#4d3000` | Warning state background |
| `--color-warning-text` | `#e8b060` | Warning text on warning background |
| `--color-success` | `#4a9c4a` | Success state foreground |
| `--color-success-text` | `#78c078` | Success text on dark background |

### 3.3 Color Tokens — Component

Component tokens specialize semantic tokens for individual UI components.

#### Tab Bar

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-tab-bg` | `#242118` | Tab bar background |
| `--color-tab-active-bg` | `#16140f` | Active tab background (matches terminal bg) |
| `--color-tab-active-fg` | `#e8e3d8` | Active tab title text |
| `--color-tab-inactive-bg` | `transparent` | Inactive tab background |
| `--color-tab-inactive-fg` | `#9c9890` | Inactive tab title text |
| `--color-tab-hover-bg` | `#2c2921` | Tab hover background |
| `--color-tab-hover-fg` | `#9c9890` | Tab hover title text |
| `--color-tab-close-fg` | `#6b6660` | Close button icon resting |
| `--color-tab-close-hover-fg` | `#ccc7bc` | Close button icon on hover |
| `--color-tab-new-fg` | `#6b6660` | New tab button icon resting |
| `--color-tab-new-hover-fg` | `#ccc7bc` | New tab button icon on hover |

#### SSH Session Indicators

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-ssh-badge-bg` | `#1a3a52` | SSH connected badge background |
| `--color-ssh-badge-fg` | `#7ab3d3` | SSH connected badge text/icon |
| `--color-ssh-connected` | `#7ab3d3` | SSH connected accent color (alias of `--color-ssh-badge-fg`; used by the reconnection separator rule line) |
| `--color-ssh-disconnected-bg` | `#3d1212` | SSH disconnected badge background |
| `--color-ssh-disconnected-fg` | `#d97878` | SSH disconnected badge text/icon |
| `--color-ssh-connecting-fg` | `#d48a20` | SSH connecting state indicator |

#### Pane Borders

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-pane-border-active` | `#4a92bf` | Active pane border |
| `--color-pane-border-inactive` | `#35312a` | Inactive pane border |

#### Scrollbar

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--color-scrollbar-track` | `transparent` | Scrollbar track background |
| `--color-scrollbar-thumb` | `#4a4640` | Scrollbar thumb resting |
| `--color-scrollbar-thumb-hover` | `#6b6660` | Scrollbar thumb on hover |

### 3.4 Color Tokens — Terminal Surface

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--term-bg` | `#16140f` | Terminal background |
| `--term-fg` | `#ccc7bc` | Terminal default foreground |
| `--term-cursor-bg` | `#7ab3d3` | Block cursor fill |
| `--term-cursor-fg` | `#16140f` | Character under cursor |
| `--term-cursor-unfocused` | `#7ab3d3` | Hollow outline cursor when pane unfocused |
| `--term-selection-bg` | `#2e6f9c` | Selection background (focused pane) |
| `--term-selection-fg` | `inherit` | Selection foreground (no forced inversion) |
| `--term-selection-bg-inactive` | `#1a3a52` | Selection background (unfocused pane) |
| `--term-selection-flash` | `#5499c7` | Copy flash target color (HSL 205, 55%, 59% — `--term-selection-bg` lightness raised by 20 percentage points) |
| `--term-search-match-bg` | `#4d3000` | Non-active search match background |
| `--term-search-match-fg` | `#e8b060` | Non-active search match foreground |
| `--term-search-active-bg` | `#6b5c22` | Active (current) search match background |
| `--term-search-active-fg` | `#e8e3d8` | Active search match foreground |
| `--term-hyperlink-fg` | `#7ab3d3` | Hyperlink text color |
| `--term-hyperlink-underline` | `#4a92bf` | Hyperlink underline color |

### 3.5 Typography Tokens

| Token | Value | Description |
|-------|-------|-------------|
| `--font-ui` | `system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif` | UI chrome font stack |
| `--font-mono-ui` | `"JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", monospace` | Monospace UI font (shortcut displays, path inputs) |
| `--font-terminal` | `"JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", "Courier New", monospace` | Terminal font stack |
| `--font-size-terminal` | `14px` | Default terminal font size |
| `--line-height-terminal` | `1.2` | Terminal line height |
| `--font-size-ui-2xs` | `10px` | Badge counters, status dots with text |
| `--font-size-ui-xs` | `11px` | Section headings (uppercase labels), tooltips |
| `--font-size-ui-sm` | `12px` | Secondary labels, shortcut keys, monospace UI text |
| `--font-size-ui-base` | `13px` | Primary UI text: tab titles, menu items, form fields |
| `--font-size-ui-md` | `14px` | Terminal font size reference, dialog body text |
| `--font-size-ui-lg` | `16px` | Dialog headings, modal titles |
| `--font-size-ui-xl` | `20px` | Reserved for major headings |
| `--font-weight-normal` | `400` | Default text weight |
| `--font-weight-medium` | `500` | Slight emphasis (button labels) |
| `--font-weight-semibold` | `600` | Active tab title, section headings |

### 3.6 Spacing Tokens

Base unit: 4px. All spacing values are multiples of 4px.

| Token | Value | Usage |
|-------|-------|-------|
| `--space-0` | `0px` | Reset |
| `--space-1` | `4px` | Tight internal padding: icon-to-label gap, inline badge margin |
| `--space-2` | `8px` | Component internal padding: button, tab, input |
| `--space-3` | `12px` | Default padding for compact containers |
| `--space-4` | `16px` | Standard section padding |
| `--space-5` | `20px` | Generous element spacing |
| `--space-6` | `24px` | Section separation |
| `--space-8` | `32px` | Major layout separation |
| `--space-10` | `40px` | Large-scale spacing |
| `--space-12` | `48px` | Modal/panel internal margins |

### 3.7 Sizing Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--size-tab-height` | `40px` | Tab bar item height |
| `--size-toolbar-height` | `40px` | Toolbar height |
| `--size-divider-hit` | `8px` | Pane divider interactive hit area width |
| `--size-scrollbar-width` | `8px` | Scrollbar track width |
| `--size-icon-sm` | `14px` | Icons in tab bar |
| `--size-icon-md` | `16px` | Icons in toolbars, context menus |
| `--size-icon-lg` | `20px` | Icons in dialog headers |
| `--size-target-min` | `44px` | Minimum interactive target size (WCAG 2.5.5) |
| `--size-badge` | `6px` | Activity dot indicator diameter |
| `--size-scroll-arrow-badge` | `4px` | Scroll arrow activity badge diameter |
| `--size-status-bar-height` | `28px` | Status bar height |
| `--size-search-overlay-width` | `360px` | Search overlay width |
| `--size-preferences-panel-width` | `640px` | Preferences panel width |
| `--size-preferences-nav-width` | `180px` | Preferences section navigation width |
| `--size-connection-manager-width` | `400px` | Connection manager panel width |
| `--size-cursor-underline-height` | `2px` | Height of the underline cursor style |
| `--size-cursor-bar-width` | `2px` | Width of the bar cursor style |

### 3.8 Border Radius Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-none` | `0px` | Panels, tab bar, terminal area, dividers |
| `--radius-sm` | `2px` | Buttons, text inputs, dropdown items, tab items |
| `--radius-md` | `4px` | Modals, tooltips, context menus, search overlay |
| `--radius-full` | `9999px` | Status dots, activity badges, toggle thumb |

### 3.9 Shadow Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--shadow-overlay` | `0 8px 32px rgba(0, 0, 0, 0.6)` | Modals, connection manager panel |
| `--shadow-raised` | `0 2px 8px rgba(0, 0, 0, 0.4)` | Context menus, tooltips, dropdowns |

### 3.10 Z-Index Scale

| Token | Value | Usage |
|-------|-------|-------|
| `--z-base` | `0` | Default stacking context (terminal area, tab bar) |
| `--z-divider` | `10` | Pane dividers (above terminal content) |
| `--z-scrollbar` | `15` | Scrollbar (above terminal content) |
| `--z-search` | `20` | Search overlay |
| `--z-dropdown` | `30` | Dropdowns, context menus, tooltips |
| `--z-overlay` | `40` | Preferences panel, connection manager |
| `--z-modal-backdrop` | `49` | Modal backdrop (sits behind the dialog panel) |
| `--z-modal` | `50` | Dialogs, confirmation prompts |
| `--z-tooltip` | `60` | Tooltips (topmost) |

### 3.11 Transition Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--duration-instant` | `0ms` | Focus rings, hover background changes |
| `--duration-fast` | `80ms` | Dismissals, fade-outs, tooltip disappearance |
| `--duration-base` | `100ms` | Modal/popover appearance, search overlay entrance |
| `--duration-slow` | `300ms` | Scrollbar fade, theme switch cross-fade, non-critical transitions |
| `--ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | Dismissals (accelerating out) |
| `--ease-out` | `cubic-bezier(0, 0, 0.2, 1)` | Appearances (decelerating in) |
| `--ease-linear` | `linear` | Spinners, continuous rotation |

---

## 4. Typography System

### 4.1 Terminal Font

**Requirements:** Fixed-width (monospace), Unicode support including box-drawing characters (U+2500 block), CJK ideographs, and common emoji. Ligatures are acceptable but not required.

**Font stack:** `--font-terminal`
```
"JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", "Courier New", monospace
```

**Default size:** `--font-size-terminal` (14px). User-configurable in preferences (FS-PREF-006).

**Line height:** `--line-height-terminal` (1.2). Tight enough for density; loose enough to prevent ascenders/descenders from touching across lines.

**Rationale:** JetBrains Mono is the most commonly pre-installed developer font on Linux systems. The fallback chain covers common developer setups. `"Courier New"` is a last-resort fallback for bare systems. (AD.md §4.1)

### 4.2 UI Font

**Requirements:** Variable-weight sans-serif that integrates with the host desktop environment. Weights 400, 500, and 600 must be available.

**Font stack:** `--font-ui`
```
system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif
```

**Rationale:** Using the system font stack respects the user's desktop typographic preferences and avoids bundling a custom font that would create a visual seam between TauTerm and the rest of the desktop. On Linux, this typically resolves to Noto Sans, Ubuntu, or Inter depending on the desktop environment. (AD.md §4.1)

### 4.3 Monospace UI Font

**Font stack:** `--font-mono-ui`
```
"JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", monospace
```

**Usage:** Shortcut key displays in preferences, path inputs in the connection manager, inline code/command references in error messages.

### 4.4 Type Scale

| Level Name | Token | Size | Weight | Line Height | Letter Spacing | Usage |
|-----------|-------|------|--------|-------------|----------------|-------|
| Badge | `--font-size-ui-2xs` | 10px | `--font-weight-medium` (500) | 1.2 | 0 | Badge counters, status dot labels |
| Caption | `--font-size-ui-xs` | 11px | `--font-weight-semibold` (600) | 1.3 | 0.06em | Section headings (all-caps), tooltip text |
| Label | `--font-size-ui-sm` | 12px | `--font-weight-normal` (400) | 1.4 | 0 | Secondary labels, shortcut key text, descriptions |
| Body | `--font-size-ui-base` | 13px | `--font-weight-normal` (400) | 1.4 | 0 | Primary UI text: tab titles, menu items, form fields, dialog body |
| Body-emphasis | `--font-size-ui-base` | 13px | `--font-weight-semibold` (600) | 1.4 | 0 | Active tab title only |
| Content | `--font-size-ui-md` | 14px | `--font-weight-normal` (400) | 1.4 | 0 | Dialog body text where larger size aids readability |
| Heading | `--font-size-ui-lg` | 16px | `--font-weight-semibold` (600) | 1.3 | 0 | Dialog headings, modal titles |
| Title | `--font-size-ui-xl` | 20px | `--font-weight-semibold` (600) | 1.2 | 0 | Reserved; major headings if needed |

### 4.5 Hierarchy Rules

1. **Primary UI text** — Body level. Default for all labels, tab titles, menu items, form fields.
2. **Section headings** — Caption level, all-caps, letter-spacing 0.06em, color `--color-text-heading`. Used for preference panel sections and grouped form labels. Small-caps headings establish hierarchy without consuming vertical space.
3. **Active tab title** — Body-emphasis level. The only surface where semibold weight is used on body-size text, distinguishing the active tab from inactive ones without relying on color alone.
4. **Secondary text** — Label level, color `--color-text-secondary`. Descriptions, subtitles, supplementary labels.
5. **Monospace UI elements** — Label level in `--font-mono-ui`. Shortcut displays, path fields.
6. **No weight other than 400, 500, or 600** is used anywhere in the UI.

---

## 5. Color System

### 5.1 Layering Model

The Umbra color system uses an elevation model where progressively lighter surfaces indicate higher z-order:

```
Layer 0: --color-bg-base       (#0e0d0b)  Window chrome, deepest
Layer 1: --color-bg-surface     (#242118)  Tab bar, sidebars
Layer 2: --color-bg-raised      (#2c2921)  Menus, dropdowns, tooltips
Layer 3: --color-bg-overlay      (#16140f at 60% opacity)  Modal backdrop
```

The terminal background (`--term-bg`, `#16140f`) sits between Layer 0 and Layer 1 — darker than the tab bar to create a visual well that draws the eye inward toward terminal content.

The active tab background matches `--term-bg` to create a seamless visual connection between the selected tab and its terminal content.

### 5.2 Semantic Color Roles

| Role | Token | Usage |
|------|-------|-------|
| Accent (interactive) | `--color-accent` | Focused pane border, focus ring, primary button, active divider |
| Accent (subtle) | `--color-accent-subtle` | Tinted backgrounds in accent contexts (SSH badge) |
| Accent (text) | `--color-accent-text` | Accent-colored text on dark backgrounds |
| Error | `--color-error` | Error icons, destructive button default state |
| Error (background) | `--color-error-bg` | Error notification background, SSH disconnected badge |
| Error (text) | `--color-error-text` | Error description text |
| Warning | `--color-warning` | Warning icons, caution indicators |
| Warning (background) | `--color-warning-bg` | Warning notification background |
| Warning (text) | `--color-warning-text` | Warning description text |
| Success | `--color-success` | Success icons, connected indicators |
| Success (text) | `--color-success-text` | Success description text |
| Activity | `--color-activity` | Background tab output activity indicator |
| Bell | `--color-bell` | Bell event indicator |
| Process end | `--color-process-end` | Process terminated indicator |

### 5.3 ANSI 16-Color Palette

The ANSI terminal palette with contrast ratios against `--term-bg` (`#16140f`) is defined in AD.md §3.2. The values are not reproduced here to avoid maintenance duplication — AD.md is authoritative.

All palette entries mapped to tokens `--term-color-0` through `--term-color-15`. ANSI Black (indices 0 and 8) are excluded from contrast requirements as they serve as background/dimmed-text colors. All other entries meet WCAG AA (4.5:1) contrast against `--term-bg`.

### 5.4 Accessibility Compliance

All color pairings used in the UI meet WCAG 2.1 AA minimums:

| Pair | Foreground | Background | Ratio | Threshold | Result |
|------|-----------|------------|-------|-----------|--------|
| Primary UI text on surface | `#ccc7bc` | `#242118` | 8.1:1 | 4.5:1 (normal text) | Pass |
| Active tab on active-bg | `#e8e3d8` | `#16140f` | 10.3:1 | 4.5:1 (normal text) | Pass |
| Inactive tab on tab-bg | `#9c9890` | `#242118` | 6.0:1 | 4.5:1 (normal text) | Pass |
| Tab hover text on hover-bg | `#9c9890` | `#2c2921` | 4.6:1 | 4.5:1 (normal text) | Pass |
| Accent text on bg-base | `#7ab3d3` | `#0e0d0b` | 7.8:1 | 4.5:1 (normal text) | Pass |
| Error text on error-bg | `#d97878` | `#3d1212` | 4.9:1 | 4.5:1 (normal text) | Pass |
| Warning text on warning-bg | `#e8b060` | `#4d3000` | 5.4:1 | 4.5:1 (normal text) | Pass |
| Terminal fg on terminal bg | `#ccc7bc` | `#16140f` | 8.4:1 | 4.5:1 (normal text) | Pass |
| Focus ring on bg-base | `#4a92bf` | `#0e0d0b` | 5.9:1 | 3:1 (UI component) | Pass |
| Placeholder on surface | `#6b6660` | `#242118` | 3.1:1 | 3:1 (UI component) | Pass |
| Secondary text on base | `#9c9890` | `#0e0d0b` | 6.5:1 | 4.5:1 (normal text) | Pass |
| SSH badge text on badge-bg | `#7ab3d3` | `#1a3a52` | 4.5:1 | 4.5:1 (normal text) | Pass |
| SSH disconnected text on bg | `#d97878` | `#3d1212` | 4.9:1 | 4.5:1 (normal text) | Pass |
| Search active fg on active-bg | `#e8e3d8` | `#6b5c22` | 5.2:1 | 4.5:1 (normal text) | Pass |
| Search match fg on match-bg | `#e8b060` | `#4d3000` | 5.4:1 | 4.5:1 (normal text) | Pass |

**Inactive tab text (corrected, TUITC-UX-060):** `--color-tab-inactive-fg` was previously `#6b6660` (neutral-500) at 3.1:1 on `#242118`. WCAG 1.4.3 applies to tab title text (normal text, not non-text component) — the 3:1 exemption is inapplicable. Token raised to `#9c9890` (neutral-400), achieving ~6.0:1. Visual hierarchy is preserved by font-weight difference (active = 600, inactive = 400) and background surface difference (active bg `#16140f` vs inactive bg `#242118`).

---

## 6. Layout & Spatial Model

### 6.1 Window Anatomy

The TauTerm window is composed of four horizontal bands stacked vertically, plus optional overlay panels:

```
+----------------------------------------------------------+
|  Tab Bar            (--size-tab-height: 40px)             |
+----------------------------------------------------------+
|                                                          |
|  Terminal Area      (remaining height)                   |
|    +----------+---------+                                 |
|    | Pane A  | Pane B  |  (panes within a tab)           |
|    |         |         |                                 |
|    +----------+---------+                                 |
|                                                          |
+----------------------------------------------------------+
|  Status Bar         (--size-status-bar-height: 28px)      |
+----------------------------------------------------------+
```

**Overlay panels** (not always visible):
- Search overlay: anchored top-right of the active pane
- Preferences panel: centered overlay with backdrop
- Connection manager: right-side slide-in panel
- Context menu: positioned at cursor
- Dialogs: centered overlay with backdrop

### 6.2 Tab Bar Region

- **Height:** `--size-tab-height` (40px).
- **Background:** `--color-tab-bg` (`#242118`).
- **Bottom border:** 1px solid `--color-border` (`#35312a`), separating tab bar from terminal area.
- **Internal padding:** `--space-0` vertical, `--space-1` (4px) horizontal on the bar itself.
- **Tab items** are laid out horizontally, left-aligned. The new-tab button is the last element.
- **Overflow:** When tabs exceed available width, the tab bar scrolls horizontally. A subtle gradient fade (8px wide, `--color-tab-bg` to transparent) at the right edge indicates hidden tabs. Scroll position is controllable via mouse wheel on the tab bar or left/right arrow buttons that appear at overflow. Each scroll arrow button (Lucide `ChevronLeft` / `ChevronRight`) displays a small amber dot badge (`--size-scroll-arrow-badge`, 4px diameter, `--color-indicator-output`) in its top-right corner when any hidden tab on that side has an active output notification indicator. The badge uses `--color-indicator-bell` instead when any hidden tab on that side has a bell notification.

### 6.3 Terminal Area Region

- **Background:** `--term-bg` (`#16140f`).
- **Occupies:** All vertical space between the tab bar bottom border and the status bar top border.
- **Padding:** `--space-0` — terminal content extends edge-to-edge within its pane. Any visual margin is achieved through the pane border width, not padding.
- **Panes** fill this region. A single pane fills 100% of the terminal area. Multiple panes are separated by dividers.

### 6.4 Status Bar Region

- **Height:** `--size-status-bar-height` (28px).
- **Background:** `--color-bg-base` (`#0e0d0b`).
- **Top border:** 1px solid `--color-border` (`#35312a`).
- **Internal padding:** `--space-2` (8px) horizontal.
- **Content:** Left-aligned: active pane shell name and current working directory (truncated with ellipsis if too long). Right-aligned (in order): Settings button, SSH connection status (if applicable), terminal dimensions (cols x rows).
- **Settings button:** Lucide `Settings` icon, `--size-icon-md` (16px), ghost button variant. Hit area `--size-target-min` (44px) height clamped to `--size-status-bar-height` (28px), width `--size-target-min` (44px). Icon color: `--color-icon-default` resting, `--color-icon-active` on hover, `--color-active-bg` background on active/pressed. Action: opens the Preferences panel (same as Ctrl+,). Tooltip: "Settings (Ctrl+,)" shown after `--duration-slow` (300ms) hover delay. ARIA label: "Open settings".
- **Font:** `--font-size-ui-sm` (12px), `--font-weight-normal` (400), color `--color-text-secondary` (`#9c9890`).

### 6.5 Pane Dividers

- **Visual line:** 1px solid `--color-divider` (`#35312a`) centered within the hit area.
- **Hit area:** `--size-divider-hit` (8px) wide/tall (perpendicular to the split direction). This invisible hit target ensures easy mouse targeting.
- **Hover state:** The 1px line changes to `--color-divider-active` (`#4a92bf`). Cursor changes to `col-resize` (vertical divider) or `row-resize` (horizontal divider).
- **Drag state:** Line remains `--color-divider-active` throughout the drag. Adjacent panes resize in real-time.

### 6.6 Active Pane Indication

When multiple panes exist within a tab:
- The **active (focused) pane** has a 2px border on all sides in `--color-pane-border-active` (`#4a92bf`).
- **Inactive panes** have a 1px border on all sides in `--color-pane-border-inactive` (`#35312a`).
- The border is drawn inside the pane area (no layout shift on focus change). The active pane uses 2px inset; inactive panes use 1px inset with 1px transparent to maintain identical content area size.

When only one pane exists in a tab, no pane border is drawn (the terminal area is borderless).

### 6.7 Grid and Alignment

- All vertical spacing between elements within chrome regions (tab bar, status bar, preferences panel) uses tokens from the `--space-*` scale.
- All horizontal alignment within chrome regions uses `--space-*` tokens for margins and padding.
- Text baselines within the tab bar are vertically centered within `--size-tab-height`.
- Icons are vertically centered relative to adjacent text, with `--space-1` (4px) gap between icon and text.

### 6.8 Minimum and Default Window Size

- **Minimum window size:** 640 x 400 pixels. Below this threshold, the window manager prevents further shrinking.
- **Default window size:** 1024 x 720 pixels (fits comfortably on a 1080p display with taskbar).
- **Rationale:** 640px width accommodates 80 columns of terminal text at 14px font size. 400px height provides at minimum `--size-tab-height` (40px) + `--size-status-bar-height` (28px) + approximately 20 rows of terminal text.

---

## 7. Component Specifications

### 7.1 Tab Bar

#### 7.1.1 Tab Bar Container

- **Element:** Horizontal flex row spanning the full window width, containing two zones:
  1. **Tab bar zone** (`.tab-bar`, `flex: 1 0 0`) — occupies all available width minus the SSH connections toggle (§7.1.8). Contains: scroll arrows, tab items, new-tab button.
  2. **SSH connections toggle** (§7.1.8, `flex-shrink: 0`) — anchored to the right edge, always visible.
- **Height:** `--size-tab-height` (40px).
- **Background:** `--color-tab-bg` (`#242118`).
- **Bottom border:** 1px solid `--color-border`.
- **Layout of tab bar zone:** Flex row, items left-aligned. New-tab button is the rightmost item after all tabs. The tab bar zone uses `overflow: hidden` and `flex: 1 0 0` so that adding tabs never expands the zone beyond its allocated width — the SSH connections toggle must remain visible at all times.
- **ARIA role:** `tablist` on the tab bar zone.

#### 7.1.2 Tab Item

**Anatomy:**
```
+-[SSH badge]-[Title text]-----------[Activity dot]-[Close btn]-+
+---------------------------------------------------------------+
```

- **Height:** `--size-tab-height` (40px).
- **Min width:** 120px. **Max width:** 240px.
- **Horizontal padding:** `--space-3` (12px) left, `--space-2` (8px) right.
- **Border radius:** `--radius-sm` (2px) on top-left and top-right only; bottom corners are `--radius-none`.
- **ARIA role:** `tab`. `aria-selected="true"` for active tab.
- **Title font:** `--font-size-ui-base` (13px), `--font-ui`.
- **Title truncation:** Ellipsis when text exceeds available width.
- **Gap between elements:** `--space-1` (4px).

**States:**

| State | Background | Text Color | Text Weight | Border |
|-------|-----------|------------|-------------|--------|
| Active | `--color-tab-active-bg` (`#16140f`) | `--color-tab-active-fg` (`#e8e3d8`) | `--font-weight-semibold` (600) | none (seamless with terminal) |
| Inactive | `--color-tab-inactive-bg` (transparent) | `--color-tab-inactive-fg` (`#9c9890`) | `--font-weight-normal` (400) | none |
| Hover (inactive) | `--color-tab-hover-bg` (`#2c2921`) | `--color-tab-hover-fg` (`#9c9890`) | `--font-weight-normal` (400) | none |
| Focus (keyboard) | Same as inactive + focus ring | Same as inactive | `--font-weight-normal` (400) | 2px solid `--color-focus-ring`, offset 2px inset |

**Interaction:**
- **Click:** Switches to this tab (mouse).
- **Middle-click:** Closes tab (with confirmation if process is running per FS-PTY-008).
- **Double-click on title area:** Enters inline rename mode (FS-TAB-006).
- **Right-click:** Opens tab context menu.
- **Drag:** Initiates tab reorder (FS-TAB-005).
- **Keyboard:** Focus via Tab key within tablist; Left/Right arrows navigate between tabs; Enter/Space activates; Delete closes.

#### 7.1.3 Tab Activity Indicators

Activity indicators appear between the title text and the close button. They communicate background activity per FS-NOTIF-001 through FS-NOTIF-004.

| Indicator | Visual | Color | Icon/Shape | When |
|-----------|--------|-------|------------|------|
| Output activity | Filled dot, `--size-badge` (6px) diameter | `--color-activity` (`#78c078`) | CSS filled circle | Non-active tab/pane produces output (FS-NOTIF-001) |
| Process ended (exit 0) | Icon, `--size-icon-sm` (14px) | `--color-process-end` (`#9c9890`) | Lucide `CheckCircle` | Process exits with status 0 in non-active tab (FS-NOTIF-002) |
| Process ended (non-zero) | Icon, `--size-icon-sm` (14px) | `--color-error` (`#c44444`) | Lucide `XCircle` | Process exits with non-zero status in non-active tab (FS-NOTIF-002) |
| Bell | Icon, `--size-icon-sm` (14px) | `--color-bell` (`#d48a20`) | Lucide `Bell` | BEL received in non-active tab (FS-NOTIF-004) |

All indicators are cleared when the user switches to the indicated tab (FS-NOTIF-003). When multiple indicators would be active simultaneously, the most recent event takes priority.

**Scroll arrow aggregated activity badges:** When tabs overflow the tab bar and some are hidden off-screen, the scroll arrow buttons (ChevronLeft / ChevronRight) each display a small dot badge (`--size-scroll-arrow-badge`, 4px diameter) in the top-right corner of the arrow button when any hidden tab on that side has an active notification indicator. The badge color is `--color-indicator-output` (`#78c078`) for output activity or `--color-indicator-bell` (`#d48a20`) for bell activity. Bell takes priority if both are present on the same side.

#### 7.1.4 Tab Close Button

- **Size:** `--size-target-min` (44px) hit area; visual icon `--size-icon-sm` (14px).
- **Icon:** Lucide `X`.
- **Resting color:** `--color-tab-close-fg` (`#6b6660`).
- **Hover color:** `--color-tab-close-hover-fg` (`#ccc7bc`); background `--color-hover-bg` (`#2c2921`), `--radius-sm`.
- **Active (pressed):** Background `--color-active-bg` (`#35312a`).
- **Focus ring:** 2px solid `--color-focus-ring`, offset 2px.
- **Behavior:** On click, closes the tab. If a foreground process is running, triggers confirmation dialog (FS-PTY-008).
- **Visibility:** Always visible on active tab. On inactive tabs, visible only on hover over the tab item.

**Keyboard accessibility note:** The close button carries `tabindex="-1"` and is intentionally excluded from sequential Tab navigation. Keyboard users close a tab by focusing the tab item and pressing Delete. The close button's `focus-visible` ring remains active for programmatic focus (e.g. assistive technology scripts). This avoids three focus stops per tab (tab-item → close-button → next-tab-item) which would degrade keyboard navigation flow.

#### 7.1.5 New Tab Button

- **Position:** After the last tab item in the tab bar.
- **Size:** `--size-target-min` (44px) x `--size-tab-height` (40px).
- **Icon:** Lucide `Plus`, `--size-icon-sm` (14px).
- **Resting color:** `--color-tab-new-fg` (`#6b6660`).
- **Hover:** Icon `--color-tab-new-hover-fg` (`#ccc7bc`); background `--color-hover-bg`.
- **Active:** Background `--color-active-bg`.
- **Focus ring:** 2px solid `--color-focus-ring`, offset 2px.
- **ARIA label:** "New tab".
- **Tooltip:** "New Tab (Ctrl+Shift+T)" — shown after `--duration-slow` (300ms) hover delay.
- **Overflow behaviour (FS-TAB-009):** If the tab bar is in horizontal scroll mode when the new tab is created, the tab bar scrolls to bring the new tab into view (see §12.2).

#### 7.1.6 Tab Inline Rename

Triggered by double-clicking the tab title or pressing F2 when a tab has keyboard focus (FS-TAB-006).

- The tab title text is replaced by a text input field.
- **Input field:** Background `--term-bg` (`#16140f`), border 1px solid `--color-focus-ring` (`#4a92bf`), `--radius-sm`, text color `--color-tab-active-fg` (`#e8e3d8`), font matches tab title.
- **Width:** Expands to fill available tab width minus close button and padding.
- **Behavior:** Input is pre-filled with current label (or empty if reverting). Enter confirms. Escape cancels. Clicking outside confirms. Empty submission clears the user label and reverts to process/OSC-driven title.
- **Focus:** The input field receives focus immediately on activation.

#### 7.1.7 SSH Badge (on Tab)

For tabs hosting SSH sessions (FS-SSH-002):

- **Position:** Before the title text, with `--space-1` (4px) gap.
- **Visual:** Rounded rectangle (`--radius-sm`), padding `--space-1` horizontal.
- **Content:** Lucide `Network` icon at `--size-icon-sm` (14px).

**States by SSH lifecycle (FS-SSH-010):**

| SSH State | Badge Background | Badge Icon Color | Icon |
|-----------|-----------------|------------------|------|
| Connecting | transparent | `--color-ssh-connecting-fg` (`#d48a20`) | Lucide `Network` (with rotation animation) |
| Authenticating | transparent | `--color-ssh-connecting-fg` (`#d48a20`) | Lucide `Network` (pulsing opacity) |
| Connected | `--color-ssh-badge-bg` (`#1a3a52`) | `--color-ssh-badge-fg` (`#7ab3d3`) | Lucide `Network` |
| Disconnected | `--color-ssh-disconnected-bg` (`#3d1212`) | `--color-ssh-disconnected-fg` (`#d97878`) | Lucide `WifiOff` |
| Closed | transparent | `--color-text-muted` (`#6b6660`) | Lucide `XCircle` |

The **Closed** state represents a session explicitly closed by the user (not a network drop). It is visually distinct from Disconnected: muted color, `XCircle` icon, no error background.

#### 7.1.8 SSH Connections Toggle Button

A fixed-width button anchored to the right edge of the tab row, outside the scrollable tab area. It toggles the SSH connections panel (§7.7, FS-SSH-031).

- **Position:** Right-most element of the tab row — a direct flex sibling of the tab bar zone, NOT inside it. It must remain fully visible at all times regardless of how many tabs are open.
- **Size:** `--size-target-min` (44px) × `--size-tab-height` (40px). `flex-shrink: 0` — never shrinks.
- **Icon:** Lucide `Network`, `--size-icon-sm` (16px).
- **Left border:** 1px solid `--color-border` (visual separator from the tab area).
- **Bottom border:** 1px solid `--color-border`.
- **Resting color:** `--color-text-secondary`.
- **Hover:** Icon `--color-text-primary`; background `--color-hover-bg`.
- **Active (panel open):** Icon `--color-accent`; background `--color-tab-active-bg`.
- **Focus ring:** 2px solid `--color-focus-ring`, offset 2px.
- **ARIA label:** "Open SSH connections" / "Close SSH connections" (toggles). `aria-pressed` reflects open/closed state.
- **Tooltip:** "SSH Connections" — shown after `--duration-slow` (300ms) hover delay.

**Constraint:** Because the tab bar zone uses `flex: 1 0 0` (basis 0, shrink 0), the toggle always receives its full 44px regardless of tab count. This is a hard layout requirement — do not set `flex-basis: auto` on the tab bar zone.

### 7.2 Pane Divider

- **Orientation:** Vertical (for left/right split) or horizontal (for top/bottom split).
- **Visual width:** 1px solid `--color-divider` (`#35312a`).
- **Hit area:** `--size-divider-hit` (8px) centered on the visual line.
- **Cursor:** `col-resize` for vertical divider, `row-resize` for horizontal divider.

**States:**

| State | Visual Line Color | Cursor |
|-------|------------------|--------|
| Default | `--color-divider` (`#35312a`) | default |
| Hover | `--color-divider-active` (`#4a92bf`) | `col-resize` / `row-resize` |
| Dragging | `--color-divider-active` (`#4a92bf`) | `col-resize` / `row-resize` |

**Behavior:** Drag to resize adjacent panes. Minimum pane size is 20 columns wide and 5 rows tall (calculated from current font size and cell dimensions). Double-click on divider resets adjacent panes to equal size.

#### 7.2.1 Pane Activity Indicators (Inactive Panes)

For inactive (visible but not focused) panes in a split layout, the pane border provides activity feedback:

- **Output activity:** On output in an inactive pane, the pane border (`--color-pane-border-inactive`) briefly pulses to `--color-indicator-output` (`#78c078`) for 800ms, then returns to `--color-pane-border-inactive`. This is a CSS `border-color` animation with `--ease-out` easing.
- **Bell activity:** On BEL received in an inactive pane, the pane border pulses to `--color-indicator-bell` (`#d48a20`) for 800ms, then returns to `--color-pane-border-inactive`. Same animation mechanics.
- **Process exit:** On process exit (zero or non-zero) in an inactive pane, the pane border turns `--color-error` (`#c44444`) for 1500ms, then returns to `--color-pane-border-inactive`.
- **Reduced motion:** When `prefers-reduced-motion: reduce` is active, the border color changes instantly (no transition) and holds for the specified duration before reverting.

### 7.3 Terminal Area

#### 7.3.1 Cursor Styles

The cursor style is determined by the running application via DECSCUSR (FS-VT-030) and is user-configurable as a default in preferences (FS-PREF-006).

| Style | Appearance | Token Usage |
|-------|-----------|-------------|
| Block (steady) | Filled rectangle covering the cell | Fill: `--term-cursor-bg` (`#7ab3d3`); character: `--term-cursor-fg` (`#16140f`) |
| Block (blinking) | Same as steady, toggling visibility | On/off at configurable rate (default 530ms per FS-VT-032) |
| Underline (steady) | `--size-cursor-underline-height` horizontal line at cell bottom | Color: `--term-cursor-bg` (`#7ab3d3`) |
| Underline (blinking) | Same as steady, toggling visibility | On/off at configurable rate |
| Bar (steady) | `--size-cursor-bar-width` vertical line at cell left edge | Color: `--term-cursor-bg` (`#7ab3d3`) |
| Bar (blinking) | Same as steady, toggling visibility | On/off at configurable rate |

**Unfocused state (FS-VT-034):** When the pane loses focus, the cursor renders as a hollow outline rectangle of the current shape in `--term-cursor-unfocused` (`#7ab3d3`). Never filled, never invisible.

#### 7.3.2 Selection Highlight

- **Focused pane:** Selected cells use `--term-selection-bg` (`#2e6f9c`) as background. Foreground is `--term-selection-fg` (`inherit` — preserves original text color).
- **Unfocused pane:** Selected cells use `--term-selection-bg-inactive` (`#1a3a52`).
- **Selection operates on cell boundaries** (FS-CLIP-001), not pixel boundaries.

#### 7.3.3 Scrollbar

- **Width:** `--size-scrollbar-width` (8px).
- **Position:** Right edge of the pane, overlaying terminal content (no layout displacement).
- **Track:** `--color-scrollbar-track` (transparent).
- **Thumb:** `--color-scrollbar-thumb` (`#4a4640`), `--radius-full` (pill shape).
- **Thumb min height:** 32px (ensures grabbable target even with large scrollback).
- **Z-index:** `--z-scrollbar` (15).

**States:**

| State | Thumb Color | Track | Visibility |
|-------|------------|-------|------------|
| Idle (at bottom, all fits) | hidden | hidden | Hidden when the viewport is at the bottom of the scrollback buffer **and** the buffer contains no content above the current viewport (i.e., all output fits within the visible area) |
| Idle (viewport scrolled up) | `--color-scrollbar-thumb` (`#4a4640`) | transparent | Visible, fades out after 1.5s of no interaction using `--duration-slow` (300ms) |
| Hover (over thumb) | `--color-scrollbar-thumb-hover` (`#6b6660`) | transparent | Visible |
| Dragging | `--color-scrollbar-thumb-hover` (`#6b6660`) | transparent | Visible |
| Hover (over scrollbar area) | `--color-scrollbar-thumb` (`#4a4640`) | transparent | Visible (thumb appears on hover in scrollbar zone) |

### 7.4 Search Overlay

Triggered by Ctrl+Shift+F or context menu "Search" (FS-SEARCH-007).

#### 7.4.1 Layout

- **Position:** Top-right corner of the active pane, with `--space-2` (8px) offset from top and right edges.
- **Width:** `min(var(--size-search-overlay-width), calc(100% - 2 * var(--space-md)))` — the overlay shrinks to fit the pane with `--space-md` (`--space-4`, 16px) margin on each side when the pane is narrower than 360px.
- **Height:** Auto (content-driven, single row of controls).
- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border` (`#35312a`).
- **Border radius:** `--radius-md` (4px).
- **Shadow:** `--shadow-raised`.
- **Z-index:** `--z-search` (20).
- **ARIA role:** `search`.

#### 7.4.2 Anatomy

```
+-[Search input field]--[Match count]--[Prev][Next]--[Close]-+
+------------------------------------------------------------+
```

- **Search input:** Flex-grow, `--font-size-ui-base` (13px), placeholder "Search..." in `--color-text-tertiary`. Background `--term-bg` (`#16140f`), border 1px solid `--color-border`, `--radius-sm`. On focus: border `--color-focus-ring`.
- **Match count:** `--font-size-ui-sm` (12px), `--color-text-secondary`. Format: "3 of 42" or "No matches". Min-width: 64px to prevent layout shift.
- **Prev/Next buttons:** `--size-target-min` (44px) hit area, icon `--size-icon-sm` (14px). Lucide `ChevronUp` (prev) and `ChevronDown` (next). Standard button hover/active states (§7.14).
- **Close button:** Lucide `X`, `--size-icon-sm` (14px). Standard close button styling.
- **Internal padding:** `--space-2` (8px).
- **Gap between elements:** `--space-1` (4px).

#### 7.4.3 Search Match Highlighting

- **Non-active matches:** Background `--term-search-match-bg` (`#4d3000`), foreground `--term-search-match-fg` (`#e8b060`).
- **Active (current) match:** Background `--term-search-active-bg` (`#6b5c22`), foreground `--term-search-active-fg` (`#e8e3d8`). The active match scrolls to center in the viewport (FS-SEARCH-006).

#### 7.4.4 Interaction

- **Enter:** Navigate to next match.
- **Shift+Enter:** Navigate to previous match.
- **Escape:** Close search overlay, clear highlights.
- **Search executes on each keystroke** (incremental search). First result appears per FS-SEARCH-005.

### 7.5 Connection Status Indicator

Displayed in the status bar (right-aligned) for SSH sessions, and as a badge on the tab (§7.1.7).

#### 7.5.1 Status Bar Indicator

For the active pane, when it hosts an SSH session:

| SSH State | Text | Icon | Color |
|-----------|------|------|-------|
| Connecting | "Connecting to {host}..." | Lucide `Network` (rotating) | `--color-ssh-connecting-fg` (`#d48a20`) |
| Authenticating | "Authenticating..." | Lucide `Network` (pulsing) | `--color-ssh-connecting-fg` (`#d48a20`) |
| Connected | "{user}@{host}" | Lucide `Network` | `--color-ssh-badge-fg` (`#7ab3d3`) |
| Disconnected | "Disconnected" | Lucide `WifiOff` | `--color-ssh-disconnected-fg` (`#d97878`) |
| Closed | "Closed" | Lucide `XCircle` | `--color-text-muted` (`#6b6660`) |

For local sessions, no connection indicator is shown in the status bar — the absence of an indicator is the signal for "local."

#### 7.5.2 Disconnection Overlay (in-pane)

When an SSH session transitions to Disconnected (FS-SSH-022):

- A horizontal banner appears at the bottom of the pane, above the last terminal output.
- **Background:** `--color-error-bg` (`#3d1212`).
- **Border-top:** 1px solid `--color-error` (`#c44444`).
- **Height:** Auto (content-driven), minimum `--size-target-min` (44px).
- **Padding:** `--space-3` (12px).
- **Layout:** Flex row. Left: Lucide `WifiOff` icon (`--size-icon-md`, 16px) + disconnect reason text. Right: "Reconnect" button (primary variant, §7.14).
- **Reason text:** `--font-size-ui-base` (13px), `--color-error-text` (`#d97878`). Human-readable per FS-UX-001 (e.g., "Connection lost: server did not respond to keepalive.").
- **Reconnect button:** Labeled "Reconnect" with Lucide `RefreshCw` icon. Primary button variant.

**Reconnect transition state:** After the user clicks Reconnect, the banner transitions to a reconnecting state:
- The icon changes to Lucide `Network` with rotation animation, color `--color-ssh-connecting-fg` (`#d48a20`).
- The text changes to "Reconnecting to {host}..." in `--color-warning-text` (`#e8b060`).
- The Reconnect button is replaced by a "Cancel" ghost button (§7.14), which aborts the reconnection attempt.
- Terminal content remains visible behind the banner.
- **On success:** The banner dismisses and the reconnection separator (§7.19) appears in the scrollback.
- **On failure:** The banner reverts to the Disconnected state with the reason text updated to reflect the new failure (e.g., "Reconnection failed: connection refused.").

### 7.6 Preferences Panel

Triggered by Ctrl+, or the Settings button in the status bar (FS-PREF-005).

#### 7.6.1 Layout

- **Type:** Modal overlay (centered, with backdrop).
- **Backdrop:** `--color-bg-overlay` at 60% opacity.
- **Panel width:** `--size-preferences-panel-width` (640px).
- **Panel max-height:** 80vh.
- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border`.
- **Border radius:** `--radius-md` (4px).
- **Shadow:** `--shadow-overlay`.
- **Z-index:** `--z-overlay` (40).
- **Internal padding:** `--space-6` (24px).

#### 7.6.2 Internal Structure

```
+-[Header: "Preferences"  ----------------------------  [X]]--+
+-[Section Nav]--+-[Section Content]---------------------------+
|  Keyboard       |                                          |
|  Appearance     |  (scrollable content area)               |
|  Terminal       |                                          |
|  Connections    |                                          |
|  Themes         |                                          |
+-----------------+------------------------------------------+
```

**Section navigation order rationale:** Language is not a separate nav section. It is a subsection within Appearance (after Themes quick-select, font, and line height controls). This placement ensures that a user opening Preferences for the first time to change the display language finds it in the first content-heavy section they encounter, without scrolling past Terminal Behavior, Connections, and Themes. Language is a display preference (how the UI looks to the user) — grouping it with font, size, and theme is semantically consistent.

- **Header:** `--font-size-ui-lg` (16px), `--font-weight-semibold`, `--color-text-primary`. Close button (Lucide `X`) top-right.
- **Section navigation:** Left column, width `--size-preferences-nav-width` (180px). Vertical list of section labels. Active section: `--color-accent-text` (`#7ab3d3`), left border 2px solid `--color-accent`. Inactive: `--color-text-secondary`. Hover: `--color-hover-bg` background.
- **Section content:** Right area, scrollable independently if content exceeds height.
- **Section separator:** `--space-6` (24px) between sections within content area.
- **Focus trap:** Keyboard focus is trapped within the panel while open. Tab cycles through section nav, then through form controls in the active section. Escape closes the panel.

#### 7.6.3 Preference Sections (FS-PREF-004)

**Keyboard section:**
- List of configurable application shortcuts (FS-KBD-002).
- Each row: action label (Body level) + keyboard shortcut recorder (§7.17).
- Section heading: "KEYBOARD" in Caption level.

**Appearance section:**
- Theme quick-select dropdown (§7.16) — this is a shortcut for switching the active theme. The full theme management surface (create, edit, duplicate, delete) is in the Themes section below.
- Font family input (text input with monospace preview).
- Font size input (number input, range 8-32).
- Line height input (number input, range 1.0-2.0, step 0.1).
- **Language subsection** — see below.
- Section heading: "APPEARANCE" in Caption level.

**Language subsection (within Appearance):**
- A dropdown (§7.16) listing available locales. v1 options: "English" and "Français" (each option displays the language name in its own language).
- Selecting a locale applies it immediately to all visible UI strings without any page reload or application restart (FS-I18N-004). When the locale changes, all text elements transition smoothly using `opacity` at `--duration-fast` (150ms) — a brief fade that confirms the change happened intentionally without being distracting. This transition applies to all elements bound to the i18n catalogue; terminal content (which is not locale-resolved) is unaffected.
- The selected locale is persisted to `preferences.json` and restored on next launch (FS-I18N-005).
- If `preferences.json` contains an unknown locale code on launch, the application silently falls back to English (FS-I18N-006); no error dialog is shown.
- The dropdown uses standard keyboard navigation (arrow keys to cycle options, Enter to confirm, Escape to cancel).
- Subsection heading: "LANGUAGE" in Caption level, rendered as a minor heading within the Appearance section (visually subordinate to the "APPEARANCE" section heading).
- **Placement rationale:** Language is a display preference — it controls how the UI appears to the user, analogous to font and theme. Placing it within Appearance ensures it is visible immediately when a user opens Preferences without scrolling past multiple sections. This directly addresses discoverability for Sam (UR §2.3 — occasional user, not expected to know the settings structure).

**Terminal Behavior section:**
- Cursor shape selector (dropdown: Block, Underline, Bar).
- Cursor blink rate (number input, ms, default 530).
- Scrollback buffer size (number input, lines, default 10000) with real-time memory estimate below the field (FS-SB-002). Estimate format: "~{N} MB per pane" in `--font-size-ui-sm`, `--color-text-secondary`.
- Bell notification type (dropdown: Visual, Audible, Disabled).
- Word delimiter set (text input, monospace font).
- Section heading: "TERMINAL BEHAVIOR" in Caption level.

**Connections section:**
- Displays an **inline view** of the connection list embedded directly inside the Preferences panel. This is the same connection list content as §7.7.3, rendered inline within the Preferences section content area rather than in a separate slide-in panel. All connection CRUD operations (create, edit, duplicate, delete) are accessible from this inline view.
- The standalone Connection Manager (§7.7, right-side slide-in) remains separately accessible from the tab bar context menu or a dedicated keyboard shortcut. Both views operate on the same underlying connection data.
- Known-hosts import action: "Import from ~/.ssh/known_hosts" button (secondary variant).
- OSC 52 global default toggle.
- Section heading: "CONNECTIONS" in Caption level.

**Themes section:**
- The **Appearance section** provides a quick-select dropdown to switch the active theme. The **Themes section** is the full management surface: creating, editing, duplicating, and deleting themes. The active theme can be changed from either location.
- Theme list with active indicator.
- "Create Theme" button (primary variant).
- Per-theme actions (visible on hover): Edit (opens theme editor, §7.20), Duplicate, Delete (with confirmation; disabled for the default Umbra theme).
- Section heading: "THEMES" in Caption level.


### 7.7 Connection Manager

The Connection Manager is accessible as a standalone right-side slide-in panel (from tab bar context menu or keyboard shortcut) and also as an inline view embedded in the Preferences panel Connections section (§7.6.3). Both views share the same data and list components.

#### 7.7.1 Layout (Standalone Slide-in)

- **Type:** Right-side slide-in panel over the terminal area.
- **Width:** `--size-connection-manager-width` (400px).
- **Height:** Full terminal area height (below tab bar, above status bar).
- **Background:** `--color-bg-raised` (`#2c2921`).
- **Left border:** 1px solid `--color-border`.
- **Shadow:** `--shadow-overlay`.
- **Z-index:** `--z-overlay` (40).
- **Internal padding:** `--space-4` (16px).

#### 7.7.2 Header

- **Title:** "Connections" at Heading level.
- **Close button:** Lucide `X`, top-right.
- **"New Connection" button:** Below title. Primary button variant, full width. Lucide `Plus` icon.

#### 7.7.3 Connection List

Connections are displayed under **collapsible group section headings**. Each group heading shows the group label in `--color-text-muted` (`#6b6660`), `--font-size-ui-sm` (12px, Label level), with a `ChevronDown` toggle icon (`--size-icon-sm`, 14px, `--color-icon-default`). Clicking the heading collapses/expands the group. The chevron rotates 90 degrees clockwise when the group is collapsed, with a 150ms `--ease-out` transition. Connections without an assigned group appear under an implicit "Ungrouped" section at the bottom of the list.

**Connection List Item Anatomy:**
```
+-[SSH icon]--[Label / host:port]----------[Actions ...]--+
|              [user@host]                                  |
+-----------------------------------------------------------+
```

- **Height:** Auto, minimum `--size-target-min` (44px).
- **Padding:** `--space-3` (12px) horizontal, `--space-2` (8px) vertical.
- **Border-bottom:** 1px solid `--color-border-subtle`.
- **Primary text:** Label (if set) or `host:port`, Body level, `--color-text-primary`.
- **Secondary text:** `user@host`, Label level, `--color-text-secondary`.
- **Icon:** Lucide `Server`, `--size-icon-md`, `--color-icon-default`.

**States:**

| State | Background | Border |
|-------|-----------|--------|
| Default | transparent | `--color-border-subtle` bottom |
| Hover | `--color-hover-bg` | unchanged |
| Focus | transparent | 2px solid `--color-focus-ring` (inset) |

**Actions (visible on hover, always accessible via keyboard/context menu):**
- **Open in new tab:** Lucide `ExternalLink`. Tooltip: "Open in new tab".
- **Open in new pane:** Lucide `SplitSquareVertical`. Tooltip: "Open in pane".
- **Edit:** Lucide `Pencil`. Tooltip: "Edit connection".
- **Duplicate:** Lucide `Copy`. Tooltip: "Duplicate".
- **Delete:** Lucide `Trash2`, color `--color-error`. Tooltip: "Delete". Triggers confirmation dialog.

Each action button: `--size-target-min` (44px) hit area, `--size-icon-sm` (14px) icon, `--color-icon-default` resting, `--color-icon-active` on hover.

#### 7.7.4 Connection Edit Form

Displayed inline within the connection manager when creating or editing a connection (replaces the list temporarily, or as a slide-in sub-panel).

**Fields (FS-SSH-030):**
- Label (optional): text input (§7.15).
- Group (optional): text input with autocomplete from existing group names. As the user types, a dropdown of matching existing group names appears below the field (§7.16 dropdown styling). The user may select an existing group or type a new group name.
- Host: text input (required).
- Port: number input (default 22).
- Username: text input (required).
- Authentication method: radio group — "Identity file" or "Password".
  - If identity file: file path input (text input with browse button).
  - If password: password input (stored securely per FS-CRED-001, never displayed in preferences file).
- OSC 52 write toggle: toggle (§7.16) with label "Allow clipboard write from remote" and description "When enabled, the remote server can set your clipboard content."

**Action buttons:** "Save" (primary), "Cancel" (ghost).

### 7.8 Context Menu

Triggered by right-click in the terminal area (FS-A11Y-006) or on tabs.

#### 7.8.1 Terminal Area Context Menu

**Items:**
1. Copy (Lucide `Copy`) — enabled when text is selected
2. Paste (Lucide `ClipboardPaste`)
3. --- separator ---
4. Search (Lucide `Search`)
5. --- separator ---
6. Split Top / Bottom (Lucide `SplitSquareHorizontal`)
7. Split Left / Right (Lucide `SplitSquareVertical`)
8. --- separator ---
9. Close Pane (Lucide `X`) — omitted when only one pane exists

#### 7.8.2 Tab Context Menu

**Items:**
1. New Tab (Lucide `Plus`)
2. --- separator ---
3. Rename (Lucide `Pencil`)
4. Duplicate Tab
5. --- separator ---
6. Split Top / Bottom (Lucide `SplitSquareHorizontal`)
7. Split Left / Right (Lucide `SplitSquareVertical`)
8. --- separator ---
9. Close Tab (Lucide `X`)
10. Close Other Tabs

_Note: "Duplicate Tab" and "Close Other Tabs" are UXD additions not required by FS-TAB. They are included as convenience features pending explicit v1 scope validation._

#### 7.8.3 Context Menu Styling

- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border`.
- **Border radius:** `--radius-md` (4px).
- **Shadow:** `--shadow-raised`.
- **Z-index:** `--z-dropdown` (30).
- **Padding:** `--space-1` (4px) vertical.
- **Min width:** 180px. **Max width:** 280px.
- **ARIA role:** `menu`.

**Menu Item:**
- **Height:** `--size-target-min` (44px).
- **Padding:** `--space-3` (12px) horizontal.
- **Font:** `--font-size-ui-base` (13px), `--color-text-primary`.
- **Icon:** `--size-icon-md` (16px), `--color-icon-default`, positioned left with `--space-2` (8px) gap to text.
- **Shortcut hint:** Right-aligned, `--font-size-ui-sm` (12px), `--font-mono-ui`, `--color-text-secondary`.
- **ARIA role:** `menuitem`.

**Menu Item States:**

| State | Background | Text Color |
|-------|-----------|------------|
| Default | transparent | `--color-text-primary` |
| Hover | `--color-hover-bg` | `--color-text-primary` |
| Focus | `--color-hover-bg` | `--color-text-primary` |
| Active | `--color-active-bg` | `--color-text-primary` |
| Disabled | transparent | `--color-text-tertiary` |

**Separator:** 1px solid `--color-border`, `--space-1` (4px) vertical margin, full width minus `--space-3` (12px) horizontal margin.

### 7.9 Dialog / Modal

Used for confirmations (FS-PTY-008), SSH host key verification (FS-SSH-011), and destructive action confirmations.

#### 7.9.1 Backdrop

- **Color:** `--color-bg-overlay` (`#16140f`) at 60% opacity.
- **Z-index:** `--z-modal-backdrop` (49).
- **Behavior:** Clicking the backdrop does NOT close the dialog (confirmation dialogs require explicit action).

#### 7.9.2 Dialog Panel

- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border`.
- **Border radius:** `--radius-md` (4px).
- **Shadow:** `--shadow-overlay`.
- **Z-index:** `--z-modal` (50).
- **Width:** 420px (small dialog), 560px (medium, e.g., host key verification). Max-width: 90vw.
- **Padding:** `--space-6` (24px).
- **ARIA role:** `alertdialog` (for confirmations) or `dialog` (for informational).

**Anatomy:**
```
+-[Heading]--------------------------------------------+
|  [Body text / description]                       |
|                                                  |
|  [Technical details -- collapsible if present]    |
|                                                  |
|  ----------------------  [Secondary] [Primary]    |
+------------------------------------------------------+
```

- **Heading:** `--font-size-ui-lg` (16px), `--font-weight-semibold`, `--color-text-primary`.
- **Body:** `--font-size-ui-md` (14px), `--color-text-primary`. `--space-3` (12px) top margin.
- **Technical details:** `--font-size-ui-sm` (12px), `--font-mono-ui`, `--color-text-secondary`, background `--color-bg-surface`, `--radius-sm`, `--space-2` padding. Collapsible via a "Show details" toggle.
- **Action buttons:** Right-aligned in a row, `--space-2` (8px) gap between buttons. `--space-6` (24px) top margin from body content. Primary action on the right; secondary on the left.

#### 7.9.3 Destructive Confirmation Dialog

For closing tabs/panes with running processes (FS-PTY-008):
- **Heading:** "Close tab?" or "Close pane?"
- **Body:** "{N} process(es) still running. Closing will terminate them."
- **Primary action:** "Close" — destructive button variant (§7.14).
- **Secondary action:** "Cancel" — ghost button variant. **Cancel is the default focused action** (safe default).

#### 7.9.4 SSH Host Key Verification Dialog (FS-SSH-011)

**First connection:**
- **Width:** 560px (medium).
- **Heading:** "Verify Server Identity"
- **Body:** "TauTerm is connecting to `{host}` for the first time. To confirm you are connecting to the right server, verify the fingerprint below with your server administrator. If you are unsure, click Reject."
- **Technical details (always visible, not collapsible):** Key type (e.g., "ED25519"), SHA-256 fingerprint in `--font-mono-ui`.
- **Primary action:** "Accept" — secondary button variant (non-default, requires deliberate click).
- **Secondary action:** "Reject" — ghost button variant. **Reject is the default focused action** (safe default).

**Key change (MITM warning):**
- **Width:** 560px.
- **Heading:** "Server Identity Changed" — in `--color-error` text.
- **Icon:** Lucide `ShieldAlert` (`--size-icon-lg`, `--color-error`) before heading. A changed server key is treated as a security error (not a warning) because it may indicate a MITM attack. The `ShieldAlert` icon reinforces the security context.
- **Body:** "The identity of `{host}` has changed since your last connection. This may indicate a man-in-the-middle attack. Contact your server administrator to verify this change before accepting."
- **Technical details (always visible):** Two rows — "Previously known: {old fingerprint}" and "Now presenting: {new fingerprint}" in `--font-mono-ui`. Background `--color-warning-bg`, border 1px solid `--color-warning`.
- **Primary action:** "Accept New Key" — destructive button variant (non-default).
- **Secondary action:** "Reject" — ghost button variant. **Reject is the default focused action.**

### 7.10 Tooltip

- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border`.
- **Border radius:** `--radius-md` (4px).
- **Shadow:** `--shadow-raised`.
- **Z-index:** `--z-tooltip` (60).
- **Padding:** `--space-1` (4px) vertical, `--space-2` (8px) horizontal.
- **Font:** `--font-size-ui-xs` (11px), `--color-text-primary`.
- **Max width:** 240px. Text wraps if exceeded.
- **Delay:** Appears after 300ms (`--duration-slow`) of hover. Disappears immediately on mouse leave.
- **Position:** Prefer below the trigger element; flip above if insufficient space below. Horizontally centered on the trigger; shift to stay within viewport.
- **ARIA:** Trigger element has `aria-describedby` pointing to the tooltip. Tooltip has `role="tooltip"`.

### 7.11 Notification / Activity Badge

See §7.1.3 for tab-level activity indicators.

**Visual bell flash (FS-VT-091):** When a BEL is received in the active pane and the bell type is "visual":
- The terminal area border briefly flashes to `--color-bell` (`#d48a20`) for 100ms (`--duration-base`), then returns to normal.
- No layout shift, no overlay — just a border color flash.

### 7.12 Copy Flash Animation (FS-CLIP, UR §6.3)

When text is selected (auto-copy to PRIMARY selection per FS-CLIP-004):
- The selection background briefly flashes to `--term-selection-flash` (`#5499c7`, HSL 205, 55%, 59% — `--term-selection-bg` lightness raised by 20 percentage points) for 80ms (`--duration-fast`), then returns to `--term-selection-bg`. This provides visual confirmation that the selection was registered.
- Implementation: A CSS animation on the selection layer that transitions from `--term-selection-flash` back to `--term-selection-bg`.

### 7.13 First-Launch Context Menu Hint (FS-UX-002)

- **Position:** Bottom-right corner of the terminal area, `--space-4` (16px) offset from edges.
- **Visual:** Pill-shaped container (`--radius-md`), background `--color-bg-raised`, border 1px solid `--color-border-subtle`, `--shadow-raised`.
- **Content:** Text "Right-click for more options" in `--font-size-ui-sm`, `--color-text-secondary`. Mouse icon (Lucide `MousePointerClick` if available, or text-only) preceding text.
- **Padding:** `--space-2` (8px) vertical, `--space-3` (12px) horizontal.
- **Behavior:** Disappears permanently after the user right-clicks in the terminal area once. State is persisted in preferences.
- **Non-blocking:** The hint does not intercept mouse or keyboard events. It is purely visual.

### 7.14 Button Variants

All button variants share: `--radius-sm` (2px), `--font-size-ui-base` (13px), `--font-weight-medium` (500), height `--size-target-min` (44px), horizontal padding `--space-4` (16px). Icons (when present) are `--size-icon-sm` (14px) with `--space-1` (4px) gap to label text.

#### Primary Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | `--color-accent` (`#4a92bf`) | `--color-text-inverted` (`#0e0d0b`) | none |
| Hover | `--color-blue-500` (`#2e6f9c`) | `--color-text-inverted` | none |
| Active | `--color-blue-600` (`#1e4d6e`) | `--color-text-inverted` | none |
| Focus | `--color-accent` | `--color-text-inverted` | 2px solid `--color-focus-ring`, offset 2px `--color-focus-ring-offset` |
| Disabled | `--color-neutral-700` (`#35312a`) | `--color-text-tertiary` (`#6b6660`) | none |

#### Secondary Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | transparent | `--color-accent-text` (`#7ab3d3`) | 1px solid `--color-accent` |
| Hover | `--color-accent-subtle` (`#1a3a52`) | `--color-accent-text` | 1px solid `--color-accent` |
| Active | `--color-blue-700` (`#1a3a52`) | `--color-accent-text` | 1px solid `--color-accent` |
| Focus | transparent | `--color-accent-text` | 2px solid `--color-focus-ring`, offset 2px |
| Disabled | transparent | `--color-text-tertiary` | 1px solid `--color-neutral-700` |

#### Ghost Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | transparent | `--color-text-primary` (`#ccc7bc`) | none |
| Hover | `--color-hover-bg` (`#2c2921`) | `--color-text-primary` | none |
| Active | `--color-active-bg` (`#35312a`) | `--color-text-primary` | none |
| Focus | transparent | `--color-text-primary` | 2px solid `--color-focus-ring`, offset 2px |
| Disabled | transparent | `--color-text-tertiary` | none |

#### Destructive Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | `--color-error` (`#c44444`) | `--color-neutral-100` (`#f5f2ea`) | none |
| Hover | `--color-red-500` (`#9c2c2c`) | `--color-neutral-100` | none |
| Active | `--color-red-700` (`#3d1212`) | `--color-neutral-100` | none |
| Focus | `--color-error` | `--color-neutral-100` | 2px solid `--color-focus-ring`, offset 2px |
| Disabled | `--color-neutral-700` | `--color-text-tertiary` | none |

### 7.15 Text Input / Form Field

#### 7.15.1 Anatomy

```
[Label]
+-------------------------------+
| Placeholder or value text       |
+-------------------------------+
[Helper text or error message]
```

- **Label:** `--font-size-ui-sm` (12px), `--font-weight-medium` (500), `--color-text-secondary`. `--space-1` (4px) bottom margin.
- **Input field:** Height `--size-target-min` (44px). Background `--term-bg` (`#16140f`). Border 1px solid `--color-border`. `--radius-sm` (2px). Horizontal padding `--space-3` (12px). Font `--font-size-ui-base` (13px), `--color-text-primary`.
- **Placeholder:** `--color-text-tertiary` (`#6b6660`).
- **Helper text:** `--font-size-ui-sm` (12px), `--color-text-secondary`. `--space-1` (4px) top margin.
- **Error text:** `--font-size-ui-sm` (12px), `--color-error-text` (`#d97878`). `--space-1` (4px) top margin.

**States:**

| State | Border | Background | Additional |
|-------|--------|-----------|------------|
| Default | 1px `--color-border` | `--term-bg` | — |
| Hover | 1px `--color-neutral-600` (`#4a4640`) | `--term-bg` | — |
| Focus | 2px `--color-focus-ring` (`#4a92bf`) | `--term-bg` | Focus ring replaces border |
| Error | 1px `--color-error` (`#c44444`) | `--term-bg` | Error text shown below |
| Disabled | 1px `--color-border-subtle` | `--color-bg-surface` (`#242118`) | Text `--color-text-tertiary`, cursor not-allowed |

### 7.16 Toggle / Checkbox and Dropdown

#### Toggle

- **Track size:** 36px wide, 20px tall. `--radius-full`.
- **Thumb:** 16px diameter circle, `--radius-full`. 2px inset from track edge.
- **Hit area:** `--size-target-min` (44px) square.

| State | Track | Thumb |
|-------|-------|-------|
| Unchecked | `--color-neutral-700` (`#35312a`) | `--color-neutral-400` (`#9c9890`) |
| Checked | `--color-accent` (`#4a92bf`) | `--color-neutral-100` (`#f5f2ea`) |
| Hover (unchecked) | `--color-neutral-600` | `--color-neutral-300` |
| Hover (checked) | `--color-blue-500` | `--color-neutral-100` |
| Focus | +2px `--color-focus-ring` ring | unchanged |
| Disabled (unchecked) | `--color-neutral-750` | `--color-neutral-600` |
| Disabled (checked) | `--color-blue-700` | `--color-neutral-500` |

**Transition:** Thumb slides 16px (track width - thumb width - 4px inset) over `--duration-base` (100ms) with `--ease-out`.

#### Dropdown / Select

**Closed state:** Identical to text input (§7.15) with a Lucide `ChevronDown` icon (`--size-icon-sm`, `--color-icon-default`) right-aligned inside the field.

**Open state:**
- The dropdown menu appears below the trigger field.
- **Background:** `--color-bg-raised`.
- **Border:** 1px solid `--color-border`.
- **Border radius:** `--radius-md` (4px).
- **Shadow:** `--shadow-raised`.
- **Z-index:** `--z-dropdown` (30).
- **Max height:** 240px (scrollable).
- **Option items:** Same styling as context menu items (§7.8.3). Active/selected option has left border 2px solid `--color-accent` and background `--color-accent-subtle`.

### 7.17 Keyboard Shortcut Recorder

Displayed inline in the Keyboard section of the preferences panel. Each row shows an action name and its current binding.

TauTerm's application shortcuts are intercepted within the WebView — they are not registered as OS-level global shortcuts. This ensures they remain capturable by the shortcut recorder component. Any shortcut involving Super/Meta key that cannot be captured in a WebView context must be excluded from the recorder's accepted input.

**Anatomy:**
```
[Action label]                    [Shortcut display field]
```

- **Shortcut display field:** Width 160px, height `--size-target-min` (44px). `--font-mono-ui`, `--font-size-ui-sm` (12px). Background `--term-bg`. Border 1px solid `--color-border`. `--radius-sm`.

**States:**

| State | Border | Content | Background |
|-------|--------|---------|-----------|
| Inactive | 1px `--color-border` | Current shortcut text (e.g., "Ctrl+Shift+T") in `--color-text-primary` | `--term-bg` |
| Recording | 2px `--color-accent` | "Press keys..." in `--color-accent-text`, pulsing opacity | `--color-accent-subtle` |
| Captured | 2px `--color-success` | New shortcut text in `--color-success-text` | `--term-bg` |
| Conflict | 2px `--color-error` | New shortcut + "Already used by {action}" in `--color-error-text` below the field | `--color-error-bg` |

**Interaction:**
- Click the field or press Enter while focused to enter Recording state.
- Press the desired key combination — it is captured and displayed.
- Press Enter to confirm, Escape to cancel and revert to the previous binding.
- If the captured shortcut conflicts with another binding, the Conflict state is shown with the name of the conflicting action. The user must resolve the conflict (change one of the bindings) before confirming.

### 7.18 Process Terminated Pane (FS-PTY-005, FS-PTY-006)

When a shell process exits, the pane transitions to a terminated state:

- The terminal content remains visible (scrollback preserved).
- A horizontal banner appears at the bottom of the pane.
- **Background:** `--color-bg-surface` (`#242118`).
- **Border-top:** 1px solid `--color-border`.
- **Height:** Auto, minimum `--size-target-min` (44px).
- **Padding:** `--space-3` (12px).
- **Layout:** Flex row. Left: exit status text. Right: "Restart" (primary button) and "Close" (ghost button).

**Exit status text:**
- Exit 0: Lucide `CheckCircle` (`--color-success`, 16px) + "Process exited" in `--color-text-primary`.
- Non-zero: Lucide `XCircle` (`--color-error`, 16px) + "Process exited with code {N}" in `--color-text-primary`. Technical details (signal name if applicable) in `--color-text-secondary` below.

### 7.19 SSH Reconnection Separator (FS-SSH-042)

When an SSH session reconnects, a visual separator is injected into the scrollback buffer at the exact line where the session resumed.

#### Anatomy

A full-width horizontal rule with a left-aligned label:

```
── reconnected [HH:MM:SS] ──────────────────────────────────────
```

If the timestamp is unavailable at the moment of injection, the label is: `── reconnected ──`.

#### Visual Spec

- **Rule:** 1px horizontal line rendered at the vertical center of the label row, spanning the full pane width. Color: `--color-ssh-connected`.
- **Label text:** Left-aligned, overlaid on the rule. Color: `--color-text-secondary`. Font: `--font-ui`, `--font-size-ui-xs` (11px), `--font-weight-normal` (400) — no bold.
- **Padding:** `--space-1` (4px) top and bottom of the separator row.

#### Behavior

- The separator is injected into the scrollback at the moment reconnection is confirmed (SSH session enters Connected state after a prior Disconnected state).
- It is not interactive: not selectable, not clickable. It does not respond to mouse or keyboard events.
- It is a UI overlay rendered by the frontend, not content from the PTY. It does not appear in clipboard copies of terminal content.

**Token references:** `--color-ssh-connected`, `--color-text-secondary`, `--font-ui`, `--font-size-ui-xs`, `--space-1`.

### 7.20 Theme Editor (FS-THEME-003, FS-THEME-004)

The theme editor is displayed within the Preferences panel Themes section (§7.6.3) when the user creates or edits a custom theme. It replaces the theme list content area temporarily (same pattern as the connection edit form in §7.7.4).

#### 7.20.1 Layout

- **Position:** Inline within the Preferences panel section content area (right side).
- **Header:** Theme name input (text input, §7.15, required) + "Back to themes" link/button (ghost, Lucide `ChevronLeft` + "Themes").
- **Scrollable body:** All token fields organized into sections.
- **Footer:** "Save" (primary button) + "Cancel" (ghost button), right-aligned, sticky at the bottom of the scrollable area.

#### 7.20.2 Token Field Sections

Fields are organized into logical groups matching the token architecture:

**Required Tokens (FS-THEME-004)** — these must all be defined for a valid theme:

| Field Label | Token | Input Type |
|-------------|-------|-----------|
| Terminal Background | `--term-bg` | Color picker (§7.20.3) |
| Terminal Foreground | `--term-fg` | Color picker |
| Cursor Color | `--term-cursor-bg` | Color picker |
| Selection Background | `--term-selection-bg` | Color picker |
| ANSI Black | `--term-color-0` | Color picker |
| ANSI Red | `--term-color-1` | Color picker |
| ANSI Green | `--term-color-2` | Color picker |
| ANSI Yellow | `--term-color-3` | Color picker |
| ANSI Blue | `--term-color-4` | Color picker |
| ANSI Magenta | `--term-color-5` | Color picker |
| ANSI Cyan | `--term-color-6` | Color picker |
| ANSI White | `--term-color-7` | Color picker |
| ANSI Bright Black | `--term-color-8` | Color picker |
| ANSI Bright Red | `--term-color-9` | Color picker |
| ANSI Bright Green | `--term-color-10` | Color picker |
| ANSI Bright Yellow | `--term-color-11` | Color picker |
| ANSI Bright Blue | `--term-color-12` | Color picker |
| ANSI Bright Magenta | `--term-color-13` | Color picker |
| ANSI Bright Cyan | `--term-color-14` | Color picker |
| ANSI Bright White | `--term-color-15` | Color picker |

Section heading: "REQUIRED" in Caption level.

**Optional Tokens (FS-THEME-005)** — organized into sub-sections:

_Terminal Surface:_
- Cursor Text (`--term-cursor-fg`), Cursor Unfocused (`--term-cursor-unfocused`), Selection Foreground (`--term-selection-fg`), Selection Background Inactive (`--term-selection-bg-inactive`), Search Match BG (`--term-search-match-bg`), Search Match FG (`--term-search-match-fg`), Search Active BG (`--term-search-active-bg`), Search Active FG (`--term-search-active-fg`), Hyperlink FG (`--term-hyperlink-fg`), Hyperlink Underline (`--term-hyperlink-underline`).

_Typography:_
- Terminal Font (`--font-terminal`): text input with monospace preview.
- Terminal Font Size (`--font-size-terminal`): number input, range 8-32.
- Terminal Line Height (`--line-height-terminal`): number input, range 1.0-2.0, step 0.1.

_UI Backgrounds:_
- Base (`--color-bg-base`), Surface (`--color-bg-surface`), Raised (`--color-bg-raised`), Overlay (`--color-bg-overlay`).

_UI Text:_
- Primary (`--color-text-primary`), Secondary (`--color-text-secondary`), Tertiary (`--color-text-tertiary`), Inverted (`--color-text-inverted`), Heading (`--color-text-heading`).

_UI Accent:_
- Accent (`--color-accent`), Accent Subtle (`--color-accent-subtle`), Accent Text (`--color-accent-text`), Focus Ring (`--color-focus-ring`).

_UI Borders:_
- Border (`--color-border`), Border Subtle (`--color-border-subtle`), Divider (`--color-divider`), Divider Active (`--color-divider-active`).

_UI Components:_
- All `--color-tab-*`, `--color-pane-*`, `--color-scrollbar-*`, `--color-ssh-*` tokens — each as a color picker field.

_Status:_
- Error (`--color-error`), Error BG (`--color-error-bg`), Error Text (`--color-error-text`), Warning (`--color-warning`), Warning BG (`--color-warning-bg`), Warning Text (`--color-warning-text`), Success (`--color-success`), Success Text (`--color-success-text`), Activity (`--color-activity`), Bell (`--color-bell`), Process End (`--color-process-end`).

Each optional sub-section heading uses Caption level. Optional fields that are left empty inherit from the Umbra default — this is indicated by placeholder text "Inherited from Umbra" in `--color-text-tertiary`.

#### 7.20.3 Color Picker

Each color field consists of:
- **Color swatch:** 24px x 24px square, `--radius-sm`, displaying the current color value. Border 1px solid `--color-border`.
- **Hex input:** Text input (§7.15 styling), width 100px, `--font-mono-ui`, placeholder "#000000". Accepts hex (3, 4, 6, or 8 digit), `rgb()`, `hsl()`, and `oklch()` CSS color values.
- **Click on swatch:** Opens a popover color picker (positioned below the swatch). The popover contains:
  - A 200px x 160px saturation/lightness gradient area (click/drag to select).
  - A hue slider (horizontal, full width).
  - An opacity slider (horizontal, full width) — only shown if the token semantically supports opacity.
  - The hex input is synchronized bidirectionally with the picker.
- **Popover styling:** `--color-bg-raised`, `--shadow-raised`, `--radius-md`, `--z-dropdown`, `--space-2` internal padding.

#### 7.20.4 Contrast Advisory Warnings

Contrast warnings appear inline below the relevant color fields. They are non-blocking (the user may save the theme regardless) and advisory.

- **Foreground on background:** Below the `--term-fg` field, if the contrast ratio of `--term-fg` on `--term-bg` is below 4.5:1, display: "Contrast {ratio}:1 — below WCAG AA minimum (4.5:1)" in `--color-warning-text` (`#e8b060`), `--font-size-ui-sm`. Lucide `AlertTriangle` icon (`--size-icon-sm`, `--color-warning`) precedes the text.
- **Cursor on background:** Below `--term-cursor-bg`, if contrast on `--term-bg` is below 3:1: "Cursor contrast {ratio}:1 — may be hard to see (minimum 3:1 recommended)" in `--color-warning-text`.
- **ANSI palette:** Below each ANSI color field (`--term-color-1` through `--term-color-7`, `--term-color-9` through `--term-color-15`), if contrast on `--term-bg` is below 4.5:1: "Contrast {ratio}:1 against terminal background" in `--color-warning-text`.

Warnings update in real-time as the user adjusts colors.

#### 7.20.5 Validation and Save

- **Required tokens:** All 20 required tokens (§7.20.2) must have valid color values before saving. If any are missing or invalid, the Save button is disabled and the invalid fields show the Error state (§7.15).
- **Valid color format:** All color values must parse as valid CSS color values. Invalid input shows Error state on the hex input field with error text "Invalid color value".
- **On save:** The theme is persisted to preferences. If this is the newly created theme, it becomes the active theme. The editor returns to the theme list.
- **On cancel:** All changes are discarded. The editor returns to the theme list.

### 7.21 Deprecated SSH Algorithm Warning Banner (FS-SSH-014)

When an SSH connection is established and the negotiated host key algorithm is deprecated (specifically: `ssh-rsa` with SHA-1, or `ssh-dss`), a non-blocking inline banner appears at the top of the pane.

#### 7.21.1 Anatomy

```
+-[Icon]--[Warning text]--------------------------------------[X]-+
+------------------------------------------------------------------+
```

- **Position:** Top of the pane, above terminal content. Terminal content flows below the banner (the banner displaces content downward, not overlays it).
- **Background:** `--color-warning-bg` (`#4d3000`).
- **Border-bottom:** 1px solid `--color-warning` (`#d48a20`).
- **Height:** Auto (content-driven), minimum `--size-target-min` (44px).
- **Padding:** `--space-3` (12px) horizontal, `--space-2` (8px) vertical.
- **Layout:** Flex row. Left: Lucide `AlertTriangle` icon (`--size-icon-md`, 16px, `--color-warning`). Center: warning text (flex-grow). Right: dismiss button.

#### 7.21.2 Content

- **Warning text:** `--font-size-ui-base` (13px), `--color-warning-text` (`#e8b060`). Format: "Connection uses deprecated algorithm: {algorithm name}. The server should be updated to use a modern key exchange algorithm."
  - Example: "Connection uses deprecated algorithm: ssh-rsa (SHA-1). The server should be updated to use a modern key exchange algorithm."
- **Gap between icon and text:** `--space-2` (8px).

#### 7.21.3 Dismiss Button

- **Icon:** Lucide `X`, `--size-icon-sm` (14px).
- **Hit area:** `--size-target-min` (44px).
- **Icon color:** `--color-warning-text` (`#e8b060`) resting, `--color-neutral-100` (`#f5f2ea`) on hover.
- **Background:** transparent resting, `--color-warning` (`#d48a20`) at 20% opacity on hover, `--radius-sm`.
- **Behavior:** On click, the banner is dismissed permanently for this pane session. The banner does not reappear unless a new connection is established to the same host. Dismissal state is not persisted across application restarts — the warning reappears on each new connection to a server using deprecated algorithms.

#### 7.21.4 Interaction

- **Non-blocking:** The banner does not prevent terminal interaction. Keyboard input passes through to the PTY. The terminal content is visible and scrollable below the banner.
- **Keyboard:** The dismiss button is focusable and activatable via Enter/Space. It appears in the tab order after the pane's terminal content focus.

---

## 8. Interaction Patterns

### 8.1 Mouse Interactions

| Interaction | Response Time | Feedback |
|------------|---------------|----------|
| Hover on interactive element | `--duration-instant` (0ms) | Background changes to hover state |
| Click on button | `--duration-instant` (0ms) | Background changes to active state |
| Click release | `--duration-instant` (0ms) | Returns to hover state (if still hovering) or default |
| Double-click on tab title | `--duration-instant` (0ms) | Enters inline rename mode |
| Right-click | `--duration-instant` (0ms) | Context menu appears |
| Drag start (tab reorder) | After 4px of movement | Tab lifts visually (shadow appears), ghost position indicator shown |
| Drag start (divider resize) | `--duration-instant` (0ms) | Divider color changes to active |

### 8.2 Focus Management

- **Focus ring style:** 2px solid `--color-focus-ring` (`#4a92bf`), with 2px offset in `--color-focus-ring-offset` (`#0e0d0b`). Applied via `outline` property (not `box-shadow`) for correct behavior across border-radius values.
- **Focus ring timing:** `--duration-instant` (0ms) — focus rings appear and disappear instantly (no transition).
- **Tab order:** Tab bar tabs → Tab bar new-tab button → Terminal area → Status bar elements. Within the terminal area, Tab key is captured by the PTY; pane navigation uses dedicated shortcuts.
- **Focus trap in modals:** When a dialog or the preferences panel is open, Tab key cycles only through focusable elements within the modal. Shift+Tab cycles backward. Focus starts on the default action (typically the safe/cancel action for destructive dialogs).
- **Focus restoration:** When a modal closes, focus returns to the element that triggered it.
- **Auto-focus on active pane (FS-UX-003):** The active terminal pane's viewport receives keyboard focus automatically — without requiring a mouse click — in three situations: (1) on application launch, (2) when a new tab is created, (3) when the user switches to a different tab. Focus is applied immediately after mount, without scrolling the page. This does not apply to terminated panes.

**Input fields:** Use an inset outline to keep the focus ring within the field's border, avoiding visual overlap with adjacent elements. Use `outline` rather than `box-shadow` for focus rings on inputs, so the ring respects `prefers-reduced-motion` and renders correctly in clipped containers.

### 8.3 Scroll Behavior

#### 8.3.1 Scroll Policy

TauTerm uses a **position-freeze + passive indicator** scroll policy.

- **Mouse wheel in terminal:** Scrolls scrollback buffer. Scroll direction matches system setting. Scroll amount: 3 lines per wheel tick (configurable by OS).
- **Position freeze during output:** When the user has scrolled into the scrollback and new output arrives from the PTY, the viewport stays at its current position. No auto-scroll occurs. This allows reading historical output without interruption (FS-SB-009).
- **Automatic return to live on PTY input:** When the user sends keyboard input to the PTY while scrolled into the scrollback, the viewport returns to the live bottom instantly. No user action is required (FS-SB-010).
- **Manual return to live:** Click `ScrollToBottomButton` or press `End` to reset `scroll_offset` to 0 immediately.
- **Smooth vs. instant:** Programmatic scrolling (search navigation, scroll-to-bottom, PTY-input auto-return) is instant (no smooth scroll). User scrolling (mouse wheel, scrollbar drag) is handled natively by the OS.

**Not in v1:** line-count badge on the button, tail-mode toggle, auto-scroll on output, Escape interception (deferred — potential conflict with vim and alternate-screen applications).

#### 8.3.2 ScrollToBottomButton Component

A passive indicator that appears whenever `scroll_offset > 0` to signal that the viewport is not at the live bottom.

**Position and shape:**
- `position: absolute`, anchored to bottom-right of the terminal viewport
- Offset from each edge: `var(--space-3)`
- `z-index: var(--z-scrollbar)` (15)
- `border-radius: var(--radius-full)` (pill shape)
- Minimum size: 33×33px

**Anatomy:** Lucide `ArrowDown` icon at 16px. No text label.

**Visibility:** Rendered only when `scroll_offset > 0`. Hidden (not merely transparent) when `scroll_offset === 0`.

**Entrance / exit transition:**
- Appearance: `opacity` 0 → 1, `var(--duration-fast) ease-out`
- Disappearance: `opacity` 1 → 0, `var(--duration-fast) ease-out`
- `prefers-reduced-motion: reduce`: transition suppressed entirely (instant show/hide).

**Visual states:**

| State | Background | Border | Icon color | Shadow |
|-------|-----------|--------|-----------|--------|
| Idle | `var(--color-bg-raised)` | `1px solid var(--color-border)` | `var(--color-icon-default)` | `var(--shadow-raised)` |
| Hover | `var(--color-hover-bg)` | `1px solid var(--color-border)` | `var(--color-icon-active)` | `var(--shadow-raised)` |
| Active | `var(--color-active-bg)` | `1px solid var(--color-border)` | `var(--color-icon-active)` | none |
| Focus | `var(--color-bg-raised)` | `1px solid var(--color-border)` | `var(--color-icon-active)` | `var(--shadow-raised)` + focus ring 2px `var(--color-focus-ring)` offset `var(--color-focus-ring-offset)` |

**Accessibility:**
- `role="button"`, `tabindex="0"`
- `aria-label` bound to i18n key `scroll_to_bottom`
- Minimum hit target: 33×33px (the pill itself); surrounding spacing brings the effective touch area to ≥ 44px when combined with the `var(--space-3)` offset from the viewport edge

### 8.4 Drag & Drop

#### Tab Reorder (FS-TAB-005)

- **Initiation:** The drag initiation threshold is delegated to the native HTML5 DnD API mechanism (managed by the OS/compositor). No additional application-level threshold is implemented, as the native threshold is sufficient to prevent accidental drags.
- **Visual feedback:** The dragged tab gets `--shadow-raised`, opacity 0.9. A 2px-wide vertical insertion indicator (`--color-accent`) appears between tabs at the target position.
- **Cursor:** `grabbing`.
- **Drop:** Tab moves to the indicated position. No animation on drop (instant repositioning).
- **Cancel:** Drag to outside the tab bar or press Escape. Tab returns to original position.

#### Pane Resize (FS-PANE-003)

- **Initiation:** Mouse down on pane divider hit area.
- **Visual feedback:** Divider line color changes to `--color-divider-active`. Panes resize in real-time (no ghost/preview).
- **Constraints:** Minimum pane dimensions enforced (20 columns, 5 rows). Divider stops at minimum boundaries.
- **Debounce:** Resize events are debounced (FS-PTY-010).

### 8.5 Clipboard

- **Select text (auto-copy to PRIMARY):** Mouse drag to select. On mouse release, text is copied to PRIMARY selection (FS-CLIP-004). Copy flash animation (§7.12) provides visual confirmation.
- **Copy to CLIPBOARD:** Right-click → Copy (FS-CLIP-006), or explicit keyboard shortcut if configured.
- **Paste from CLIPBOARD:** Ctrl+Shift+V (FS-CLIP-005).
- **Multi-line paste warning (FS-CLIP-009):** When bracketed paste is NOT active and pasted text contains newlines, a confirmation dialog appears. Heading: "Paste multi-line text?" Body: "The text contains {N} lines. Pasting multi-line text directly into a terminal may execute commands unintentionally." Action: "Paste" (primary), "Cancel" (ghost, default focus). A toggle "Don't ask again" at the bottom of the dialog, persisted in preferences.

### 8.6 SSH Connection Interruption Feedback

Per FS-SSH-022, disconnection is detected within 1 second:

1. **Immediate (0-1s):** Tab SSH badge transitions to Disconnected state (§7.1.7). Status bar indicator changes to "Disconnected" (§7.5.1).
2. **Pane overlay (after detection):** Disconnection banner appears at bottom of pane (§7.5.2) with reason text and "Reconnect" button.
3. **Terminal content:** Remains visible and scrollable. No content is lost.
4. **Reconnect action:** Accessible from the pane banner (primary button), the tab context menu, and the connection manager. On reconnect, a separator line (§7.19) marks the boundary.

---

## 9. Motion & Animation

### 9.1 Philosophy

Motion in TauTerm is purposeful, brief, and non-distracting. Every animation communicates a state change or provides feedback — none exist for decoration. All animations respect `prefers-reduced-motion: reduce` by being disabled entirely (replaced with instant state changes).

### 9.2 Entrance and Exit Transitions

| Surface | Entrance | Exit | Reduced Motion |
|---------|----------|------|----------------|
| Modal/dialog | Opacity 0→1, `--duration-base` (100ms), `--ease-out` | Opacity 1→0, `--duration-fast` (80ms), `--ease-in` | Instant |
| Tooltip | Opacity 0→1, `--duration-base` (100ms), `--ease-out` | Instant (0ms) | Instant |
| Context menu | Opacity 0→1, `--duration-base` (100ms), `--ease-out` | Instant (0ms) | Instant |
| Search overlay | Opacity 0→1 + translateY(-4px→0), `--duration-base` (100ms), `--ease-out` | Opacity 1→0 + translateY(0→-4px), `--duration-fast` (80ms), `--ease-in` | Instant |
| Preferences panel | Opacity 0→1, `--duration-base` (100ms), `--ease-out` | Opacity 1→0, `--duration-fast` (80ms), `--ease-in` | Instant |
| Connection manager | TranslateX(100%→0), `--duration-base` (100ms), `--ease-out` | TranslateX(0→100%), `--duration-fast` (80ms), `--ease-in` | Instant |
| Dropdown menu | Opacity 0→1 + translateY(-4px→0), `--duration-base` (100ms), `--ease-out` | Instant (0ms) | Instant |
| First-launch hint | Opacity 0→1, `--duration-slow` (300ms), `--ease-out` (delayed 2s after first terminal output) | Opacity 1→0, `--duration-slow`, `--ease-in` | Instant |

### 9.3 State Transitions

| Transition | Duration | Easing | Reduced Motion |
|-----------|----------|--------|----------------|
| Tab switch (active tab change) | Instant (0ms) | — | — |
| Pane resize (live) | Instant (0ms) | — | — |
| Theme switch | Cross-fade `--duration-slow` (300ms), `--ease-linear` | All token values transition simultaneously | Instant |
| Toggle thumb slide | `--duration-base` (100ms) | `--ease-out` | Instant |
| Hover background change | `--duration-instant` (0ms) | — | — |
| Focus ring | `--duration-instant` (0ms) | — | — |
| Scrollbar fade-in | `--duration-base` (100ms) | `--ease-out` | Instant |
| Scrollbar fade-out | `--duration-slow` (300ms) after 1.5s idle | `--ease-in` | Instant |
| Visual bell flash | `--duration-base` (100ms) | `--ease-linear` | Instant (single frame flash) |
| Copy flash | `--duration-fast` (80ms) | `--ease-linear` | None (skip entirely) |
| SSH connecting spinner | Continuous rotation, `--ease-linear`, 1s per revolution | — | Static icon (no rotation) |
| SSH authenticating pulse | Opacity 0.5→1→0.5, `--duration-slow` (300ms) | `--ease-linear` | Static icon |
| Pane border activity pulse | 800ms hold, `--ease-out` return | Border color change | Instant change, 800ms hold, instant return |
| Connection group chevron | 150ms | `--ease-out` | Instant |

### 9.4 `prefers-reduced-motion` Policy

When `prefers-reduced-motion: reduce` is active:
- All entrance/exit animations are replaced with instant opacity changes (0→1 or 1→0, 0ms).
- The toggle thumb jumps instead of sliding.
- The SSH connecting spinner is a static `Network` icon (no rotation).
- The copy flash animation is skipped entirely.
- The visual bell flash is reduced to a single frame (appears for one repaint cycle, then disappears).
- Theme switching applies token changes instantly with no cross-fade.
- Pane border activity pulses change color instantly (no transition), hold for the specified duration, then revert instantly.
- Connection group chevron rotation is instant.

---

## 10. Iconography

### 10.1 Icon Set

**Source:** Lucide-svelte (per CLAUDE.md stack requirement).

**Stroke weight:** 1.5px (Lucide default). Not overridden — heavier strokes would read as aggressive against the restrained chrome. (AD.md §6)

**Color rule:** Icons inherit the text color of their container by default. Override colors: `--color-accent` for active/accent state, `--color-error` for error state, `--color-warning` for warning state, `--color-success` for success state.

### 10.2 Size Variants

| Size | Token | Value | Usage Context |
|------|-------|-------|---------------|
| Small | `--size-icon-sm` | 14px | Tab bar (close button, activity indicators, SSH badge) |
| Medium | `--size-icon-md` | 16px | Toolbars, context menu items, status bar, form field icons, notification icons |
| Large | `--size-icon-lg` | 20px | Dialog headers, connection manager header, large call-to-action icons |

### 10.3 Icon Vocabulary

| Concept | Lucide Icon | Size | Context |
|---------|-------------|------|---------|
| New tab | `Plus` | sm/md | Tab bar new-tab button, context menu |
| Close tab/pane | `X` | sm/md | Tab close button, dialog close, search close, context menu |
| Split top/bottom (horizontal split) | `SplitSquareHorizontal` | md | Context menu — "Split Top / Bottom" |
| Split left/right (vertical split) | `SplitSquareVertical` | md | Context menu — "Split Left / Right" |
| SSH session (connected) | `Network` | sm | Tab SSH badge, status bar |
| SSH disconnected | `WifiOff` | sm/md | Tab SSH badge, disconnection banner |
| SSH closed | `XCircle` | sm | Tab SSH badge (Closed state), status bar |
| SSH reconnect | `RefreshCw` | sm | Reconnect button |
| Process ended (success) | `CheckCircle` | sm | Tab activity indicator, terminated pane banner |
| Process ended (failure) | `XCircle` | sm | Tab activity indicator, terminated pane banner |
| Bell | `Bell` | sm | Tab activity indicator |
| Preferences/Settings | `Settings` | md | Status bar |
| Search | `Search` | md | Context menu, search overlay |
| Copy | `Copy` | md | Context menu |
| Paste | `ClipboardPaste` | md | Context menu |
| Connection manager | `Server` | md/lg | Connection list items |
| Security error (MITM) | `ShieldAlert` | lg | MITM host key change dialog |
| Warning | `AlertTriangle` | md/lg | Warning dialogs, deprecated SSH algorithm banner |
| Error | `AlertCircle` | md | Error messages |
| Drag handle | `GripVertical` | sm | Tab drag affordance (visible on hover) |
| Scroll to bottom | `ArrowDown` | md | Scrollback navigation indicator |
| Edit/Rename | `Pencil` | sm/md | Connection edit, tab rename context menu |
| Duplicate | `Copy` | sm | Connection manager duplicate action |
| Delete | `Trash2` | sm | Connection manager delete action |
| Dropdown indicator | `ChevronDown` | sm | Dropdown/select fields |
| Search prev | `ChevronUp` | sm | Search overlay navigation |
| Search next | `ChevronDown` | sm | Search overlay navigation |
| Tab scroll left | `ChevronLeft` | sm | Tab bar overflow scroll |
| Tab scroll right | `ChevronRight` | sm | Tab bar overflow scroll |
| Open externally | `ExternalLink` | sm | Connection manager "open in new tab" |
| Group expand/collapse | `ChevronDown` / `ChevronRight` | sm | Connection list group headings |

### 10.4 Status Dots

Activity dots (§7.1.3) are CSS-rendered filled circles, not Lucide icons. They use `--size-badge` (6px) diameter with `--radius-full`. This distinction is intentional: filled dots communicate "presence" while outline icons communicate "action." (AD.md §6)

---

## 11. Accessibility

### 11.1 Contrast Audit

All color pairings used in the UI are documented in §5.4 with measured contrast ratios. Summary:

- **All normal text** (under 18px): meets 4.5:1 minimum. Lowest ratio: tab hover text on hover-bg at 4.6:1.
- **All UI components** (borders, icons, non-text): meets 3:1 minimum. Lowest ratio: placeholder/disabled text on surface at 3.1:1.
- **All ANSI palette colors** (indices 1-7, 9-15): meet 4.5:1 minimum against `--term-bg`. Lowest ratio: ANSI Green normal at 4.6:1.
- **Focus ring** (`--color-focus-ring` on `--color-bg-base`): 5.9:1, exceeds the 3:1 UI component minimum.

### 11.2 Keyboard Navigation Map

#### Global Navigation

| Key | Action | Context |
|-----|--------|---------|
| `Tab` | Move focus to next focusable element | Global (outside terminal) |
| `Shift+Tab` | Move focus to previous focusable element | Global |
| `Ctrl+Shift+T` | New tab | Always (application shortcut) |
| `Ctrl+Shift+W` | Close active tab | Always |
| `Ctrl+Tab` | Next tab | Always |
| `Ctrl+Shift+Tab` | Previous tab | Always |
| `Ctrl+,` | Open preferences | Always |
| `Ctrl+Shift+F` | Open search overlay | When terminal pane is focused |
| `Ctrl+Shift+V` | Paste from clipboard | When terminal pane is focused |

#### Tab Bar Navigation

| Key | Action | Context |
|-----|--------|---------|
| `Left` / `Right` | Navigate between tabs | When tab bar has focus |
| `Enter` / `Space` | Activate focused tab | When tab has focus |
| `Delete` | Close focused tab | When tab has focus |
| `F2` | Rename focused tab (inline) | When tab has focus |

_F2 for tab rename is a UXD addition; FS-KBD-003 should be updated to include it in the configurable shortcut list._

#### Pane Navigation

Default shortcuts for pane operations (deferred from FS-KBD-003):

| Key | Action |
|-----|--------|
| `Ctrl+Shift+D` | Split left/right panes |
| `Ctrl+Shift+E` | Split top/bottom panes |
| `Ctrl+Shift+ArrowLeft` | Focus pane to the left |
| `Ctrl+Shift+ArrowRight` | Focus pane to the right |
| `Ctrl+Shift+ArrowUp` | Focus pane above |
| `Ctrl+Shift+ArrowDown` | Focus pane below |
| `Ctrl+Shift+Q` | Close active pane |

These are user-configurable defaults (FS-KBD-002).

#### Modal / Overlay Navigation

| Key | Action | Context |
|-----|--------|---------|
| `Tab` | Cycle through focusable elements within modal | Modal/dialog open |
| `Shift+Tab` | Cycle backward | Modal/dialog open |
| `Escape` | Close modal/overlay | Modal/dialog/search/preferences open |
| `Enter` | Activate focused button | Dialog open |

#### Search Overlay Navigation

| Key | Action |
|-----|--------|
| `Enter` | Next match |
| `Shift+Enter` | Previous match |
| `Escape` | Close search, clear highlights |

### 11.3 ARIA Roles and Landmark Structure

| Element | ARIA Role | Notes |
|---------|-----------|-------|
| Tab bar container | `tablist` | Contains tab items |
| Tab item | `tab` | `aria-selected="true"` for active |
| Terminal pane | `region` | `aria-label="Terminal {N}"` where N is pane number |
| Tab panel (pane container) | `tabpanel` | Associated with its tab via `aria-labelledby` |
| Search overlay | `search` | `aria-label="Search in terminal output"` |
| Context menu | `menu` | — |
| Context menu item | `menuitem` | — |
| Preferences panel | `dialog` | `aria-label="Preferences"`, `aria-modal="true"` |
| Confirmation dialog | `alertdialog` | `aria-modal="true"` |
| Connection manager | `complementary` | `aria-label="Connection Manager"` |
| Status bar | `status` | Live region for SSH state changes |
| Tooltip | `tooltip` | Referenced by trigger's `aria-describedby` |

### 11.4 Reduced Motion Policy

See §9.4. All animations are disabled when `prefers-reduced-motion: reduce` is active. No motion is essential for understanding the UI — all state changes are communicated through color and shape changes that persist without animation.

### 11.5 Touch Target Minimums

All interactive elements have a minimum hit area of `--size-target-min` (44px) in both dimensions. This applies even on desktop, as touchscreen laptops exist (FS-A11Y-002). Where the visual element is smaller than 44px (e.g., close button icon at 14px), the hit area extends invisibly to meet the minimum.

### 11.6 Non-Color Indicators

Per FS-A11Y-004, every status communicated through color also has a secondary indicator:

| Status | Color Signal | Non-Color Signal |
|--------|-------------|-----------------|
| Tab activity (output) | Green dot (`--color-activity`) | Filled circle shape (distinct from all icon shapes) |
| Process ended (success) | Gray icon (`--color-process-end`) | `CheckCircle` icon |
| Process ended (failure) | Red icon (`--color-error`) | `XCircle` icon |
| Bell | Amber icon (`--color-bell`) | `Bell` icon |
| SSH connected | Blue badge | `Network` icon + badge shape |
| SSH disconnected | Red badge | `WifiOff` icon (different from `Network`) |
| SSH connecting | Amber icon | Rotating animation (or static icon in reduced motion) |
| SSH closed | Muted icon | `XCircle` icon (different from `WifiOff`) |
| Active pane | Blue border | 2px border thickness (vs 1px inactive) |
| Active tab | Light text + matched bg | Semibold weight (vs normal weight) |
| Error state | Red text/bg | `AlertCircle` icon + error text content |
| Warning state | Amber text/bg | `AlertTriangle` icon + warning text content |
| Security error (MITM) | Red text/bg | `ShieldAlert` icon + error text content |
| Focus | Blue ring | 2px outline (visible shape change) |
| Deprecated SSH algorithm | Amber banner | `AlertTriangle` icon + descriptive warning text |
| Pane activity (output) | Green border pulse | Temporal change (border color shift for 800ms) |
| Pane activity (bell) | Amber border pulse | Temporal change (border color shift for 800ms) |

---

## 12. Responsiveness & Window Resizing

### 12.1 Minimum Window Size

**640 x 400 pixels.** Enforced at the window manager level. Below this, the UI cannot guarantee:
- 80 columns of terminal text at the default font size
- Readable tab bar with at least one tab
- Usable status bar content

### 12.2 Tab Bar at Narrow Widths

- Tabs maintain their minimum width (120px) regardless of window width.
- When total tab width exceeds available bar width, the tab bar enters **horizontal scroll mode:**
  - Tabs scroll horizontally via mouse wheel on the tab bar area.
  - Left/right scroll indicators (Lucide `ChevronLeft` / `ChevronRight`, 24px wide) appear at the edges when tabs overflow in that direction. These are click targets that scroll by one tab width.
  - Each scroll arrow displays a small dot badge (`--size-scroll-arrow-badge`, 4px, positioned in the top-right corner of the arrow button) when any hidden tab on that side has an active notification indicator. Badge color: `--color-indicator-output` for output activity, `--color-indicator-bell` for bell (bell takes priority).
  - The new-tab button remains fixed at the right edge of the tab bar (does not scroll with tabs).
  - **New-tab scroll-into-view (FS-TAB-009):** When a new tab is created in overflow mode, the tab bar scrolls to bring the new tab into view. The scroll position updates smoothly; if the new tab is already visible, no scroll occurs.
- **At 640px window width with default settings:** Approximately 4-5 tabs fit before scrolling activates.

### 12.3 Preferences Panel at Minimum Size

- The preferences panel has a fixed width of `--size-preferences-panel-width` (640px).
- **When the window is narrower than 680px** (panel width + margin): the panel switches to full-width mode with `--space-4` (16px) margins on left and right.
- **When the window is shorter than 480px:** the panel max-height reduces to fill available height minus `--space-4` margins top and bottom. The section content area scrolls.
- The section navigation sidebar (180px) remains visible. At the minimum, section labels may truncate with ellipsis but remain readable.

### 12.4 Pane Minimum Sizes

- **Minimum pane dimensions:** 20 columns wide, 5 rows tall (calculated from `--font-size-terminal` and cell dimensions).
- **At minimum window size (640x400):** After subtracting tab bar (40px) and status bar (28px), the terminal area is 640x332. This accommodates a single pane of approximately 80 columns by 19 rows at 14px font size. Splitting into panes at this size is permitted but constrained by minimum pane dimensions.

### 12.5 No Mobile Breakpoints

TauTerm is a desktop-only application (UR §10). There are no mobile breakpoints. However, the above graceful degradation rules ensure usability on small windows (e.g., tiling window managers where TauTerm may occupy a quarter of the screen on a 1080p display — approximately 960x540px).

---

## 13. Theme Extensibility

### 13.1 Token Mapping for User Themes

User-created themes (FS-THEME-003) override the same CSS custom properties defined in §3. The theming system operates at the `:root` level — a user theme is a CSS file that redeclares token values.

### 13.2 Required Theme Tokens (FS-THEME-004)

A valid user theme defines at minimum:

| Token | Purpose |
|-------|---------|
| `--term-bg` | Terminal background |
| `--term-fg` | Terminal default foreground |
| `--term-cursor-bg` | Cursor fill color |
| `--term-selection-bg` | Selection background |
| `--term-color-0` through `--term-color-15` | ANSI 16-color palette |

### 13.3 Optional Theme Tokens (FS-THEME-005)

A user theme may also define any of the following:

| Category | Tokens |
|----------|--------|
| Terminal surface | `--term-cursor-fg`, `--term-cursor-unfocused`, `--term-selection-fg`, `--term-selection-bg-inactive`, `--term-search-*`, `--term-hyperlink-*`, `--term-selection-flash` |
| Typography | `--font-terminal`, `--font-size-terminal`, `--line-height-terminal` |
| UI backgrounds | `--color-bg-base`, `--color-bg-surface`, `--color-bg-raised`, `--color-bg-overlay` |
| UI text | `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary`, `--color-text-inverted`, `--color-text-heading` |
| UI accent | `--color-accent`, `--color-accent-subtle`, `--color-accent-text`, `--color-focus-ring` |
| UI borders | `--color-border`, `--color-border-subtle`, `--color-divider`, `--color-divider-active` |
| UI components | All `--color-tab-*`, `--color-pane-*`, `--color-scrollbar-*`, `--color-ssh-*` tokens |
| Status | `--color-error`, `--color-error-bg`, `--color-error-text`, `--color-warning`, `--color-warning-bg`, `--color-warning-text`, `--color-success`, `--color-success-text`, `--color-activity`, `--color-bell`, `--color-process-end` |
| Primitives | All `--color-neutral-*`, `--color-blue-*`, `--color-amber-*`, `--color-red-*`, `--color-green-*` (if the theme defines a wholly different palette) |

> **Note:** Overriding primitive tokens is valid in the context of user theming. The rule that "components never consume primitives directly" applies to component implementation code — components always reference semantic tokens. When a user theme overrides a primitive, the change propagates automatically through all semantic tokens that map to it. This is the intended cascade mechanism for users who want to define a wholly different palette without redefining every semantic token.

Tokens not defined by a user theme inherit from the Umbra default.

### 13.4 Non-Themeable Tokens

The following tokens are structural and are not exposed in the theme editor — user themes cannot override them:

| Token | Reason |
|-------|--------|
| `--space-*` | Spacing affects layout integrity; inconsistent spacing breaks component alignment |
| `--size-tab-height`, `--size-toolbar-height`, `--size-status-bar-height` | Fixed dimensions tied to layout calculations |
| `--size-target-min` | Accessibility requirement (44px minimum) — cannot be reduced |
| `--size-divider-hit` | Interaction requirement — cannot be reduced below 8px |
| `--size-cursor-underline-height`, `--size-cursor-bar-width` | Structural cursor dimensions |
| `--radius-*` | Structural choice of the design system |
| `--shadow-*` | Tied to the elevation model |
| `--z-*` | Stacking order is structural |
| `--duration-*`, `--ease-*` | Motion tokens are structural |
| `--font-ui`, `--font-mono-ui` | UI chrome fonts are fixed to system stack |

### 13.5 Theme Validation Rules

When a user creates or imports a theme, the following validations are performed:

1. **Required tokens present:** All tokens listed in §13.2 must be defined.
2. **Valid color values:** All color tokens must parse as valid CSS color values (hex, rgb, hsl, oklch).
3. **Minimum contrast enforcement:** `--term-fg` on `--term-bg` is checked for a minimum 4.5:1 contrast ratio. If not met, the theme editor displays a warning (non-blocking — the user may save the theme, but the warning persists).
4. **ANSI palette contrast advisory:** Each of `--term-color-1` through `--term-color-7` and `--term-color-9` through `--term-color-15` is checked against `--term-bg`. Any pair below 4.5:1 triggers an advisory warning listing the affected colors.
5. **Cursor visibility:** `--term-cursor-bg` on `--term-bg` is checked for a minimum 3:1 contrast ratio.

Validation failures on required tokens (items 1-2) prevent saving. Contrast warnings (items 3-5) are advisory — they inform the user but do not block saving. This respects user autonomy while making accessibility implications visible.

**Editor chrome accessibility invariant:** During theme editing, the editor's own interface (form labels, input fields, buttons, navigation, validation messages) always renders using the active system tokens (Umbra defaults or the user's last confirmed active theme), not the theme currently being edited. Only the designated preview area — a terminal viewport sample — reflects the work-in-progress custom theme in real time. This invariant ensures the editor remains accessible and usable even when the user is authoring a theme with poor contrast or extreme colors.

The preview area is explicitly bounded (a labeled box within the editor panel) and is distinct from the editor controls. It carries a visible label: "Preview" (using `--color-text-secondary`).

Traceability: FS-A11Y-005 (to be added), FS-PREF-003.

---

## 14. Traceability Matrix

This table maps major UX/UI decisions in this document to their source requirements in UR.md and FS.md.

| UXD Section | Decision | UR Source | FS Source |
|-------------|----------|-----------|-----------|
| §2.1 | Terminal content is primary; chrome at lower visual temperature | UR §2.1 (Alex — low-distraction interface) | — |
| §2.2 | Every feature reachable by mouse and keyboard | UR §3.1 (dual modality) | FS-A11Y-005 |
| §2.3 | Status is honest and immediate | UR §2.2 (Jordan — at-a-glance status) | FS-SSH-022, FS-A11Y-004 |
| §2.4 | Sensible defaults, zero required configuration | UR §2.3 (Sam — no config for basic use) | — |
| §3 | Complete design token system | UR §8.3 (design tokens) | FS-THEME-008 |
| §3.1 | Primitive tokens use `--color-` prefix | — | Tailwind 4 namespace collision avoidance |
| §4.1 | Terminal font stack with JetBrains Mono primary | UR §8.2 (font family in themes) | FS-PREF-006 (font configurable) |
| §5.3 | ANSI 16-color palette with 4.5:1+ contrast | UR §8.2 (ANSI palette in themes) | FS-VT-020, FS-VT-023 |
| §5.4 | All pairings meet WCAG 2.1 AA | — | FS-A11Y-001 |
| §6.1 | Window anatomy: tab bar, terminal area, status bar | UR §4.1 (multi-tab), UR §4.2 (panes) | FS-TAB-001, FS-PANE-001 |
| §6.4 | Settings button in status bar | UR §3.1 (dual modality — mouse access to settings) | FS-PREF-005 |
| §6.5 | Pane divider: 1px line, 8px hit area | UR §4.2 (pane resize) | FS-PANE-003 |
| §6.6 | Active pane: 2px blue border | UR §4.2 (pane navigation) | FS-PANE-006 |
| §6.8 | Minimum window 640x400 | — | FS-A11Y-002 (target sizes) |
| §7.1 | Tab bar with activity indicators | UR §4.1 (activity notification) | FS-TAB-007, FS-NOTIF-001-004 |
| §7.1.2 | Tab reorder via drag | UR §4.1 (tabs reorderable) | FS-TAB-005 |
| §7.1.3 | Distinct indicators for output, process end, bell; aggregated scroll arrow badges | UR §4.1 (activity notification) | FS-NOTIF-001, FS-NOTIF-002, FS-NOTIF-004 |
| §7.1.6 | Inline rename on double-click, F2, and context menu | UR §4.1 (configurable title) | FS-TAB-006 |
| §7.1.7 | SSH badge on tab with lifecycle states (incl. Closed) | UR §9.1 (at-a-glance SSH distinction) | FS-SSH-002, FS-SSH-010 |
| §7.2.1 | Pane activity indicators (border pulse) | UR §4.1 (activity notification for panes) | FS-NOTIF-001, FS-NOTIF-004 |
| §7.3.1 | Cursor styles: block, underline, bar (steady/blinking) | — | FS-VT-030, FS-VT-032 |
| §7.3.1 | Unfocused cursor as hollow outline | — | FS-VT-034 |
| §7.4 | Search overlay with match highlighting | UR §7.2 (search in output) | FS-SEARCH-001-007 |
| §7.5.2 | SSH disconnection banner with reconnect CTA and reconnecting state | UR §9.1 (clear notification) | FS-SSH-022, FS-SSH-041 |
| §7.6 | Preferences panel with sections | UR §5.2 (preferences UI) | FS-PREF-002, FS-PREF-004 |
| §7.6.3 | Connections section: inline view in Preferences | UR §9.2 (saved connections accessible from preferences) | FS-SSH-030-034 |
| §7.7 | Connection manager (standalone) with grouped list and CRUD operations | UR §9.2 (saved connections) | FS-SSH-030-034 |
| §7.8 | Context menu in terminal area | UR §3.1 (dual modality), UR §3.2 (discoverability) | FS-A11Y-006 |
| §7.9.3 | Destructive confirmation with safe default | UR §2.2 (Jordan — no silent failures) | FS-PTY-008 |
| §7.9.4 | SSH host key verification dialogs; MITM uses ShieldAlert in error color | UR §9.3 (security) | FS-SSH-011 |
| §7.13 | First-launch context menu hint | UR §2.3 (Sam — discoverability) | FS-UX-002 |
| §7.14 | Button variants (primary, secondary, ghost, destructive) | UR §3.1 (dual modality — visible controls) | — |
| §7.17 | Keyboard shortcut recorder with conflict detection; WebView-level interception note | UR §6 (configurable shortcuts) | FS-KBD-002 |
| §7.18 | Process terminated pane with restart/close | — | FS-PTY-005, FS-PTY-006 |
| §7.19 | Reconnection separator injected into scrollback at reconnect; full-width rule + left-aligned timestamp label; non-interactive UI overlay | — | FS-SSH-042 |
| §7.20 | Theme editor with color pickers and contrast advisory | UR §8.2 (user-created themes) | FS-THEME-003, FS-THEME-004 |
| §7.21 | Deprecated SSH algorithm warning banner | UR §9.3 (security awareness) | FS-SSH-014 |
| §8.2 | Focus trap in modals | UR §3.1 (keyboard completeness) | FS-A11Y-003 |
| §8.5 | Auto-copy to PRIMARY, paste from CLIPBOARD | UR §6.3 (clipboard) | FS-CLIP-004, FS-CLIP-005 |
| §8.5 | Multi-line paste confirmation | — | FS-CLIP-009 |
| §8.6 | SSH interruption feedback within 1 second | UR §9.1 (interruption notification) | FS-SSH-022 |
| §9 | All animations respect prefers-reduced-motion | — | FS-A11Y-001 (WCAG compliance) |
| §10 | Lucide icon set with 1.5px stroke | — (CLAUDE.md stack) | — |
| §11.2 | Pane navigation shortcuts (Ctrl+Shift+Arrow) | UR §4.2 (pane keyboard navigation) | FS-KBD-003, FS-PANE-005 |
| §11.2 | Split shortcuts (Ctrl+Shift+D/E) | UR §4.2 (pane splits) | FS-KBD-003 |
| §11.5 | 44px minimum touch targets | — | FS-A11Y-002 |
| §11.6 | Non-color indicators for all status states | — | FS-A11Y-004 |
| §12 | Graceful degradation at narrow widths | UR §2.1 (Alex — tiling WM use) | — |
| §13 | Theme extensibility via token override | UR §8.2 (user themes) | FS-THEME-003, FS-THEME-009 |
| §13.5 | Contrast validation on user themes; editor chrome accessibility invariant (editor always renders with active system tokens, not work-in-progress theme) | UR §8.3 (visual consistency) | FS-THEME-008, FS-A11Y-001, FS-A11Y-005 (to be added), FS-PREF-003 |
| §15 | IPC contract — deferred to ARCHITECTURE.md §4 (authoritative) | — | Cross-cutting (FS-SSH-010, FS-NOTIF, FS-VT, FS-SB) |
| §2.7 | i18n as design constraint: all strings are locale-resolved keys, no hardcoded copy | UR 10 §10.1 (language support) | FS-I18N-001 |
| §7.6.3 Language subsection (within Appearance) | Language selector dropdown in Appearance section of Preferences; immediate apply with `--duration-fast` opacity transition; persisted; discoverability for Sam (UR §2.3) | UR 10 §10.1; UR §2.3 (Sam — discoverability) | FS-I18N-003, FS-I18N-004, FS-I18N-005, FS-I18N-006 |

---

## 15. IPC Contract

> **This section is superseded by [`docs/ARCHITECTURE.md` §4](ARCHITECTURE.md#4-ipc-contract)**, which is the single source of truth for all data shapes, command signatures, and event payloads exchanged between the Rust backend and the Svelte frontend.
>
> ARCHITECTURE.md §4 covers: the complete `invoke()` command list (29 commands), all `listen()` events (8 events), TypeScript interfaces, Rust structs, and the authoritative decisions that override earlier drafts (pane layout tree, `SessionStateChanged` shape, `close_pane` return type, `notification-changed` payload).
>
> This document references ARCHITECTURE.md §4 for IPC concerns — it does not restate the contract here.
