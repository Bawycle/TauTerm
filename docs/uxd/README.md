<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Index

This directory contains the UX/UI Design documentation for TauTerm, split from the original `docs/UXD.md` for easier navigation.

> **Version:** 1.4.0 | **Status:** Draft
> **Input documents:** [UR.md](../UR.md), [FS.md](../FS.md), [AD.md](../AD.md)

FS ID references (e.g. `FS-TAB-009`) are unchanged throughout all files in this directory.

---

## Files

| File | Contents |
|------|----------|
| [01-principles.md](01-principles.md) | §1 Purpose & Scope, §2 Design Principles — SSOT rules, authoritativeness, the 7 design principles |
| [02-tokens.md](02-tokens.md) | §3 Design Token System, §4 Typography System, §5 Color System, §6 Layout & Spatial Model |
| [03-components.md](03-components.md) | §7 Component Specifications — all 22 sub-sections (§7.1 Tab Bar through §7.22 Full-Screen Mode) |
| [04-interaction.md](04-interaction.md) | §8 Interaction Patterns, §9 Motion & Animation, §10 Iconography |
| [05-accessibility.md](05-accessibility.md) | §11 Accessibility, §12 Responsiveness & Window Resizing |
| [06-themes.md](06-themes.md) | §13 Theme Extensibility — required/optional tokens, non-themeable tokens, validation rules |
| [07-traceability.md](07-traceability.md) | §14 Traceability Matrix, §15 IPC Contract (stub reference to ARCHITECTURE.md §4) |

---

## Quick Reference

| Concern | File | Anchor |
|---------|------|--------|
| Design principles | [01-principles.md](01-principles.md) | §2 |
| SSOT / scope rules | [01-principles.md](01-principles.md) | §1.2 |
| Color tokens (primitives) | [02-tokens.md](02-tokens.md) | §3.1 |
| Color tokens (semantic) | [02-tokens.md](02-tokens.md) | §3.2 |
| Color tokens (component) | [02-tokens.md](02-tokens.md) | §3.3 |
| Terminal surface tokens | [02-tokens.md](02-tokens.md) | §3.4 |
| Spacing & sizing tokens | [02-tokens.md](02-tokens.md) | §3.6, §3.7 |
| Typography system | [02-tokens.md](02-tokens.md) | §4 |
| Color system & contrast table | [02-tokens.md](02-tokens.md) | §5 |
| Window anatomy | [02-tokens.md](02-tokens.md) | §6.1 |
| Tab Bar spec | [03-components.md](03-components.md) | §7.1 |
| Pane Divider spec | [03-components.md](03-components.md) | §7.2 |
| Terminal Area (cursor, selection, scrollbar) | [03-components.md](03-components.md) | §7.3 |
| Search Overlay spec | [03-components.md](03-components.md) | §7.4 |
| Connection Status / SSH disconnection | [03-components.md](03-components.md) | §7.5 |
| Preferences Panel spec | [03-components.md](03-components.md) | §7.6 |
| Connection Manager spec | [03-components.md](03-components.md) | §7.7 |
| Context Menu spec | [03-components.md](03-components.md) | §7.8 |
| Dialog / Modal spec | [03-components.md](03-components.md) | §7.9 |
| Button variants | [03-components.md](03-components.md) | §7.14 |
| Text Input / Form Field | [03-components.md](03-components.md) | §7.15 |
| Toggle & Dropdown | [03-components.md](03-components.md) | §7.16 |
| Keyboard Shortcut Recorder | [03-components.md](03-components.md) | §7.17 |
| Process Terminated Pane | [03-components.md](03-components.md) | §7.18 |
| SSH Reconnection Separator | [03-components.md](03-components.md) | §7.19 |
| Theme Editor | [03-components.md](03-components.md) | §7.20 |
| Deprecated SSH Algorithm Banner | [03-components.md](03-components.md) | §7.21 |
| Full-Screen Mode | [03-components.md](03-components.md) | §7.22 |
| Mouse interaction patterns | [04-interaction.md](04-interaction.md) | §8.1 |
| Focus management | [04-interaction.md](04-interaction.md) | §8.2 |
| Scroll behavior | [04-interaction.md](04-interaction.md) | §8.3 |
| Drag & Drop | [04-interaction.md](04-interaction.md) | §8.4 |
| Clipboard | [04-interaction.md](04-interaction.md) | §8.5 |
| SSH interruption feedback | [04-interaction.md](04-interaction.md) | §8.6 |
| Motion philosophy | [04-interaction.md](04-interaction.md) | §9.1 |
| Entrance/exit transitions (incl. full-screen) | [04-interaction.md](04-interaction.md) | §9.2 |
| `prefers-reduced-motion` policy | [04-interaction.md](04-interaction.md) | §9.4 |
| Icon set & vocabulary | [04-interaction.md](04-interaction.md) | §10 |
| Contrast audit | [05-accessibility.md](05-accessibility.md) | §11.1 |
| Keyboard navigation map | [05-accessibility.md](05-accessibility.md) | §11.2 |
| ARIA roles | [05-accessibility.md](05-accessibility.md) | §11.3 |
| Non-color indicators | [05-accessibility.md](05-accessibility.md) | §11.6 |
| Tab bar overflow / scrolling | [05-accessibility.md](05-accessibility.md) | §12.2 |
| Theme required tokens | [06-themes.md](06-themes.md) | §13.2 |
| Theme optional tokens | [06-themes.md](06-themes.md) | §13.3 |
| Non-themeable tokens | [06-themes.md](06-themes.md) | §13.4 |
| Theme validation rules | [06-themes.md](06-themes.md) | §13.5 |
| Traceability matrix | [07-traceability.md](07-traceability.md) | §14 |
| IPC contract reference | [07-traceability.md](07-traceability.md) | §15 |
