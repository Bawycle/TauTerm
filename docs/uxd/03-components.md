<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Component Specifications

> Part of the [UX/UI Design](README.md).

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
- **Border radius:** `--radius-sm` (4px) on top-left and top-right only; bottom corners are `--radius-none`.
- **ARIA role:** `tab`. `aria-selected="true"` for active tab.
- **Title font:** `--font-size-ui-base` (13px), `--font-ui`.
- **Title truncation:** Ellipsis when text exceeds available width.
- **Gap between elements:** `--space-1` (4px).

**States:**

| State | Background | Text Color | Text Weight | Border |
|-------|-----------|------------|-------------|--------|
| Active | `--color-tab-active-bg` (`#16140f`) | `--color-tab-active-fg` (`#e8e3d8`) | `--font-weight-semibold` (600) | `border-bottom: 2px solid var(--color-accent)` (`#4a92bf`) |
| Inactive | `--color-tab-inactive-bg` (transparent) | `--color-tab-inactive-fg` (`#9c9890`) | `--font-weight-normal` (400) | none |
| Hover (inactive) | `--color-tab-hover-bg` (`#2c2921`) | `--color-tab-hover-fg` (`#9c9890`) | `--font-weight-normal` (400) | none |
| Focus (keyboard) | Same as inactive + focus ring | Same as inactive | `--font-weight-normal` (400) | 2px solid `--color-focus-ring`, offset 3px inset |

**Transition:** `background-color, color, border-color var(--duration-fast) var(--ease-out)` on state changes.

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
- **Focus ring:** 2px solid `--color-focus-ring`, offset 3px.
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
- **Focus ring:** 2px solid `--color-focus-ring`, offset 3px.
- **ARIA label:** "New tab".
- **Tooltip:** "New Tab (Ctrl+Shift+T)" — shown after `--duration-slow` (300ms) hover delay.
- **Overflow behaviour (FS-TAB-009):** If the tab bar is in horizontal scroll mode when the new tab is created, the tab bar scrolls to bring the new tab into view (see [§12.2](05-accessibility.md#122-tab-bar-at-narrow-widths)).

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
  *Radius rationale:* `--radius-sm` is used here because this is a **label-type badge** (an icon inside a shaped container), not a punctual indicator like an activity dot. The rounded rectangle container shape makes it identifiable as a distinct widget, not a free-floating icon. This is the same rationale as buttons and text inputs — a container that houses content uses `--radius-sm`, while point indicators (dots, filled circles) use `--radius-full`.
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
- **Focus ring:** 2px solid `--color-focus-ring`, offset 3px.
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
| Block (blinking) | Same as steady, toggling visibility | On/off at configurable rate (default 533ms on / 266ms off per FS-VT-032) |
| Underline (steady) | `--size-cursor-underline-height` horizontal line at cell bottom | Color: `--term-cursor-bg` (`#7ab3d3`) |
| Underline (blinking) | Same as steady, toggling visibility | On/off at configurable rate |
| Bar (steady) | `--size-cursor-bar-width` vertical line at cell left edge | Color: `--term-cursor-bg` (`#7ab3d3`) |
| Bar (blinking) | Same as steady, toggling visibility | On/off at configurable rate |

**Unfocused state (FS-VT-034):** When the pane loses focus, the cursor renders as a hollow outline rectangle of the current shape in `--term-cursor-unfocused` (`#7ab3d3`). Never filled, never invisible.

##### 7.3.1.1 Cursor Pixel Composition

**Block cursor:**
- The block cursor is an overlay layer painted on top of the cell background, before the character glyph.
- Rendering order (back to front): cell background → cursor fill (`--term-cursor-bg`) → character glyph redrawn in `--term-cursor-fg`.
- The character glyph is always redrawn on top of the cursor fill so that it remains legible. The cursor fill color and the character-under-cursor color form a guaranteed-contrast pair: `--term-cursor-bg` (`#7ab3d3`) and `--term-cursor-fg` (`#16140f`) achieve 8.7:1 contrast.
- Cursor fill opacity: 1.0 — never partially transparent.

**Underline cursor:**
- A `--size-cursor-underline-height` (2px) horizontal bar aligned to the cell's baseline region — positioned at `cell_height - 2px` from the top of the cell (2px from the bottom edge).
- The character glyph in the cell is not recolored; it renders with its normal fg/bg.
- Color: `--term-cursor-bg` (`#7ab3d3`).

**Bar cursor:**
- A `--size-cursor-bar-width` (2px) vertical bar at the left edge of the cell.
- The character glyph in the cell is not recolored.
- Color: `--term-cursor-bg` (`#7ab3d3`).

**Blinking behavior:**
- Blink is a hard toggle: the cursor is fully visible (`opacity: 1`) for `--term-blink-on-duration` (533ms), then fully invisible (`opacity: 0`) for `--term-blink-off-duration` (266ms). No fade transition. The abrupt toggle matches terminal convention and maximizes position legibility at peripheral vision.
- The blink cycle resets (cursor immediately becomes visible) when: the pane gains focus, the cursor moves, or the user types. This ensures the cursor is always visible immediately after an action.
- When blinking is suspended (pane loses focus), the cursor transitions from its blinking state to the unfocused hollow outline without completing the current blink cycle.

**Unfocused cursor:**
- Rendered as a hollow outline rectangle matching the cursor style's outer boundary.
- Stroke width: `--size-cursor-outline-width` (1px), color `--term-cursor-unfocused` (`#7ab3d3`).
- For block cursor: full cell outline. For underline cursor: outline of the 2px bar's outer rectangle. For bar cursor: outline of the 2px bar's outer rectangle.
- No fill. Blink is suspended.

**Cursor during scrollback (scroll_offset > 0):**
- The cursor is hidden when the viewport is scrolled above the cursor's row (i.e., the cursor row is not visible in the current viewport). It does not render off-screen.
- When the viewport scrolls back to the cursor's row, the cursor reappears immediately.

#### 7.3.2 Selection Highlight

- **Focused pane:** Selected cells use `--term-selection-bg` (`#2e6f9c`) as background. Foreground is `--term-selection-fg` (`inherit` — preserves original text color).
- **Unfocused pane:** Selected cells use `--term-selection-bg-inactive` (`#1a3a52`).
- **Selection operates on cell boundaries** (FS-CLIP-001), not pixel boundaries.

##### 7.3.2.1 Selection Layering and Edge Cases

**Layer order (back to front):** cell background → selection background → text glyph (in its original color) → search highlight (when applicable) → cursor.

The selection layer sits above the cell background but below the text glyph. This means the text glyph always paints over the selection color, ensuring readability without forced color inversion.

**Selection background opacity:** 1.0 — never partially transparent, to prevent selection color contaminating underlying content visually.

**Text color inside selection:** `--term-selection-fg` is `inherit` — the text glyph keeps its original foreground color. Forced inversion (swapping fg/bg inside selection) is not applied. Rationale: with `--term-selection-bg` at `#2e6f9c` (4.9:1 on `--term-bg`), the most common text colors (ANSI White at 8.4:1, neutral-300 at 8.4:1) achieve ≥ 3.5:1 on the selection background — sufficient for this transient overlay. Edge case: if a specific fg color falls below 3:1 contrast on the selection background, the text remains technically distinguishable against the non-selected adjacent cells; forced inversion for individual cells is not implemented (adds implementation complexity, creates visual noise on partial-word selections).

**Cursor cell inside selection:** The cursor is drawn on top of the selection layer. For a block cursor: the cursor fill (`--term-cursor-bg`) covers the selection background on that one cell; the character is redrawn in `--term-cursor-fg`. The selection is visually "punched through" by the cursor on its cell, which is the standard behavior in kitty, WezTerm, and Alacritty.

**Selection vs. search highlights:** Search match highlights and selection highlights can overlap. Priority: **selection takes precedence** over search highlights on any cell covered by the active selection. Rationale: the user's active selection is the current intent; obscuring it with a search match background would be confusing.

**Legibility failure case (fg ≈ selection bg):** If a cell's foreground color is within a perceptual distance that makes it effectively invisible on `--term-selection-bg`, no automatic override is applied. This case is extremely rare with the Umbra ANSI palette and is documented as a known limitation. A future theme-validation warning (in the theme editor, §7.9) may flag low-contrast fg-on-selection-bg pairs.

#### 7.3.3 Scrollbar

- **Width:** `--size-scrollbar-width` (8px).
- **Position:** Right edge of the pane, overlaying terminal content (no layout displacement). The terminal grid always occupies 100% of the pane width — the scrollbar floats above it via `position: absolute` (or equivalent overlay positioning) with `z-index: var(--z-scrollbar)`. There is no layout reflow when the scrollbar appears or disappears.
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

#### 7.3.4 Text Attribute Rendering (FS-VT-024)

SGR (Select Graphic Rendition) text attributes are rendered as follows. Each attribute is independent and composable unless noted.

##### Bold (SGR 1) — FS-VT-026

Rendered as `font-weight: bold` (700). The font stack (`--font-terminal`) includes fonts that have genuine bold faces (JetBrains Mono Bold, Fira Code Bold, Cascadia Code Bold). Synthetic bold (browser-generated stroke thickening) is acceptable as fallback when no bold face is available — the visual result is slightly degraded but not incorrect.

**Bold + ANSI color interaction (color promotion):** When SGR 1 is active and the foreground color is an ANSI index from the normal range (indices 1–7), the color is promoted to its bright counterpart (indices 9–15). Index 0 (Black) is not promoted — bold black on a dark background has no useful rendering. This promotion matches the behavior of xterm, kitty, and WezTerm with `bold_is_bright = true`. The effect: bold text appears both heavier and brighter, reinforcing the emphasis signal through two channels (weight and luminance).

Color promotion applies only to ANSI indexed colors 1–7. It does not apply to truecolor RGB values or to 256-color palette entries outside indices 1–7.

##### Dim / Faint (SGR 2) — FS-VT-027

Rendered by multiplying the cell's foreground color alpha by `--term-dim-opacity` (0.5). The background color is unaffected.

This approach is preferred over darkening the color value because it works correctly with truecolor and does not require computing a modified color for every possible fg value. At 50% opacity, `--term-fg` (`#ccc7bc` at 8.4:1 on `--term-bg`) yields an effective contrast of approximately 4.2:1 — just below the WCAG 4.5:1 threshold for body text. This is an accepted trade-off: dim text is semantically "less important" content; the slight contrast reduction reinforces the semantic. The value 0.5 is chosen to remain above 3:1 (the WCAG threshold for non-text elements and large text) for the typical terminal fg color.

If SGR 1 (Bold) and SGR 2 (Dim) are both active simultaneously, the bold face is applied and the dim opacity is applied on top. The two attributes do not cancel each other.

##### Italic (SGR 3) — FS-VT-028

Rendered as `font-style: italic`. Fonts in `--font-terminal` that have genuine italic faces (JetBrains Mono Italic, Fira Code — note: Fira Code has no italic; the browser will synthesize it) will use them. Browser-synthesized oblique (slanting) is acceptable as fallback. No special handling for fonts without italic.

##### Blink — Normal (SGR 5) and Rapid (SGR 6) — FS-VT-029

Both SGR 5 (Blink) and SGR 6 (Rapid Blink) are rendered identically: the text content of the cell toggles between `opacity: 1` and `opacity: 0` using CSS animation. The cycle: visible for `--term-blink-on-duration` (533ms), invisible for `--term-blink-off-duration` (266ms). The asymmetric 2:1 on/off ratio ensures the text is readable for the majority of each cycle.

The blink animation uses a CSS `@keyframes` with `animation-timing-function: step-start` (immediate toggle, no easing). Only the text glyph blinks — the cell background does not. This makes it possible to read the text by waiting for the visible phase even without memorizing the content.

The background color of a blinking cell is constant (the cell's normal background), providing a stable visual anchor at the cell position. This prevents the layout from appearing to "jump" as glyphs disappear.

Blink animation is paused globally when the window loses focus (to avoid battery drain on background windows).

**Rapid Blink (SGR 6):** Rendered identically to SGR 5. Distinguishing blink rates is not implemented — the visual difference is imperceptible in practice and adds implementation complexity for zero user benefit.

##### Hidden / Invisible (SGR 8) — FS-VT-036

The text glyph is not rendered (effectively `color: transparent` — the character occupies its cell space but is not visible). The cell background renders normally. Hidden text is not revealed visually under selection: `--term-selection-fg: inherit` means the text color (transparent) remains transparent even when the selection background is applied. This matches standard terminal behavior (FS-VT-036). To copy hidden text, the user selects the region; the underlying bytes are captured regardless of visibility.

##### Strikethrough (SGR 9) — FS-VT-037

A horizontal line drawn through the cell at `--term-strikethrough-position` (50% of `cell_height` from the top), with thickness `--term-strikethrough-thickness` (1px).

Color: uses the current foreground color of the cell (same as the text). If SGR 58 (underline color) has been set, it is **not** applied to the strikethrough — SGR 58 controls underline color only, not strikethrough. The strikethrough line is a simple overlay on top of the text glyph.

Position rationale: 50% (vertical midpoint) is the typographic standard for strikethrough in monospace rendering. At 14px font size, this places the line at approximately 8–9px from top, crossing the x-height of most glyphs. This matches kitty and WezTerm behavior.

##### Reverse Video / Inverse (SGR 7) — FS-VT-038

The cell's foreground and background colors are swapped. This applies to all color types:
- Terminal default colors: `--term-fg` and `--term-bg` are swapped.
- ANSI indexed colors: the indexed value is used as-is for the swapped position.
- Truecolor RGB: the RGB values are used as-is for the swapped position.

No additional adjustment is applied after the swap. If the result of a swap produces a low-contrast pairing (e.g., a truecolor fg that is dark on a dark bg after swap), no automatic correction is made — the application that set the colors is responsible for choosing values that are legible when inverted.

If SGR 7 (Reverse) is combined with SGR 1 (Bold), color promotion (see Bold above) is applied after the swap — i.e., to the color that ends up in the foreground position after reversal.

#### 7.3.5 Extended Underline Styles (FS-VT-025)

SGR 4:2, 4:3, 4:4, and 4:5 define extended underline styles. These are a SHOULD-level requirement (FS-VT-025). The rendering approach uses CSS `text-decoration` or equivalent canvas-drawn lines.

**Underline color:** When SGR 58 has been set for a cell, that color is used for all underline variants. When SGR 58 has not been set, `--term-underline-color-default` (`inherit`) is used — the underline adopts the cell's current foreground color.

| SGR variant | Style name | Rendering |
|-------------|------------|-----------|
| SGR 4:0 | No underline | Line removed |
| SGR 4:1 | Single underline (default) | Single solid line, 1px, positioned at bottom of descender zone (`cell_height - 1px`) |
| SGR 4:2 | Double underline | Two solid lines, each 1px, with 1px gap between them, bottom-aligned in the same position as single underline (bottom line at `cell_height - 1px`, top line at `cell_height - 3px`) |
| SGR 4:3 | Curly/wavy underline | CSS `text-decoration-style: wavy` if the renderer uses HTML/CSS; canvas: sine wave, amplitude 1.5px, period `2 × cell_width`, positioned at `cell_height - 2px` baseline |
| SGR 4:4 | Dotted underline | Dots of 1px diameter with 2px spacing, aligned to the single underline position |
| SGR 4:5 | Dashed underline | Dashes of `4px` length with `2px` gap, aligned to the single underline position |

**Token usage:** All underline variants use `--term-underline-color-default` when no SGR 58 color is set. No per-variant color tokens are defined — the color source is always either the SGR 58 value or the cell foreground.

### 7.4 Search Overlay

Triggered by Ctrl+Shift+F or context menu "Search" (FS-SEARCH-007).

#### 7.4.1 Layout

- **Position:** Top-right corner of the active pane, with `--space-2` (8px) offset from top and right edges.
- **Width:** `min(var(--size-search-overlay-width), calc(100% - 2 * var(--space-md)))` — the overlay shrinks to fit the pane with `--space-md` (`--space-4`, 16px) margin on each side when the pane is narrower than 360px.
- **Height:** Auto (content-driven, single row of controls).
- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border-overlay` (`#4a4640`).
- **Border radius:** `--radius-md` (8px).
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

#### 7.5.2 Connection-in-Progress Overlay (in-pane)

When an SSH pane is in the `connecting` or `authenticating` state (FS-SSH-010), a centered overlay is displayed within the pane. At this stage the pane has no terminal content yet — nothing to preserve behind a banner — so a full-pane centered overlay is appropriate.

**Component:** `SshConnectingOverlay`

**Position:** `position: absolute; inset: 0` — covers the entire pane.

**Layout:** `display: flex; flex-direction: column; align-items: center; justify-content: center; gap: var(--space-3)`.

**Background:** None — the overlay is transparent, showing the pane's natural `--term-bg`.

**Pointer events:** `none` — the overlay is informational only, not interactive.

**Icon:** Lucide `Network`, size `--size-icon-lg` (20px), color `--color-ssh-connecting-fg` (`#d48a20`).
- `connecting` state: continuous rotation animation, 1 revolution per second, `animation-timing-function: linear`.
- `authenticating` state: opacity 0.5→1→0.5, period `--duration-slow` (300ms) per cycle, `animation-timing-function: linear`, infinite repeat.
- `prefers-reduced-motion: reduce`: animation disabled on both states — static icon.

**Label text:** `--font-size-ui-sm` (12px), `--color-text-muted` (`#6b6660`), `font-family: var(--font-ui)`.
- `connecting` state: i18n key `ssh_overlay_connecting` ("Connecting…" / "Connexion en cours…").
- `authenticating` state: i18n key `ssh_overlay_authenticating` ("Authenticating…" / "Authentification en cours…").

**Dismissal:** When `sshState` transitions to `connected`, the overlay disappears instantly (`--duration-instant`, 0ms). No exit animation — the user is waiting for the terminal, not for chrome to finish animating.

**ARIA:** `role="status"`, `aria-live="polite"`.

#### 7.5.3 Disconnection Overlay (in-pane)

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
- **Border:** 1px solid `--color-border-overlay` (`#4a4640`).
- **Border radius:** `--radius-md` (8px).
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
- **Section navigation:** Left column, width `--size-preferences-nav-width` (180px). Vertical list of section labels. Active section: `--color-accent-text` (`#7ab3d3`), left border 2px solid `--color-accent`. Inactive: `--color-text-secondary`. Hover: `--color-hover-bg` background. Transition: `background-color, color, border-color var(--duration-fast) var(--ease-out)`.
- **Section content:** Right area, scrollable independently if content exceeds height.
- **Section headings within content** (e.g., "KEYBOARD", "APPEARANCE", "CONNECTIONS", "THEMES"): Caption level — `--font-size-ui-xs` (11px), `--font-weight-semibold` (600), `text-transform: uppercase`, `letter-spacing: var(--letter-spacing-label)` (0.09em), `--color-text-heading`.
- **Section separator:** `--space-6` (24px) between sections within content area.
- **Focus trap:** Keyboard focus is trapped within the panel while open. Tab cycles through section nav, then through form controls in the active section. Escape closes the panel.
- **Nested interactive overlays:** Any dropdown, select, or combobox rendered inside the Preferences panel must use a portal to `<body>` (see §7.16 — Dropdown / Select). The panel's `overflow: hidden` and independent scroll region would otherwise clip the list. This applies to all dropdowns in the Appearance section (theme quick-select, language), Terminal Behavior section (cursor shape, bell type), and any future select controls added to the panel.

#### 7.6.3 Preference Sections (FS-PREF-004)

**Helper text convention for preference controls:**

Every preference control (text input, number input, dropdown, toggle) in the Preferences panel SHOULD carry a `helper` prop with a concise, user-facing description of what the field controls and any constraints the user needs to know (e.g., "Applies to new panes only"). Helper text is defined in the i18n catalogue and follows the key naming pattern `preferences_{section}_{field}_hint` (e.g., `preferences_terminal_scrollback_lines_hint`, `preferences_appearance_font_family_hint`). This ensures all descriptions are translatable and centrally maintainable. The `helper` prop on `TextInput` (§7.15) and `Dropdown` (§7.16) renders the text below the control and wires `aria-describedby` on the input/trigger element to the helper text container.

**Keyboard section:**
- List of configurable application shortcuts (FS-KBD-002).
- Each row: action label (Body level) + keyboard shortcut recorder (§7.17).
- An instruction paragraph above the shortcut list (body text, `--color-text-secondary`) describes how to record a new shortcut (click the field, press the desired key combination).
- Section heading: "KEYBOARD" in Caption level.

**Appearance section:**
- Theme quick-select dropdown (§7.16) — this is a shortcut for switching the active theme. The full theme management surface (create, edit, duplicate, delete) is in the Themes section below. Carry a `helper` describing the theme's scope (applies immediately, affects all terminal windows).
- Font family input (text input with monospace preview). Carry a `helper` noting that any monospace font installed on the system can be used.
- Font size input (number input, range 8-32). Carry a `helper` noting the valid range.
- Opacity input. Carry a `helper` clarifying that it controls terminal background transparency only — text and UI chrome are not affected.
- **Language subsection** — see below. The language dropdown carries a `helper` noting that the change applies immediately without restart.
- Section heading: "APPEARANCE" in Caption level.

> **Design note — line height:** Line height is a per-theme property (`UserTheme.line_height`, range 1.0–2.0), configured in the Theme Editor (§7.20). It is not a global Appearance preference. Rationale: line height is intrinsically tied to the theme's font choice — a condensed font may require a different line height than a tall one. A global line-height preference would create an override-layering conflict with the per-theme value. The i18n key `preferences_appearance_line_height` belongs in the Theme Editor context.

**Language subsection (within Appearance):**
- A dropdown (§7.16) listing available locales. v1 options: "English" and "Français" (each option displays the language name in its own language).
- Selecting a locale applies it immediately to all visible UI strings without any page reload or application restart (FS-I18N-004). When the locale changes, all text elements transition smoothly using `opacity` at `--duration-fast` (150ms) — a brief fade that confirms the change happened intentionally without being distracting. This transition applies to all elements bound to the i18n catalogue; terminal content (which is not locale-resolved) is unaffected.
- The selected locale is persisted to `preferences.json` and restored on next launch (FS-I18N-005).
- If `preferences.json` contains an unknown locale code on launch, the application silently falls back to English (FS-I18N-006); no error dialog is shown.
- The dropdown uses standard keyboard navigation (arrow keys to cycle options, Enter to confirm, Escape to cancel).
- Subsection heading: "LANGUAGE" in Caption level, rendered as a minor heading within the Appearance section (visually subordinate to the "APPEARANCE" section heading).
- **Placement rationale:** Language is a display preference — it controls how the UI appears to the user, analogous to font and theme. Placing it within Appearance ensures it is visible immediately when a user opens Preferences without scrolling past multiple sections. This directly addresses discoverability for Sam (UR §2.3 — occasional user, not expected to know the settings structure).

**Terminal Behavior section:**
- Cursor shape selector (dropdown: Block, Underline, Bar). Carry a `helper` noting that this sets the default shape; terminal applications can override it via escape sequences, and it is restored after a terminal reset.
- Cursor blink rate (number input, ms — visible phase duration, default 533). The invisible phase is computed as half the visible phase (2:1 on/off ratio per FS-VT-032). Carry a `helper` noting the unit (milliseconds, visible phase duration).
- Scrollback buffer size (number input, lines, default 10000) with real-time memory estimate below the field (FS-SB-002). Estimate format: "~{N} MB per pane" in `--font-size-ui-sm`, `--color-text-secondary`. Carry a `helper` explicitly stating that the setting **applies to new panes only** — existing pane buffers are not resized (this is an architectural constraint of the ScreenBuffer design; see FS-PREF behavioral constraints table).
- Bell notification type (dropdown: Visual, Audible, Disabled). Carry a `helper` describing each option briefly.
- Word delimiter set (text input, monospace font). Carry a `helper` explaining that these characters act as word boundaries during double-click selection.
- Section heading: "TERMINAL BEHAVIOR" in Caption level.

**Connections section:**
- Displays an **inline view** of the connection list embedded directly inside the Preferences panel. This is the same connection list content as §7.7.3, rendered inline within the Preferences section content area rather than in a separate slide-in panel. All connection CRUD operations (create, edit, duplicate, delete) are accessible from this inline view.
- The standalone Connection Manager (§7.7, right-side slide-in) remains separately accessible from the tab bar context menu or a dedicated keyboard shortcut. Both views operate on the same underlying connection data.
- Known-hosts import action: "Import from ~/.ssh/known_hosts" button (secondary variant).
- OSC 52 global default toggle.
- Section heading: "CONNECTIONS" in Caption level.

**Themes section:**
- A description paragraph at the top of the section (body text, `--color-text-secondary`) briefly explains what themes control (terminal colors, background, cursor, ANSI palette) and how to create or customize them. This orients users who land here without prior context.
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
- **Left border:** 1px solid `--color-border-overlay` (`#4a4640`).
- **Shadow:** `--shadow-overlay`.
- **Z-index:** `--z-overlay` (40).
- **Internal padding:** `--space-4` (16px) on all four sides for all content regions (header, list, form). The header has its own `padding: --space-4` and a bottom border; the content area below it also uses `padding: 0 --space-4` horizontally so items are never flush against the panel edge.

#### 7.7.2 Header

- **Title:** "Connections" at Heading level.
- **Close button:** Lucide `X`, top-right.
- **"New Connection" button:** Rendered below the header, inside the scrollable content area. Primary button variant, full width. Lucide `Plus` icon.

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

**Transition:** `background-color var(--duration-fast) var(--ease-out)` on state changes.

**Actions (visible on hover, always accessible via keyboard/context menu):**
- **Open in new tab:** Lucide `ExternalLink`. Tooltip: "Open in new tab".
- **Open in new pane:** Lucide `SplitSquareVertical`. Tooltip: "Open in pane".
- **Edit:** Lucide `Pencil`. Tooltip: "Edit connection".
- **Duplicate:** Lucide `Copy`. Tooltip: "Duplicate".
- **Delete:** Lucide `Trash2`, color `--color-error`. Tooltip: "Delete". Triggers confirmation dialog.

Each action button: `--size-target-min` (44px) hit area, `--size-icon-sm` (14px) icon, `--color-icon-default` resting, `--color-icon-active` on hover.

#### 7.7.4 Connection Edit Form

Displayed inline within the connection manager when creating or editing a connection (replaces the list temporarily, or as a slide-in sub-panel).

**Layout in standalone mode:**
- The form replaces the list view entirely inside the slide-in panel.
- The form container takes `flex: 1` and `overflow-y: auto` so it scrolls independently when its content exceeds the available panel height. This ensures the action buttons at the bottom are always reachable without resizing the window.
- Padding: `--space-4` (16px) on all sides (matching the panel's internal padding spec in §7.7.1).
- The form title ("New Connection" / "Edit Connection") is rendered at the top of the scrollable area, not as a fixed header.

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

**Action buttons:** "Save" (primary), "Cancel" (ghost). Placed at the bottom of the scrollable form with `--space-6` (24px) top margin separating them from the last field.

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
- **Border:** 1px solid `--color-border-overlay` (`#4a4640`).
- **Border radius:** `--radius-md` (8px).
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

**Transition:** `background-color, color var(--duration-fast) var(--ease-out)` on state changes.

**Separator:** 1px solid `--color-border`, `--space-1` (4px) vertical margin, full width minus `--space-3` (12px) horizontal margin.

### 7.9 Dialog / Modal

Used for confirmations (FS-PTY-008), SSH host key verification (FS-SSH-011), SSH credential prompts (FS-SSH-015, FS-SSH-017, FS-SSH-018), and destructive action confirmations.

#### 7.9.1 Backdrop

- **Color:** `--color-bg-overlay` (`#16140f`) at 60% opacity.
- **Z-index:** `--z-modal-backdrop` (49).
- **Behavior:** Clicking the backdrop does NOT close the dialog (confirmation dialogs require explicit action).

#### 7.9.2 Dialog Panel

- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border-overlay` (`#4a4640`).
- **Border radius:** `--radius-md` (8px).
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

For closing tabs/panes with a non-shell foreground process active (FS-PTY-008). This dialog is not shown when only an idle shell is at the prompt — the condition is defined in FS-PTY-008.
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

#### 7.9.5 SSH Credential Prompt Dialog (FS-SSH-015, FS-SSH-017, FS-SSH-018, FS-CRED-007)

Shown when SSH authentication requires a password or keyboard-interactive response and no credential is available from the keychain (or when the keychain is unavailable per FS-CRED-005).

**Trigger contexts:**
- Auth method is "password" and no keychain entry exists for this connection (FS-CRED-007).
- Auth method is "password" and keychain is unavailable (FS-CRED-005).
- Server requests keyboard-interactive authentication and a challenge prompt is provided.
- Re-prompt after an authentication failure (FS-SSH-017).

**Width:** 420px (small). **ARIA role:** `dialog`.

**Anatomy:**
```
+-[LockKeyhole icon] [Title]--------------------+
|  [Intro text]                                 |
|                                               |
|  Username  [readonly field]                   |
|  [prompt label / "Password"]  [input field]   |
|                                               |
|  [x] Save in keychain         [Cancel] [OK]   |
+-----------------------------------------------+
```

**Header:**
- **Icon:** Lucide `LockKeyhole`, `--size-icon-md` (16px), `--color-icon-default`, inline-left of the title.
- **Title:** "Authenticate" (default) — `--font-size-ui-lg` (16px), `--font-weight-semibold`, `--color-text-primary`.

**Intro text:**
- Default (no prior failure): "{username}@{host}" — `--font-size-ui-sm` (12px), `--color-text-secondary`.
- On retry (FS-SSH-017): "Authentication failed. Please try again." — same size, `--color-error`. Prefixed with Lucide `AlertCircle` icon (`--size-icon-sm`, 14px, `--color-error`).
- Keychain unavailable (FS-CRED-005): appended notice below the intro text — "Credential storage is unavailable. Your password will not be saved." — `--font-size-ui-xs` (11px), `--color-text-secondary`. The "Save in keychain" toggle is hidden in this state.
- Maximum retries reached before dialog closes: not shown as a dialog state — the connection aborts and the session transitions to Disconnected.

**Username field:**
- Read-only text input; shows the username from the saved connection.
- Styling: same as §7.15 text input but with `background-color: var(--color-bg-surface)` (visually distinct from editable fields) and `cursor: default`.
- Label: "Username" — `--font-size-ui-xs` (11px), `--font-weight-semibold`, `--color-text-secondary`.

**Password / challenge field:**
- Label: server-provided keyboard-interactive prompt text when present; otherwise "Password" — `--font-size-ui-xs` (11px), `--font-weight-semibold`, `--color-text-secondary`.
- Input type: `password` (masked). No show/hide toggle — omitted intentionally to reduce interaction complexity in an authentication context.
- Autofocus on dialog open.
- Height: `--size-target-min` (44px). Full width.
- `Enter` key submits the form.
- Styling: §7.15 text input.

**"Save in keychain" toggle:**
- A checkbox-style toggle (§7.16), label "Save password in keychain" — `--font-size-ui-sm` (13px), `--color-text-primary`.
- **Default state: unchecked** (FS-SSH-018). The user must opt in explicitly.
- Hidden when keychain is unavailable (FS-CRED-005).
- Positioned below the password field, above the action buttons. Full-width row with the checkbox left-aligned and the label immediately to its right.
- Hit area: minimum `--size-target-min` (44px) height.

**Action buttons:**
- **"Cancel"** — ghost button variant. Cancels the connection attempt (FS-SSH-016). Focus is NOT placed here by default; the password field is focused.
- **"Connect"** (i18n key `action_connect`) — primary button variant. Disabled when the password field is empty. Clicking submits credentials. "Connect" is used instead of the generic "OK" to communicate the action explicitly: the user is initiating a connection, not confirming an abstract dialog.
- Layout: right-aligned row per §7.9.2 pattern. `--space-2` (8px) gap between buttons.

**Focus management:**
- On open: focus is placed on the password input field, not on "OK". This allows immediate typing.
- Focus trap active within the dialog (Bits UI `FocusScope`).
- Tab order: username field (not focusable, skipped) → password field → "Save in keychain" checkbox → "Cancel" button → "OK" button.
- On close (submit or cancel): focus returns to the element that triggered the connection attempt (e.g., the connection list item or the reconnect button).
- `Escape` key cancels (same as clicking "Cancel").

**Backdrop:** `--color-bg-overlay` at 60% opacity. Clicking the backdrop does NOT close the dialog — the user must make an explicit choice (FS-SSH-016).

**Retry counter:** Not displayed in the UI. The backend tracks retry count per FS-SSH-017; the frontend only receives re-prompt events with a `failed: true` flag. Showing a counter would create unnecessary anxiety and is not actionable information.

### 7.10 Tooltip

- **Background:** `--color-bg-raised` (`#2c2921`).
- **Border:** 1px solid `--color-border-overlay` (`#4a4640`).
- **Border radius:** `--radius-md` (8px).
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

All button variants share: `--radius-sm` (4px), `--font-size-ui-base` (13px), `--font-weight-medium` (500), height `--size-target-min` (44px), horizontal padding `--space-4` (16px). Icons (when present) are `--size-icon-sm` (14px) with `--space-1` (4px) gap to label text. All variants apply `transition: background-color, color, border-color var(--duration-fast) var(--ease-out)` for smooth state changes.

#### Primary Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | `--color-accent` (`#4a92bf`) | `--color-text-inverted` (`#0e0d0b`) | none |
| Hover | `--color-blue-500` (`#2e6f9c`) | `--color-text-inverted` | none |
| Active | `--color-blue-600` (`#1e4d6e`) | `--color-text-inverted` | none |
| Focus | `--color-accent` | `--color-text-inverted` | 2px solid `--color-focus-ring`, offset 3px `--color-focus-ring-offset` |
| Disabled | `--color-neutral-700` (`#35312a`) | `--color-text-tertiary` (`#6b6660`) | none |

#### Secondary Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | transparent | `--color-accent-text` (`#7ab3d3`) | 1px solid `--color-accent` |
| Hover | `--color-accent-subtle` (`#1a3a52`) | `--color-accent-text` | 1px solid `--color-accent` |
| Active | `--color-blue-700` (`#1a3a52`) | `--color-accent-text` | 1px solid `--color-accent` |
| Focus | transparent | `--color-accent-text` | 2px solid `--color-focus-ring`, offset 3px |
| Disabled | transparent | `--color-text-tertiary` | 1px solid `--color-neutral-700` |

#### Ghost Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | transparent | `--color-text-primary` (`#ccc7bc`) | none |
| Hover | `--color-hover-bg` (`#2c2921`) | `--color-text-primary` | none |
| Active | `--color-active-bg` (`#35312a`) | `--color-text-primary` | none |
| Focus | transparent | `--color-text-primary` | 2px solid `--color-focus-ring`, offset 3px |
| Disabled | transparent | `--color-text-tertiary` | none |

#### Destructive Button

| State | Background | Text | Border |
|-------|-----------|------|--------|
| Default | `--color-error` (`#c44444`) | `--color-neutral-100` (`#f5f2ea`) | none |
| Hover | `--color-red-500` (`#9c2c2c`) | `--color-neutral-100` | none |
| Active | `--color-red-700` (`#3d1212`) | `--color-neutral-100` | none |
| Focus | `--color-error` | `--color-neutral-100` | 2px solid `--color-focus-ring`, offset 3px |
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
- **Input field:** Height `--size-target-min` (44px). Background `--term-bg` (`#16140f`). Border 1px solid `--color-border`. `--radius-sm` (4px). Horizontal padding `--space-3` (12px). Font `--font-size-ui-base` (13px), `--color-text-primary`.
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

**Props:**

| Prop | Type | Description |
|---|---|---|
| `value` | `string` | Currently selected option value. |
| `options` | `{ value: string; label: string }[]` | Available options. |
| `label` | `string` | Visible label rendered above the trigger. |
| `helper` | `string \| undefined` | Optional descriptive text rendered below the trigger field. Follows the same pattern as `TextInput.helper` (§7.15): same typographic style (`--font-size-ui-sm`, `--color-text-secondary`), same `aria-describedby` wiring between the trigger element and the helper text container. |
| `disabled` | `boolean` | Disables the control. |

**Closed state:** Identical to text input (§7.15) with a Lucide `ChevronDown` icon (`--size-icon-sm`, `--color-icon-default`) right-aligned inside the field.

**Open state:**
- The dropdown menu appears below the trigger field by default.
- **Background:** `--color-bg-raised`.
- **Border:** 1px solid `--color-border-overlay` (`#4a4640`).
- **Border radius:** `--radius-md` (8px).
- **Shadow:** `--shadow-raised`.
- **Z-index:** `--z-dropdown` (30).
- **Max height:** 240px (scrollable).
- **Option items:** Same styling as context menu items (§7.8.3). Active/selected option has left border 2px solid `--color-accent` and background `--color-accent-subtle`.

**Portal and collision detection:**
- The dropdown list is rendered via a portal to `<body>` (Bits UI `Select.Content` with `use:portal`). This prevents clipping by scrollable ancestors or CSS-transformed containers — a concern inside the Preferences panel, which has `overflow: hidden` and an independent scroll region.
- Positioning is delegated to Floating UI (integrated in Bits UI): the list flips to open above the trigger when vertical space below is insufficient (`avoidCollisions: true`).
- **Side offset:** `sideOffset: 4` (4px gap between the trigger bottom edge and the list top edge). This matches the current implementation and is the canonical value — do not override it at the component level.

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

This banner appears only when the shell exits with a non-zero code or is terminated by a signal. For a clean exit (code 0), the pane closes immediately and this banner is never shown (FS-PTY-005).

When the pane transitions to the terminated state:

- The terminal content remains visible (scrollback preserved).
- A horizontal banner appears at the bottom of the pane.
- **Background:** `--color-bg-surface` (`#242118`).
- **Border-top:** 1px solid `--color-border`.
- **Height:** Auto, minimum `--size-target-min` (44px).
- **Padding:** `--space-3` (12px).
- **Layout:** Flex row. Left: exit status text. Right: "Restart" (primary button) and "Close" (ghost button).

**Exit status text:**
- Non-zero exit: Lucide `XCircle` (`--color-error`, 16px) + "Process exited with code {N}" in `--color-text-primary`. Technical details (signal name if applicable) in `--color-text-secondary` below.
- Signal termination: same visual treatment; signal name shown in technical details if available.

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
- **Header:** Theme name input (text input, §7.15, required) + "Back to themes" link/button (ghost, Lucide `ChevronLeft` + "Themes"). The name input carries a `helper` noting that the name must be unique and is used to identify the theme in the theme list. i18n key: `preferences_themes_editor_name_hint`.
- **Scrollable body:** All token fields organized into sections. Each token section heading carries a brief descriptive sentence in body text (`--color-text-secondary`) explaining what that group of tokens controls. Color picker fields carry a `helper` (i18n key pattern: `preferences_themes_editor_{token_name}_hint`) describing what the color affects (e.g., "Used for all terminal text that does not have an explicit SGR foreground color set.").
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

### 7.22 Full-Screen Mode (FS-FULL-001 – FS-FULL-006)

Full-screen mode expands the terminal content to fill the entire display, temporarily suppressing the tab bar and status bar. The design follows the Umbra principle of *managed density*: chrome recedes so the terminal surface can occupy the available space without visual interruption. The chrome is not destroyed — it can be recalled when needed.

#### 7.22.1 Layout in Full-Screen Mode

In full-screen mode the window anatomy changes from the four-band layout (§6.1) to a single surface:

```
+----------------------------------------------------------+
|                                                          |
|  Terminal Area      (100% width × 100% height)           |
|    (pane dividers, scrollbars, and in-pane overlays      |
|     remain within this surface)                          |
|                                                          |
+----------------------------------------------------------+
```

- The **tab bar** and **status bar** are hidden from view (not destroyed — their state is preserved).
- All in-pane elements remain visible: pane dividers, scrollbars, the scroll-to-bottom button, the search overlay, in-pane SSH banners, the process-terminated banner, and the deprecated-algorithm banner.
- The **window frame** (title bar, OS window borders) is suppressed — this is handled by Tauri at the window manager level. TauTerm UX takes effect only after the OS transition completes.

#### 7.22.2 Entrance and Exit Transition

Full-screen transitions are driven by the OS/window-manager compositor, not by TauTerm CSS. TauTerm cannot animate the physical expansion of pixels to fill the screen — that is outside the WebView boundary. TauTerm does control the *chrome visibility change*:

| Surface | Entrance (entering full screen) | Exit (leaving full screen) |
|---------|--------------------------------|---------------------------|
| Tab bar | `opacity` 1 → 0, `--duration-fast` (80ms), `--ease-in` — hides as full screen activates | `opacity` 0 → 1, `--duration-base` (100ms), `--ease-out` — reveals as full screen exits |
| Status bar | Same as tab bar: `opacity` 1 → 0, `--duration-fast`, `--ease-in` | `opacity` 0 → 1, `--duration-base`, `--ease-out` |
| Full-screen indicator badge | `opacity` 0 → 1, `--duration-base`, `--ease-out` — appears after bars have hidden | `opacity` 1 → 0, `--duration-fast`, `--ease-in` |

**Timing rationale:** Hiding is faster than revealing. The `--ease-in` on exit from view matches the convention used by all other dismissals in the system (§9.2). The 80ms exit is fast enough to feel instant but prevents a jarring cut.

**`prefers-reduced-motion: reduce`:** All of the above transitions collapse to instant state changes (0ms). The chrome surfaces appear or disappear without animation.

#### 7.22.3 Auto-Hide and Recall of Chrome

When in full-screen mode:

- The tab bar and status bar are hidden by default.
- **Recall via hover:** Moving the mouse cursor to the top 4px of the window triggers the tab bar to appear. Moving it to the bottom 4px triggers the status bar to appear. The recall animation is `opacity` 0 → 1, `--duration-base` (100ms), `--ease-out`. The bars auto-hide again after the cursor moves away from the bar region for 1.5 seconds (same idle timeout as the scrollbar fade-out in §9.3), with `opacity` 1 → 0, `--duration-slow` (300ms), `--ease-in`.
- **Recall via keyboard:** Pressing the full-screen toggle shortcut (F11) exits full-screen entirely and restores chrome. No separate "peek" shortcut exists — the keyboard path is exit, not recall. This is consistent with the precedent set by virtually every Linux desktop application and avoids adding a non-standard interaction that users would need to discover.
- The hover zones are 4px tall to minimize accidental recalls while typing or moving the mouse normally.

**`prefers-reduced-motion: reduce`:** Recall and auto-hide transitions are instant.

#### 7.22.4 Full-Screen Indicator

The user needs a persistent, non-intrusive signal that the window is in full-screen mode, and a mouse-accessible path to exit.

**Full-Screen Exit Badge:**

- **Position:** `position: fixed`, top-right corner of the viewport, offset `--space-3` (12px) from each edge.
- **Z-index:** `--z-fullscreen-chrome` (see §3.10 — above all in-pane overlays but below modals and the recalled tab bar).
- **Anatomy:** Lucide `Minimize2` icon, `--size-icon-md` (16px), inside a pill-shaped container.
- **Size:** minimum 33×33px (same as scroll-to-bottom button). `border-radius: var(--radius-full)`.
- **Colors:**

| State | Background | Border | Icon color |
|-------|-----------|--------|-----------|
| Idle | `var(--color-bg-raised)` at `opacity: 0.7` | `1px solid var(--color-border)` | `var(--color-icon-default)` |
| Hover | `var(--color-hover-bg)` at `opacity: 1` | `1px solid var(--color-border)` | `var(--color-icon-active)` |
| Active | `var(--color-active-bg)` | `1px solid var(--color-border)` | `var(--color-icon-active)` |
| Focus | `var(--color-bg-raised)` | focus ring: 2px `var(--color-focus-ring)`, offset `var(--color-focus-ring-offset)` | `var(--color-icon-active)` |

- **Opacity:** The idle state uses `opacity: 0.7` to keep the badge present but at low visual temperature. Hover brings it to full opacity. This follows the Umbra principle — status indicators exist at reduced visual weight, stepping forward only when attended to.
- **Behavior:** Clicking or activating the badge exits full-screen mode (equivalent to F11).
- **Tooltip:** "Exit full screen (F11)", shown after `--duration-slow` (300ms) hover delay. `aria-describedby` on the badge element.
- **ARIA:** `role="button"`, `tabindex="0"`, `aria-label` bound to i18n key `exit_fullscreen`.
- **Auto-hide with chrome:** When the tab bar is recalled (via hover, §7.22.3), the exit badge hides — the tab bar and status bar provide sufficient context that the user is in full-screen mode. It reappears when the bars hide again.

**Status bar indicator (normal mode):** Outside of full-screen mode, the status bar carries no full-screen indicator — there is nothing unusual to communicate. When entering full-screen, the status bar hides before an indicator in it would be useful. The exit badge (above) is the primary discoverable affordance.

#### 7.22.5 Keyboard Navigation in Full-Screen Mode

Full-screen mode does not alter the keyboard navigation model. All global shortcuts remain active. The tab bar and status bar, though hidden, are not removed from the DOM — when recalled via hover they re-enter the natural tab order. Focus management:

- If focus was in the tab bar or status bar when full-screen was activated, focus moves to the active terminal pane.
- The full-screen exit badge is in the tab order at all times (it is always visible in full-screen, unlike the recalled bars). It receives focus via `Tab` cycling, positioned after the terminal area in the tab order.
- When full-screen is exited, focus returns to the active terminal pane (not the exit badge), consistent with the pattern that dismissing an overlay returns focus to the triggering context.

#### 7.22.6 Token References

All token values are defined in §3 (tokens) and §7.14 (buttons).

| Token | Usage in this component |
|-------|------------------------|
| `--duration-fast` | Tab bar / status bar fade-out on enter |
| `--duration-base` | Tab bar / status bar fade-in on exit; badge entrance; recall entrance |
| `--duration-slow` | Auto-hide fade-out after cursor leaves |
| `--ease-in` | Dismissal easing |
| `--ease-out` | Appearance easing |
| `--z-fullscreen-chrome` | Exit badge z-index |
| `--color-bg-raised` | Badge idle background |
| `--color-hover-bg` | Badge hover background |
| `--color-active-bg` | Badge active background |
| `--color-border` | Badge border |
| `--color-icon-default` | Badge idle icon |
| `--color-icon-active` | Badge hover/active/focus icon |
| `--color-focus-ring` | Badge focus ring |
| `--color-focus-ring-offset` | Badge focus ring offset |
| `--radius-full` | Badge shape |
| `--space-3` | Badge offset from viewport edges |
| `--size-icon-md` | Badge icon size |
