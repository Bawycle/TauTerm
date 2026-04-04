# Functional Test Report — 2026-04-04-b

> **Author:** test-engineer / rust-dev / frontend-dev
> **Protocol:** `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`
> **Date:** 2026-04-04
> **Follows:** `report-functional-2026-04-04.md` (session a)

---

## Executive Summary

Second execution of the functional test protocol, implementing the action plan from session-a. All P0 security fixes and P1 foundation work have been completed.

**Rust (nextest):** 167 tests — 167 PASS, 0 FAIL, 0 SKIP
**Frontend (vitest):** 110 tests — 110 PASS, 0 FAIL, 0 SKIP

Net additions this session:
- **+13 Rust tests** (was 154, now 167): path validation, send_input size limit, PreferencesStore::load_or_default, test renames
- **+86 frontend tests** (was 24, now 110): 4 new modules with full test coverage

TEST-I18N-004 is now **PASS** (was `#[ignore]` — `load_or_default()` is implemented).

---

## Test Execution Results

### Rust (nextest)

```
Summary [0.106s] 167 tests run: 167 passed, 0 skipped
```

**New tests added by module:**

| Module | Tests added | Notes |
|---|---|---|
| `commands::input_cmds` | +3 | SEC-IPC-006: size limit at boundary, oversized rejected, empty accepted |
| `platform::validation` | +8 | validate_ssh_identity_path (5 cases), validate_shell_path (4 cases) |
| `preferences::schema` | +1 | TEST-I18N-004 — `i18n_004_preferences_store_falls_back_to_en_for_unknown_language` (was `#[ignore]`, now PASS) |

**Tests renamed:**

| Old name | New name | Reason |
|---|---|---|
| `sec_cred_003_private_key_path_visible_in_debug` | `sec_cred_003_private_key_path_redacted_in_debug` | FINDING-001 resolved — path is now redacted |

### Frontend (vitest)

```
Test Files  6 passed (6)
Tests       110 passed (110)
Duration    1.17s
```

**New test files created:**

| File | Tests | Scenarios covered |
|---|---|---|
| `lib/terminal/keyboard.test.ts` | 42 | TEST-KBD-001–004 + Backspace, Enter, Tab, Alt/Meta combos |
| `lib/terminal/selection.test.ts` | 14 | TEST-CLIP-001–003 + wide chars, reversed selection, multi-line |
| `lib/layout/split-tree.test.ts` | 11 | Single/horizontal/vertical/nested splits, findPane, leafPanes |
| `lib/theming/validate.test.ts` | 19 | TEST-THEME-003: valid theme, missing tokens, invalid colors, null input |

---

## Coverage Matrix (changes from session-a)

| TEST-ID | Layer | Status | Change |
|---|---|---|---|
| TEST-I18N-004 | Unit (Rust) | **PASS** | Was SKIP (`#[ignore]`) — `load_or_default()` implemented |
| TEST-KBD-001 | Frontend Unit | **PASS** | Was BLOCKED — `lib/terminal/keyboard.ts` created |
| TEST-KBD-002 | Frontend Unit | **PASS** | Was BLOCKED |
| TEST-KBD-003 | Frontend Unit | **PASS** | Was BLOCKED |
| TEST-KBD-004 | Frontend Unit | **PASS** | Was BLOCKED |
| TEST-CLIP-001 | Frontend Unit | **PASS** | Was BLOCKED — `lib/terminal/selection.ts` created |
| TEST-CLIP-002 | Frontend Unit | **PASS** | Was BLOCKED |
| TEST-CLIP-003 | Frontend Unit | **PASS** | Was BLOCKED |
| TEST-THEME-003 | Frontend Unit | **PASS** | Was BLOCKED — `lib/theming/validate.ts` created |
| TEST-PTY-009 | Unit (Rust) | **PASS** | Was BLOCKED — `validate_ssh_identity_path()` implemented |
| TEST-PREF-001 | Integration | BLOCKED | `save()` not yet implemented |
| TEST-PREF-002 | Integration | BLOCKED | `load_or_default()` implemented but `save()` still missing |

All previously PASS tests remain PASS. No regression.

---

## New Files Created

### Rust

- `src-tauri/src/platform/validation.rs` — `validate_ssh_identity_path()`, `validate_shell_path()` with 9 unit tests
- `src-tauri/src/preferences/store.rs` — added `load_or_default()` method

### Frontend

- `src/lib/terminal/keyboard.ts` — VT sequence mapping for keyboard input
- `src/lib/terminal/keyboard.test.ts`
- `src/lib/terminal/selection.ts` — cell-based selection state management
- `src/lib/terminal/selection.test.ts`
- `src/lib/layout/split-tree.ts` — pane tree mirroring IPC PaneNode contract
- `src/lib/layout/split-tree.test.ts`
- `src/lib/theming/validate.ts` — design token validation
- `src/lib/theming/validate.test.ts`

---

## Implementation Notes

### keyboard.ts — Uint8Array comparison in vitest (jsdom)

`TextEncoder().encode(s)` and `new Uint8Array([...])` are structurally equal under `toEqual` for multi-byte sequences. For control character comparisons, `Array.from()` conversion is used in assertions to avoid jsdom environment-specific Uint8Array identity issues. This is a test implementation detail only — the production code is correct.

### validate.ts — hsl() not accepted by design

`hsl()` is intentionally excluded from valid color formats. The Umbra design system uses `#rrggbb`, `rgb()`, and `oklch()` exclusively (per `docs/AD.md` and `src/app.css`). Accepting `hsl()` would create inconsistencies with the token system. If this changes, one regex pattern addition in `COLOR_PATTERNS` suffices.

### selection.ts — cols parameter

`SelectionManager.getSelectedText(getCell, cols)` takes a `cols` parameter to delimit complete lines. This will be supplied by `TerminalPane` at integration time.

### preferences store — XDG_CONFIG_HOME in tests

TEST-I18N-004 uses `std::env::set_var("XDG_CONFIG_HOME", ...)` inside `unsafe {}` blocks (required by Rust edition 2024) to redirect the preferences file path during tests. The test is not `#[ignore]` anymore and runs cleanly.

---

## Remaining Blocked Tests (unchanged from session-a)

All PTY, SSH, E2E, and integration tests that were blocked in session-a remain blocked for the same reasons (stubs not yet implemented). See `report-functional-2026-04-04.md` §Blocked Tests for the full list.

Notable still-blocked:
- TEST-PREF-001/002: `PreferencesStore::save()` not yet implemented
- All TEST-PTY-*: `LinuxPtySession` not yet implemented
- All E2E: requires `pnpm tauri build` + tauri-driver

---

## Next Steps

1. **Implement `PreferencesStore::save()`** — unblocks TEST-PREF-001/002, SEC-PATH-005
2. **Implement `LinuxPtySession`** — unblocks TEST-PTY-001/002/003, TEST-VT-001, and the full PTY pipeline
3. **Wire PTY → VtProcessor → screen-update event pipeline** — unblocks TEST-VT-013/016/017
4. **Implement `VtProcessor::search()`** — unblocks TEST-VT-021/022
5. **Add `EventEmitter` trait to `SessionRegistry`** — prerequisite for TEST-IPC-001/002/003 and the pipeline wiring
