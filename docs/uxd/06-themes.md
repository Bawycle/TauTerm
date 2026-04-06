<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Theme Extensibility

> Part of the [UX/UI Design](README.md).

---

## 13. Theme Extensibility

### 13.0 Built-in Themes

TauTerm ships with three built-in themes. Built-in themes differ from user-created themes in the following ways:

**Non-destructive:** Built-in themes cannot be deleted. The delete action is absent from the context menu and theme editor for any built-in entry. (FS-THEME-001)

**Non-editable:** Built-in themes cannot be modified in the theme editor. Selecting a built-in theme in the editor renders the token values in read-only display fields, not input fields. A "Duplicate to edit" affordance allows the user to create an editable copy. (FS-THEME-011)

**Always available:** Built-in themes are present from first launch, before the user has created any theme, and persist across preference resets. (FS-THEME-013)

**The three built-in themes are:**

| Name | Type | Artistic reference | Section |
|------|------|--------------------|---------|
| **Umbra** | Dark (default) | Eclipse shadow — warm neutrals, steel blue accent | AD.md §7 |
| **Solstice** | Light | Nordic winter solstice — cool neutrals, arctic blue accent | AD.md §8 |
| **Archipel** | Dark | Caribbean archipelago — blue-green depths, coral/turquoise/lime accents | AD.md §9 |

All palette specifications (hex values, HSL values, contrast ratios) belong exclusively in `docs/AD.md` §7–9. This section does not duplicate those values.

#### Theme switcher UI

In the theme switcher (accessible from Preferences > Themes), built-in themes appear in a labeled group titled **"Built-in"**, visually separated from user-created themes (labeled **"My themes"**). The grouping uses a section heading at `--color-text-heading` weight and size, with a `--space-2` top margin between groups.

Built-in theme entries display:
- Theme name (using `--color-text-primary`)
- Theme type badge: "Light" or "Dark" (using `--color-text-tertiary`, `--font-size-ui-xs`)
- Active indicator: a `Check` icon at `--color-accent` when this theme is active (see §13.6)
- "Built-in" label (using `--color-text-tertiary`, `--font-size-ui-xs`)
- No delete button, no edit button

User-created theme entries display:
- Theme name
- Theme type badge
- Active indicator
- Edit button (pencil icon, `Pencil` from Lucide, md size)
- Delete button (`Trash2` from Lucide, md size) — not shown for built-in themes

The "Built-in" group is always shown first, before user-created themes, regardless of alphabetical order. Within the built-in group, order is fixed: Umbra, Solstice, Archipel.

#### "Duplicate to edit" access points

The "Duplicate to edit" action for built-in themes is exposed through three independent access paths, none of which relies on hover state alone:

1. **Context menu (right-click):** Right-clicking any theme entry (built-in or user-created) in the Themes section list opens a context menu. For built-in entries, the context menu contains: "Duplicate to edit". For user-created entries: "Edit", "Duplicate", "Delete".

2. **Overflow menu (⋯ button):** Each theme entry in the list shows a `MoreHorizontal` icon button (Lucide, md size) in its trailing position. This button is always visible when the theme entry is selected/active, and visible on hover for non-selected entries. It opens an inline dropdown with the same items as the context menu for that entry type.

3. **Selected-state inline button:** When a built-in theme entry is the currently selected entry (keyboard focus or last clicked), a "Duplicate to edit" button (ghost variant, `Copy` icon + label) appears inline within the entry row. This button is always visible without hover when the entry is selected, providing an unambiguous affordance for keyboard and mouse users alike.

These three paths are additive. No single path is the exclusive route. The overflow menu (path 2) is the canonical primary affordance because it is present on all entry types and does not require the entry to be active.

#### Theme creation paths

Two creation paths are supported. They are not mutually exclusive, but they have an intentional prominence hierarchy:

**Primary path — Duplication:** The user right-clicks or opens the overflow menu on any existing theme (built-in or user-created) and selects "Duplicate to edit" (or "Duplicate" for user themes). This creates a copy with all token values pre-filled from the source theme. The theme editor opens immediately with the copy. This path is recommended because it guarantees a fully valid theme from the first edit — no token is left undefined, and the starting contrast ratios are known-good.

**Secondary path — Blank canvas:** A "New theme" button (secondary variant, `Plus` icon) is present in the Themes section header bar, placed to the right of the section heading and left of any section-level controls. It creates a new theme pre-filled with Umbra defaults (all required and optional tokens). The theme editor opens immediately. "New theme" appears less prominent than per-entry duplication: it is a secondary button, not primary, and is positioned outside the theme list itself.

The blank-canvas path uses Umbra as its baseline rather than an empty state. This avoids the failure mode of a blank-canvas theme with undefined tokens rendering as unreadable (e.g., black text on black background). Umbra defaults serve as a safe, minimally functional starting point regardless of the user's eventual target palette.

The label "Duplicate to edit" (rather than "Duplicate") is used specifically for built-in themes to communicate that the purpose of duplication is to create an editable copy, since built-in themes are read-only. For user-created themes, "Duplicate" is sufficient because they are already editable.

#### Default theme behavior

On first launch (no stored preference), Umbra is active. The active theme ID is stored in preferences as a string identifier (`"umbra"`, `"solstice"`, `"archipel"`, or a user-created theme UUID). Built-in theme IDs are reserved strings that cannot be used as user theme IDs.

### 13.1 Token Mapping for User Themes

User-created themes (FS-THEME-003) override the same CSS custom properties defined in [§3](02-tokens.md#3-design-token-system). The theming system operates at the `:root` level — a user theme is a CSS file that redeclares token values.

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

### 13.6 Theme Switching Interaction

This section defines the complete interaction model for switching the active theme. It covers both the full Themes section and the quick-select affordance in the Appearance section of the Preferences panel.

#### Entry points

Theme switching is accessible from two locations within the Preferences panel (§7.6.3):

- **Appearance section — quick-select dropdown:** A compact dropdown (§7.16) shows the name of the currently active theme. Opening it lists all available themes (built-in and user-created) in a single flat list for rapid switching. This is a convenience affordance for users who know which theme they want. It does not expose theme management actions (no edit, duplicate, delete).

- **Themes section — full theme list:** The primary management surface. Contains the grouped list (Built-in / My themes per FS-THEME-012), the active indicator, per-entry overflow menus, the "New theme" button, and access to the theme editor. Theme switching in this list is the canonical path. (FS-THEME-006)

#### Live preview behavior

Selecting a theme entry in the Themes section list — by single click or by pressing Enter when the entry has keyboard focus — applies the theme immediately and visibly. There is no "Apply" or "Confirm" step. The entire UI (tab bar, terminal viewport, status bar, the preferences panel itself) reflects the new theme on the next rendered frame. (FS-THEME-006)

Theme application does not happen on hover. Hovering over an unselected theme entry shows the entry's hover state (background highlight) but does not change the active theme. This decision is deliberate: hover-preview in a list of three or more themes would produce rapid uncontrolled visual flicker as the cursor moves, making the list unusable. Applying on click (or Enter) is the correct interaction model.

The quick-select dropdown in the Appearance section follows the same rule: selecting an option from the dropdown applies the theme immediately on selection (no separate confirm action).

The theme switch animation (cross-fade `--duration-slow` 300ms, `--ease-linear`, all token values simultaneously) is defined in §9.3. Under `prefers-reduced-motion: reduce`, the switch is instant with no cross-fade.

#### Reverting a theme switch

There is no dedicated "undo" for theme switching. Reverting is accomplished by selecting the previous theme from the list. This is the expected mental model: the user selects the theme they want active; the list always shows all available themes; selecting any of them activates it immediately. No state is lost — all custom themes remain in the list.

#### Visual representation of theme entries

Each entry in the theme list (both Themes section and quick-select dropdown) displays the following elements:

| Element | Detail |
|---------|--------|
| Theme name | `--color-text-primary`, `--font-size-ui-sm` |
| Color swatch strip | A horizontal strip of 6 color swatches in this fixed order: `--term-bg`, `--term-fg`, `--color-accent`, `--term-cursor-bg`, `--term-color-1` (red), `--term-color-6` (cyan). Each swatch is a filled rectangle, 16×16px, `--radius-sm`. The strip reads as a compact visual fingerprint of the theme's palette. |
| Light / Dark badge | "Light" or "Dark" text label, `--color-text-tertiary`, `--font-size-ui-xs` |
| Built-in label | Present only for built-in theme entries. Text: "Built-in", same typographic treatment as the Light/Dark badge. Placed adjacent to it. |
| Active indicator | A `Check` icon (Lucide, sm size, `--color-accent`) in the leading position of the entry, visible only when this theme is currently active. Hidden (not merely invisible) for inactive entries; space is reserved so the list does not shift on activation. |
| Overflow menu button | `MoreHorizontal` icon (Lucide, md size, `--color-text-tertiary`). Always visible when the entry has keyboard focus or is the active theme. Visible on hover for all other entries. Never hidden entirely — it becomes visible on hover, not on some other trigger. |

The quick-select dropdown shows a simplified version: theme name + color swatch strip only. The active theme entry in the dropdown additionally shows the `Check` icon.

#### Keyboard navigation

Within the Themes section list, keyboard navigation follows standard list patterns:

| Key | Action |
|-----|--------|
| `Arrow Down` / `Arrow Up` | Move focus to the next / previous theme entry in the list. Focus wraps at the list boundaries within each group; it does not wrap across groups. |
| `Enter` | Activate the focused entry: apply the theme immediately (same effect as a single click). |
| `Tab` | Move focus out of the theme list to the next focusable control in the Preferences panel (following the standard tab order defined in §8.2). |
| `Shift+Tab` | Move focus to the previous focusable control. |
| `Space` | Same as `Enter` on a theme entry. On the overflow menu button within an entry: opens the overflow menu. |
| `Escape` | If the overflow menu is open: closes it, returns focus to the overflow menu button. If no menu is open: no effect within the list (does not close the Preferences panel). |

When the Preferences panel opens to the Themes section, initial focus is on the currently active theme entry. This allows a user who opens Preferences to immediately use arrow keys to cycle through themes and see the effect live.

#### Active theme indicator

The active theme is identified at a glance by the `Check` icon in the leading position of its entry row. This is the sole persistent visual indicator of which theme is active — it is always visible without any interaction required. The icon uses `--color-accent` to make it distinct from the surrounding text content.

The active entry does not use a background highlight as its primary active indicator, because the background highlight is already used for hover and keyboard focus states. Using a dedicated icon (Check) avoids overloading the background color with two meanings.

#### Conflict note — §7.6.3 per-theme actions

The Themes section spec in §7.6.3 of `03-components.md` currently states that per-theme actions are "visible on hover." This conflicts with the access-point decisions in §13.0 (which require actions to be reachable without hover). The §13.0 spec is authoritative for this behavior. The `frontend-dev` implementation of §7.6.3 must follow §13.0: the overflow menu button is the canonical action trigger and must be visible on focus and on active-theme entries without requiring hover. The §7.6.3 prose should be updated to align with §13.0 during the next pass on `03-components.md`.
