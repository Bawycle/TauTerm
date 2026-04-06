<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Accessibility & Responsiveness

> Part of the [UX/UI Design](README.md).

---

## 11. Accessibility

### 11.1 Contrast Audit

All color pairings used in the UI are documented in [§5.4](02-tokens.md#54-accessibility-compliance) with measured contrast ratios. Summary:

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
| `F11` | Toggle full-screen mode | Always |
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
| Full-screen exit badge | `button` | `aria-label` bound to i18n key `exit_fullscreen`; `aria-describedby` pointing to tooltip |
| Full-screen mode announcement | `aria-live="polite"` region | Hidden `<span>` at `position: absolute; clip-path: inset(50%)` announces the mode change to screen readers: "Entered full screen" / "Exited full screen". Located at the top of the root layout. |
| Tooltip | `tooltip` | Referenced by trigger's `aria-describedby` |

### 11.4 Reduced Motion Policy

See [§9.4](04-interaction.md#94-prefers-reduced-motion-policy). All animations are disabled when `prefers-reduced-motion: reduce` is active. No motion is essential for understanding the UI — all state changes are communicated through color and shape changes that persist without animation.

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
| Full-screen mode active | Exit badge visible in corner | `Minimize2` icon shape + `aria-live` announcement + tooltip text |

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
