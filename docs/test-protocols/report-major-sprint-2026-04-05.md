<!-- SPDX-License-Identifier: MPL-2.0 -->

# Test Report — Major Sprint
**Date:** 2026-04-05
**Sprint:** TODO.md §Majeurs — Fonctionnalités absentes + Raccourcis clavier + Préférences non câblées (14 items)

---

## Summary

| Category | Result |
|---|---|
| Rust tests (nextest) | **240 / 240 passed**, 2 skipped |
| Frontend tests (vitest) | **595 / 595 passed**, 42 todo |
| TypeScript/Svelte check (`pnpm check`) | **0 errors, 0 warnings** |
| Rust clippy `-D warnings` | **clean** |
| Rust fmt (`cargo fmt --check`) | **clean** (1 formatting issue fixed in `session/registry.rs`) |
| Frontend prettier (`pnpm prettier --check src/`) | **clean** (29 files reformatted) |

All checks pass. No regressions introduced versus the previous sprint report (`report-blocking-ipc-2026-04-05.md`, 235 Rust tests, 522 vitest tests).

---

## Items Implemented

### 1. IPC type drift Rust ↔ TypeScript (ARCHITECTURE 4.6) — #3

- `src/lib/ipc/types.ts` — `Preferences` TypeScript fully aligned with the Rust schema: `keyboard: KeyboardPrefs`, `themes: UserTheme[]`, `TerminalPrefs.bell: BellType` (enum), `UserTheme` struct with all colour fields
- `Language`, `BellType`, `CursorStyle`, `UserTheme` enums/types are now the single source of truth shared by all frontend consumers
- Tests: `src/lib/ipc/types.test.ts` — round-trip serialisation, enum variant coverage

### 2. Login shell premier tab (FS-PTY-013) — #4

- `TabBar.svelte` / `TerminalView.svelte` — first tab `create_tab` call includes `login: true`; subsequent splits pass `login: false`
- Rust `CreateTabConfig` — `login` field with `#[serde(default)]`, forwarded to `open_session` as `-l` flag
- Tests: `session/registry.rs` — `test_sprint_001_*` (4 tests): default false, login true round-trip, arg selection

### 3. Primary selection X11 (FS-CLIP-004) — #5

- `platform/clipboard_linux.rs` — `set_primary()` method writing to `LinuxClipboardKind::Primary` via `arboard`
- `ClipboardBackend` trait extended with `set_primary(&self, text: &str) -> Result<(), ClipboardError>`
- `SelectionManager` in the frontend calls `write_primary_selection` IPC command on mouse-up after text selection
- Tests: `platform/clipboard_linux/tests::test_sprint_002b_set_primary_is_part_of_clipboard_backend_trait`

### 4. Close confirmation dialog (FS-PTY-008) — #6

- `TerminalView.svelte` — `close_tab` / `close_pane` check `get_pane_running_process` before closing; a `Dialog` component shows when a process is active, requiring explicit confirmation
- The previously commented TODO at `TerminalView.svelte:157` is now resolved
- Tests: `src/lib/components/__tests__/TerminalPane.test.ts` — dialog shown / skipped based on process state

### 5. Tab inline rename (FS-TAB-006) — #7

- `TabBar.svelte` — double-click and F2 on a tab enter inline rename mode (input replaces label); Escape cancels, Enter confirms via `invoke('rename_tab')`
- Context menu "Rename" entry wired to the same inline input
- Tests: `src/lib/components/__tests__/TabBarRename.test.ts` — 8 scenarios (double-click, F2, Escape, Enter, context menu, max-length enforcement)

### 6. Tab drag-and-drop (FS-TAB-005) — #8

- `TabBar.svelte` — HTML5 drag events (`ondragstart`, `ondragover`, `ondrop`) implemented; drop triggers `invoke('reorder_tab', { tabId, newIndex })`
- Visual drag indicator on dragover; tab order reflects backend state after reorder event
- Tests: `src/lib/components/TabBar.test.ts` — drag-start, drop valid index, drop same index (noop)

### 7. Double-click word select / triple-click line select (FS-CLIP-002, FS-CLIP-003) — #9

- `SelectionManager` — `selectWord(col, row)` using `word_delimiters` from preferences, `selectLine(row)` for full line
- `TerminalPane.svelte` — click counter logic; double-click dispatches `selectWord`, triple-click dispatches `selectLine`
- Tests: `src/lib/terminal/selection.test.ts` — word boundaries with default delimiters, custom delimiters, line select

### 8. Pane shortcuts (FS-KBD-003) — #10

- `TerminalView.svelte` — `handleGlobalKeydown` now intercepts: Ctrl+Shift+D (split horizontal), Ctrl+Shift+E (split vertical), Ctrl+Shift+Q (close pane), Ctrl+Shift+Arrow (navigate panes), Ctrl+Tab / Ctrl+Shift+Tab (cycle tabs), F2 (rename active tab)
- Shortcuts fire `invoke()` commands; PTY does not receive these key events
- Tests: `src/lib/components/__tests__/shortcuts.test.ts` — 12 scenarios, one per shortcut + edge cases

### 9. Keyboard shortcuts persisted (FS-KBD-002) — #11

- `KeyboardShortcutRecorder.svelte` — `onchange` handler calls `invoke('update_preferences', { patch: { keyboard: … } })`
- `TerminalView.svelte` — on `load_preferences`, `handleGlobalKeydown` reads live shortcut config instead of hardcoded values
- Tests: `src/lib/components/__tests__/KeyboardShortcutRecorder.test.ts` — record, conflict detection, save invoked, load applied

### 10. Terminal preferences wired (FS-PREF-003, FS-PREF-006) — #12

- `PreferencesPanel.svelte` — cursor shape, bell type, cursor blink rate dropdowns now read from `prefs.terminal` and call `invoke('update_preferences')` on change
- No more hardcoded values; each dropdown is a controlled component
- Tests: `src/lib/components/__tests__/PreferencesPanel.test.ts` — cursor shape change, bell type change, blink rate change (3 scenarios)

### 11. ConnectionManager mounted in TerminalView (FS-SSH-031, FS-SSH-032) — #13

- `TerminalView.svelte` — `<ConnectionManager>` now mounted in the sidebar; toggled via `showConnections` state
- SSH saved connections are accessible from the UI; `open_ssh_connection` IPC path is reachable end-to-end
- Tests: `src/lib/components/__tests__/ConnectionManager.test.ts` — mount, open, close, connection selected

### 12. Split layout arborescent (FS-PANE-001, FS-PANE-003) — #14

- `SplitPane.svelte` — recursive component consuming `SplitNode` from `split-tree.ts`; draggable separator changes split ratio via `PointerEvent` tracking
- `TerminalView.svelte` — replaces flat flex layout with `<SplitPane>` tree driven by `splitRoot: SplitNode`
- Rust `split_pane` / `close_pane` commands update the in-memory tree; `split-tree-changed` event triggers re-render
- Tests: `src/lib/layout/split-tree.test.ts` — insert, remove, rebalance; `src/lib/components/__tests__/TerminalPane.test.ts` — render tree with 2 panes

### 13. Theme editor UI (FS-THEME-003 à 006) — #15

- `PreferencesPanel.svelte` — new "Themes" section: list of user themes, Create / Edit / Delete actions
- Edit mode shows colour pickers for all `UserTheme` colour fields; Save calls `invoke('upsert_theme')`; Delete calls `invoke('delete_theme')`
- Preview swatch applied via CSS custom properties
- Tests: `src/lib/theming/validate.test.ts` — theme struct validation; `src/lib/components/__tests__/PreferencesPanel.test.ts` — CRUD scenarios (4 tests)

### 14. Extended test protocols (sprint #16 + #17)

- `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md` — extended with 13 new functional scenarios covering the 14 sprint items
- `docs/test-protocols/security-pty-ipc-ssh-credentials-csp-osc52.md` — extended with 10 new security scenarios (shortcut injection, theme XSS, drag-and-drop CSRF-equivalent, credential dialog)
- Rust nextest: 5 new tests (`test_sprint_001_*` login shell, `test_sprint_002b_*` primary clipboard)
- Vitest: 73 new tests across the 14 feature areas

---

## Skipped tests

| Test | Reason |
|---|---|
| `vt::search::tests::search_soft_wrap_word_spanning_two_rows_is_found` | `#[ignore]` — cross-row soft-wrap search not yet implemented (tracked in TODO.md §Mineurs) |
| `platform::credentials_linux::tests::credential_store_secretservice_round_trip` | `#[ignore]` — requires active D-Bus keyring daemon; blocked on CI environment (tracked in TODO.md §Tests manquants) |

---

## Formatting fixes applied (not feature regressions)

- `src-tauri/src/session/registry.rs:806` — `assert!` call reformatted to multi-line form to satisfy `cargo fmt`
- 29 frontend files reformatted by `prettier --write` (whitespace / trailing comma normalisation introduced by sprint commits)

---

## Technical debt identified during sprint

| Item | Nature | Impact |
|---|---|---|
| Scrollbar non interactive | Pointer-events disabled on `TerminalPane.svelte` scrollbar | Minor UX — FS-SB-007 |
| `file://` scheme rejected by `validate_url_scheme` | Local sessions cannot use file:// URIs | Minor — FS-VT-073 |
| Paste confirmation multiline | No dialog when pasting multi-line text with bracketed paste inactive | Should — FS-CLIP-009 |
| Tab contrast WCAG AA | Inactive tab title ≈ 2.5:1, below 4.5:1 threshold | Accessibility — TUITC-UX-060 |
| FS-SSH-013 erratum | VKILL/VEOF opcodes inverted in `docs/FS.md` | Doc debt — implementation is correct |
| Searchbar cross-row | `search_scrollback` searches line-by-line; soft-wrapped words spanning rows not found | Minor — SEARCH-SOFT-001 |
| Premier lancement context menu hint | Backend ready, frontend not wired | Minor — FS-UX-002 |
| AppImage config | `tauri.conf.json` uses `"targets": "all"` without specific AppImage config | Minor — FS-DIST-001 |
| i18n hardcoded strings | Several ARIA labels not routed through Paraglide | Minor — FS-I18N-001 |

---

## Known remaining gaps (still in TODO.md after this sprint)

The following items were out of scope and remain open:

- Scrollbar interactive (FS-SB-007)
- Context menu hint on first launch (FS-UX-002)
- AppImage packaging (FS-DIST-001 à 006)
- Hardcoded i18n strings in TabBar/TerminalPane/TerminalView (FS-I18N-001)
- `file://` scheme rejected (FS-VT-073)
- Paste confirmation multiline (FS-CLIP-009)
- Tab contrast WCAG AA (TUITC-UX-060)
- FS-SSH-013 doc erratum (VKILL/VEOF)
- Cross-row scrollback search (SEARCH-SOFT-001)
- SecretService integration test (keyring daemon required)
- E2E tests (wdio — requires production build and PTY wiring)

---

## Sign-off

All 240 Rust tests pass (nextest). All 595 frontend tests pass (vitest). TypeScript/Svelte check clean. Clippy clean. Fmt clean. Prettier clean.

The 14 sprint items are considered **test-complete**. No regressions against the previous sprint baseline.
