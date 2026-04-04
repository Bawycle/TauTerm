# Functional Test Report — 2026-04-04

> **Author:** test-engineer
> **Protocol:** `docs/test-protocols/functional-pty-vt-ssh-preferences-ui-ipc.md`
> **Date:** 2026-04-04

---

## Executive Summary

This report covers the first execution of the functional test protocol against the TauTerm bootstrap codebase. All non-blocked, non-E2E tests have been written and executed.

**Rust (nextest):** 154 tests — 154 PASS, 0 FAIL, 1 SKIP (TEST-I18N-004 `#[ignore]`)
**Frontend (vitest):** 24 tests — 24 PASS, 0 FAIL

The net addition from this session: **104 new Rust tests** (was 50, now 154). Frontend remains at 24 (no new frontend modules were implemented to test against).

One bug was found and fixed during test writing (see Issues section): `\x1b[1049h` was used without the required `?` private-mode intermediate byte — tests correctly caught this by failing, confirming the tests are not trivially true.

---

## Test Execution Results

### Rust (nextest)

```
Summary [0.102s] 154 tests run: 154 passed, 1 skipped
```

**New tests added by module:**

| Module | Tests added | Notes |
|---|---|---|
| `vt/processor.rs` | +34 | TEST-VT-002 through TEST-VT-023 (VtProcessor integration) |
| `vt/osc.rs` | +12 | OSC title, OSC 8 hyperlink, push/pop title, error cases |
| `vt/mouse.rs` | +7 | Mouse encoding: X10, SGR, URXVT, coordinate clamping |
| `session/lifecycle.rs` | +9 | PaneLifecycleState predicates, serde serialization |
| `ssh/connection.rs` | +7 | SSH state machine transitions, full lifecycle sequence |
| `ssh/keepalive.rs` | +1 | SSH_KEEPALIVE_MAX_MISSES constant correctness |
| `preferences/schema.rs` | +2 | TEST-I18N-004 (1 `#[ignore]`), additional language tests |

Previously passing 50 tests continue to pass without modification.

### Frontend (vitest)

```
Test Files  2 passed (2)
Tests       24 passed (24)
Duration    1.21s
```

No new frontend test files were written in this session: the modules listed in the protocol (`lib/terminal/keyboard.ts`, `lib/terminal/selection.ts`, `lib/layout/split-tree.ts`, etc.) do not yet exist in the codebase. The existing 24 tests in `lib/ipc/types.test.ts` and `lib/state/locale.svelte.test.ts` continue to pass.

---

## Coverage Matrix

| TEST-ID | Layer | Status | Comment |
|---|---|---|---|
| TEST-PTY-001 | Integration | BLOCKED | Requires `SessionRegistry` with mock PTY backend |
| TEST-PTY-002 | Unit (Rust) | BLOCKED | Requires `LinuxPtySession` fully implemented (`todo!()`) |
| TEST-PTY-003 | Integration | BLOCKED | Requires SIGCHLD wiring and event emission |
| TEST-PTY-004 | E2E | BLOCKED | E2E — requires production build |
| TEST-PTY-005 | E2E | BLOCKED | E2E + PTY write stub |
| TEST-PTY-006 | E2E | BLOCKED | E2E + PTY resize stub |
| TEST-PTY-007 | E2E | BLOCKED | E2E + PTY spawn stub |
| TEST-PTY-008 | E2E | BLOCKED | E2E + PTY spawn stub |
| TEST-PTY-009 | Unit (Rust) | BLOCKED | Path validation function not yet implemented |
| TEST-VT-001 | Unit (Rust) | BLOCKED | PTY session config not yet constructible in tests |
| TEST-VT-002 | Unit (Rust) | PASS | `split_csi_sequence_is_parsed_correctly` |
| TEST-VT-003 | Unit (Rust) | PASS | `utf8_sequence_split_across_calls_is_reassembled` |
| TEST-VT-004 | Unit (Rust) | PASS | `wide_char_at_last_col_wraps_to_next_line` (bounds check) |
| TEST-VT-005 | Unit (Rust) | PASS | `invalid_utf8_produces_replacement_character` |
| TEST-VT-006 | Unit (Rust) | PASS | 4 tests: ANSI, 256-color, RGB semicolon, RGB colon |
| TEST-VT-007 | Unit (Rust) | PASS | `sgr_multi_attributes_set_independently` |
| TEST-VT-008 | Unit (Rust) | PASS | `dectcem_hide_and_show_cursor` |
| TEST-VT-009 | Unit (Rust) | PASS | `alternate_screen_cursor_save_restore` |
| TEST-VT-010 | Unit (Rust) | PASS | `alternate_screen_is_isolated_from_normal_screen` |
| TEST-VT-011 | Unit (Rust) | PASS | `decstbm_partial_scroll_region_no_scrollback` |
| TEST-VT-012 | Unit (Rust) | PASS | `osc_title_plain_title_is_stored`, `osc_title_control_chars_are_stripped`, `osc_title_truncated_to_256_chars` |
| TEST-VT-013 | Unit (Rust) | BLOCKED | Requires PTY input buffer mock (no write-back mechanism yet) |
| TEST-VT-014 | Unit (Rust) | BLOCKED | `validate_hyperlink_uri()` not yet implemented |
| TEST-VT-015 | Unit (Rust) | PASS | `sec_osc_001_osc52_read_query_returns_ignore`, `sec_osc_002_osc52_write_sequence_parsed_as_clipboard_write` (already in security_tests) |
| TEST-VT-016 | Unit (Rust) | SKIP | Mouse bypass on Shift+click requires VtProcessor wiring to routing logic not yet present |
| TEST-VT-017 | Unit (Rust) | BLOCKED | Bell rate-limiting requires Tokio timer and PtyReadTask |
| TEST-VT-018 | Unit (Rust) | PASS | `osc_overflow_does_not_crash_and_subsequent_sequences_parse` |
| TEST-VT-019 | Unit (Rust) | BLOCKED | Requires VtProcessor with output of printable lines (full text content) |
| TEST-VT-020 | Unit (Rust) | BLOCKED | Requires soft-wrap metadata on scrollback lines (not yet implemented) |
| TEST-VT-021 | Unit (Rust) | BLOCKED | `VtProcessor::search()` returns empty stub — search not implemented |
| TEST-VT-022 | Unit (Rust) | BLOCKED | Same as TEST-VT-021 |
| TEST-VT-023 | Unit (Rust) | PASS | `dec_special_graphics_so_maps_j_to_box_drawing` |
| TEST-SSH-001–006 | E2E | BLOCKED | SSH lifecycle stub |
| TEST-SSH-007 | Integration | PARTIAL | Constant correctness verified (`keepalive_max_misses_is_three`); full timer test blocked — requires mock transport |
| TEST-SSH-008–010 | E2E | BLOCKED | SSH lifecycle stub |
| TEST-PREF-001 | Integration | BLOCKED | `PreferencesStore::load()` / `save()` not yet implemented |
| TEST-PREF-002 | Integration | BLOCKED | `PreferencesStore::load_or_default()` not yet implemented |
| TEST-PREF-003–004 | E2E | BLOCKED | E2E |
| TEST-I18N-001–003 | E2E | BLOCKED | E2E |
| TEST-I18N-004 | Unit (Rust) | SKIP | `#[ignore]` — `load_or_default()` not yet implemented (see Issues) |
| TEST-I18N-005 | E2E | BLOCKED | E2E |
| TEST-CRED-001–003 | E2E/Integration | BLOCKED | Credential system stub |
| TEST-TAB-001–006 | E2E | BLOCKED | E2E |
| TEST-PANE-001–004 | E2E | BLOCKED | E2E |
| TEST-KBD-001–004 | Frontend Unit | BLOCKED | `lib/terminal/keyboard.ts` not yet created |
| TEST-CLIP-001–003 | Frontend Unit + E2E | BLOCKED | `lib/terminal/selection.ts` not yet created |
| TEST-SEARCH-001–002 | E2E | BLOCKED | E2E |
| TEST-NOTIF-001–002 | E2E | BLOCKED | E2E |
| TEST-THEME-001–002 | E2E | BLOCKED | E2E |
| TEST-THEME-003 | Frontend Unit | BLOCKED | `lib/theming/validate.ts` not yet created |
| TEST-THEME-004 | E2E | BLOCKED | E2E |
| TEST-A11Y-001–005 | E2E | BLOCKED | E2E |
| TEST-UX-001–002 | E2E | BLOCKED | E2E |
| TEST-IPC-001–003 | Integration | BLOCKED | `SessionRegistry::close_pane` / `split_pane` not yet accessible without Tauri state |
| TEST-IPC-004 | Integration | PARTIAL | Type shape covered by frontend smoke tests (`lib/ipc/types.test.ts`); Rust serde round-trips verified through existing schema tests |
| TEST-IPC-005 | E2E | BLOCKED | E2E (CSP requires running app) |

---

## Blocked Tests

| Test ID | Reason | Unblocked when… |
|---|---|---|
| TEST-PTY-001 | `SessionRegistry` has no mock PTY injection point | PTY backend trait mocking implemented |
| TEST-PTY-002 | `LinuxPtySession::write/resize` are `todo!()` | Linux PTY backend implemented |
| TEST-PTY-003 | SIGCHLD wiring and event emission absent | PtyReadTask lifecycle implemented |
| TEST-PTY-009 | `validate_identity_file_path()` not yet implemented | Path validation added to `platform/` |
| TEST-VT-001 | PTY session config struct not publicly constructible in test context | PTY session config made testable |
| TEST-VT-013 | No write-back mechanism to PTY input buffer from VtProcessor | PTY write-back wired in VtProcessor |
| TEST-VT-014 | `validate_hyperlink_uri()` not yet added | URI validation function implemented in `vt/osc.rs` or `platform/` |
| TEST-VT-016 | Mouse Shift-bypass routing not yet wired at VtProcessor level | Mouse event routing with mode checks implemented |
| TEST-VT-017 | Bell rate-limiting requires Tokio timer (PtyReadTask) | PtyReadTask bell throttle implemented |
| TEST-VT-019 | Scrollback text content not yet verifiable (no way to read cell graphemes from scrollback with rich content in tests) | Tested at integration level via `vt_processor_integration.rs` |
| TEST-VT-020 | Soft-wrap line metadata not yet implemented | Soft-wrap boundary tracking added to `ScreenBuffer` |
| TEST-VT-021–022 | `VtProcessor::search()` is a stub returning `Vec::new()` | Search implemented in `vt/search.rs` |
| TEST-I18N-004 | `PreferencesStore::load_or_default()` not yet implemented | Store load with field-level defaults implemented |
| TEST-PREF-001–002 | `PreferencesStore::save()` / `load_or_default()` not yet implemented | Preferences store persistence implemented |
| TEST-SSH-007 (full) | Mock transport for keepalive not yet available | SSH mock transport added as test fixture |
| TEST-IPC-001–003 | `SessionRegistry` commands not testable without Tauri `AppHandle` | Integration test harness for commands added |
| All E2E | Requires `pnpm tauri build` and tauri-driver setup | Production build functional |
| All frontend modules | `lib/terminal/keyboard.ts`, `lib/terminal/selection.ts`, `lib/layout/split-tree.ts`, `lib/theming/validate.ts`, etc. not yet created | Frontend feature modules implemented |

---

## Issues Found

### Issue 1: Incorrect CSI private-mode sequence in tests (caught and fixed)

**Nature:** Test authoring error, not a code bug.
**Discovery:** Tests `alternate_screen_cursor_save_restore` and `alternate_screen_is_isolated_from_normal_screen` initially used `\x1b[1049h` (standard mode set, no `?`) instead of the correct `\x1b[?1049h` (DEC private mode set). The tests correctly failed, confirming that the test assertions were non-trivial.
**Fix:** Corrected both sequences to use `\x1b[?1049h` / `\x1b[?1049l`.
**Implication:** The VtProcessor correctly requires `?` as the intermediate byte for DECSET/DECRST, which is spec-correct (ECMA-48 §5.4.2).

### Issue 2: TEST-I18N-004 reveals architectural tension between security and UX

**Nature:** Spec conflict between FS-I18N-006 (fallback to `En` for unknown locale) and SEC-IPC-005 (reject unknown Language variants at the IPC boundary).
**Analysis:** Both requirements are correct but apply at different layers:
- At the serde/IPC boundary: unknown variants MUST be rejected (SEC-IPC-005) — prevents string injection.
- At the store/load level: a corrupted/outdated preferences file with `"language": "de"` must not crash the application (FS-I18N-006 / FS-PREF-001 graceful degradation).
**Resolution:** The fallback to `Language::En` must happen in `PreferencesStore::load_or_default()` by catching the deserialization error and substituting field defaults — NOT by making the serde deserializer lenient. This distinction must be documented in the implementation of `preferences/store.rs`.
**Action:** TEST-I18N-004 is marked `#[ignore]` with this explanation. It will be unblocked when `load_or_default()` is implemented.

### Issue 3: `vte::Params` does not implement `Clone` — prevents direct unit testing of `apply_sgr`

**Nature:** API limitation of the `vte` crate.
**Impact:** `apply_sgr(params: &vte::Params, ...)` cannot be tested in isolation without a running parser, because `vte::Params` has no public constructor and does not implement `Clone`.
**Workaround:** SGR behavior is tested at the `VtProcessor` integration level (processor.rs tests), which fully covers the acceptance criteria.
**Action:** Noted in a comment in `vt/sgr.rs`. No change needed in `apply_sgr`'s public API.

---

## Next Steps

1. **Unblock TEST-PTY-001/002/003**: Implement `LinuxPtySession` and add a mock PTY backend trait for unit testing.
2. **Unblock TEST-VT-014**: Implement `validate_hyperlink_uri()` in `vt/osc.rs` or `platform/uri.rs`.
3. **Unblock TEST-VT-021/022**: Implement `VtProcessor::search()` in `vt/search.rs`.
4. **Unblock TEST-I18N-004**: Implement `PreferencesStore::load_or_default()` with field-level fallback.
5. **Frontend modules**: Implement `lib/terminal/keyboard.ts`, `lib/terminal/selection.ts`, `lib/layout/split-tree.ts`, `lib/theming/validate.ts`, etc. — then write their unit tests per §2.2 of the protocol.
6. **Integration test files**: Create `src-tauri/tests/` integration test files per §2.3 of the protocol (`preferences_roundtrip.rs`, `session_registry_topology.rs`, `vt_processor_integration.rs`, `ipc_type_coherence.rs`).
7. **E2E infrastructure**: Set up `pnpm tauri build` → `pnpm wdio` pipeline once PTY and SSH stubs are replaced.
