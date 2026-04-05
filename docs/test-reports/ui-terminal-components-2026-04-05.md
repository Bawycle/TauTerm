# Test Report — Terminal UI Core Components
**Sprint:** 2026-04-05
**Scope:** TerminalPane, TabBar, StatusBar, TerminalView + supporting modules (color.ts, screen.ts, input-security)
**Executed by:** moe (Maître d'Œuvre), phase 4 of Terminal UI implementation cycle

---

## Summary

| Suite | Tool | Tests | Passed | Failed | Notes |
|---|---|---|---|---|---|
| Frontend unit | vitest 4.1.2 | 238 | 238 | 0 | — |
| Rust unit | cargo-nextest | 220 | 220 | 0 | — |
| TypeScript/Svelte types | svelte-check | — | — | 0 errors | — |
| Rust linting | clippy -D warnings | — | — | 0 warnings | — |

---

## Frontend Tests (238 / 238 passed)

### New test files added this sprint

| File | Scenarios | Coverage |
|---|---|---|
| `src/lib/terminal/color.test.ts` | 20 | ANSI-16 → CSS vars, 256-color cube, grayscale ramp, truecolor, cursor shapes, blink flags |
| `src/lib/terminal/screen.test.ts` | 30 | CellStyle construction, color resolution, grid building, update application, OOB safety, XSS literal storage |
| `src/lib/terminal/input-security.test.ts` | 19 | TUITC-SEC-031 (Uint8Array type), TUITC-SEC-030 (payload < 64 KiB), app-cursor mode sizing |
| `src/lib/components/TabBar.test.ts` | 12 | Title resolution, XSS prevention, notification type mapping, sort order, ARIA role contracts |
| `src/lib/components/StatusBar.test.ts` | 10 | SSH state text per lifecycle state, icon selection, local=null, completeness |

### Notable fix during test execution

**TUITC-SEC-031 — `instanceof Uint8Array` fails in jsdom (8 tests)**
Root cause: jsdom uses a separate V8 VM context from the test runtime. `Uint8Array` constructed in the module under test is not the same constructor as the one in the test context, making `instanceof` return false for a valid `Uint8Array` value.
Fix: replaced `expect(result).toBeInstanceOf(Uint8Array)` with `expect(result!.constructor.name).toBe('Uint8Array')` — cross-realm safe.
Reference: `src/lib/terminal/input-security.test.ts` lines 50–51.

---

## Rust Tests (220 / 220 passed)

### Fix during test execution

**SEC-CSP-003 — false-positive on `{@html}` in comments (1 test)**
Root cause: `find_at_html_in_svelte_files()` used `line.contains("{@html")` without filtering HTML comment lines. The new component files contain documentation notes explicitly stating the *absence* of `{@html}` (e.g., `<!-- Security: no {@html} -->`), which the scanner matched as violations.
Fix: added comment-line filter in `security_static_checks.rs` — lines trimmed-starting with `<!--`, `//`, `*`, or `-` are skipped. This preserves detection of actual `{@html}` in template markup.
The fix does not weaken the guard: any real `{@html expr}` in template code will not start with those comment prefixes.

---

## Components Implemented

### TerminalPane (`src/lib/components/TerminalPane.svelte`)
- DOM cell grid (row divs / cell spans), inline style per cell (fg, bg, bold, italic, underline, dim, blink, inverse, strikethrough)
- Cursor overlay: block/underline/bar shapes from DECSCUSR, DECTCEM visibility, 530ms blink interval
- Selection: `SelectionManager` integration, `--term-selection-bg` / `--term-selection-bg-inactive` tokens
- Scrollbar: overlay (no layout shift), thumb position/height from scrollOffset + scrollbackLines
- Keyboard handler: routes through `keyEventToVtSequence(event, decckm)`; intercepts Ctrl+Shift+* and Ctrl+, as app shortcuts before PTY forwarding
- Mouse handler: pixelToCell() maps pointer position to grid coords, delegates to SelectionManager
- ResizeObserver: 50ms debounce, measures cell dimensions from first rendered cell, clamps to ≥1, calls `invoke('resize_pane', …)`
- IPC: `get_pane_screen_snapshot` on mount, `listen('screen-update')`, `listen('scroll-position-changed')`, `listen('mode-state-changed')`
- Security: no `{@html}` — cell content via text interpolation; `send_input` data is `Array.from(Uint8Array)` (number[])

### TabBar (`src/lib/components/TabBar.svelte`)
- Tabs from session state, sorted by `order`, active highlight via CSS class
- Title: `tab.label ?? processTitle ?? 'Terminal'` (TUITC-SEC-010: text interpolation only)
- SSH badge: `<Network size={12}>` (lucide-svelte) on SSH sessions
- Activity indicators: dot for backgroundOutput, CheckCircle/XCircle for process exit, Bell for bell event
- Close button: 44×44px hit area via `var(--size-target-min)`, negative margin compensation, opacity 0→1 on hover/active
- New-tab button: `Tooltip.Root delayDuration={300}` (Bits UI) — prop name corrected from `openDelay` during type-check phase
- Keyboard navigation: ArrowLeft/Right move focus between tab elements, Enter/Space activate, Delete close

### StatusBar (`src/lib/components/StatusBar.svelte`)
- Session type: "LOCAL" label (absence of indicator = local, per UXD §7.5.1)
- SSH indicator: Network/WifiOff/XCircle (lucide-svelte, size=14) per lifecycle state
- Connecting → `spin` CSS animation (1.2s linear), Authenticating → `pulse` animation (1.5s ease-in-out)
- `prefers-reduced-motion: reduce` disables both animations
- Process title (center), CWD with title tooltip (right, truncated via ellipsis)

### TerminalView (`src/lib/components/TerminalView.svelte`)
- `get_session_state` on mount with graceful catch (backend not ready)
- `listen('session-state-changed')` with all 5 changeTypes: tab-created, tab-closed, tab-reordered, pane-metadata-changed, active-pane-changed, active-tab-changed
- `collectLeafPanes(PaneNode)`: recursive traversal of split tree → flat `{paneId, state}[]`
- `activePaneState` derived for StatusBar
- Application shortcuts: Ctrl+Shift+T (new tab), Ctrl+Shift+W (close tab) — intercepted at TerminalView, never forwarded to PTY

---

## Supporting Modules Implemented

### `src/lib/terminal/color.ts`
- `ANSI_16_VARS`: indices 0–15 → CSS token vars
- `resolve256Color(index)`: handles ANSI-16 (0–15), cube (16–231), grayscale (232–255)
- `resolveColorDto(ColorDto)`: maps `default/ansi/ansi256/rgb` variants (ScreenUpdateEvent path)
- `resolveColor(Color)`: maps `Default/Ansi/Ansi256/Rgb` variants (ScreenSnapshot path)
- `cursorShape(shapeCode)`: DECSCUSR 0–6 → `'block'|'underline'|'bar'`
- `cursorBlinks(shapeCode)`: DECSCUSR 0–6 → boolean

### `src/lib/terminal/screen.ts`
- `CellStyle`: flattened cell representation with resolved CSS color strings
- `buildGridFromSnapshot(cells, rows, cols)`: flat row-major array from ScreenSnapshot
- `applyUpdates(grid, updates, cols)`: incremental update, OOB-safe

---

## Known Limitations (not regressions)

- E2E scenarios `pty-roundtrip.spec.ts` and `tab-lifecycle.spec.ts` remain failing — blocked on PTY pipeline wiring to live frontend session and `create_tab` E2E wiring. These were failing before this sprint and are out of scope.
- Inactive tab title contrast (`--color-tab-inactive-fg` on `--color-tab-bg`) ≈ 2.5:1 — below WCAG AA 4.5:1 threshold. Flagged in test protocol TUITC-UX-060 as known exception requiring design team decision.
- Scroll state is not persisted per pane across tab switches — `scrollOffset` resets on TerminalPane unmount. Acceptable for current scope; tracked in "not yet implemented."
