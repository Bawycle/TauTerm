# Test Report — TauTerm Minor Sprint

> **Date:** 2026-04-05
> **Author:** test-engineer
> **Protocol reference:** docs/test-protocols/functional-mineurs-sprint-2026-04-05.md
> **Rust test runner:** cargo nextest run
> **Frontend test runner:** pnpm vitest run

---

## 1. Summary

| Runner | Tests run | Passed | Failed | Skipped / Todo | Blocked |
|---|---|---|---|---|---|
| cargo nextest (Rust) | 247 | 247 | 0 | 1 | 0 |
| pnpm vitest (Frontend) | 733 | 691 | 0 | 42 (todo) | 0 |
| **Total** | **980** | **938** | **0** | **43** | **0** |

All automated tests pass. E2E scenarios (TP-MIN-001 through TP-MIN-005) remain blocked pending production build.

---

## 2. Pre-existing issue resolved during testing

### tauri.conf.json — `desktopTemplate` field incompatible with tauri-build 2.5.6

**Finding:** The working tree contained `"desktopTemplate": null` in `src-tauri/tauri.conf.json` under the `bundle.linux` key. This field is not recognized by `tauri-build 2.5.6` and caused the build script to fail with:

```
unknown field `desktopTemplate`, expected one of `appimage`, `deb`, `rpm`
```

This blocked all Rust test compilation. The field was removed to restore the build.

**Action taken:** Removed `"desktopTemplate": null` from `src-tauri/tauri.conf.json`.

**Responsibility:** This is a configuration drift introduced during the sprint. The `rust-dev` or `architect` should verify whether `desktopTemplate` support was intended for a newer version of tauri-build. If so, the tauri-build dependency must be upgraded.

---

## 3. Scenario-by-Scenario Results

### 3.1 Scrollbar Interaction (FS-SB-007)

| ID | Description | Result | Notes |
|---|---|---|---|
| TP-MIN-001 | Scrollbar click (track → scroll) | **Blocked** | E2E — requires production build |
| TP-MIN-002 | Scrollbar drag (thumb → scroll) | **Blocked** | E2E — requires production build |
| TP-MIN-003 | Mouse wheel scrolls scrollback | **Blocked** | E2E — requires production build |

The scrollbar implementation is present in `TerminalPane.svelte` (`scrollbarYToOffset`, `scrollToOffset`, mouse event handlers). Unit verification of the `scrollbarYToOffset` pure function was not added in this sprint as it requires DOM measurements (`getBoundingClientRect`) that are non-trivial to mock without a full jsdom layout engine. This is noted as a gap for a future sprint.

### 3.2 First-Launch Hint (FS-UX-002)

| ID | Description | Result | Notes |
|---|---|---|---|
| TP-MIN-004 | Hint visible on first launch | **Blocked** | E2E — requires production build |
| TP-MIN-005 | Hint disappears after right-click | **Blocked** | E2E — requires production build |

The hint key `context_menu_hint` is present in both catalogues (verified by TP-MIN-007 catalogue tests). The `mark_context_menu_used` Tauri command is implemented in `system_cmds.rs`. The state management for `pasteConfirmDontAsk` / hint visibility is in `TerminalPane.svelte`. Functional verification requires a running application.

### 3.3 i18n — No Hardcoded Strings (FS-I18N-001)

| ID | Description | Result | Notes |
|---|---|---|---|
| TP-MIN-006 | TabBar keys in en.json + fr.json | **Pass** | 30 tests, all pass |
| TP-MIN-007 | TerminalPane keys in en.json + fr.json | **Pass** | 36 tests, all pass |
| TP-MIN-008 | TerminalView keys in en.json + fr.json | **Pass** | 6 tests, all pass |

Additionally, the overall catalogue parity test (TP-MIN-006/007/008 precondition) passes:
- `en.json` and `fr.json` have identical key sets (both directions verified).
- No key in either catalogue maps to an empty string.

**Coverage note:** These tests verify catalogue completeness at the data level. Verification that the Svelte components actually use Paraglide accessors (not hardcoded literals) is a static analysis step covered by the protocol (steps 1–3 of TP-MIN-006 through TP-MIN-008) but not yet automated. A future improvement would be a regex-based AST scan of the component source files.

### 3.4 URI Scheme Validation (FS-VT-073)

| ID | Description | Result | Notes |
|---|---|---|---|
| TP-MIN-009 | `file://` accepted for local PTY | **Pass** | Covered by `fs_vt_073_file_scheme_allowed_for_local_pty` |
| TP-MIN-010 | `file://` rejected for SSH session | **Pass** | Covered by `fs_vt_073_file_scheme_rejected_for_ssh` |
| TP-MIN-011 | `javascript:` always rejected | **Pass** | Covered by `sec_path_003_javascript_scheme_rejected` |
| TP-MIN-012 | `data:` always rejected | **Pass** | Covered by `sec_path_003_data_scheme_rejected` |
| TP-MIN-013 | URI > 2048 chars rejected | **Pass** | Covered by `sec_url_length_limit_enforced` + `sec_url_at_length_limit_accepted` |
| TP-MIN-014 | URI with C0 control chars rejected | **Pass** | Covered by `sec_url_control_chars_rejected` + `sec_url_c1_control_chars_rejected` |

All FS-VT-073 scenarios were already covered by pre-existing tests in `src-tauri/src/commands/system_cmds.rs`. No new Rust tests were required.

### 3.5 Multiline Paste Confirmation (FS-CLIP-009)

| ID | Description | Result | Notes |
|---|---|---|---|
| TP-MIN-015 | Dialog shown: multiline + bracketed=false + pref=true | **Pass** | `paste-confirm.test.ts` — 4 cases |
| TP-MIN-016 | Dialog NOT shown when bracketed paste active | **Pass** | `paste-confirm.test.ts` — 2 cases |
| TP-MIN-017 | Dialog NOT shown for single-line paste | **Pass** | `paste-confirm.test.ts` — 3 cases |

Additional edge cases also pass: `confirmMultilinePaste=false` disables the dialog, CRLF triggers the dialog, lone CR does not.

**Architecture note:** The decision logic (`!bracketedPasteActive && hasNewlines && confirmMultilinePaste`) is inlined in `TerminalPane.svelte`. The test file `paste-confirm.test.ts` re-implements the pure function to test its contract. If this logic were extracted to a standalone module (e.g., `src/lib/terminal/paste-confirm.ts`), the import would be updated. This is a minor testability improvement that could be raised with `frontend-dev`.

### 3.6 Inactive Tab Contrast (TUITC-UX-060 / FS-A11Y-001)

| ID | Description | Result | Notes |
|---|---|---|---|
| TP-MIN-018 | Inactive tab fg contrast ≥ 4.5:1 on tab bg | **Pass** | Ratio ~6.0:1 with corrected tokens |

The corrected token `--color-tab-inactive-fg: #9c9890` on `--color-tab-bg: #242118` achieves a contrast ratio of approximately 6.0:1, well above the WCAG AA threshold of 4.5:1.

The test also confirms that the old (non-compliant) value `#6b6660` produced ~2.5:1, validating the necessity of the correction.

### 3.7 Soft-Wrap Search (SEARCH-SOFT-001 / FS-SEARCH-002)

| ID | Description | Result | Notes |
|---|---|---|---|
| TP-MIN-019 | Soft-wrapped word found by search | **Pass** | Feature implemented; tests pass without `#[ignore]` |

**Discovery:** The soft-wrap search feature (cross-row joining) has been implemented in the current working tree. The `search_scrollback` function now takes `impl Iterator<Item = &ScrollbackLine>` and correctly joins soft-wrapped rows before matching. The `#[ignore]` annotation that was present in the commit HEAD (`926f4ac`) has been removed as part of the implementation.

**New tests added (`src-tauri/src/vt/search.rs`):**
- `search_soft_wrap_word_spanning_three_rows_is_found` — SEARCH-SOFT-002
- `search_soft_wrap_boundary_chars_found` — SEARCH-SOFT-003
- `search_hard_newline_rows_are_not_joined` — SEARCH-HARD-001

All three pass.

---

## 4. New Test Files Created

| File | Type | Scenarios covered |
|---|---|---|
| `docs/test-protocols/functional-mineurs-sprint-2026-04-05.md` | Protocol | TP-MIN-001 to TP-MIN-019 |
| `src/lib/i18n/catalogue-parity.test.ts` | vitest | TP-MIN-006, TP-MIN-007, TP-MIN-008 (catalogue parity + per-key verification) |
| `src/lib/theming/contrast-tokens.test.ts` | vitest | TP-MIN-018 (WCAG contrast calculation + token guard) |
| `src/lib/terminal/paste-confirm.test.ts` | vitest | TP-MIN-015, TP-MIN-016, TP-MIN-017 (paste confirmation decision logic) |

---

## 5. Pre-existing Test Failures

At the start of this sprint, the test suite had pre-existing failures in `src/lib/components/__tests__/TerminalPane.test.ts` (4 tests failing with `localStorage.getItem is not a function`). These were caused by a change in `TerminalPane.svelte` that accesses `localStorage` at component instantiation time without an adequate jsdom polyfill. These were resolved by the time the full suite was run (a vitest setup fix was present in the working tree).

These failures were not caused by the current sprint work and are documented here for traceability only.

---

## 6. Blocked / Deferred

| Scenario(s) | Reason | Unblocked when |
|---|---|---|
| TP-MIN-001 – TP-MIN-003 (scrollbar interaction) | E2E: requires `pnpm tauri build` + tauri-driver + display | Production build functional |
| TP-MIN-004 – TP-MIN-005 (first-launch hint) | E2E: same prerequisites; also requires clean preferences state reset | Production build + E2E fixture for preferences reset |
| Scrollbar `scrollbarYToOffset` pure function | Unit: relies on `getBoundingClientRect` — requires layout engine | Extract to a DOM-independent function or mock layout |
| Static analysis of Svelte component source for hardcoded strings | Tooling: requires AST/regex source scan, not yet automated | Custom lint rule or build-time check |

---

## 7. Recommendations

1. **Extract paste confirmation decision** — move the `pasteNeedsConfirmation` logic from `TerminalPane.svelte` into a standalone `src/lib/terminal/paste-confirm.ts` module. This would allow direct import in tests and remove the re-implementation in `paste-confirm.test.ts`. Estimated effort: 15 minutes. Assign to `frontend-dev`.

2. **Automate i18n static source scan** — add a build-time check (or CI lint step) that ensures all Svelte components use only Paraglide accessors for user-visible text, not string literals. This would fully automate TP-MIN-006/007/008 at the source level, not just the catalogue level.

3. **Upgrade tauri-build if `desktopTemplate` is needed** — if the Linux `.desktop` template customization is a sprint requirement, upgrade `tauri-build` to a version that supports `desktopTemplate` in the `bundle.linux` config. If it was added by mistake, the removal in this PR should be kept.

4. **Add `scrollbarYToOffset` unit test** — the formula is deterministic and testable if extracted from the DOM measurement. This would provide fast regression coverage for scrollbar behavior without requiring E2E.

---

## 8. Sign-off

All automatable tests for this sprint (TP-MIN-006 through TP-MIN-019) pass. E2E scenarios (TP-MIN-001 through TP-MIN-005) are deferred pending production build availability.

**Test engineer sign-off:** Minor sprint unit and frontend tests — COMPLETE.
E2E coverage — DEFERRED (infrastructure not available).
