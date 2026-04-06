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

### 11.2 TerminalPane Component Split

`TerminalPane.svelte` is the most complex component in the application. To prevent it from becoming an unmaintainable monolith, the following governance rule applies:

**If `TerminalPane.svelte` exceeds 250 lines, extract reactive logic to `TerminalPane.svelte.ts`.**

`TerminalPane.svelte.ts` exports composables (functions returning reactive state):
- IPC event subscription management (`$effect` with cleanup)
- `ScreenGrid` instance and diff application
- Selection state machine integration
- Cursor blink timer state

`TerminalPane.svelte` retains only: the component template markup, event handler binding (`on:keydown`, `on:mousedown`, etc.), and calls to the composable functions. This separation keeps the template readable and the logic testable in isolation.

This pattern applies to any component that grows beyond 250 lines. The rule is: **logic goes in the `.svelte.ts` composable file; template and DOM bindings stay in the `.svelte` file.**

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
