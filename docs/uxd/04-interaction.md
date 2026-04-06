<!-- SPDX-License-Identifier: MPL-2.0 -->

# UX/UI Design — Interaction Patterns, Motion & Iconography

> Part of the [UX/UI Design](README.md).

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

- **Select text (auto-copy to PRIMARY):** Mouse drag to select. On mouse release, text is copied to PRIMARY selection (FS-CLIP-004). Copy flash animation ([§7.12](03-components.md#712-copy-flash-animation-fs-clip-ur-63)) provides visual confirmation.
- **Copy to CLIPBOARD:** Right-click → Copy (FS-CLIP-006), or explicit keyboard shortcut if configured.
- **Paste from CLIPBOARD:** Ctrl+Shift+V (FS-CLIP-005).
- **Multi-line paste warning (FS-CLIP-009):** When bracketed paste is NOT active and pasted text contains newlines, a confirmation dialog appears. Heading: "Paste multi-line text?" Body: "The text contains {N} lines. Pasting multi-line text directly into a terminal may execute commands unintentionally." Action: "Paste" (primary), "Cancel" (ghost, default focus). A toggle "Don't ask again" at the bottom of the dialog, persisted in preferences.

### 8.6 SSH Connection Interruption Feedback

Per FS-SSH-022, disconnection is detected within 1 second:

1. **Immediate (0-1s):** Tab SSH badge transitions to Disconnected state ([§7.1.7](03-components.md#717-ssh-badge-on-tab)). Status bar indicator changes to "Disconnected" ([§7.5.1](03-components.md#751-status-bar-indicator)).
2. **Pane overlay (after detection):** Disconnection banner appears at bottom of pane ([§7.5.2](03-components.md#752-disconnection-overlay-in-pane)) with reason text and "Reconnect" button.
3. **Terminal content:** Remains visible and scrollable. No content is lost.
4. **Reconnect action:** Accessible from the pane banner (primary button), the tab context menu, and the connection manager. On reconnect, a separator line ([§7.19](03-components.md#719-ssh-reconnection-separator-fs-ssh-042)) marks the boundary.

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
| Full-screen — tab bar hide | Opacity 1→0, `--duration-fast` (80ms), `--ease-in` (on full-screen enter) | Opacity 0→1, `--duration-base` (100ms), `--ease-out` (on full-screen exit) | Instant |
| Full-screen — status bar hide | Same as tab bar hide row | Same as tab bar hide row | Instant |
| Full-screen — exit badge | Opacity 0→1, `--duration-base` (100ms), `--ease-out` (after bars have hidden) | Opacity 1→0, `--duration-fast` (80ms), `--ease-in` (on full-screen exit) | Instant |
| Full-screen — chrome recall (hover) | Opacity 0→1, `--duration-base` (100ms), `--ease-out` | Opacity 1→0, `--duration-slow` (300ms) after 1.5s idle, `--ease-in` | Instant |

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
| Terminal dimensions overlay (resize start) | Instant (0ms) — appears immediately | — | — |
| Terminal dimensions overlay (fade-out, 2s after resize end) | `--duration-slow` (300ms), opacity 1→0 | `--ease-in` | Instant disappear (no transition) |
| Full-screen chrome auto-hide (after cursor leaves recalled bar) | `--duration-slow` (300ms) after 1.5s idle, opacity 1→0 | `--ease-in` | Instant |

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
- Full-screen chrome transitions (tab bar/status bar hide/show, exit badge fade, recalled chrome fade) are all instant. The exit badge appears or disappears without animation.

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
| Exit full screen | `Minimize2` | md | Full-screen exit badge |
| Tab scroll left | `ChevronLeft` | sm | Tab bar overflow scroll |
| Tab scroll right | `ChevronRight` | sm | Tab bar overflow scroll |
| Open externally | `ExternalLink` | sm | Connection manager "open in new tab" |
| Group expand/collapse | `ChevronDown` / `ChevronRight` | sm | Connection list group headings |

### 10.4 Status Dots

Activity dots ([§7.1.3](03-components.md#713-tab-activity-indicators)) are CSS-rendered filled circles, not Lucide icons. They use `--size-badge` (6px) diameter with `--radius-full`. This distinction is intentional: filled dots communicate "presence" while outline icons communicate "action." (AD.md §6)
