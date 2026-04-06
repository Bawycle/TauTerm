<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Theme Extensibility

> Part of the [UX/UI Design](README.md).

---

## 13. Theme Extensibility

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
