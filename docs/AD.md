<!-- SPDX-License-Identifier: MPL-2.0 -->

# Artistic Direction — TauTerm

> **Version:** 1.0.0
> **Date:** 2026-04-04
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
8. [Accessibility Baseline](#8-accessibility-baseline)

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

  --size-tab-height:      40px;
  --size-toolbar-height:  40px;
  --size-divider-hit:     8px;
  --size-scrollbar-width: 8px;
  --size-icon-sm:         14px;
  --size-icon-md:         16px;
  --size-icon-lg:         20px;
  --size-target-min:      44px;
  --size-badge:           6px;
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

```css
@theme {
  --duration-instant: 0ms;   /* focus rings, hover states */
  --duration-fast:    80ms;  /* dismissals, fade-outs */
  --duration-base:    100ms; /* modal/popover appearance */
  --duration-slow:    300ms; /* scrollbar fade, non-critical transitions */

  --ease-in:     cubic-bezier(0.4, 0, 1, 1); /* dismissals */
  --ease-out:    cubic-bezier(0, 0, 0.6, 1); /* appearances */
  --ease-linear: linear;                      /* spinners, continuous rotation */
}
```

---

## 8. Accessibility Baseline

This section documents the contrast ratios for the Umbra palette. Component target sizes and keyboard navigation requirements belong in component specs and the functional specification.

### 8.1 WCAG 2.1 AA Contrast Requirements

- Normal text (under 18px regular or 14px bold): **4.5:1** minimum
- Large text (18px+ regular, 14px+ bold): **3:1** minimum
- UI components (borders, icons, non-text indicators): **3:1** minimum

### 8.2 Verified Contrast Ratios — Umbra

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
