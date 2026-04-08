<!-- SPDX-License-Identifier: MPL-2.0 -->

# TauTerm — Frontend Architecture

> Part of the [Architecture](README.md).

---

## 11. Frontend Architecture

### 11.1 Module Map

```
src/
  routes/
    +page.svelte          — main view: mounts PaneTree for the active tab
    +layout.svelte        — global keydown handler (gated on !isRecordingShortcut);
                            IPC event subscriptions lifecycle; SSR disabled
    +layout.ts            — export const ssr = false

  lib/
    ipc/
      types.ts            — TypeScript mirrors of all Rust IPC types: SessionState,
                            TabState, PaneNode, PaneState, SshLifecycleState,
                            ScreenUpdateEvent, CellUpdate, CellAttrs, Preferences,
                            UserTheme, SshConnectionConfig, TauTermError, etc.
      commands.ts         — typed invoke() wrappers for all 28 IPC commands
      events.ts           — typed listen() wrappers for all 7 events; return unsubscribe fn
      errors.ts           — TauTermError type + user-facing message helper +
                            error code → display string mapping

    state/
      session.svelte.ts   — SessionState replica; delta merge; getPane(id) helper
      ssh.svelte.ts       — SSH state keyed by PaneId
      notifications.svelte.ts — notification badges per pane/tab; cleared on activation
      preferences.svelte.ts   — Preferences replica; optimistic update
      scroll.svelte.ts    — scroll position per PaneId
      locale.svelte.ts    — reactive locale state; setLocale(lang) writes to preferences;
                            getLocale() returns current locale (FS-I18N-003, FS-I18N-004, FS-I18N-005)

    terminal/
      grid.ts             — ScreenGrid: applyDiff(), applySnapshot(), getAttributeRuns()
      mouse.ts            — mouse event routing: PTY vs TauTerm; xterm encoding
      selection.ts        — selection state machine: drag, word-select, line-select,
                            cell-boundary snapping, word delimiters (FS-CLIP-002, FS-CLIP-003)
      keyboard.ts         — keydown → PTY encoding: C0, Alt prefix, function keys,
                            modified keys (FS-KBD-004 through FS-KBD-012)
      hyperlinks.ts       — OSC 8: cell range → URI tracking, hover detection,
                            URI scheme validation before open_url invoke
      virtualization.ts   — row virtualization: visible viewport computation, DOM recycling
      ansi-palette.ts     — ANSI indices 0-15 → theme CSS tokens; 256-color cube/ramp;
                            truecolor passthrough

    layout/
      split-tree.ts       — SplitNode type; buildFromPaneNode(); updateRatio(); findLeaf()
      resize.ts           — drag resize math; minimum pane constraint; debounce SIGWINCH

    theming/
      apply.ts            — applyTheme(theme): void; setProperty() on :root;
                            cross-fade transition (FS-THEME-006)
      tokens.ts           — UMBRA_DEFAULT_TOKENS: fallback reference + reset to default
      validate.ts         — client-side validation before save: required tokens,
                            CSS.supports(), contrast ratio checks

    preferences/
      contrast.ts         — WCAG relativeLuminance(), contrastRatio(); pure math
      memory-estimate.ts  — lines → bytes → MB string; pure (FS-SB-002)
      shortcuts.ts        — conflict detection; key combo normalization;
                            isRecordingShortcut: boolean (reactive export — see §11.3)

  components/
    terminal/
      TerminalPane.svelte         — primary container; composes all sub-components;
                                   ResizeObserver → resize_pane; keydown capture
      TerminalPane.svelte.ts      — composable: IPC subscriptions, ScreenGrid instance,
                                   selection state, cursor state (see §11.2)
      TerminalViewport.svelte     — scrollable viewport; virtualized rows
      TerminalRow.svelte          — one line: attribute runs → <span> elements
      TerminalCursor.svelte       — 6 DECSCUSR shapes; blink; focused/unfocused outline
      TerminalSelection.svelte    — selection overlay; copy flash; PRIMARY clipboard
      TerminalScrollbar.svelte    — overlay scrollbar with auto-hide (FS-SB-007)
      ScrollToBottom.svelte       — scroll-to-bottom indicator (UXD §8.3)
      SearchOverlay.svelte        — search input, match count, prev/next (FS-SEARCH-007)
      DisconnectBanner.svelte     — SSH Disconnected / Reconnecting states (UXD §7.5.2)
      TerminatedBanner.svelte     — process exited; restart/close actions (FS-PTY-006)
      DeprecatedAlgoBanner.svelte — deprecated SSH algorithm warning (FS-SSH-014)
      ReconnectSeparator.svelte   — reconnection timestamp separator (FS-SSH-042)
      FirstLaunchHint.svelte      — right-click hint, first launch only (FS-UX-002)

    tabs/
      TabBar.svelte
      TabItem.svelte
      TabInlineRename.svelte
      TabActivityIndicator.svelte
      SshBadge.svelte
      TabContextMenu.svelte
      TabScrollArrow.svelte

    layout/
      PaneTree.svelte     — recursive component: leaf → TerminalPane,
                            split → PaneTree + PaneDivider + PaneTree
      PaneDivider.svelte  — 1px visual line, 8px hit area; drag-to-resize;
                            double-click → equal split (UXD §7.2)

    preferences/
      PreferencesPanel.svelte
      sections/
        KeyboardSection.svelte
        AppearanceSection.svelte
        TerminalBehaviorSection.svelte
        ConnectionsSection.svelte
        ThemesSection.svelte
      shared/
        ShortcutRow.svelte
        ShortcutRecorder.svelte     — activates isRecordingShortcut flag (see §11.3)
        ThemeEditor.svelte
        ColorPicker.svelte
        ContrastAdvisory.svelte
        MemoryEstimate.svelte

    connections/
      ConnectionManager.svelte      — slide-in panel; composes List + Form
      ConnectionList.svelte         — grouped list; reusable in Preferences
      ConnectionListItem.svelte     — item: icon, label, hover actions
      ConnectionEditForm.svelte     — create/edit form; client-side path validation

    overlays/
      ConfirmDialog.svelte          — reusable dialog: heading, body, action variant
      HostKeyDialog.svelte          — host key verification (first-connect + key-change)
      ContextMenu.svelte            — base context menu (Bits UI Menu)
```

### 11.2 Composable Layer

The project uses `lib/composables/` for all Svelte 5 composables (`.svelte.ts` files). Each composable encapsulates reactive state and IPC subscriptions for one concern. Complex composables are split into focused sub-composables; the main composable imports them and merges their return values into a single public API.

#### useTerminalPane composables

```
lib/composables/
  useTerminalPane.svelte.ts          — CORE: grid/cursor/mode state, IPC subscriptions,
                                       DOM refs, selection, onMount/onDestroy lifecycle.
                                       Composes all sub-composables below.
  useTerminalPane.cursor-blink.svelte.ts — cursorVisible, blink timer, asymmetric 2:1 cycle
  useTerminalPane.visual-fx.svelte.ts    — bellFlashing, borderPulse, selectionFlashing timers
  useTerminalPane.scrollbar-state.svelte.ts — scrollbarVisible, fade timer, drag state
  useTerminalPane.paste-dialog.svelte.ts — pasteConfirmOpen, pasteConfirmText, handlers
  useTerminalPane.resize.svelte.ts       — ResizeObserver, cellMeasureProbe, sendResize,
                                           font-size $effect
```

#### useTerminalView composables

```
lib/composables/
  useTerminalView.svelte.ts              — re-exports public API; delegates to sub-modules
  useTerminalView.core.svelte.ts         — ViewState interface + createViewState factory:
                                           reactive state bag, $effect timers, focus guard,
                                           initial onMount fetches (session/prefs/connections)
  useTerminalView.lifecycle.svelte.ts    — setupViewListeners(): all IPC event listener setup
                                           (session-state-changed, ssh-state-changed,
                                           host-key-prompt, credential-prompt,
                                           notification-changed, mode-state-changed,
                                           fullscreen-state-changed, window-close-requested)
  useTerminalView.io-handlers.svelte.ts  — createIoHandlers(): SSH CRUD, auth dialogs,
                                           paste, search, keyboard shortcuts, handleGlobalKeydown
```

#### TabBar composables

```
lib/composables/
  useTabBarRename.svelte.ts    — renamingTabId, renameValue, startRename/confirmRename/cancelRename,
                                 $effect for external requestedRenameTabId
  useTabBarDnd.svelte.ts       — dragTabId, dropIndicatorIndex, drag event handlers
  useTabBarContextMenu.svelte.ts — contextMenuTabId, coordinates, open/close/rename handlers
```

**Governance rule:** logic goes in `.svelte.ts` composable files; template and DOM bindings stay in `.svelte` files. When a composable exceeds ~600 lines, extract cohesive sub-concerns into named sub-composables following the pattern above. All sub-composables accept getter functions (not raw values) for Svelte 5 reactivity across module boundaries.

### 11.3 Keyboard Shortcut Interception and Recording

The global `keydown` handler in `+layout.svelte` intercepts application shortcuts (FS-KBD-001). `ShortcutRecorder.svelte` (inside the Preferences panel) also needs to capture keyboard input to record new shortcuts — if the global handler fires first, the shortcut recorder cannot receive the keys it needs to record.

**Decision:** `lib/preferences/shortcuts.ts` exports a reactive flag:

```typescript
// shortcuts.ts
export let isRecordingShortcut = $state(false);
```

The global handler in `+layout.svelte` gates all shortcut interception on `!isRecordingShortcut`:

```typescript
// +layout.svelte (keydown handler)
if (isRecordingShortcut) return; // pass all keys through to ShortcutRecorder
// ... normal shortcut dispatch
```

`ShortcutRecorder.svelte` sets `isRecordingShortcut = true` when it enters recording mode (focus or explicit activation) and resets it to `false` on Enter, Escape, or blur. The flag is the single coordination point between the global interceptor and any component that legitimately needs to capture all keyboard input.

**Generalization:** any future component that needs to capture keyboard input unconditionally (e.g., a search input inside the terminal, a modal text field that must receive Escape) follows the same pattern: import and set `isRecordingShortcut` for the duration of its capture window. The name reflects the original use case but the mechanism is general.

### 11.4 Focus Management

Terminal emulators depend on reliable keyboard focus — if the active viewport loses focus, all keyboard input is silently dropped. The frontend uses a two-tier architecture: prevention at source (tier 1) and a safety-net focus guard (tier 2).

#### 11.4.1 Tier 1 — Prevention at source

Toolbar buttons that do not need keyboard input themselves use `onmousedown={(e) => e.preventDefault()}` in their Svelte template. This prevents the browser from transferring DOM focus to the button element on mouse click; the button's `onclick` handler still fires normally, and keyboard activation via Tab + Enter/Space is unaffected.

Components using this pattern:

- `ScrollToBottomButton.svelte`
- SSH toggle in `TerminalView.svelte`
- Fullscreen button in `TerminalView.svelte`
- Scroll arrows in `TabBarScroll.svelte`

**Exception — `TabBarItem.svelte`:** tabs carry `draggable="true"`. On WebKitGTK, `onmousedown.preventDefault()` interferes with drag-and-drop initiation, so tabs are excluded from this pattern. The focus guard (tier 2) covers the gaps left by this exception.

#### 11.4.2 Tier 2 — Focus guard (safety net)

Location: `useTerminalView.core.svelte.ts`, function `onFocusIn`, registered in `onMount` and removed in `onDestroy`.

```
document.addEventListener('focusin', onFocusIn, { capture: true })
```

The guard fires whenever a `focusin` event bubbles. Its logic:

1. Check that `e.target === document.body` — focus has fallen to the document root with no real target.
2. Check that no `[role="dialog"][aria-modal="true"]` element exists in the document — a modal is open and owns focus intentionally.
3. Check that `activeViewportEl` is currently in the document.
4. If all three conditions hold, call `activeViewportEl.focus({ preventScroll: true })`.

This catches cases where tier 1 is absent (tabs) or insufficient (elements that disappear from the DOM after an action, such as the new-tab button before an `$effect` fires, or a tab close button when the tab it belongs to is being removed).

#### 11.4.3 Active viewport registration

`ViewState` in `useTerminalView.core.svelte.ts` holds `activeViewportEl: HTMLElement | null` as `$state`. This reference is kept current through a callback chain:

- `TerminalPane.svelte` declares an `onviewportactive?: (el: HTMLElement | null) => void` callback prop.
- When a pane is active and `viewportEl` is available, the pane calls `onviewportactive(viewportEl)`; the cleanup path calls `onviewportactive(null)`.
- The prop is threaded down: `TerminalView.svelte` → `SplitPane.svelte` → `TerminalPane.svelte`.
- `TerminalView.svelte` wires it: `onviewportactive={(el) => { tv.activeViewportEl = el; }}`.

`useTerminalPane.svelte.ts` also contains an existing `$effect` that calls `viewportEl?.focus({ preventScroll: true })` when `props.active()` transitions to `true`. This handles the normal pane-activation flow (user clicking a pane, backend reassigning `active_pane_id`). The guard in tier 2 covers the edge cases that this `$effect` cannot anticipate.

#### 11.4.4 Explicit focus restoration

**Rule:** every component or handler that releases keyboard focus — by closing a panel, completing an async transition, or ending a modal interaction — must actively call `activeViewportEl?.focus({ preventScroll: true })` (with modal guard) when done. Never rely on focus returning by itself. This mirrors the pattern used by GNOME Terminal (`gtk_widget_grab_focus` after every transition) and iTerm2 (`makeFirstResponder:` after tab switch).

**Timing constraint:** for transitions driven by OS events (e.g. window manager fullscreen confirmation), restoration must happen *after* the transition is stable — not in the `onclick` handler. In TauTerm, the backend emits `fullscreen-state-changed` after a 200 ms WM stabilisation delay; focus is restored inside that event handler, not in the button's `onclick`.

| Flow | Location | Mechanism |
|---|---|---|
| Search overlay close | `useTerminalView.io-handlers.svelte.ts` `handleSearchClose()` | `s.activeViewportEl?.focus({ preventScroll: true })` |
| Tab rename complete | `TerminalView.svelte` `onRenameComplete` prop | `tv.activeViewportEl?.focus()` with modal guard |
| Tab bar Escape | `TabBar.svelte` `handleTabKeydown` | `onEscapeTabBar?.()` callback |
| Tab bar printable key | `TabBar.svelte` `handleTabKeydown` catch-all | `onEscapeTabBar?.()` — tab bar is a transient navigation surface, not a permanent focus owner |
| SSH panel close | `TerminalView.svelte` `ConnectionManager` `onclose` prop | `tv.activeViewportEl?.focus()` with modal guard |
| Preferences panel close | `TerminalView.svelte` `PreferencesPanel` `onclose` prop | `tv.activeViewportEl?.focus()` — no modal guard needed (modal already dismissed) |
| Fullscreen toggle | `useTerminalView.core.svelte.ts` `onFullscreenStateChanged` handler | `bag.activeViewportEl?.focus()` — deferred to post-WM-stabilisation event |
| Bits UI dialogs | Bits UI `FocusScope` | Automatic restoration to the element that opened the dialog |
| Pane split | Backend sets `active_pane_id` to the new pane | `$effect` in `useTerminalPane` fires |
| Pane close | Backend reassigns `active_pane_id` | `$effect` in `useTerminalPane` fires |

#### 11.4.5 Multi-pane rule

In a split layout, `activeViewportEl` always points to the viewport of the pane identified by `activePaneId` in the current `TabState`. There is no separate "last focused pane" tracking — `activePaneId` is the single source of truth, and `activeViewportEl` is a derived reference to the DOM node that corresponds to it.
