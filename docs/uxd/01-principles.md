<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Principles & Scope

> Part of the [UX/UI Design](README.md).

> **Version:** 1.4.0
> **Date:** 2026-04-04
> **Input documents:** [User Requirements (UR.md)](../UR.md), [Functional Specifications (FS.md)](../FS.md), [Artistic Direction (AD.md)](../AD.md)

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

**Verification:** Every interactive element in [§7](03-components.md) specifies both a mouse interaction and a keyboard interaction. Every component spec includes a focus state. The keyboard navigation map in [§11](05-accessibility.md#112-keyboard-navigation-map) covers all interactive surfaces.

### 2.3 Status Is Honest and Immediate

The interface reflects the system's actual state at all times. Connection drops, process termination, and activity in background tabs are communicated promptly (FS-SSH-022), using both color and a non-color indicator (icon, text, or shape).

**Grounding:** Jordan (UR §2.2) manages many SSH sessions and needs at-a-glance status. FS-SSH-022 requires disconnection detection within 1 second. FS-A11Y-004 requires non-color-only indicators. AD.md §1.3 lists "Honesty" as a design value.

**Verification:** Every status state in [§7](03-components.md) specifies a color token AND a secondary indicator (icon or text). SSH lifecycle states ([§7.5](03-components.md#75-connection-status-indicator)) each have a distinct visual treatment with both color and icon.

### 2.4 Sensible Defaults, Zero Required Configuration

TauTerm is usable immediately after installation. Default shortcuts, theme, font, and behavior require no adjustment for productive use. Configuration is an optimization layer.

**Grounding:** Sam (UR §2.3) needs "sensible defaults, discoverable preferences UI, no configuration required for basic use." AD.md §2.4 states the theme must read "as a coherent product, not an assembled collection of defaults."

**Verification:** Every configurable value in [§7](03-components.md) and [§8](04-interaction.md) specifies an explicit default. The default theme (Umbra) passes all accessibility checks in [§11](05-accessibility.md) without user adjustment.

### 2.5 Precision Over Decoration

Spacing, sizing, and color choices are deliberate, consistent, and derived from the token system. No approximate alignment, no decorative elements, no visual effects that do not communicate state.

**Grounding:** AD.md §1.3 ("Precision: nothing is approximately aligned or roughly the right size"). AD.md §1.3 ("Restraint: each new visual element must justify its existence").

**Verification:** Every dimension in [§7](03-components.md) is a token reference. No component uses a color value outside the token vocabulary defined in [§3](02-tokens.md#3-design-token-system). No animation exists without a stated purpose in [§9](04-interaction.md#9-motion--animation).

### 2.6 Durability Over Novelty

TauTerm is used for hours daily. The visual design must not fatigue the eye, produce surprising behaviors, or require relearning. Consistency across sessions builds trust; familiarity is a feature.

**Grounding:** AD.md §1.3 ("Durability: TauTerm is used for hours daily. The visual design must not fatigue the eye or demand relearning."). Alex (UR §2.1) and Jordan (UR §2.2) are daily users.

**Verification:** No animation exceeds `--duration-slow` (300ms). All hover/focus states follow the same pattern (token pair + timing) across components. The color palette uses warm-shifted neutrals (AD.md §3.1) to reduce luminance extremes that cause long-session fatigue.

### 2.7 Internationalisation as a Design Constraint

All user-visible strings referenced in component specifications throughout this document are logical string keys, not hardcoded text. At render time, each key is resolved to the active locale's value via the i18n message catalogue (see ARCHITECTURE.md §10.5). This constraint applies equally to button labels, section headings, placeholder text, tooltips, error messages, and all other copy.

**Implication for component specs:** when this document specifies text such as "Reconnect" or "THEMES", these are the English-locale display values used as examples. The actual rendered string is always looked up from the active catalogue. No string in component source code may be hardcoded.

**Grounding:** FS-I18N-001 (no hardcoded UI string), FS-I18N-004 (immediate apply on locale change).

**Verification:** No string literal appears in any component `.svelte` file outside of an i18n accessor call. Locale switching in Preferences immediately updates all visible text with no reload.
