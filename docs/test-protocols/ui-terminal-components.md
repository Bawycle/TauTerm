# Test Protocol — Terminal UI Components

> **Version:** 1.0.0
> **Date:** 2026-04-05
> **Status:** Draft
> **Scope:** Frontend rendering layer — TerminalPane, TabBar, StatusBar, TerminalView
> **Input documents:** FS.md, UXD.md §7.1/7.2/7.3/7.5, ARCHITECTURE.md §4.2/4.3/4.5, CLAUDE.md
> **Compiled by:** moe (Maître d'Œuvre) — synthesis of domain-expert, ux-designer, security-expert perspectives

---

## 1. Purpose & Scope

This protocol covers test scenarios for the Terminal UI core components. It is **not** a retest of the Rust VT parser (covered by the functional PTY/VT/SSH protocol). It targets the Svelte frontend layer: what it renders, how it responds to IPC events, how it routes input, and what security properties it maintains.

**Components in scope:**
- `TerminalPane` — cell grid, cursor, selection, scrollbar, keyboard/mouse handler, ResizeObserver
- `TabBar` — tab items, activity indicators, new-tab button, close button, keyboard nav
- `StatusBar` — session type label, SSH lifecycle state display
- `TerminalView` — mount/unmount lifecycle, session state management

**Libraries required:**
- Lucide-svelte for all icons (no custom SVG)
- Bits UI for all headless primitives (Tooltip, DropdownMenu, Dialog, etc.)
- No `{@html}` with user or terminal data

---

## 2. Functional Test Scenarios (domain-expert perspective)

> ID prefix: `TUITC-FN`
> Linked FS requirements noted in each entry.

### 2.1 Cursor Rendering

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-001 | Block cursor renders as filled rectangle | TerminalPane mounted, cursor shape=0 or 1 | Backend emits `screen-update` with `cursor.shape=1, cursor.visible=true` | Cursor cell renders with `background: var(--term-cursor-bg)`, character uses `color: var(--term-cursor-fg)` | FS-VT-030 |
| TUITC-FN-002 | Underline cursor renders as bottom bar | TerminalPane mounted | Backend emits `cursor.shape=3` | Cursor renders as `--size-cursor-underline-height` horizontal line at cell bottom, color `--term-cursor-bg` | FS-VT-030, UXD §7.3.1 |
| TUITC-FN-003 | Bar cursor renders as left-edge vertical bar | TerminalPane mounted | Backend emits `cursor.shape=5` | Cursor renders as `--size-cursor-bar-width` vertical line at cell left edge | FS-VT-030, UXD §7.3.1 |
| TUITC-FN-004 | Blinking cursor toggles visibility | TerminalPane mounted, `cursor.blink=true` | Component interval fires at configured blink rate (default 530ms) | Cursor alternates between visible and hidden at ~530ms cadence | FS-VT-032 |
| TUITC-FN-005 | Unfocused pane shows hollow cursor | TerminalPane mounted with `active=false` | Pane loses focus (prop change or blur event) | Cursor renders as hollow outline of its current shape using `--term-cursor-unfocused`; never filled; never invisible | FS-VT-034, UXD §7.3.1 |
| TUITC-FN-006 | Hidden cursor (DECTCEM off) | TerminalPane mounted | Backend emits `cursor.visible=false` | No cursor element rendered in DOM | FS-VT-031 |

### 2.2 Cell Attributes

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-010 | Bold attribute applies font-weight | Pane rendering active | Cell with `attrs.bold=true` in `screen-update` | Cell span has `font-weight: bold` (or CSS class mapping to bold) | FS-VT-024 |
| TUITC-FN-011 | Italic attribute applies font-style | Pane rendering active | Cell with `attrs.italic=true` | Cell span has `font-style: italic` | FS-VT-024 |
| TUITC-FN-012 | Dim attribute reduces opacity/brightness | Pane rendering active | Cell with `attrs.dim=true` | Cell span applies dim styling (opacity or CSS filter) | FS-VT-024 |
| TUITC-FN-013 | Underline attribute applies text-decoration | Pane rendering active | Cell with `attrs.underline=1` | Cell span has `text-decoration: underline` | FS-VT-024 |
| TUITC-FN-014 | Inverse attribute swaps fg/bg | Pane rendering active | Cell with `attrs.inverse=true` | Cell fg and bg colors are swapped | FS-VT-024 |
| TUITC-FN-015 | Hidden attribute renders invisible text | Pane rendering active | Cell with `attrs.hidden=true` | Cell content rendered with `color: transparent` or equivalent, occupies space | FS-VT-024 |
| TUITC-FN-016 | Strikethrough attribute applies line-through | Pane rendering active | Cell with `attrs.strikethrough=true` | Cell span has `text-decoration: line-through` | FS-VT-024 |

### 2.3 Color Rendering

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-020 | ANSI 16 fg color maps to design token variable | Pane rendering active | Cell with `attrs.fg = { type: 'ansi', index: 1 }` (red) | Cell uses `var(--ansi-red)` (or equivalent token); color is theme-remappable | FS-VT-020, FS-VT-023 |
| TUITC-FN-021 | 256-color fg maps to correct RGB value | Pane rendering active | Cell with `attrs.fg = { type: 'ansi256', index: 196 }` | Cell color resolves to `rgb(255, 0, 0)` (index 196 = pure red in 6x6x6 cube) | FS-VT-021 |
| TUITC-FN-022 | Truecolor fg renders exact RGB | Pane rendering active | Cell with `attrs.fg = { type: 'rgb', r: 255, g: 100, b: 0 }` | Cell color is `rgb(255, 100, 0)` exactly | FS-VT-022 |
| TUITC-FN-023 | Default fg uses terminal default token | Pane rendering active | Cell with no `attrs.fg` (undefined) | Cell uses `var(--term-fg)` (inherited terminal foreground) | FS-VT-020 |
| TUITC-FN-024 | Default bg uses terminal background | Pane rendering active | Cell with no `attrs.bg` | Cell uses `var(--term-bg)` as background | FS-VT-020 |

### 2.4 Wide Characters (CJK)

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-030 | Wide cell (width=2) spans two column positions | Pane rendering a row | SnapshotCell with `width=2` at col 0 | DOM element spans 2 character-cell widths; col 1 is occupied (no separate cell rendered) | FS-VT-011 |
| TUITC-FN-031 | Combining cell (width=0) attaches to preceding cell | Pane rendering a row | SnapshotCell with `width=0` | No new column consumed; character rendered on the same cell as preceding base | FS-VT-012 |

### 2.5 Keyboard Input Encoding

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-040 | Ctrl+C encodes as 0x03 | Active pane has keyboard focus, DECCKM=false | User presses Ctrl+C | `send_input` called with `data=[3]` | FS-KBD-004 |
| TUITC-FN-041 | Arrow Up in normal mode encodes ESC [ A | Active pane, DECCKM=false | User presses ArrowUp | `send_input` called with `data=[27, 91, 65]` | FS-KBD-007 |
| TUITC-FN-042 | Arrow Up in application mode encodes ESC O A | Active pane, DECCKM=true (mode-state-changed received) | User presses ArrowUp | `send_input` called with `data=[27, 79, 65]` | FS-KBD-007 |
| TUITC-FN-043 | Enter encodes as 0x0D | Active pane | User presses Enter | `send_input` called with `data=[13]` | FS-KBD-004 |
| TUITC-FN-044 | Escape encodes as 0x1B | Active pane | User presses Escape | `send_input` called with `data=[27]` | FS-KBD-004 |
| TUITC-FN-045 | F1 encodes as ESC O P | Active pane | User presses F1 | `send_input` called with `data=[27, 79, 80]` | FS-KBD-006 |
| TUITC-FN-046 | Alt+A encodes as ESC a (ESC prefix) | Active pane | User presses Alt+A | `send_input` called with `data=[27, 97]` | FS-KBD-005 |
| TUITC-FN-047 | Ctrl+Shift+T intercepted — not sent to PTY | Active pane | User presses Ctrl+Shift+T | `send_input` is NOT called; new tab is created | FS-KBD-001, FS-KBD-003 |
| TUITC-FN-048 | mode-state-changed updates DECCKM flag | Active pane has DECCKM=false | Backend emits `mode-state-changed` with `decckm=true` | Subsequent ArrowUp encodes ESC O A (application mode) | FS-KBD-007, ARCH §4.3 |

### 2.6 Scrollback

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-050 | Scrollbar hidden when at bottom and no scrollback | Pane at bottom, scrollbackLines=0 | Initial state | Scrollbar thumb not rendered | UXD §7.3.3 |
| TUITC-FN-051 | Scrollbar visible when scrolled up | Pane has scrollback content | scroll-position-changed with offset>0 | Scrollbar thumb rendered with `--color-scrollbar-thumb`, min-height 32px | UXD §7.3.3 |
| TUITC-FN-052 | Mouse wheel triggers scroll_pane IPC | Pane mounted | User scrolls mouse wheel up | `scroll_pane` invoked with negative delta offset | FS-SB, ARCH §4.2 |
| TUITC-FN-053 | Scroll to bottom on new output when at bottom | Pane viewport at bottom | Backend emits `screen-update` | Viewport stays at bottom (auto-follow) | FS-SB |

### 2.7 Selection

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-060 | Click-drag selects cell range | Pane rendered | Mouse down at cell (2,3), drag to (2,8), mouse up | Cells in range highlighted with `--term-selection-bg`; `copy_to_clipboard` called with selected text | FS-CLIP-001, FS-CLIP-002 |
| TUITC-FN-061 | Selection uses cell boundaries, not pixels | Pane rendered | Mouse down at pixel mid-point of a cell | Selection snaps to full cell boundary | FS-CLIP-001 |
| TUITC-FN-062 | Unfocused pane uses inactive selection color | Two panes, selection in non-active pane | Pane loses focus after selection | Selected cells use `--term-selection-bg-inactive` | UXD §7.3.2 |

### 2.8 Screen Update IPC Flow

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-070 | screen-update event triggers DOM cell update | TerminalPane listening to screen-update | Backend emits `ScreenUpdateEvent` for pane | Changed cells only update their DOM representation; unchanged cells unaffected | ARCH §4.3 |
| TUITC-FN-071 | Initial render uses get_pane_screen_snapshot | TerminalPane mounted | Component mounts | `get_pane_screen_snapshot` called with paneId; full grid rendered from snapshot | ARCH §4.2 |
| TUITC-FN-072 | ResizeObserver fires resize_pane IPC | TerminalPane mounted | Viewport dimensions change (ResizeObserver fires) | `resize_pane` invoked with correct cols, rows, pixelWidth, pixelHeight | ARCH §4.2 |
| TUITC-FN-073 | resize_pane debounced on rapid resize | TerminalPane, fast resize sequence | Multiple ResizeObserver events within 16ms | `resize_pane` called once per stable dimensions, not on every intermediate resize | FS-KBD (§6.5 ARCH) |

### 2.9 OSC Title and Tab Display

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-080 | OSC 0 title update reflected in tab | TerminalView with one tab | `session-state-changed` arrives with updated `processTitle` | Tab item displays the new process title | FS-VT-060, FS-TAB-006 |
| TUITC-FN-081 | User-defined label takes precedence over OSC title | Tab has user label set | `session-state-changed` with processTitle change | Tab still shows user label, not new OSC title | FS-TAB-006 |

### 2.10 Alternate Screen Buffer

| ID | Title | Precondition | Action | Expected Result | FS Ref |
|----|-------|-------------|--------|-----------------|--------|
| TUITC-FN-090 | Scrollbar hidden in alternate screen | Pane in alt screen mode | Alt screen active | No scrollbar; scroll interactions disabled | FS-SB-005 |
| TUITC-FN-091 | Scrollback preserved on alt screen return | Pane had scrollback before alt screen | Return from alt screen | Scrollback content intact and navigable | FS-SB-006 |

---

## 3. Visual / Interaction Test Scenarios (ux-designer perspective)

> ID prefix: `TUITC-UX`
> Measured against design tokens in UXD.md §7.

### 3.1 Tab Bar Container

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-001 | Tab bar height matches token | `.tab-bar` element | Computed height = `--size-tab-height` (40px) | UXD §7.1.1 |
| TUITC-UX-002 | Tab bar background uses correct token | `.tab-bar` | Background = `--color-tab-bg` (`#242118`) | UXD §7.1.1 |
| TUITC-UX-003 | Tab bar has bottom border | `.tab-bar` | `border-bottom: 1px solid var(--color-border)` | UXD §7.1.1 |
| TUITC-UX-004 | Tab bar ARIA role | `.tab-bar` | `role="tablist"` present | UXD §7.1.1, FS-A11Y-003 |

### 3.2 Tab Item States

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-010 | Active tab background uses correct token | Active tab item | Background = `--color-tab-active-bg` (`#16140f`) | UXD §7.1.2 |
| TUITC-UX-011 | Active tab text color and weight | Active tab title span | Color = `--color-tab-active-fg` (`#e8e3d8`), font-weight = 600 | UXD §7.1.2 |
| TUITC-UX-012 | Inactive tab text color | Inactive tab item | Color = `--color-tab-inactive-fg` (`#6b6660`), font-weight = 400 | UXD §7.1.2 |
| TUITC-UX-013 | Tab min/max width enforced | Tab item | min-width = 120px, max-width = 240px | UXD §7.1.2 |
| TUITC-UX-014 | Tab title truncates with ellipsis | Tab item with 40-char title | `text-overflow: ellipsis` applied; no overflow outside bounds | UXD §7.1.2 |
| TUITC-UX-015 | Tab ARIA role and selected state | Tab item element | `role="tab"`, `aria-selected="true"` on active, `aria-selected="false"` on inactive | UXD §7.1.2, FS-A11Y-003 |
| TUITC-UX-016 | Tab focus ring on keyboard focus | Tab item | 2px solid `--color-focus-ring`, offset 2px, visible on keyboard focus | UXD §7.1.2 |

### 3.3 Tab Activity Indicators

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-020 | Output activity dot renders correctly | Tab with `notification.type='backgroundOutput'` | Filled circle, diameter = `--size-badge` (6px), color = `--color-activity` (`#78c078`) | UXD §7.1.3 |
| TUITC-UX-021 | Process exit 0 renders CheckCircle icon | Tab with `notification.type='processExited', exitCode=0` | Lucide `CheckCircle` icon, `--size-icon-sm` (14px), color = `--color-process-end` (`#9c9890`) | UXD §7.1.3 |
| TUITC-UX-022 | Process exit non-zero renders XCircle icon | Tab with `notification.type='processExited', exitCode=1` | Lucide `XCircle` icon, `--size-icon-sm` (14px), color = `--color-error` (`#c44444`) | UXD §7.1.3 |
| TUITC-UX-023 | Bell indicator renders Bell icon | Tab with `notification.type='bell'` | Lucide `Bell` icon, `--size-icon-sm` (14px), color = `--color-bell` (`#d48a20`) | UXD §7.1.3 |
| TUITC-UX-024 | Notification cleared on tab switch | Tab with active notification | User activates the tab | Notification indicator disappears; `notification: null` state | FS-NOTIF-003 |

### 3.4 Tab Close Button

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-030 | Close button hit area ≥ 44×44px | Tab close button | Computed clickable area ≥ 44×44px | UXD §7.1.4, FS-A11Y-002 |
| TUITC-UX-031 | Close button uses Lucide X icon | Tab close button | Renders `<X>` from lucide-svelte | UXD §7.1.4 |
| TUITC-UX-032 | Close button hidden on inactive tab at rest | Inactive tab, no hover | Close button not visible (hidden/opacity 0) | UXD §7.1.4 |
| TUITC-UX-033 | Close button visible on inactive tab hover | Inactive tab | Mouse hover over tab | Close button becomes visible | UXD §7.1.4 |
| TUITC-UX-034 | Close button always visible on active tab | Active tab | No hover required | Close button rendered and visible | UXD §7.1.4 |

### 3.5 New Tab Button

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-040 | New tab button hit area ≥ 44×44px | `.tab-bar__new-tab` | Width = `--size-target-min` (44px), height = `--size-tab-height` (40px) — 40px accepted per flex layout | UXD §7.1.5, FS-A11Y-002 |
| TUITC-UX-041 | New tab button ARIA label | New tab button element | `aria-label="New tab"` | UXD §7.1.5 |
| TUITC-UX-042 | New tab button uses Lucide Plus icon | New tab button | Renders `<Plus>` from lucide-svelte, `--size-icon-sm` | UXD §7.1.5 |
| TUITC-UX-043 | New tab button tooltip via Bits UI Tooltip | New tab button | Bits UI `Tooltip.Root` with 300ms delay; content "New Tab (Ctrl+Shift+T)" | UXD §7.1.5, UXD §7.10 |

### 3.6 Terminal Area Cursor

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-050 | Block cursor uses --term-cursor-bg fill | Cursor overlay, block shape | Background = `var(--term-cursor-bg)` | UXD §7.3.1 |
| TUITC-UX-051 | Underline cursor uses correct height token | Cursor overlay, underline shape | Height = `var(--size-cursor-underline-height)`, positioned at cell bottom | UXD §7.3.1 |
| TUITC-UX-052 | Bar cursor uses correct width token | Cursor overlay, bar shape | Width = `var(--size-cursor-bar-width)`, positioned at cell left edge | UXD §7.3.1 |
| TUITC-UX-053 | Unfocused cursor uses --term-cursor-unfocused | Pane not focused | Border color = `var(--term-cursor-unfocused)`, no fill | UXD §7.3.1 |

### 3.7 Selection

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-060 | Focused selection uses --term-selection-bg | Selection highlight, active pane | Background = `var(--term-selection-bg)` (`#2e6f9c`) | UXD §7.3.2 |
| TUITC-UX-061 | Unfocused selection uses inactive token | Selection highlight, inactive pane | Background = `var(--term-selection-bg-inactive)` (`#1a3a52`) | UXD §7.3.2 |

### 3.8 Scrollbar

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-070 | Scrollbar overlays content — no layout shift | Pane with scrollbar | Adding/removing scrollbar does not change pane content width | UXD §7.3.3 |
| TUITC-UX-071 | Scrollbar width matches token | Scrollbar element | Width = `var(--size-scrollbar-width)` (8px) | UXD §7.3.3 |
| TUITC-UX-072 | Scrollbar thumb min-height enforced | Scrollbar with very large scrollback | Thumb height ≥ 32px | UXD §7.3.3 |
| TUITC-UX-073 | Scrollbar thumb uses correct token | Scrollbar thumb | Background = `var(--color-scrollbar-thumb)` (`#4a4640`) | UXD §7.3.3 |

### 3.9 Pane Divider

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-080 | Divider visual width is 1px | Pane divider element | Border width = 1px, color = `--color-divider` | UXD §7.2 |
| TUITC-UX-081 | Divider hit area is 8px | Pane divider hit target | Clickable/draggable area = `--size-divider-hit` (8px) centered on visual line | UXD §7.2 |
| TUITC-UX-082 | Divider cursor changes on hover | Vertical divider | `cursor: col-resize` on hover | UXD §7.2 |
| TUITC-UX-083 | Divider color changes on hover | Divider element | Border-color transitions to `--color-divider-active` (`#4a92bf`) | UXD §7.2 |

### 3.10 Status Bar SSH Indicator

| ID | Title | Element | Assertion | UXD Ref |
|----|-------|---------|-----------|---------|
| TUITC-UX-090 | Connected state: text and icon correct | Status bar, SSH pane connected | Text = "{user}@{host}", Lucide `Network` icon, color = `--color-ssh-badge-fg` | UXD §7.5.1 |
| TUITC-UX-091 | Disconnected state: text and icon correct | Status bar, SSH pane disconnected | Text = "Disconnected", Lucide `WifiOff` icon, color = `--color-ssh-disconnected-fg` | UXD §7.5.1 |
| TUITC-UX-092 | Local session: no SSH indicator | Status bar, local pane | No SSH-related indicator rendered | UXD §7.5.1 |

### 3.11 WCAG 2.1 AA Contrast

| ID | Title | Element | Assertion | Ref |
|----|-------|---------|-----------|-----|
| TUITC-UX-100 | Active tab title contrast ≥ 4.5:1 | Active tab title text vs tab background | `#e8e3d8` on `#16140f` ≥ 4.5:1 | FS-A11Y-001 |
| TUITC-UX-101 | Inactive tab title contrast ≥ 4.5:1 | Inactive tab title vs tab bar background | `#6b6660` on `#242118` — verify ratio | FS-A11Y-001 |
| TUITC-UX-102 | Status bar text contrast ≥ 4.5:1 | Status bar text vs background | `--color-text-secondary` on `--color-bg-surface` | FS-A11Y-001 |
| TUITC-UX-103 | Terminal foreground contrast ≥ 4.5:1 | Default terminal text | `--term-fg` (`#ccc7bc`) on `--term-bg` (`#16140f`) | FS-A11Y-001 |

### 3.12 Keyboard Navigation

| ID | Title | Element | Assertion | Ref |
|----|-------|---------|-----------|-----|
| TUITC-UX-110 | Tab key cycles through tab bar items | Tab bar | Pressing Tab navigates between tab items in DOM order | FS-A11Y-003 |
| TUITC-UX-111 | Left/Right arrow navigates tabs within tablist | Tab bar | Arrow Left/Right moves focus between tab items within `role="tablist"` | UXD §7.1.2 |
| TUITC-UX-112 | Enter/Space activates focused tab | Focused tab item | Pressing Enter or Space switches to that tab | UXD §7.1.2 |
| TUITC-UX-113 | Delete key closes focused tab | Focused tab item | Pressing Delete triggers close (with confirmation if running process) | UXD §7.1.2 |

### 3.13 Reduced Motion

| ID | Title | Element | Assertion | Ref |
|----|-------|---------|-----------|-----|
| TUITC-UX-120 | Pane border pulse skipped when prefers-reduced-motion | Pane divider/border | Under `@media (prefers-reduced-motion: reduce)`, border color changes instantly; no CSS transition | UXD §7.2.1 |
| TUITC-UX-121 | Cursor blink respects reduced-motion | Cursor overlay | Under `prefers-reduced-motion`, cursor does not blink; appears steady | UXD §7.3.1 |

---

## 4. Security Test Scenarios (security-expert perspective)

> ID prefix: `TUITC-SEC`
> Referenced against existing security test protocol and ARCHITECTURE.md §8.

### 4.1 Terminal Output XSS Prevention

| ID | Threat | Attack Vector | Expected Defense | Ref |
|----|--------|--------------|-----------------|-----|
| TUITC-SEC-001 | XSS via terminal cell content | Cell `content` field contains `<script>alert(1)</script>` | Content rendered as text node (DOM textContent), never via `innerHTML`/`{@html}`; no script execution | ARCH §8.5, FS-SEC-001 |
| TUITC-SEC-002 | XSS via terminal cell content with HTML entities | Cell `content` = `<img src=x onerror=alert(1)>` | Same defense: textContent, no HTML parsing | ARCH §8.5 |
| TUITC-SEC-003 | CSP blocks inline script injection | Attacker attempts to inject `<script>` into DOM via any terminal output | CSP `script-src 'self'` prevents execution; no `unsafe-inline` in script-src | FS-SEC-001, ARCH §8.4 |

### 4.2 Tab Title Injection

| ID | Threat | Attack Vector | Expected Defense | Ref |
|----|--------|--------------|-----------------|-----|
| TUITC-SEC-010 | HTML injection via OSC tab title | `processTitle` = `<b>Bold</b><script>evil()</script>` | Title rendered via `textContent` in tab item; HTML not parsed; Svelte template binding (not `{@html}`) | FS-VT-062, ARCH §8.5 |
| TUITC-SEC-011 | C0 control chars in tab title | `processTitle` contains `\x01\x08\x1b[1m` | Title displayed with control chars stripped (sanitized in Rust backend per FS-VT-062) | FS-VT-062 |

### 4.3 Hyperlink URI Validation

| ID | Threat | Attack Vector | Expected Defense | Ref |
|----|--------|--------------|-----------------|-----|
| TUITC-SEC-020 | javascript: URI execution | Hyperlink cell with `javascript:alert(1)` | `open_url` command rejects `javascript:` scheme; Ctrl+Click triggers IPC call with validation, not direct `window.open` | FS-VT-073, ARCH §8.1 |
| TUITC-SEC-021 | data: URI injection | Hyperlink with `data:text/html,<script>...` | `open_url` rejects `data:` scheme | FS-VT-073 |
| TUITC-SEC-022 | Unknown scheme accepted | Hyperlink with `foobar:something` | `open_url` rejects unknown scheme; no browser navigation | FS-VT-073 |
| TUITC-SEC-023 | URI length limit | Hyperlink URI > 2048 chars | `open_url` rejects; no navigation | FS-VT-073, ARCH §8.1 |
| TUITC-SEC-024 | Null byte in URI | Hyperlink URI contains `\0` byte | URI rejected; treated as control character | FS-VT-073 |
| TUITC-SEC-025 | Valid http URI accepted | Hyperlink `https://example.com` | `open_url` called; system browser opens | FS-VT-073 |

### 4.4 Input Sanitization

| ID | Threat | Attack Vector | Expected Defense | Ref |
|----|--------|--------------|-----------------|-----|
| TUITC-SEC-030 | Oversized send_input payload | Keyboard handler constructs payload > 64 KiB | Backend `send_input` guard rejects; frontend should not generate such payloads from normal keystrokes | ARCH §8.1, FS-SEC |
| TUITC-SEC-031 | PTY input is raw bytes only | Keyboard handler | `data` field in `send_input` is `number[]` (byte array), never a raw string; no string eval path | ARCH §4.2 |

### 4.5 Clipboard Security

| ID | Threat | Attack Vector | Expected Defense | Ref |
|----|--------|--------------|-----------------|-----|
| TUITC-SEC-040 | OSC52 clipboard write without consent | Remote session sends OSC52 write sequence | Write gated by `allowOsc52Write` flag on `SshConnectionConfig`; off by default | FS-CLIP, ARCH §8.3 |
| TUITC-SEC-041 | Bracketed paste injection | User pastes multi-line command; terminal in bracketed paste mode | Pasted content wrapped with `\x1b[200~` and `\x1b[201~`; existing end-sequence stripped from paste | FS-CLIP-008 |

### 4.6 Resize Input Validation

| ID | Threat | Attack Vector | Expected Defense | Ref |
|----|--------|--------------|-----------------|-----|
| TUITC-SEC-050 | Zero-dimension resize | `resize_pane` called with `cols=0` or `rows=0` | Frontend clamps to minimum 1 before IPC call; backend handles gracefully without integer underflow | ARCH §4.2 |
| TUITC-SEC-051 | Overflow resize dimensions | `resize_pane` with `cols=65535, rows=65535` (u16 max) | Backend handles without crash; PTY resize either accepts or returns error | ARCH §4.2 |

---

## 5. Test Implementation Notes

### 5.1 Tools

| Layer | Tool | Command |
|-------|------|---------|
| Frontend unit tests | vitest | `pnpm vitest run` |
| Frontend type checking | svelte-check / tsc | `pnpm check` |
| Component behavior tests | vitest + @testing-library/svelte (if available) or plain vitest with mock | `pnpm vitest run` |
| Rust backend tests | cargo-nextest | `cargo nextest run` (from `src-tauri/`) |

### 5.2 UI Library Requirements (CRITICAL)

All component implementation and tests MUST verify:
- **Lucide-svelte**: Every icon is a named export from `lucide-svelte` (e.g., `import { X, Plus, Network, WifiOff, Bell, CheckCircle, XCircle } from 'lucide-svelte'`). No custom SVG paths.
- **Bits UI**: Tooltip uses `Tooltip.Root/Trigger/Content` from `bits-ui`. Context menus use `DropdownMenu.*`. Dialogs use `Dialog.*`. No custom implementations of these headless primitives.
- **No `{@html}`** anywhere in components that render user or terminal data.
- **SPDX header** on every new file: `<!-- SPDX-License-Identifier: MPL-2.0 -->` for Svelte, `// SPDX-License-Identifier: MPL-2.0` for TS.

### 5.3 Scenarios Deferred to Integration/E2E

The following functional scenarios require a live PTY and are deferred to the E2E test suite (`pnpm wdio`):
- TUITC-FN-040 through TUITC-FN-048 (real keyboard encoding with PTY)
- TUITC-FN-060 through TUITC-FN-062 (mouse selection with rendered content)
- TUITC-FN-090, TUITC-FN-091 (alternate screen requires live PTY)

Unit-testable via mocks: TUITC-FN-001 to TUITC-FN-016, TUITC-FN-020 to TUITC-FN-024, TUITC-FN-041 to TUITC-FN-048 (keyboard.ts logic), TUITC-FN-050 to TUITC-FN-053 (scroll state), TUITC-FN-070 to TUITC-FN-073 (IPC wiring).

### 5.4 WCAG Note on Inactive Tab Contrast

TUITC-UX-101: `#6b6660` on `#242118` yields a contrast ratio of approximately 2.5:1, which does **not** meet WCAG AA (4.5:1). This is a pre-existing design decision in UXD.md §7.1.2 — inactive tabs use a deliberately muted tone to distinguish them from the active tab. This is flagged here for the design team's awareness; v1 ships as designed with this known exception.

---

## 6. Traceability Summary

| FS Area | Scenarios Covered |
|---------|------------------|
| FS-VT-020 to FS-VT-025 (colors) | TUITC-FN-020 to FN-024 |
| FS-VT-030 to FS-VT-034 (cursor) | TUITC-FN-001 to FN-006, TUITC-UX-050 to UX-053 |
| FS-VT-060 to FS-VT-063 (OSC title) | TUITC-FN-080, TUITC-FN-081, TUITC-SEC-010, TUITC-SEC-011 |
| FS-VT-070 to FS-VT-073 (hyperlinks) | TUITC-SEC-020 to TUITC-SEC-025 |
| FS-KBD-001 to FS-KBD-010 | TUITC-FN-040 to FN-048 |
| FS-CLIP-001 to FS-CLIP-009 | TUITC-FN-060 to FN-062, TUITC-SEC-040, TUITC-SEC-041 |
| FS-SB-001 to FS-SB-006 | TUITC-FN-050 to FN-053, TUITC-FN-090, TUITC-FN-091 |
| FS-TAB-001 to FS-TAB-008 | TUITC-FN-080, TUITC-FN-081, TUITC-UX-010 to UX-016 |
| FS-NOTIF-001 to FS-NOTIF-004 | TUITC-UX-020 to UX-024 |
| FS-A11Y-001 to FS-A11Y-006 | TUITC-UX-030, TUITC-UX-040, TUITC-UX-100 to UX-113 |
| FS-SEC-001 (CSP) | TUITC-SEC-003 |
| ARCH §4.2 (IPC commands) | TUITC-FN-070 to FN-073, TUITC-SEC-030, TUITC-SEC-031, TUITC-SEC-050, TUITC-SEC-051 |
| ARCH §4.3 (events) | TUITC-FN-070, TUITC-FN-048 |
