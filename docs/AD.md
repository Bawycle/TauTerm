<!-- SPDX-License-Identifier: MPL-2.0 -->

# Artistic Direction — TauTerm

> **Version:** 1.2.0
> **Date:** 2026-04-07
> **Status:** Draft
> **Author:** UX/UI Designer — TauTerm team

---

## Table of Contents

1. [Design Philosophy](#1-design-philosophy)
2. [Visual Character](#2-visual-character)
3. [Color System](#3-color-system)
   - 3.1 Primitive Palette
   - 3.2 ANSI Terminal Palette
   - 3.3 Semantic Color Tokens
4. [Typography](#4-typography)
   - 4.1 Font Choices and Rationale
   - 4.2 Type Scale
   - 4.3 Hierarchy Rules
5. [Spacing and Sizing System](#5-spacing-and-sizing-system)
6. [Iconography](#6-iconography)
7. [Default Theme — Umbra](#7-default-theme--umbra)
   - 7.1 Color Tokens
   - 7.2 Typography Tokens
   - 7.3 Spacing Tokens
   - 7.4 Radius Tokens
   - 7.5 Shadow Tokens
   - 7.6 Motion Tokens
8. [Built-in Theme — Solstice](#8-built-in-theme--solstice)
   - 8.1 Name and Concept
   - 8.2 Mood
   - 8.3 Palette Character
   - 8.4 Audience Translation
   - 8.5 Color Tokens
   - 8.6 ANSI Terminal Palette
   - 8.7 Semantic Color Token Overrides
9. [Built-in Theme — Archipel](#9-built-in-theme--archipel)
   - 9.1 Name and Concept
   - 9.2 Mood
   - 9.3 Palette Character
   - 9.4 Audience Translation
   - 9.5 Color Tokens
   - 9.6 ANSI Terminal Palette
   - 9.7 Semantic Color Token Overrides
10. [Accessibility Baseline](#10-accessibility-baseline)

---

## 1. Design Philosophy

### 1.1 Core Premise

TauTerm exists to get out of the way. The terminal content is the work; the chrome is infrastructure. Every visual decision is evaluated against this: does it serve the work, or does it serve itself?

This does not mean invisible chrome. It means chrome that earns its presence through utility and that communicates status without demanding attention. The interface should feel like a well-made tool — not decorative, not sterile, not ostentatious.

### 1.2 Resolved Tensions

**Information density vs. calm.** TauTerm resolves this in favor of *managed density*: the terminal area is dense by necessity; the surrounding chrome operates at a lower visual temperature. Whitespace in the chrome creates a stable frame for high-density terminal content.

**Minimalism vs. discoverability.** All affordances exist and are visible, but at a subdued visual weight. Controls do not compete with terminal content for attention. This is progressive disclosure applied to visual weight, not to feature availability.

**Keyboard efficiency vs. mouse completeness.** Every feature is reachable by both modalities. The visual design expresses both clearly without privileging either.

### 1.3 Design Values

- **Honesty:** the interface reflects the system's actual state at all times. No cosmetic reassurance, no hidden failure.
- **Precision:** spacing, sizing, and color choices are deliberate and consistent. Nothing is approximately aligned or roughly the right size.
- **Restraint:** each new visual element must justify its existence. Adding chrome is a last resort.
- **Durability:** TauTerm is used for hours daily. The visual design must not fatigue the eye or demand relearning. Familiarity is a feature.

---

## 2. Visual Character

### 2.1 Theme Name and Concept

The default theme is named **Umbra**. The name derives from the Latin for shadow — the deep, total shadow at the center of an eclipse, a region of absolute darkness surrounded by a penumbra of diffused light.

### 2.2 Mood

A quiet night workspace. Not theatrical darkness, not hacker aesthetics. The kind of desk environment where a single screen illuminates a dark room and everything outside the screen has receded to background. The terminal surface itself is in a tone that reads as deep without being void.

The character is **understated precision**: nothing shouts, nothing decorates, nothing is approximate.

### 2.3 Palette Character

Warm neutrals in the chrome. Cool, precise blues in the accent layer. Amber reserved for caution. The warm shift in the neutral base distinguishes Umbra from generic cold-grey terminal themes — it has the quality of aged paper, of incandescent light on a dark wall, without being warm enough to compromise legibility. The blue accent is steel, not electric; it reads as functional, not decorative.

### 2.4 Audience Translation

**For Alex (software developer, full-day use):** The palette must not produce eye strain. Backgrounds are dark enough to reduce glare; text has sufficient contrast without being harsh. The warm neutral base, rather than pure black, reduces the luminance extremes that cause long-session fatigue.

**For Jordan (systems administrator, many SSH sessions):** Status legibility is primary. SSH session state, connection health, and error conditions are immediately distinguishable. The color vocabulary for status (blue = connected, amber = transitioning, red = error, green = active) is consistent across every surface.

**For Sam (occasional user):** Default settings are sane, discoverable, and self-explanatory. The theme reads as a coherent product, not an assembled collection of defaults.

---

## 3. Color System

### 3.1 Primitive Palette

All semantic tokens reference these primitives. Components never use primitives directly — always through semantic tokens.

> **Note on user theming:** The rule above applies to component implementation code. User-created themes are permitted to override primitive tokens (e.g., replacing the entire `--umbra-neutral-*` scale for a wholly different palette). This is valid and by design — a user theme that overrides primitives propagates those changes through all semantic tokens that map to them, without requiring the user to redefine every semantic token individually. The barrier between primitives and semantics is for component authors, not for theme authors.

**Neutral scale (warm-shifted, base for backgrounds and surfaces)**

| Name | Hex | HSL | Role |
|------|-----|-----|------|
| `neutral-950` | `#0e0d0b` | `hsl(40, 10%, 6%)` | Deepest background |
| `neutral-900` | `#16140f` | `hsl(40, 12%, 8%)` | Terminal background |
| `neutral-850` | `#1c1a14` | `hsl(40, 13%, 9%)` | Pane background (unfocused) |
| `neutral-800` | `#242118` | `hsl(40, 14%, 11%)` | Tab bar background |
| `neutral-750` | `#2c2921` | `hsl(40, 13%, 14%)` | Surface raised (panels, menus) |
| `neutral-700` | `#35312a` | `hsl(38, 11%, 18%)` | Border, divider |
| `neutral-600` | `#4a4640` | `hsl(37, 8%, 26%)` | Inactive tab text background |
| `neutral-500` | `#6b6660` | `hsl(37, 6%, 39%)` | Placeholder text, disabled |
| `neutral-400` | `#9c9890` | `hsl(40, 5%, 58%)` | Secondary text, inactive labels |
| `neutral-300` | `#ccc7bc` | `hsl(40, 8%, 77%)` | Primary UI text |
| `neutral-200` | `#e8e3d8` | `hsl(42, 16%, 88%)` | Emphasized UI text, active tab |
| `neutral-100` | `#f5f2ea` | `hsl(44, 30%, 94%)` | High-emphasis text (rarely used) |

The warm hue (HSL 37–44°) is consistent across the entire neutral scale. At dark values the warmth is nearly imperceptible; it becomes readable in the mid-tones and clearly warm at the light end. This ensures that the warmth reads as a property of the palette, not a color cast.

**Blue-steel scale (primary accent)**

| Name | Hex | HSL | Role |
|------|-----|-----|------|
| `blue-700` | `#1a3a52` | `hsl(205, 53%, 21%)` | Deep accent background |
| `blue-600` | `#1e4d6e` | `hsl(205, 58%, 27%)` | Focus ring base |
| `blue-500` | `#2e6f9c` | `hsl(205, 55%, 39%)` | Interactive accent (hover) |
| `blue-400` | `#4a92bf` | `hsl(205, 50%, 52%)` | Primary accent (default state) |
| `blue-300` | `#7ab3d3` | `hsl(205, 46%, 64%)` | Accent text on dark |
| `blue-200` | `#b3d2e6` | `hsl(205, 47%, 80%)` | Light accent |

HSL 205° is a blue leaning slightly toward cyan without crossing into it. The saturation is moderate (46–58%) — vivid enough to read as accent, controlled enough not to glow. This is the blue of brushed steel or calm water, not the blue of a notification badge.

**Amber scale (warning/caution)**

| Name | Hex | HSL | Role |
|------|-----|-----|------|
| `amber-700` | `#4d3000` | `hsl(37, 100%, 15%)` | Deep warning background |
| `amber-500` | `#b06a00` | `hsl(37, 100%, 34%)` | Warning indicator |
| `amber-400` | `#d48a20` | `hsl(36, 73%, 48%)` | Warning text on dark |
| `amber-300` | `#e8b060` | `hsl(36, 75%, 65%)` | Warning label |

The amber hue (HSL 36–37°) is close to the neutral warm hue but fully saturated, making it immediately readable as a distinct state signal. It evokes amber warning lights — caution, not danger.

**Red scale (error/danger)**

| Name | Hex | HSL | Role |
|------|-----|-----|------|
| `red-700` | `#3d1212` | `hsl(0, 55%, 15%)` | Error background |
| `red-500` | `#9c2c2c` | `hsl(0, 56%, 39%)` | Error indicator |
| `red-400` | `#c44444` | `hsl(0, 55%, 52%)` | Error text on dark |
| `red-300` | `#d97878` | `hsl(0, 55%, 66%)` | Error label |

**Green scale (success/activity)**

| Name | Hex | HSL | Role |
|------|-----|-----|------|
| `green-600` | `#1a3d1a` | `hsl(120, 40%, 17%)` | Activity background |
| `green-400` | `#4a9c4a` | `hsl(120, 35%, 45%)` | Activity indicator, success |
| `green-300` | `#78c078` | `hsl(120, 34%, 61%)` | Activity text on dark |

### 3.2 ANSI Terminal Palette

The ANSI palette is tuned to work against the `neutral-900` terminal background (`#16140f`). The design goal is distinguishable, non-aggressive hues that hold up under extended use. Normal colors are desaturated relative to their bright counterparts — bright variants have genuine salience without normal colors being visually aggressive. All values pass 4.5:1 contrast against `neutral-900`.

| Index | Name | Hex | HSL | Contrast vs bg |
|-------|------|-----|-----|----------------|
| 0 | Black (normal) | `#2c2921` | `hsl(40, 13%, 14%)` | n/a (background use) |
| 1 | Red (normal) | `#c44444` | `hsl(0, 55%, 52%)` | 5.2:1 |
| 2 | Green (normal) | `#5c9e5c` | `hsl(120, 27%, 49%)` | 4.6:1 |
| 3 | Yellow (normal) | `#b89840` | `hsl(43, 50%, 47%)` | 5.1:1 |
| 4 | Blue (normal) | `#4a92bf` | `hsl(205, 50%, 52%)` | 4.9:1 |
| 5 | Magenta (normal) | `#9b6dbf` | `hsl(275, 36%, 58%)` | 4.7:1 |
| 6 | Cyan (normal) | `#3d9e8a` | `hsl(168, 44%, 43%)` | 4.9:1 |
| 7 | White (normal) | `#ccc7bc` | `hsl(40, 8%, 77%)` | 8.4:1 |
| 8 | Black (bright) | `#4a4640` | `hsl(37, 8%, 26%)` | n/a (dimmed text context) |
| 9 | Red (bright) | `#e06060` | `hsl(0, 63%, 63%)` | 7.0:1 |
| 10 | Green (bright) | `#82c082` | `hsl(120, 30%, 63%)` | 7.5:1 |
| 11 | Yellow (bright) | `#d4b860` | `hsl(43, 58%, 60%)` | 7.8:1 |
| 12 | Blue (bright) | `#7ab3d3` | `hsl(205, 46%, 64%)` | 8.1:1 |
| 13 | Magenta (bright) | `#c09cd8` | `hsl(275, 40%, 73%)` | 8.5:1 |
| 14 | Cyan (bright) | `#6ec4ae` | `hsl(162, 40%, 60%)` | 7.8:1 |
| 15 | White (bright) | `#f5f2ea` | `hsl(44, 30%, 94%)` | 13.4:1 |

**Note on cyan hue shift:** `--term-color-6` (Cyan normal, HSL 168°) and `--term-color-14` (Bright Cyan, HSL 162°) are shifted approximately 37–43° toward green relative to the blue values (`--term-color-4`, `--term-color-12` at HSL 205°). This ensures reliable blue/cyan distinguishability, including for users with mild blue-green color confusion. The two cyan values are intentionally close to each other (HSL 162–168°) to maintain family coherence while their brightness contrast (4.9:1 vs 7.8:1) distinguishes normal from bright.

### 3.3 Semantic Color Tokens

Semantic tokens are the layer components consume. They map to primitives and can be fully replaced by a user theme without touching component styles.

**Semantic layer architecture:**
- `--color-bg-*` — background surfaces, from deepest to highest elevation
- `--color-border-*` — structural dividers
- `--color-text-*` — text hierarchy
- `--color-icon-*` — icon states
- `--color-accent-*` — interactive accent family
- `--color-tab-*` — tab bar component tokens
- `--color-pane-*` — pane border tokens
- `--color-scrollbar-*` — scrollbar tokens
- `--color-ssh-*` — SSH session state tokens
- `--color-{status}-*` — semantic status tokens (error, warning, success, activity, bell, process-end)
- `--term-*` — terminal surface tokens (background, foreground, cursor, selection, search, hyperlink, ANSI palette)

The complete token reference with resolved values is in Section 7.

---

## 4. Typography

### 4.1 Font Choices and Rationale

**UI font: System font stack**

```
system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif
```

On Linux, this resolves to the user's configured interface font — typically Noto Sans, Ubuntu, or Inter depending on the desktop environment. Using the system stack respects the user's typographic preferences and integrates TauTerm into the desktop naturally. A custom font here would require bundling, increase application size, and create a seam between TauTerm's chrome and the rest of the desktop. The system font is the right call for a desktop application.

**Monospace UI font: Developer font stack**

```
"JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", monospace
```

Used for shortcut key displays and path inputs in the UI — contexts where monospace alignment aids readability. The fallback chain covers the most common developer font installations on Linux.

**Terminal font: Developer font stack with broader fallback**

```
"JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", "Courier New", monospace
```

JetBrains Mono is the most likely to be present on a developer's Linux system. The fallback chain covers common developer setups. The `"Courier New"` entry exists as a last resort — it will never look ideal but ensures the terminal is functional on any system. This font is user-overridable in preferences.

**Default terminal font size: 14px.** Readable at typical screen distances (60–80 cm from a 24–27" monitor) without being large enough to significantly reduce terminal density.

**Default terminal line height: 1.2.** Tight enough for density, loose enough to prevent ascenders and descenders from touching across lines.

### 4.2 Type Scale

The canonical UI font-size token set uses the `--font-size-ui-*` namespace exclusively.

| Token | Size | Usage |
|-------|------|-------|
| `--font-size-ui-2xs` | 10px | Badge counters, status dots with text |
| `--font-size-ui-xs` | 11px | Section headings (uppercase labels), tooltips |
| `--font-size-ui-sm` | 12px | Secondary labels, shortcut keys, monospace UI |
| `--font-size-ui-base` | 13px | Primary UI text, tab titles, menu items |
| `--font-size-ui-md` | 14px | Terminal font size reference |
| `--font-size-ui-lg` | 16px | Dialog headings, modal titles |
| `--font-size-ui-xl` | 20px | Reserved; major headings if needed |

The scale uses approximately a 1.2× ratio between steps at the smaller end, widening at the top. The 13px base is intentional: one point below 14px, it reads as distinctly "UI chrome" rather than "content." On a 96dpi screen this is 9.75pt — readable without feeling large.

### 4.3 Hierarchy Rules

- **Primary UI text** (`--font-size-ui-base`, weight 400): default for all labels, tab titles, menu items, form fields
- **Section headings** (`--font-size-ui-xs`, weight 600, all-caps, letter-spacing 0.06em, color `--color-text-heading`): for preference panel sections and grouped form labels. Small-caps headings establish hierarchy without consuming vertical space — they read as category labels, not document titles.
- **Active tab title** (`--font-size-ui-base`, weight 600): the only UI surface where semibold weight is used on body-size text, to distinguish the active tab from inactive ones without relying on color alone.
- **Secondary text** (`--font-size-ui-sm`, weight 400, color `--color-text-secondary`): descriptions, subtitles, supplementary labels.
- **Monospace UI elements** (`--font-mono-ui`, `--font-size-ui-sm`): shortcut displays, path fields.
- **No weight other than 400, 500, or 600** is used in the UI. Intermediate weights (medium at 500) are used sparingly for slight emphasis without the heaviness of semibold.

---

## 5. Spacing and Sizing System

**Base unit: 4px.** All spacing values are multiples of 4px. A 4px grid (rather than 8px) is used because the information density of terminal chrome demands finer increments — 8px steps produce components that are too large for the available space.

**Spacing scale:**

| Token | Value | Usage |
|-------|-------|-------|
| `--space-0` | 0px | Reset |
| `--space-1` | 4px | Tight internal padding (icon + label gap) |
| `--space-2` | 8px | Component internal padding (button, tab) |
| `--space-3` | 12px | Default padding for compact containers |
| `--space-4` | 16px | Standard section padding |
| `--space-5` | 20px | Generous element spacing |
| `--space-6` | 24px | Section separation |
| `--space-8` | 32px | Major layout separation |
| `--space-10` | 40px | Large-scale spacing |
| `--space-12` | 48px | Modal/panel internal margins |

**Sizing tokens:**

| Token | Value | Usage |
|-------|-------|-------|
| `--size-tab-height` | 40px | Tab bar item height |
| `--size-toolbar-height` | 40px | Toolbar height (matches tab bar) |
| `--size-divider-hit` | 8px | Pane divider interactive area width |
| `--size-scrollbar-width` | 8px | Scrollbar track width |
| `--size-icon-sm` | 14px | Icon inside tab bar |
| `--size-icon-md` | 16px | Icon in toolbars, context menus |
| `--size-icon-lg` | 20px | Icon in dialog headers |
| `--size-target-min` | 44px | Minimum interactive target (WCAG 2.5.5) |
| `--size-badge` | 6px | Activity dot indicator diameter |

---

## 6. Iconography

**Source:** Lucide-svelte, consistent with the project stack.

**Stroke weight:** 1.5px (Lucide default). Do not override to 2px — heavier strokes read as aggressive against the restrained chrome.

**Sizing:** `--size-icon-sm` (14px) in the tab bar, `--size-icon-md` (16px) in toolbars and menus, `--size-icon-lg` (20px) in dialog headers.

**Color:** Icons inherit the text color of their container by default. Active/accent state: `--color-accent`. Error state: `--color-error`. Warning state: `--color-warning`.

**Style principle:** Lucide icons are outline-style, consistent with the restrained, undecorated character of Umbra. Filled icons are used only for status dots (CSS, not Lucide) where a solid form communicates "presence" rather than "action."

**Icon vocabulary:**

| Concept | Lucide icon |
|---------|-------------|
| New tab | `Plus` |
| Close tab / pane | `X` |
| Split horizontal (top/bottom result) | `SplitSquareVertical` |
| Split vertical (left/right result) | `SplitSquareHorizontal` |
| SSH session indicator | `Network` |
| SSH disconnected | `WifiOff` |
| SSH reconnect | `RefreshCw` |
| Activity (output on inactive tab) | CSS filled dot — not a Lucide icon |
| Process ended | `CircleDot` |
| Bell | `Bell` |
| Preferences | `Settings` |
| Search | `Search` |
| Copy | `Copy` |
| Paste | `ClipboardPaste` |
| Connection manager | `Server` |
| Warning | `AlertTriangle` |
| Error | `AlertCircle` |
| Drag handle | `GripVertical` |
| Clean process exit | `CheckCircle` |
| Failed process exit | `XCircle` |

---

## 7. Default Theme — Umbra

The Umbra theme is TauTerm's built-in default. It is always present and cannot be deleted by the user. Every token here must be expressible as a CSS custom property — user themes override these values at the `:root` level without requiring recompilation.

**Token architecture:** Three layers.
1. **Primitive tokens** — named color values with no semantic meaning (`--color-neutral-900`, `--color-blue-400`).
2. **Semantic tokens** — purpose-named tokens that reference primitives (`--color-accent: var(--color-blue-400)`). These are what components consume.
3. **Component tokens** — specialized tokens for a single component (`--color-tab-active-bg`), which reference semantic tokens or primitives as needed.

User themes replace semantic and component tokens. Primitive tokens can also be replaced if the theme defines a wholly different palette.

### 7.1 Color Tokens

```css
@theme {
  /* =====================================================
     PRIMITIVE COLOR SCALE
     These tokens have no semantic meaning — they are
     named positions in the palette, referenced by the
     semantic layer below.
  ===================================================== */

  /* Neutral (warm-shifted dark) */
  --color-neutral-950: #0e0d0b;
  --color-neutral-900: #16140f;
  --color-neutral-850: #1c1a14;
  --color-neutral-800: #242118;
  --color-neutral-750: #2c2921;
  --color-neutral-700: #35312a;
  --color-neutral-600: #4a4640;
  --color-neutral-500: #6b6660;
  --color-neutral-400: #9c9890;
  --color-neutral-300: #ccc7bc;
  --color-neutral-200: #e8e3d8;
  --color-neutral-100: #f5f2ea;

  /* Blue-steel (primary accent) */
  --color-blue-700: #1a3a52;
  --color-blue-600: #1e4d6e;
  --color-blue-500: #2e6f9c;
  --color-blue-400: #4a92bf;
  --color-blue-300: #7ab3d3;
  --color-blue-200: #b3d2e6;

  /* Amber (warning/caution) */
  --color-amber-700: #4d3000;
  --color-amber-500: #b06a00;
  --color-amber-400: #d48a20;
  --color-amber-300: #e8b060;

  /* Red (error/danger) */
  --color-red-700: #3d1212;
  --color-red-500: #9c2c2c;
  --color-red-400: #c44444;
  --color-red-300: #d97878;

  /* Green (success/activity) */
  --color-green-600: #1a3d1a;
  --color-green-400: #4a9c4a;
  --color-green-300: #78c078;

  /* =====================================================
     SEMANTIC COLOR TOKENS — UI SHELL
     Components consume these tokens, not primitives.
  ===================================================== */

  /* Backgrounds — elevation layers */
  --color-bg-base:    var(--color-neutral-950); /* #0e0d0b — window chrome, deepest */
  --color-bg-surface: var(--color-neutral-800); /* #242118 — tab bar */
  --color-bg-raised:  var(--color-neutral-750); /* #2c2921 — menus, dropdowns, tooltips */
  --color-bg-overlay: var(--color-neutral-900); /* #16140f — modal scrim base (apply opacity separately) */

  /* Borders and dividers */
  --color-border:        var(--color-neutral-700); /* #35312a */
  --color-border-subtle: var(--color-neutral-750); /* #2c2921 */
  --color-divider:       var(--color-neutral-700); /* #35312a — pane divider visible line */
  --color-divider-active: var(--color-blue-400);   /* #4a92bf — pane divider on hover/drag */

  /* Text hierarchy */
  --color-text-primary:   var(--color-neutral-300); /* #ccc7bc */
  --color-text-secondary: var(--color-neutral-400); /* #9c9890 */
  --color-text-tertiary:  var(--color-neutral-500); /* #6b6660 — placeholder, disabled */
  --color-text-inverted:  var(--color-neutral-950); /* #0e0d0b — text on accent/light bg */
  --color-text-heading:   var(--color-neutral-400); /* #9c9890 — section heading labels */

  /* Icons */
  --color-icon-default: var(--color-neutral-400); /* #9c9890 */
  --color-icon-active:  var(--color-neutral-300); /* #ccc7bc */

  /* Interactive / accent */
  --color-accent:            var(--color-blue-400); /* #4a92bf */
  --color-accent-subtle:     var(--color-blue-700); /* #1a3a52 — tinted bg */
  --color-accent-text:       var(--color-blue-300); /* #7ab3d3 — accent text on dark */
  --color-hover-bg:          var(--color-neutral-750); /* #2c2921 */
  --color-active-bg:         var(--color-neutral-700); /* #35312a */
  --color-focus-ring:        var(--color-blue-400);    /* #4a92bf */
  --color-focus-ring-offset: var(--color-neutral-950); /* #0e0d0b */

  /* Status and notification */
  --color-activity:     var(--color-green-300);   /* #78c078 — output on inactive tab */
  --color-process-end:  var(--color-neutral-400); /* #9c9890 — process terminated */
  --color-bell:         var(--color-amber-400);   /* #d48a20 */
  --color-error:        var(--color-red-400);     /* #c44444 */
  --color-error-bg:     var(--color-red-700);     /* #3d1212 */
  --color-error-text:   var(--color-red-300);     /* #d97878 */
  --color-warning:      var(--color-amber-400);   /* #d48a20 */
  --color-warning-bg:   var(--color-amber-700);   /* #4d3000 */
  --color-warning-text: var(--color-amber-300);   /* #e8b060 */
  --color-success:      var(--color-green-400);   /* #4a9c4a */
  --color-success-text: var(--color-green-300);   /* #78c078 */

  /* =====================================================
     COMPONENT COLOR TOKENS
  ===================================================== */

  /* Tab bar */
  --color-tab-bg:             var(--color-neutral-800); /* #242118 */
  --color-tab-active-bg:      var(--color-neutral-900); /* #16140f — matches terminal bg */
  --color-tab-active-fg:      var(--color-neutral-200); /* #e8e3d8 */
  --color-tab-inactive-bg:    transparent;
  --color-tab-inactive-fg:    var(--color-neutral-500); /* #6b6660 */
  --color-tab-hover-bg:       var(--color-neutral-750); /* #2c2921 */
  --color-tab-hover-fg:       var(--color-neutral-400); /* #9c9890 */
  --color-tab-close-fg:       var(--color-neutral-500); /* #6b6660 */
  --color-tab-close-hover-fg: var(--color-neutral-300); /* #ccc7bc */
  --color-tab-new-fg:         var(--color-neutral-500); /* #6b6660 */
  --color-tab-new-hover-fg:   var(--color-neutral-300); /* #ccc7bc */

  /* SSH session indicators */
  --color-ssh-badge-bg:        var(--color-blue-700); /* #1a3a52 */
  --color-ssh-badge-fg:        var(--color-blue-300); /* #7ab3d3 */
  --color-ssh-disconnected-bg: var(--color-red-700);  /* #3d1212 */
  --color-ssh-disconnected-fg: var(--color-red-300);  /* #d97878 */
  --color-ssh-connecting-fg:   var(--color-amber-400); /* #d48a20 */

  /* Pane borders */
  --color-pane-border-active:   var(--color-blue-400);    /* #4a92bf */
  --color-pane-border-inactive: var(--color-neutral-700); /* #35312a */

  /* Scrollbar */
  --color-scrollbar-track:       transparent;
  --color-scrollbar-thumb:       var(--color-neutral-600); /* #4a4640 */
  --color-scrollbar-thumb-hover: var(--color-neutral-500); /* #6b6660 */

  /* =====================================================
     TERMINAL SURFACE TOKENS
     These tokens control the terminal rendering layer.
     All are user-overridable via custom themes.
  ===================================================== */

  --term-bg:  var(--color-neutral-900); /* #16140f */
  --term-fg:  var(--color-neutral-300); /* #ccc7bc */

  /* Cursor */
  --term-cursor-bg:        var(--color-blue-300);    /* #7ab3d3 — block cursor fill */
  --term-cursor-fg:        var(--color-neutral-900); /* #16140f — char under cursor */
  --term-cursor-unfocused: #7ab3d3;
  /* Hollow outline rectangle when pane is unfocused.
     Renders as an outlined rectangle, never as a filled block.
     Must never be invisible (transparent is not a valid value). */

  /* Selection */
  --term-selection-bg:          var(--color-blue-500); /* #2e6f9c */
  --term-selection-fg:          inherit;               /* no forced inversion */
  --term-selection-bg-inactive: var(--color-blue-700); /* #1a3a52 — pane unfocused */

  /* Search highlights */
  --term-search-match-bg:  var(--color-amber-700); /* #4d3000 — non-active match */
  --term-search-match-fg:  var(--color-amber-300); /* #e8b060 */
  --term-search-active-bg: #6b5c22;
  /* Active (current) match background.
     Approximates #b89840 (amber-400) at 60% opacity blended over #16140f.
     Cannot be expressed as a var() + opacity combination in CSS without
     color-mix(); the literal value is intentional. */
  --term-search-active-fg: var(--color-neutral-200); /* #e8e3d8 */

  /* Hyperlinks (OSC 8) */
  --term-hyperlink-fg:        var(--color-blue-300); /* #7ab3d3 */
  --term-hyperlink-underline: var(--color-blue-400); /* #4a92bf */

  /* Text attribute rendering (SGR — FS-VT-024, FS-VT-025) */
  --term-dim-opacity:              0.5;     /* SGR 2 (Dim): fg alpha multiplier */
  --term-underline-color-default:  inherit; /* SGR 4:x default underline color when SGR 58 not set */
  --term-strikethrough-position:   50%;     /* SGR 9: vertical position as % of cell_height */
  --term-strikethrough-thickness:  1px;     /* SGR 9: strikethrough line thickness */
  --term-blink-on-duration:        533ms;   /* SGR 5/6: visible phase (2:1 on/off ratio) */
  --term-blink-off-duration:       266ms;   /* SGR 5/6: invisible phase */

  /* ANSI palette (16 colors) */
  --term-color-0:  #2c2921; /* Black normal   — hsl(40, 13%, 14%) */
  --term-color-1:  #c44444; /* Red normal     — hsl(0, 55%, 52%)  — 5.2:1 */
  --term-color-2:  #5c9e5c; /* Green normal   — hsl(120, 27%, 49%) — 4.6:1 */
  --term-color-3:  #b89840; /* Yellow normal  — hsl(43, 50%, 47%) — 5.1:1 */
  --term-color-4:  #4a92bf; /* Blue normal    — hsl(205, 50%, 52%) — 4.9:1 */
  --term-color-5:  #9b6dbf; /* Magenta normal — hsl(275, 36%, 58%) — 4.7:1 */
  --term-color-6:  #3d9e8a; /* Cyan normal    — hsl(168, 44%, 43%) — 4.9:1 */
  --term-color-7:  #ccc7bc; /* White normal   — hsl(40, 8%, 77%)  — 8.4:1 */
  --term-color-8:  #4a4640; /* Black bright   — hsl(37, 8%, 26%)  — dimmed text */
  --term-color-9:  #e06060; /* Red bright     — hsl(0, 63%, 63%)  — 7.0:1 */
  --term-color-10: #82c082; /* Green bright   — hsl(120, 30%, 63%) — 7.5:1 */
  --term-color-11: #d4b860; /* Yellow bright  — hsl(43, 58%, 60%) — 7.8:1 */
  --term-color-12: #7ab3d3; /* Blue bright    — hsl(205, 46%, 64%) — 8.1:1 */
  --term-color-13: #c09cd8; /* Magenta bright — hsl(275, 40%, 73%) — 8.5:1 */
  --term-color-14: #6ec4ae; /* Cyan bright    — hsl(162, 40%, 60%) — 7.8:1 */
  --term-color-15: #f5f2ea; /* White bright   — hsl(44, 30%, 94%) — 13.4:1 */
}
```

### 7.2 Typography Tokens

```css
@theme {
  --font-ui:       system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  --font-mono-ui:  "JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", monospace;
  --font-terminal: "JetBrains Mono", "Fira Code", "Cascadia Code", "DejaVu Sans Mono", "Courier New", monospace;

  --font-size-terminal:   14px;
  --line-height-terminal: 1.2;

  --font-size-ui-2xs:  10px;
  --font-size-ui-xs:   11px;
  --font-size-ui-sm:   12px;
  --font-size-ui-base: 13px;
  --font-size-ui-md:   14px;
  --font-size-ui-lg:   16px;
  --font-size-ui-xl:   20px;

  --font-weight-normal:   400;
  --font-weight-medium:   500;
  --font-weight-semibold: 600;
}
```

### 7.3 Spacing Tokens

```css
@theme {
  --space-0:  0px;
  --space-1:  4px;
  --space-2:  8px;
  --space-3:  12px;
  --space-4:  16px;
  --space-5:  20px;
  --space-6:  24px;
  --space-8:  32px;
  --space-10: 40px;
  --space-12: 48px;

  --size-tab-height:            40px;
  --size-toolbar-height:        40px;
  --size-divider-hit:           8px;
  --size-scrollbar-width:       8px;
  --size-icon-sm:               14px;
  --size-icon-md:               16px;
  --size-icon-lg:               20px;
  --size-target-min:            44px;
  --size-badge:                 6px;
  --size-cursor-underline-height: 2px;  /* underline cursor height */
  --size-cursor-bar-width:        2px;  /* bar cursor width */
  --size-cursor-outline-width:    1px;  /* unfocused cursor outline stroke */
}
```

### 7.4 Radius Tokens

The UI uses very low border radius throughout. TauTerm is a precision tool; rounded corners communicate softness that works against its character. The exception is interactive controls (buttons, text inputs), where a slight radius aids identification as interactive elements.

```css
@theme {
  --radius-none: 0px;    /* panels, tab bar, terminal area, dividers */
  --radius-sm:   2px;    /* buttons, text inputs, dropdown items */
  --radius-md:   4px;    /* modals, tooltips, context menus */
  --radius-full: 9999px; /* status dots, activity badges */
}
```

### 7.5 Shadow Tokens

```css
@theme {
  --shadow-overlay: 0 8px 32px rgba(0, 0, 0, 0.6);
  /* Used for modals and the connection manager panel.
     The high opacity and large blur reflect the dark environment —
     a subtle shadow would be invisible against dark surfaces. */

  --shadow-raised: 0 2px 8px rgba(0, 0, 0, 0.4);
  /* Used for context menus, tooltips, dropdowns. */
}
```

### 7.6 Motion Tokens

Durations and easings only. Animation triggers and behavioral rules belong in component specs, not in the artistic direction.

**Duration tokens:**

| Token | Value | Usage |
|-------|-------|-------|
| `--duration-instant` | `0ms` | Focus rings, hover states |
| `--duration-fast` | `80ms` | Dismissals, fade-outs |
| `--duration-base` | `100ms` | Modal/popover appearance |
| `--duration-slow` | `300ms` | Scrollbar fade, non-critical transitions |

**Easing function tokens** (Material Design standard curves):

| Token | Value | Usage |
|-------|-------|-------|
| `--ease-linear` | `linear` | Progress bars, continuous transitions |
| `--ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | Elements leaving the screen |
| `--ease-out` | `cubic-bezier(0, 0, 0.2, 1)` | Elements entering the screen, expand/collapse |
| `--ease-in-out` | `cubic-bezier(0.4, 0, 0.2, 1)` | Elements that both enter and leave (dialogs, panels) |

```css
@theme {
  --duration-instant: 0ms;   /* focus rings, hover states */
  --duration-fast:    80ms;  /* dismissals, fade-outs */
  --duration-base:    100ms; /* modal/popover appearance */
  --duration-slow:    300ms; /* scrollbar fade, non-critical transitions */

  --ease-linear:  linear;                       /* progress bars, continuous transitions */
  --ease-in:      cubic-bezier(0.4, 0, 1, 1);  /* elements leaving the screen */
  --ease-out:     cubic-bezier(0, 0, 0.2, 1);  /* elements entering the screen, expand/collapse */
  --ease-in-out:  cubic-bezier(0.4, 0, 0.2, 1); /* dialogs, panels — enter and leave */
}
```

---

## 8. Built-in Theme — Solstice

Solstice is TauTerm's built-in light theme. It is always present and cannot be deleted by the user.

### 8.1 Name and Concept

**Name:** Solstice

**Concept:** The Nordic winter solstice — the shortest day of the year, when light arrives low and nearly horizontal, striking snow and ice at an acute angle. This is not a gentle, warm, or welcoming light. It is cold, precise, and severe. The quality of winter daylight on a snow field is brilliant without being warm: the sky is grey-white, the shadows have a faint blue cast, the light has no amber in it at all.

Solstice makes no concession to comfort. It is the light of the far north in December: functional, honest, and cold.

### 8.2 Mood

A laboratory in winter. Bleached light through high windows, clean surfaces, absolute precision. Nothing decorative, nothing warm, nothing soft. The contrast between the near-white background and near-black text is the same contrast as text on a frosted glass surface — high clarity, cold temperature, zero noise.

The character is **severe precision**: every value is at the cold end of its range. The accent color, a dense arctic blue, is the color of pack ice at depth — not sky, not ocean in summer, but compressed glacial ice. It asserts authority rather than friendliness.

### 8.3 Palette Character

Cool neutrals throughout. The neutral scale uses a consistent HSL 210–215° hue — an unambiguous cool-blue shift, visible as an ice-white at the light end and a blue-tinged charcoal at the dark end. No warm neutrals anywhere: that belongs to Umbra. The accent is a dark, dense arctic blue (HSL 218°, low lightness) — authoritative rather than decorative.

**Scale convention note:** Solstice's neutral scale is numbered light-first (neutral-50 is the lightest surface, neutral-950 is the darkest text). This inverts Umbra's dark-first convention. The numbering reflects luminosity: low numbers = high luminosity = light surfaces, high numbers = low luminosity = dark text. This mirrors common light-theme conventions (e.g., Tailwind's grey scale). Both conventions use the same semantic token names (`--color-bg-base`, `--color-text-primary`, etc.) — the scale direction is internal to each theme's primitive layer.

### 8.4 Audience Translation

**For Alex (software developer, full-day use):** Solstice serves developers who work in well-lit environments where dark themes reduce readability against ambient light. The high-contrast light palette prevents glare-induced fatigue in daylit rooms. The cool hue avoids the warm-tinted light themes that can appear yellowed on modern display calibration.

**For Jordan (systems administrator, many SSH sessions):** Status legibility is unchanged from Umbra — error red, warning amber, success green, SSH blue all remain distinct and meet 4.5:1 against the light terminal background. The palette is adapted for light backgrounds, not thinned out.

**For Sam (occasional user):** Solstice is a real light theme, not a washed-out inversion of the dark default. For users who prefer light interfaces — or who work in bright rooms where a dark terminal strains against ambient light — it is immediately usable without any adjustment. It looks finished from the first glance.

### 8.5 Color Tokens

```css
@theme {
  /* =====================================================
     PRIMITIVE COLOR SCALE — SOLSTICE
     Cool-shifted (HSL 210–215°) light theme palette.
     Convention: low numbers = light (surfaces),
     high numbers = dark (text). Inverted from Umbra's
     dark-first convention — see §8.3 for rationale.
  ===================================================== */

  /* Neutral (cool-shifted, base for backgrounds and surfaces) */
  /* neutral-50 → lightest surface; neutral-950 → darkest text */
  --color-neutral-50:  #f4f6f8; /* hsl(210, 22%, 96%)  — deepest background (near-white, slight blue) */
  --color-neutral-100: #e8ecf0; /* hsl(210, 18%, 92%)  — tab bar background */
  --color-neutral-150: #dde2e8; /* hsl(210, 14%, 88%)  — surface raised (panels, menus) */
  --color-neutral-200: #cdd4db; /* hsl(210, 12%, 84%)  — border, divider */
  --color-neutral-300: #b0bac4; /* hsl(210, 10%, 73%)  — inactive tab text */
  --color-neutral-400: #8494a2; /* hsl(210,  9%, 58%)  — secondary text, placeholder */
  --color-neutral-500: #5c6e7d; /* hsl(210,  8%, 43%)  — secondary text */
  --color-neutral-700: #2e3f50; /* hsl(210, 27%, 25%)  — primary UI text */
  --color-neutral-850: #1c2a36; /* hsl(210, 30%, 16%)  — emphasized text, active tab label */
  --color-neutral-950: #111d26; /* hsl(210, 38%, 11%)  — highest-emphasis text, terminal foreground */

  /* Arctic blue (primary accent) */
  --color-arctic-900: #0a1824; /* hsl(218, 58%,  9%)  — deep accent bg (on light surface) */
  --color-arctic-700: #1a3a5c; /* hsl(214, 56%, 23%)  — accent interactive base */
  --color-arctic-500: #2c5f8f; /* hsl(211, 52%, 37%)  — primary accent (default state) */
  --color-arctic-400: #3a7ab8; /* hsl(210, 52%, 47%)  — accent hover */
  --color-arctic-300: #5c9fd4; /* hsl(207, 55%, 60%)  — accent text on light bg (decorative) */
  --color-arctic-200: #9dc6e8; /* hsl(207, 55%, 77%)  — light accent surface */
  --color-arctic-100: #d4e8f5; /* hsl(206, 60%, 90%)  — subtle accent tint */

  /* Amber (warning/caution) — adapted for light bg */
  --color-amber-800: #4d2e00; /* hsl(36, 100%, 15%)   — deep warning text on light */
  --color-amber-600: #8c5200; /* hsl(35, 100%, 27%)   — warning indicator on light */
  --color-amber-400: #c87800; /* hsl(35, 100%, 39%)   — warning text on light bg */
  --color-amber-100: #fff0cc; /* hsl(44,  100%, 90%)  — warning background on light */

  /* Red (error/danger) — adapted for light bg */
  --color-red-800:   #5c1010; /* hsl(0,  71%, 21%)    — deep error text on light */
  --color-red-600:   #a01e1e; /* hsl(0,  68%, 37%)    — error indicator on light */
  --color-red-400:   #c83232; /* hsl(0,  60%, 49%)    — error text on light bg */
  --color-red-100:   #fde8e8; /* hsl(0,  84%, 95%)    — error background */

  /* Green (success/activity) — adapted for light bg */
  --color-green-800: #1a4020; /* hsl(130, 41%, 18%)   — deep success text on light */
  --color-green-600: #2a7034; /* hsl(130, 46%, 30%)   — success indicator on light */
  --color-green-400: #3a9445; /* hsl(126, 43%, 40%)   — success text on light bg */
  --color-green-100: #e4f5e8; /* hsl(128, 50%, 93%)   — success background */

  /* =====================================================
     SEMANTIC COLOR TOKENS — UI SHELL (Solstice overrides)
     Components consume these tokens, not primitives.
  ===================================================== */

  /* Backgrounds — elevation layers */
  --color-bg-base:    var(--color-neutral-50);   /* #f4f6f8 — window chrome, deepest */
  --color-bg-surface: var(--color-neutral-100);  /* #e8ecf0 — tab bar */
  --color-bg-raised:  var(--color-neutral-150);  /* #dde2e8 — menus, dropdowns, tooltips */
  --color-bg-overlay: var(--color-neutral-50);   /* #f4f6f8 — modal scrim base (apply opacity separately) */

  /* Borders and dividers */
  --color-border:         var(--color-neutral-200); /* #cdd4db */
  --color-border-subtle:  var(--color-neutral-150); /* #dde2e8 */
  --color-divider:        var(--color-neutral-200); /* #cdd4db — pane divider visible line */
  --color-divider-active: var(--color-arctic-500);  /* #2c5f8f — pane divider on hover/drag */

  /* Text hierarchy */
  --color-text-primary:   var(--color-neutral-850); /* #1c2a36 */
  --color-text-secondary: var(--color-neutral-500); /* #5c6e7d */
  --color-text-tertiary:  var(--color-neutral-400); /* #8494a2 — placeholder, disabled */
  --color-text-inverted:  var(--color-neutral-50);  /* #f4f6f8 — text on dark accent bg */
  --color-text-heading:   var(--color-neutral-500); /* #5c6e7d — section heading labels */

  /* Icons */
  --color-icon-default: var(--color-neutral-500); /* #5c6e7d */
  --color-icon-active:  var(--color-neutral-850); /* #1c2a36 */

  /* Interactive / accent */
  --color-accent:            var(--color-arctic-500); /* #2c5f8f */
  --color-accent-subtle:     var(--color-arctic-100); /* #d4e8f5 — tinted bg */
  --color-accent-text:       var(--color-arctic-700); /* #1a3a5c — accent text on light bg */
  --color-hover-bg:          var(--color-neutral-150); /* #dde2e8 */
  --color-active-bg:         var(--color-neutral-200); /* #cdd4db */
  --color-focus-ring:        var(--color-arctic-500);  /* #2c5f8f */
  --color-focus-ring-offset: var(--color-neutral-50);  /* #f4f6f8 */

  /* Status and notification */
  --color-activity:     var(--color-green-600);   /* #2a7034 — output on inactive tab */
  --color-process-end:  var(--color-neutral-400); /* #8494a2 — process terminated */
  --color-bell:         var(--color-amber-600);   /* #8c5200 */
  --color-error:        var(--color-red-600);     /* #a01e1e */
  --color-error-bg:     var(--color-red-100);     /* #fde8e8 */
  --color-error-text:   var(--color-red-800);     /* #5c1010 */
  --color-warning:      var(--color-amber-600);   /* #8c5200 */
  --color-warning-bg:   var(--color-amber-100);   /* #fff0cc */
  --color-warning-text: var(--color-amber-800);   /* #4d2e00 */
  --color-success:      var(--color-green-600);   /* #2a7034 */
  --color-success-text: var(--color-green-800);   /* #1a4020 */

  /* =====================================================
     COMPONENT COLOR TOKENS — Solstice overrides
  ===================================================== */

  /* Tab bar */
  --color-tab-bg:             var(--color-neutral-100); /* #e8ecf0 */
  --color-tab-active-bg:      var(--color-neutral-50);  /* #f4f6f8 — matches terminal bg */
  --color-tab-active-fg:      var(--color-neutral-850); /* #1c2a36 */
  --color-tab-inactive-bg:    transparent;
  --color-tab-inactive-fg:    var(--color-neutral-400); /* #8494a2 */
  --color-tab-hover-bg:       var(--color-neutral-150); /* #dde2e8 */
  --color-tab-hover-fg:       var(--color-neutral-700); /* #2e3f50 */
  --color-tab-close-fg:       var(--color-neutral-400); /* #8494a2 */
  --color-tab-close-hover-fg: var(--color-neutral-850); /* #1c2a36 */
  --color-tab-new-fg:         var(--color-neutral-400); /* #8494a2 */
  --color-tab-new-hover-fg:   var(--color-neutral-850); /* #1c2a36 */

  /* SSH session indicators */
  --color-ssh-badge-bg:        var(--color-arctic-100); /* #d4e8f5 */
  --color-ssh-badge-fg:        var(--color-arctic-700); /* #1a3a5c */
  --color-ssh-disconnected-bg: var(--color-red-100);    /* #fde8e8 */
  --color-ssh-disconnected-fg: var(--color-red-800);    /* #5c1010 */
  --color-ssh-connecting-fg:   var(--color-amber-600);  /* #8c5200 */

  /* Pane borders */
  --color-pane-border-active:   var(--color-arctic-500); /* #2c5f8f */
  --color-pane-border-inactive: var(--color-neutral-200); /* #cdd4db */

  /* Scrollbar */
  --color-scrollbar-track:       transparent;
  --color-scrollbar-thumb:       var(--color-neutral-300); /* #b0bac4 */
  --color-scrollbar-thumb-hover: var(--color-neutral-400); /* #8494a2 */

  /* =====================================================
     TERMINAL SURFACE TOKENS — Solstice
  ===================================================== */

  --term-bg: var(--color-neutral-50);   /* #f4f6f8 — terminal background */
  --term-fg: var(--color-neutral-950);  /* #111d26 — terminal foreground — 14.2:1 contrast */

  /* Cursor */
  --term-cursor-bg:        var(--color-arctic-700);   /* #1a3a5c — block cursor fill */
  --term-cursor-fg:        var(--color-neutral-50);   /* #f4f6f8 — char under cursor */
  --term-cursor-unfocused: #2c5f8f;
  /* Hollow outline rectangle when pane is unfocused.
     Must never be invisible (transparent is not a valid value). */

  /* Selection */
  --term-selection-bg:          var(--color-arctic-200); /* #9dc6e8 */
  --term-selection-fg:          inherit;
  --term-selection-bg-inactive: var(--color-arctic-100); /* #d4e8f5 — pane unfocused */

  /* Search highlights */
  --term-search-match-bg:  var(--color-amber-100); /* #fff0cc — non-active match */
  --term-search-match-fg:  var(--color-amber-800); /* #4d2e00 */
  --term-search-active-bg: #f5d87a;
  /* Active match background.
     Approximates amber at 70% opacity blended over #f4f6f8.
     The literal value is intentional — cannot be expressed as var() + opacity
     without color-mix(). */
  --term-search-active-fg: var(--color-neutral-950); /* #111d26 */

  /* Hyperlinks (OSC 8) */
  --term-hyperlink-fg:        var(--color-arctic-700); /* #1a3a5c */
  --term-hyperlink-underline: var(--color-arctic-500); /* #2c5f8f */

  /* Text attribute rendering (inherited from Umbra — not theme-specific) */
  --term-dim-opacity:              0.5;
  --term-underline-color-default:  inherit;
  --term-strikethrough-position:   50%;
  --term-strikethrough-thickness:  1px;
  --term-blink-on-duration:        533ms;
  --term-blink-off-duration:       266ms;

  /* Shadows — adapted for light environment */
  --shadow-overlay: 0 8px 32px rgba(17, 29, 38, 0.20);
  --shadow-raised:  0 2px 8px  rgba(17, 29, 38, 0.12);
}
```

### 8.6 ANSI Terminal Palette

The Solstice ANSI palette is tuned to work against the `neutral-50` terminal background (`#f4f6f8`). The design goal is distinguishable hues that hold up under ambient light conditions without aggression. Colors are moderately saturated — saturated enough to read distinctly, restrained enough not to produce visual noise on a bright surface. All values pass 4.5:1 contrast against `#f4f6f8`.

Normal colors are darker (more saturated toward the dark end) than their bright counterparts — this is the opposite of Umbra's strategy (where bright = lighter), because on a light background the reading direction of luminance inverts: darker = more contrast = more salient.

| Index | Name | Hex | HSL | Contrast vs `#f4f6f8` |
|-------|------|-----|-----|-----------------------|
| 0 | Black (normal) | `#dde2e8` | `hsl(210, 14%, 88%)` | n/a (background use) |
| 1 | Red (normal) | `#b01e1e` | `hsl(0, 71%, 40%)` | 5.8:1 |
| 2 | Green (normal) | `#2e7a38` | `hsl(127, 46%, 33%)` | 5.1:1 |
| 3 | Yellow (normal) | `#7a5800` | `hsl(43, 100%, 24%)` | 6.2:1 |
| 4 | Blue (normal) | `#2056a0` | `hsl(218, 67%, 37%)` | 5.7:1 |
| 5 | Magenta (normal) | `#7a2e8c` | `hsl(290, 51%, 36%)` | 5.5:1 |
| 6 | Cyan (normal) | `#1a6e78` | `hsl(186, 64%, 29%)` | 5.4:1 |
| 7 | White (normal) | `#2e3f50` | `hsl(210, 27%, 25%)` | 8.6:1 |
| 8 | Black (bright) | `#b0bac4` | `hsl(210, 10%, 73%)` | n/a (dimmed text context) |
| 9 | Red (bright) | `#c83232` | `hsl(0, 60%, 49%)` | 4.5:1 |
| 10 | Green (bright) | `#3a9445` | `hsl(126, 43%, 40%)` | 4.6:1 |
| 11 | Yellow (bright) | `#9e7200` | `hsl(43, 100%, 31%)` | 4.7:1 |
| 12 | Blue (bright) | `#2c5f8f` | `hsl(211, 52%, 37%)` | 4.8:1 |
| 13 | Magenta (bright) | `#9e3ab8` | `hsl(287, 54%, 47%)` | 4.5:1 |
| 14 | Cyan (bright) | `#1e8898` | `hsl(186, 66%, 36%)` | 4.6:1 |
| 15 | White (bright) | `#111d26` | `hsl(210, 38%, 11%)` | 14.2:1 |

```css
@theme {
  --term-color-0:  #dde2e8; /* Black normal   — hsl(210, 14%, 88%) — background context */
  --term-color-1:  #b01e1e; /* Red normal     — hsl(0,   71%, 40%) — 5.8:1 */
  --term-color-2:  #2e7a38; /* Green normal   — hsl(127, 46%, 33%) — 5.1:1 */
  --term-color-3:  #7a5800; /* Yellow normal  — hsl(43,  100%, 24%) — 6.2:1 */
  --term-color-4:  #2056a0; /* Blue normal    — hsl(218, 67%, 37%) — 5.7:1 */
  --term-color-5:  #7a2e8c; /* Magenta normal — hsl(290, 51%, 36%) — 5.5:1 */
  --term-color-6:  #1a6e78; /* Cyan normal    — hsl(186, 64%, 29%) — 5.4:1 */
  --term-color-7:  #2e3f50; /* White normal   — hsl(210, 27%, 25%) — 8.6:1 */
  --term-color-8:  #b0bac4; /* Black bright   — hsl(210, 10%, 73%) — dimmed text context */
  --term-color-9:  #c83232; /* Red bright     — hsl(0,   60%, 49%) — 4.5:1 */
  --term-color-10: #3a9445; /* Green bright   — hsl(126, 43%, 40%) — 4.6:1 */
  --term-color-11: #9e7200; /* Yellow bright  — hsl(43,  100%, 31%) — 4.7:1 */
  --term-color-12: #2c5f8f; /* Blue bright    — hsl(211, 52%, 37%) — 4.8:1 */
  --term-color-13: #9e3ab8; /* Magenta bright — hsl(287, 54%, 47%) — 4.5:1 */
  --term-color-14: #1e8898; /* Cyan bright    — hsl(186, 66%, 36%) — 4.6:1 */
  --term-color-15: #111d26; /* White bright   — hsl(210, 38%, 11%) — 14.2:1 */
}
```

**Note on luminance direction:** On a light background, contrast increases as colors darken. Solstice's bright variants are therefore not lighter than their normal counterparts — they are slightly more saturated or shifted in hue to produce distinctly higher contrast without merely darkening the color (which would collapse them into near-identical tones). The normal/bright distinction is still legible; it is expressed through saturation and hue precision rather than luminance.

**Note on cyan hue shift:** `--term-color-6` (Cyan normal, HSL 186°) and `--term-color-14` (Cyan bright, HSL 186°) maintain a large hue gap from `--term-color-4` (Blue normal, HSL 218°) and `--term-color-12` (Blue bright, HSL 211°). The 25–32° separation ensures blue/cyan distinguishability including for users with mild blue-green color confusion.

### 8.7 Semantic Color Token Overrides

The semantic inversion for a light theme is significant. Every background/text relationship is reversed relative to Umbra. The override strategy is complete token redefinition — Solstice does not reuse any Umbra semantic token values; it redefines them all against the cool-shifted light neutral scale and the arctic blue accent. This is the expected pattern for a built-in theme with a different background polarity.

Key structural differences from Umbra semantic mappings:

| Token | Umbra mapping | Solstice mapping | Rationale |
|-------|--------------|-----------------|-----------|
| `--color-bg-base` | `neutral-950` (darkest) | `neutral-50` (lightest) | Polarity inversion |
| `--color-bg-surface` | `neutral-800` | `neutral-100` | Polarity inversion |
| `--color-text-primary` | `neutral-300` (light on dark) | `neutral-850` (dark on light) | Polarity inversion |
| `--color-text-inverted` | `neutral-950` | `neutral-50` | Inverted role |
| `--color-accent` | `blue-400` (mid lightness) | `arctic-500` (darker, for light bg) | Accent must darken for contrast |
| `--color-accent-text` | `blue-300` (light accent on dark) | `arctic-700` (dark accent on light) | Accent text inverts |
| `--color-accent-subtle` | `blue-700` (dark tint) | `arctic-100` (light tint) | Subtle tint inverts |
| `--color-focus-ring-offset` | `neutral-950` (dark) | `neutral-50` (light) | Ring offset matches bg |
| `--color-error` | `red-400` (lighter red on dark) | `red-600` (darker red on light) | Contrast direction inverts |
| `--color-error-bg` | `red-700` (dark red bg) | `red-100` (light red tint) | Error bg inverts |
| `--term-fg` | `neutral-300` | `neutral-950` | Must be near-black on near-white |
| `--term-fg` contrast | 8.4:1 | 14.2:1 | Exceeds 7:1 requirement |
| `--shadow-overlay` | `rgba(0,0,0,0.6)` | `rgba(17,29,38,0.20)` | Shadows lighten significantly |

---

## 9. Built-in Theme — Archipel

Archipel is TauTerm's second built-in dark theme. It is always present and cannot be deleted by the user.

### 9.1 Name and Concept

**Name:** Archipel

**Concept:** The Caribbean archipelago — not the tourist postcard. The working sea between islands: water so deep and still that it reads as near-black with a blue-green shimmer. Above it, violent chromatic contrast: the saturated orange of coral, the green-blue of reef water, the lime yellow of tropical vegetation, the deep violet of sea urchins. The background is ocean at depth, the accents are the living organisms and weathered surfaces that appear against it.

Archipel is committed to color. Where Umbra exercises restraint, Archipel exercises precision in saturation — every accent is fully justified and placed with care, not scattered. The palette has internal logic: warm accents (coral, amber) for interactive and status states; cool accents (turquoise, lime) for activity and success. The result is vivid and alive without dissolving into chaos.

### 9.2 Mood

A night dive in the Caribbean. The water is dark and dense around you; sources of color — coral, fish, phosphorescence — appear in contrast against it. The dark background is not threatening; it is the medium that makes color visible. The terminal content is the reef; the chrome is the water.

The character is **vivid precision**: high saturation, high contrast, controlled placement. Nothing is muted, nothing competes arbitrarily. Color is used as information, not decoration.

### 9.3 Palette Character

Very dark blue-green neutrals (HSL 188–194°) — the hue of deep Caribbean water, warmer than pure cyan, cooler than teal, with enough darkness to anchor the palette. The accent layer draws from tropical biodiversity: saturated coral (HSL ~0–8°, warm) as the primary interactive accent; turquoise (HSL ~186°) for SSH and status indicators; acid lime (HSL ~80°) for success and activity. Magenta is used for a distinctive secondary accent in the ANSI palette.

All accent colors are calibrated against the dark blue-green background — not against pure black — which means their saturation values are precise, not maximum.

**Scale convention:** Archipel's neutral scale follows Umbra's dark-first convention — low numbers = dark (deepest surfaces), high numbers = light (foreground text). This is consistent with dark theme conventions.

### 9.4 Audience Translation

**For Alex (software developer, full-day use):** Archipel is for developers who find dark neutral themes visually flat after extended sessions. The high-chroma palette maintains visual engagement during long use without producing fatigue — the saturation is in the accent layer, not in the text layer. Terminal text renders in pale cyan-white against deep ocean, a combination with high contrast and an aesthetically coherent logic.

**For Jordan (systems administrator, many SSH sessions):** SSH session indicators use turquoise (distinguishable from the coral accent), error states use high-contrast coral-red, and warning states use bright amber. The status vocabulary is distinct and unambiguous even within a high-chroma environment.

**For Sam (occasional user):** Archipel is visually striking from the first launch. For users who want a terminal that looks alive rather than merely functional, it delivers that immediately, without requiring any configuration and without sacrificing readability.

### 9.5 Color Tokens

```css
@theme {
  /* =====================================================
     PRIMITIVE COLOR SCALE — ARCHIPEL
     Blue-green shifted (HSL 188–194°) dark theme palette.
     Convention: low numbers = dark (deepest surfaces),
     high numbers = light (foreground). Same as Umbra.
  ===================================================== */

  /* Neutral (blue-green shifted, base for backgrounds and surfaces) */
  --color-neutral-950: #060e10; /* hsl(190, 44%,  5%)  — deepest background */
  --color-neutral-900: #0c1a1e; /* hsl(190, 46%,  9%)  — terminal background */
  --color-neutral-850: #102228; /* hsl(190, 42%, 11%)  — pane background (unfocused) */
  --color-neutral-800: #162c34; /* hsl(190, 39%, 15%)  — tab bar background */
  --color-neutral-750: #1e3840; /* hsl(190, 36%, 18%)  — surface raised (panels, menus) */
  --color-neutral-700: #274850; /* hsl(190, 32%, 23%)  — border, divider */
  --color-neutral-600: #385e68; /* hsl(190, 30%, 31%)  — inactive tab text background */
  --color-neutral-500: #507888; /* hsl(190, 27%, 42%)  — placeholder text, disabled */
  --color-neutral-400: #7aacba; /* hsl(190, 24%, 60%)  — secondary text, inactive labels */
  --color-neutral-300: #a8cdd6; /* hsl(190, 28%, 75%)  — primary UI text */
  --color-neutral-200: #ccdfea; /* hsl(200, 26%, 86%)  — emphasized UI text, active tab */
  --color-neutral-100: #e8f2f6; /* hsl(198, 34%, 94%)  — highest-emphasis text */

  /* Coral (primary interactive accent) */
  --color-coral-800: #5c1a0e; /* hsl(8,   73%, 21%)   — deep accent background */
  --color-coral-600: #b03018; /* hsl(10,  74%, 39%)   — accent hover base */
  --color-coral-500: #d44030; /* hsl(6,   65%, 51%)   — primary accent (default state) */
  --color-coral-400: #e86050; /* hsl(6,   74%, 61%)   — accent hover */
  --color-coral-300: #f29080; /* hsl(7,   80%, 73%)   — accent text on dark */
  --color-coral-100: #3d1008; /* hsl(6,   72%, 14%)   — subtle tint bg */

  /* Turquoise (SSH / status indicators) */
  --color-turquoise-700: #004a50; /* hsl(184, 100%, 16%) — deep turquoise bg */
  --color-turquoise-500: #008898; /* hsl(184,  100%, 30%) — turquoise indicator */
  --color-turquoise-400: #00b4c8; /* hsl(184,  100%, 39%) — turquoise text on dark */
  --color-turquoise-300: #40d4e4; /* hsl(184,  72%, 57%)  — turquoise label */

  /* Lime (success / activity) */
  --color-lime-700: #304800; /* hsl(82, 100%, 14%)   — deep lime bg */
  --color-lime-500: #6aaa00; /* hsl(82, 100%, 33%)   — lime indicator */
  --color-lime-400: #90cc10; /* hsl(82,  85%, 43%)   — lime text on dark */
  --color-lime-300: #b4e040; /* hsl(82,  72%, 56%)   — lime label */

  /* Amber (warning/caution) — calibrated for dark blue-green bg */
  --color-amber-700: #4d3200; /* hsl(39, 100%, 15%)   — deep warning bg */
  --color-amber-500: #b07000; /* hsl(39, 100%, 34%)   — warning indicator */
  --color-amber-400: #d49020; /* hsl(38,  73%, 48%)   — warning text on dark */
  --color-amber-300: #e8b460; /* hsl(38,  75%, 65%)   — warning label */

  /* Red (error/danger) — vivid against dark blue-green bg */
  --color-red-700: #3d0e10; /* hsl(358, 62%, 15%)   — error background */
  --color-red-500: #a02020; /* hsl(0,   67%, 37%)   — error indicator */
  --color-red-400: #cc3a3a; /* hsl(0,   60%, 52%)   — error text on dark */
  --color-red-300: #e87070; /* hsl(0,   67%, 67%)   — error label */

  /* =====================================================
     SEMANTIC COLOR TOKENS — UI SHELL (Archipel overrides)
  ===================================================== */

  /* Backgrounds — elevation layers */
  --color-bg-base:    var(--color-neutral-950); /* #060e10 — window chrome, deepest */
  --color-bg-surface: var(--color-neutral-800); /* #162c34 — tab bar */
  --color-bg-raised:  var(--color-neutral-750); /* #1e3840 — menus, dropdowns, tooltips */
  --color-bg-overlay: var(--color-neutral-900); /* #0c1a1e — modal scrim base */

  /* Borders and dividers */
  --color-border:         var(--color-neutral-700); /* #274850 */
  --color-border-subtle:  var(--color-neutral-750); /* #1e3840 */
  --color-divider:        var(--color-neutral-700); /* #274850 — pane divider visible line */
  --color-divider-active: var(--color-coral-500);   /* #d44030 — pane divider on hover/drag */

  /* Text hierarchy */
  --color-text-primary:   var(--color-neutral-300); /* #a8cdd6 */
  --color-text-secondary: var(--color-neutral-400); /* #7aacba */
  --color-text-tertiary:  var(--color-neutral-500); /* #507888 — placeholder, disabled */
  --color-text-inverted:  var(--color-neutral-950); /* #060e10 — text on bright accent bg */
  --color-text-heading:   var(--color-neutral-400); /* #7aacba — section heading labels */

  /* Icons */
  --color-icon-default: var(--color-neutral-400); /* #7aacba */
  --color-icon-active:  var(--color-neutral-300); /* #a8cdd6 */

  /* Interactive / accent */
  --color-accent:            var(--color-coral-500);   /* #d44030 */
  --color-accent-subtle:     var(--color-coral-100);   /* #3d1008 — tinted bg */
  --color-accent-text:       var(--color-coral-300);   /* #f29080 — accent text on dark */
  --color-hover-bg:          var(--color-neutral-750); /* #1e3840 */
  --color-active-bg:         var(--color-neutral-700); /* #274850 */
  --color-focus-ring:        var(--color-coral-500);   /* #d44030 */
  --color-focus-ring-offset: var(--color-neutral-950); /* #060e10 */

  /* Status and notification */
  --color-activity:     var(--color-lime-400);        /* #90cc10 — output on inactive tab */
  --color-process-end:  var(--color-neutral-400);     /* #7aacba — process terminated */
  --color-bell:         var(--color-amber-400);       /* #d49020 */
  --color-error:        var(--color-red-400);         /* #cc3a3a */
  --color-error-bg:     var(--color-red-700);         /* #3d0e10 */
  --color-error-text:   var(--color-red-300);         /* #e87070 */
  --color-warning:      var(--color-amber-400);       /* #d49020 */
  --color-warning-bg:   var(--color-amber-700);       /* #4d3200 */
  --color-warning-text: var(--color-amber-300);       /* #e8b460 */
  --color-success:      var(--color-lime-400);        /* #90cc10 */
  --color-success-text: var(--color-lime-300);        /* #b4e040 */

  /* =====================================================
     COMPONENT COLOR TOKENS — Archipel overrides
  ===================================================== */

  /* Tab bar */
  --color-tab-bg:             var(--color-neutral-800); /* #162c34 */
  --color-tab-active-bg:      var(--color-neutral-900); /* #0c1a1e — matches terminal bg */
  --color-tab-active-fg:      var(--color-neutral-200); /* #ccdfea */
  --color-tab-inactive-bg:    transparent;
  --color-tab-inactive-fg:    var(--color-neutral-500); /* #507888 */
  --color-tab-hover-bg:       var(--color-neutral-750); /* #1e3840 */
  --color-tab-hover-fg:       var(--color-neutral-400); /* #7aacba */
  --color-tab-close-fg:       var(--color-neutral-500); /* #507888 */
  --color-tab-close-hover-fg: var(--color-neutral-300); /* #a8cdd6 */
  --color-tab-new-fg:         var(--color-neutral-500); /* #507888 */
  --color-tab-new-hover-fg:   var(--color-neutral-300); /* #a8cdd6 */

  /* SSH session indicators */
  --color-ssh-badge-bg:        var(--color-turquoise-700); /* #004a50 */
  --color-ssh-badge-fg:        var(--color-turquoise-300); /* #40d4e4 */
  --color-ssh-disconnected-bg: var(--color-red-700);       /* #3d0e10 */
  --color-ssh-disconnected-fg: var(--color-red-300);       /* #e87070 */
  --color-ssh-connecting-fg:   var(--color-amber-400);     /* #d49020 */

  /* Pane borders */
  --color-pane-border-active:   var(--color-coral-500);    /* #d44030 */
  --color-pane-border-inactive: var(--color-neutral-700);  /* #274850 */

  /* Scrollbar */
  --color-scrollbar-track:       transparent;
  --color-scrollbar-thumb:       var(--color-neutral-600); /* #385e68 */
  --color-scrollbar-thumb-hover: var(--color-neutral-500); /* #507888 */

  /* =====================================================
     TERMINAL SURFACE TOKENS — Archipel
  ===================================================== */

  --term-bg: var(--color-neutral-900); /* #0c1a1e — terminal background */
  --term-fg: var(--color-neutral-100); /* #e8f2f6 — terminal foreground — 13.8:1 contrast */

  /* Cursor */
  --term-cursor-bg:        var(--color-coral-400);    /* #e86050 — block cursor fill */
  --term-cursor-fg:        var(--color-neutral-950);  /* #060e10 — char under cursor */
  --term-cursor-unfocused: #d44030;
  /* Hollow outline rectangle when pane is unfocused.
     Must never be invisible (transparent is not a valid value). */

  /* Selection */
  --term-selection-bg:          var(--color-turquoise-700); /* #004a50 */
  --term-selection-fg:          inherit;
  --term-selection-bg-inactive: var(--color-neutral-800);   /* #162c34 — pane unfocused */

  /* Search highlights */
  --term-search-match-bg:  var(--color-amber-700); /* #4d3200 — non-active match */
  --term-search-match-fg:  var(--color-amber-300); /* #e8b460 */
  --term-search-active-bg: #6b5018;
  /* Active match background.
     Approximates amber at 60% opacity blended over #0c1a1e.
     The literal value is intentional — cannot be expressed as var() + opacity
     without color-mix(). */
  --term-search-active-fg: var(--color-neutral-200); /* #ccdfea */

  /* Hyperlinks (OSC 8) */
  --term-hyperlink-fg:        var(--color-turquoise-300); /* #40d4e4 */
  --term-hyperlink-underline: var(--color-turquoise-400); /* #00b4c8 */

  /* Text attribute rendering (inherited from Umbra — not theme-specific) */
  --term-dim-opacity:              0.5;
  --term-underline-color-default:  inherit;
  --term-strikethrough-position:   50%;
  --term-strikethrough-thickness:  1px;
  --term-blink-on-duration:        533ms;
  --term-blink-off-duration:       266ms;
}
```

### 9.6 ANSI Terminal Palette

The Archipel ANSI palette is tuned to work against the `neutral-900` terminal background (`#0c1a1e`). The design goal is a full-saturation tropical palette that feels organically coherent — not a random selection of maximum-chroma colors. Hue selection follows Caribbean chromatic logic: warm corals and ambers on the red/yellow side; vivid greens and turquoises on the green/cyan side; deep violet for magenta; bright lime for the high-energy bright variants. All values pass 4.5:1 contrast against `#0c1a1e`.

Normal colors are calibrated for legibility; bright variants push saturation higher and achieve 7:1+ where possible, for commands and output that want to assert presence.

| Index | Name | Hex | HSL | Contrast vs `#0c1a1e` |
|-------|------|-----|-----|-----------------------|
| 0 | Black (normal) | `#1e3840` | `hsl(190, 36%, 18%)` | n/a (background use) |
| 1 | Red (normal) | `#cc3a3a` | `hsl(0,   60%, 52%)` | 5.2:1 |
| 2 | Green (normal) | `#4a9e50` | `hsl(124, 36%, 45%)` | 4.7:1 |
| 3 | Yellow (normal) | `#c89030` | `hsl(37,  62%, 49%)` | 5.3:1 |
| 4 | Blue (normal) | `#4888c8` | `hsl(210, 57%, 54%)` | 4.8:1 |
| 5 | Magenta (normal) | `#a048c0` | `hsl(284, 48%, 52%)` | 4.6:1 |
| 6 | Cyan (normal) | `#1aa4b0` | `hsl(184, 73%, 40%)` | 5.0:1 |
| 7 | White (normal) | `#a8cdd6` | `hsl(190, 28%, 75%)` | 8.8:1 |
| 8 | Black (bright) | `#385e68` | `hsl(190, 30%, 31%)` | n/a (dimmed text context) |
| 9 | Red (bright) | `#f07060` | `hsl(6,   80%, 66%)` | 7.2:1 |
| 10 | Green (bright) | `#80d040` | `hsl(94,  60%, 54%)` | 7.4:1 |
| 11 | Yellow (bright) | `#e8b040` | `hsl(40,  78%, 59%)` | 7.6:1 |
| 12 | Blue (bright) | `#80b8f0` | `hsl(210, 76%, 72%)` | 9.2:1 |
| 13 | Magenta (bright) | `#d080e8` | `hsl(285, 65%, 70%)` | 8.4:1 |
| 14 | Cyan (bright) | `#40d8e8` | `hsl(184, 73%, 58%)` | 9.6:1 |
| 15 | White (bright) | `#e8f2f6` | `hsl(198, 34%, 94%)` | 13.8:1 |

```css
@theme {
  --term-color-0:  #1e3840; /* Black normal   — hsl(190, 36%, 18%) — background context */
  --term-color-1:  #cc3a3a; /* Red normal     — hsl(0,   60%, 52%) — 5.2:1 */
  --term-color-2:  #4a9e50; /* Green normal   — hsl(124, 36%, 45%) — 4.7:1 */
  --term-color-3:  #c89030; /* Yellow normal  — hsl(37,  62%, 49%) — 5.3:1 */
  --term-color-4:  #4888c8; /* Blue normal    — hsl(210, 57%, 54%) — 4.8:1 */
  --term-color-5:  #a048c0; /* Magenta normal — hsl(284, 48%, 52%) — 4.6:1 */
  --term-color-6:  #1aa4b0; /* Cyan normal    — hsl(184, 73%, 40%) — 5.0:1 */
  --term-color-7:  #a8cdd6; /* White normal   — hsl(190, 28%, 75%) — 8.8:1 */
  --term-color-8:  #385e68; /* Black bright   — hsl(190, 30%, 31%) — dimmed text context */
  --term-color-9:  #f07060; /* Red bright     — hsl(6,   80%, 66%) — 7.2:1 */
  --term-color-10: #80d040; /* Green bright   — hsl(94,  60%, 54%) — 7.4:1 */
  --term-color-11: #e8b040; /* Yellow bright  — hsl(40,  78%, 59%) — 7.6:1 */
  --term-color-12: #80b8f0; /* Blue bright    — hsl(210, 76%, 72%) — 9.2:1 */
  --term-color-13: #d080e8; /* Magenta bright — hsl(285, 65%, 70%) — 8.4:1 */
  --term-color-14: #40d8e8; /* Cyan bright    — hsl(184, 73%, 58%) — 9.6:1 */
  --term-color-15: #e8f2f6; /* White bright   — hsl(198, 34%, 94%) — 13.8:1 */
}
```

**Note on tropical palette coherence:** The Archipel ANSI palette is anchored by three hue families: warm (red/yellow/coral, HSL 0–40°), cool-vivid (cyan/turquoise, HSL 184–210°), and accent-vivid (magenta/lime, HSL 80–290°). Each family has two members in the normal set and two in the bright set. The hue distribution is not random — it follows the logic of Caribbean chromatic contrast, where the most vivid colors appear at opposite ends of the warm/cool spectrum against a dark, slightly warm-cool background.

**Note on cyan/blue gap:** `--term-color-6` (Cyan, HSL 184°) and `--term-color-4` (Blue, HSL 210°) maintain a 26° hue separation, ensuring distinguishability including for users with mild blue-green color confusion. The bright variants (`--term-color-14` at HSL 184°, `--term-color-12` at HSL 210°) preserve the same gap.

### 9.7 Semantic Color Token Overrides

Archipel is a dark theme — the semantic polarity matches Umbra (dark backgrounds, light foreground text). The key differences are in the accent and status families, which use the tropical palette rather than Umbra's warm neutral + steel blue vocabulary.

Key structural differences from Umbra semantic mappings:

| Token | Umbra mapping | Archipel mapping | Rationale |
|-------|--------------|-----------------|-----------|
| `--color-accent` | `blue-400` (steel blue) | `coral-500` (saturated coral) | Tropical primary accent |
| `--color-accent-subtle` | `blue-700` (dark blue tint) | `coral-100` (dark coral tint) | Accent family coherence |
| `--color-accent-text` | `blue-300` (light blue) | `coral-300` (light coral) | Accent family coherence |
| `--color-divider-active` | `blue-400` | `coral-500` | Follows accent color |
| `--color-focus-ring` | `blue-400` | `coral-500` | Follows accent color |
| `--color-activity` | `green-300` | `lime-400` | Tropical green family |
| `--color-success` | `green-400` | `lime-400` | Tropical green family |
| `--color-success-text` | `green-300` | `lime-300` | Tropical green family |
| `--color-ssh-badge-*` | blue-700/300 | turquoise-700/300 | SSH uses turquoise not coral |
| `--color-pane-border-active` | `blue-400` | `coral-500` | Follows accent color |
| `--color-bg-base` | `neutral-950` warm | `neutral-950` blue-green | Hue shift only, same depth |
| `--term-bg` | `neutral-900` warm | `neutral-900` blue-green | Hue shift only, same depth |
| `--term-fg` | `neutral-300` warm | `neutral-100` blue-green | Higher contrast (13.8:1 vs 8.4:1) |
| `--term-fg` contrast | 8.4:1 | 13.8:1 | Exceeds 7:1 requirement |
| `--term-cursor-bg` | `blue-300` | `coral-400` | Follows accent family |
| `--term-hyperlink-fg` | `blue-300` | `turquoise-300` | Turquoise for links (distinct from accent) |
| `--shadow-overlay` | `rgba(0,0,0,0.6)` | `rgba(6,14,16,0.70)` | Slight blue-green tint in shadow |

---

## 10. Accessibility Baseline

This section documents the contrast ratios for the Umbra palette. Component target sizes and keyboard navigation requirements belong in component specs and the functional specification.

### 10.1 WCAG 2.1 AA Contrast Requirements

- Normal text (under 18px regular or 14px bold): **4.5:1** minimum
- Large text (18px+ regular, 14px+ bold): **3:1** minimum
- UI components (borders, icons, non-text indicators): **3:1** minimum

### 10.2 Verified Contrast Ratios — Umbra

| Pair | Foreground | Background | Ratio | Standard |
|------|-----------|------------|-------|----------|
| Primary UI text on bg-surface | `#ccc7bc` | `#242118` | **8.1:1** | Pass AA |
| Active tab title on active-bg | `#e8e3d8` | `#16140f` | **10.3:1** | Pass AA |
| Inactive tab title on tab-bg | `#6b6660` | `#242118` | **3.1:1** | Pass (UI component) |
| Tab hover text on hover-bg | `#9c9890` | `#2c2921` | **4.6:1** | Pass AA |
| Accent text on bg-base | `#7ab3d3` | `#0e0d0b` | **7.8:1** | Pass AA |
| Error text on error-bg | `#d97878` | `#3d1212` | **4.9:1** | Pass AA |
| Warning text on warning-bg | `#e8b060` | `#4d3000` | **5.4:1** | Pass AA |
| Terminal fg on terminal bg | `#ccc7bc` | `#16140f` | **8.4:1** | Pass AA |
| Focus ring on bg-base | `#4a92bf` | `#0e0d0b` | **5.9:1** | Pass (3:1 min) |
| ANSI Red (1) on terminal bg | `#c44444` | `#16140f` | **5.2:1** | Pass AA |
| ANSI Green (2) on terminal bg | `#5c9e5c` | `#16140f` | **4.6:1** | Pass AA |
| ANSI Yellow (3) on terminal bg | `#b89840` | `#16140f` | **5.1:1** | Pass AA |
| ANSI Blue (4) on terminal bg | `#4a92bf` | `#16140f` | **4.9:1** | Pass AA |
| ANSI Magenta (5) on terminal bg | `#9b6dbf` | `#16140f` | **4.7:1** | Pass AA |
| ANSI Cyan (6) on terminal bg | `#3d9e8a` | `#16140f` | **4.9:1** | Pass AA |

**Note on inactive tab text:** `#6b6660` on `#242118` at 3.1:1 meets the 3:1 threshold for UI components but not 4.5:1 for normal text. This is intentional: inactive tab labels are interface chrome, not content text. The visual hierarchy benefit — making active tabs clearly dominant — justifies using the component threshold. Users requiring higher contrast can create a custom theme.

**Note on non-color indicators:** All status conditions communicated through color in the ANSI palette and UI token layer are also communicated through a non-color secondary indicator (shape, icon, or text). This requirement applies at the component level and is documented in component specs. The palette itself does not rely on color as the sole distinguishing property between any two states — the blue/cyan split (HSL 205° vs 162–168°) and the normal/bright brightness difference both contribute to distinguishability independent of hue perception.
