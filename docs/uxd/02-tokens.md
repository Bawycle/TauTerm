<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Design Tokens, Typography, Color System & Layout

> Part of the [UX/UI Design](README.md).

---

## 3. Design Token System

All tokens are defined in AD.md §7 and expressed as Tailwind 4 `@theme` CSS custom properties. This section provides the complete reference with usage descriptions. Components in [§7](03-components.md) reference these tokens exclusively — no hardcoded values.

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
| `--color-border-overlay` | `#4a4640` | Border for floating surfaces (menus, modals, tooltips) — one step lighter than `--color-border` to lift overlays off the surface they sit above |
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
| `--color-focus-ring-offset` | `#0e0d0b` | Gap color between focus ring and element edge (used as `outline-color` on the gap layer) |

The global `:focus-visible` rule applies `outline: 2px solid var(--color-focus-ring)` with `outline-offset: 3px`. The 3px offset ensures the ring clears the element border cleanly and is visually distinct from element borders at 1px and 2px. Component specs that reference "offset 2px" in their focus state tables should be read as using this global default (updated from 2px to 3px).

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
| `--color-error-fg` | `#d97878` | Error foreground text on `--color-error-bg` — 5.34:1 (WCAG AA text ✓) |
| `--color-error-border` | `#c44444` | Error banner border — 3.73:1 on terminal bg, 3.29:1 on error bg (WCAG AA UI ✓) |
| `--color-error-text` | `#d97878` | Alias for `--color-error-fg`; retained for backward compatibility |
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

#### ANSI 16-Color Palette Tokens

The ANSI palette is tuned to work against the `--term-bg` terminal background (`#16140f`). Normal colors (0–7) are desaturated relative to their bright counterparts (8–15) — bright variants have genuine salience without normal colors being visually aggressive. All non-black values pass 4.5:1 contrast against `#16140f`. The palette is the Umbra theme's own design; it is not a clone of any existing terminal color scheme (AD.md §3.2).

| Token | Umbra hex | Role | Contrast vs bg |
|-------|-----------|------|----------------|
| `--term-color-0` | `#2c2921` | Black (normal) — default background-like color, used for reversed text | n/a (background use) |
| `--term-color-1` | `#c44444` | Red (normal) — errors, shell error prompts | 5.2:1 |
| `--term-color-2` | `#5c9e5c` | Green (normal) — success output, `git diff` additions | 4.6:1 |
| `--term-color-3` | `#b89840` | Yellow (normal) — warnings, file names in `ls` | 5.1:1 |
| `--term-color-4` | `#4a92bf` | Blue (normal) — directories, links | 4.9:1 |
| `--term-color-5` | `#9b6dbf` | Magenta (normal) — special files, prompt segments | 4.7:1 |
| `--term-color-6` | `#3d9e8a` | Cyan (normal) — info output, SSH prompts | 4.9:1 |
| `--term-color-7` | `#ccc7bc` | White (normal) — default text | 8.4:1 |
| `--term-color-8` | `#4a4640` | Black (bright) — dark gray, used for dimmed text contexts | n/a (dimmed text context) |
| `--term-color-9` | `#e06060` | Red (bright) — bold errors, diff removal | 7.0:1 |
| `--term-color-10` | `#82c082` | Green (bright) — bold success, diff addition | 7.5:1 |
| `--term-color-11` | `#d4b860` | Yellow (bright) — bold warnings | 7.8:1 |
| `--term-color-12` | `#7ab3d3` | Blue (bright) — bold directories | 8.1:1 |
| `--term-color-13` | `#c09cd8` | Magenta (bright) — bold special files | 8.5:1 |
| `--term-color-14` | `#6ec4ae` | Cyan (bright) — bold info output | 7.8:1 |
| `--term-color-15` | `#f5f2ea` | White (bright) — ANSI bright white, high-emphasis text | 13.4:1 |

#### Text Attribute Rendering Tokens (FS-VT-024, FS-VT-025)

| Token | Resolved Value | Description |
|-------|---------------|-------------|
| `--term-dim-opacity` | `0.5` | Opacity multiplier for SGR 2 (Dim/Faint) rendering |
| `--term-underline-color-default` | `inherit` | Default underline color when SGR 58 is not set — inherits current foreground color |
| `--term-strikethrough-position` | `50%` | Strikethrough line vertical position as percentage of cell height (measured from top) |
| `--term-strikethrough-thickness` | `1px` | Strikethrough line thickness |
| `--term-blink-on-duration` | `533ms` | Blink visible phase duration (SGR 5 and SGR 6) |
| `--term-blink-off-duration` | `266ms` | Blink invisible phase duration — asymmetric 2:1 on/off ratio for legibility |

#### Design Decision: Opacity-Based Dim Rendering

`--term-dim-opacity` (value: `0.5`) is applied as a CSS `opacity` multiplier to cells carrying the SGR 2 (Dim/Faint) attribute.

This is a deliberate choice over the alternative of defining separate dim color tokens for each ANSI color. The rationale:

1. **Uniform application.** A single opacity value applies consistently across all 16 ANSI colors plus the default foreground, across every bold and italic combination. There is no risk of inconsistency between how dim-red and dim-cyan look relative to their non-dim counterparts.

2. **Theme compatibility.** When a user creates a custom theme and changes any palette color, dim rendering automatically adapts — the dim variant is always `opacity × custom-color`. Separate dim tokens would require the user to update 16 additional entries to keep their custom palette coherent.

3. **Token economy.** 16 additional tokens per theme (one dim variant per ANSI color) would be added overhead with no design benefit. The opacity approach achieves the same perceptual result with one token.

The trade-off: opacity-based dimming interacts with the cell background — on a colored background the dim foreground may appear slightly different than a standalone color swatch would suggest. This is acceptable in terminal practice; most dim text appears against the default terminal background.

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
| `--letter-spacing-label` | `0.09em` | Uppercase section labels (11px semibold) — applied to Caption-level headings in PreferencesPanel and ConnectionManager |

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
| `--size-cursor-outline-width` | `1px` | Stroke width of the hollow cursor outline (unfocused pane) |

### 3.8 Border Radius Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-none` | `0px` | Terminal area, pane dividers, tab bar body — content infrastructure |
| `--radius-sm` | `4px` | Buttons, text inputs, dropdown triggers, tab items, badge containers |
| `--radius-md` | `8px` | Modals, overlays, context menus, tooltips, search overlay, dropdown content |
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
| `--z-fullscreen-chrome` | `45` | Full-screen exit badge and recalled chrome (above overlays, below modals) |
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

### 4.1.1 Cell Dimension Derivation

The terminal grid is built from discrete cell units. Cell dimensions must be computed from the actual rendered font, not inferred from CSS values alone, because different fonts at the same nominal size have different advance widths and metrics.

**`cell_width` — how to measure:**

Measure the advance width of U+2588 FULL BLOCK (█) rendered at `--font-size-terminal` in the resolved `--font-terminal` stack using a Canvas 2D context (`ctx.measureText("█").width`). U+2588 is the canonical reference character for cell width in terminal emulators (used by xterm, kitty, WezTerm): it is guaranteed to be exactly one cell wide in any compliant monospace terminal font. The result is a floating-point pixel value; use it as-is for layout arithmetic. Do not round `cell_width` — accumulation of fractional-pixel rounding across a wide terminal causes visible column misalignment.

**`cell_height` — how to compute:**

`cell_height = Math.ceil(font_size_px * line_height)`

For the default tokens: `Math.ceil(14 × 1.2) = Math.ceil(16.8) = 17px`.

Ceil is used rather than floor or round to ensure the cell box always fully contains the font's descenders. Rounding down (16px) clips descenders at 14px font size with 1.2 line height. The 1-pixel loss is invisible; the clipped descender is not.

**Wide characters (CJK, fullwidth):**

Wide characters (Unicode East Asian Width = W or F) occupy exactly `2 × cell_width` pixels. The renderer places the second logical cell as a continuation cell with no glyph — the wide glyph from the first cell paints across both cells.

**Emoji:**

Emoji are treated as wide characters (2 cells) when the renderer receives a double-width hint from the VT parser. Emoji that arrive as single-width are rendered in one cell; visual overflow is clipped to the cell boundary.

**When to recalculate:**

Cell dimensions must be recalculated when:
- The user changes the terminal font family (FS-PREF-006).
- The user changes the terminal font size (FS-PREF-006).
- On initial mount, before the first frame is painted.

Cell dimension changes require a full PTY `resize` command to the backend with the new column/row counts derived from `floor(pane_pixel_width / cell_width)` and `floor(pane_pixel_height / cell_height)`.

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
| Caption | `--font-size-ui-xs` | 11px | `--font-weight-semibold` (600) | 1.3 | `--letter-spacing-label` (0.09em) | Section headings (all-caps), tooltip text |
| Label | `--font-size-ui-sm` | 12px | `--font-weight-normal` (400) | 1.4 | 0 | Secondary labels, shortcut key text, descriptions |
| Body | `--font-size-ui-base` | 13px | `--font-weight-normal` (400) | 1.4 | 0 | Primary UI text: tab titles, menu items, form fields, dialog body |
| Body-emphasis | `--font-size-ui-base` | 13px | `--font-weight-semibold` (600) | 1.4 | 0 | Active tab title only |
| Content | `--font-size-ui-md` | 14px | `--font-weight-normal` (400) | 1.4 | 0 | Dialog body text where larger size aids readability |
| Heading | `--font-size-ui-lg` | 16px | `--font-weight-semibold` (600) | 1.3 | 0 | Dialog headings, modal titles |
| Title | `--font-size-ui-xl` | 20px | `--font-weight-semibold` (600) | 1.2 | 0 | Reserved; major headings if needed |

### 4.5 Hierarchy Rules

1. **Primary UI text** — Body level. Default for all labels, tab titles, menu items, form fields.
2. **Section headings** — Caption level, all-caps, `letter-spacing: var(--letter-spacing-label)` (0.09em), color `--color-text-heading`. Used for preference panel sections and grouped form labels. Small-caps headings establish hierarchy without consuming vertical space.
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
- **Content:** Left-aligned: active pane current working directory (truncated with ellipsis if too long). Right-aligned (in order): Settings button, SSH connection status (if applicable), terminal dimensions (cols × rows) — transient: visible only during pane resize, fades out after a 2s delay (see §9.3).
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
